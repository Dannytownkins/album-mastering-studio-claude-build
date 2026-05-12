use crate::types::*;
use std::path::Path;

use symphonia::core::audio::{AudioBufferRef, SampleBuffer};
use symphonia::core::codecs::{DecoderOptions, CODEC_TYPE_NULL};
use symphonia::core::errors::Error as SymphoniaError;
use symphonia::core::formats::FormatOptions;
use symphonia::core::io::MediaSourceStream;
use symphonia::core::meta::MetadataOptions;
use symphonia::core::probe::Hint;

const DEFAULT_TARGET_PIXELS: u32 = 1000;
const MIN_TARGET_PIXELS: u32 = 64;

#[tauri::command]
pub async fn prepare_source_playback(
    track_id: TrackId,
    track_path: String,
) -> CommandResult<PlaybackHandle> {
    let _ = track_path;
    Ok(handle(track_id, PlaybackKind::Source))
}

#[tauri::command]
pub async fn prepare_master_playback(
    track_id: TrackId,
    track_path: String,
    settings: MasteringSettings,
) -> CommandResult<PlaybackHandle> {
    let _ = (track_path, settings);
    Ok(handle(track_id, PlaybackKind::Master))
}

#[tauri::command]
pub async fn prepare_ab_preview(
    track_id: TrackId,
    track_path: String,
    settings: MasteringSettings,
    volume_match: bool,
) -> CommandResult<AbPreview> {
    let _ = (track_path, settings);
    let source_handle = handle(track_id.clone(), PlaybackKind::Source);
    let master_handle = handle(track_id.clone(), PlaybackKind::Master);
    Ok(AbPreview {
        track_id,
        source_handle,
        master_handle,
        volume_match_offset_db: if volume_match { -2.4 } else { 0.0 },
    })
}

#[tauri::command]
pub async fn prepare_waveform(
    track_id: TrackId,
    track_path: String,
    target_pixels: Option<u32>,
) -> CommandResult<WaveformPeaks> {
    if track_path.is_empty() {
        return Err(CommandError::InvalidPath("empty path".to_string()));
    }
    let path = Path::new(&track_path);
    if crate::files::has_parent_dir_component(path) {
        return Err(CommandError::InvalidPath(format!(
            "path traversal not allowed: {track_path}"
        )));
    }
    let pixels = target_pixels
        .unwrap_or(DEFAULT_TARGET_PIXELS)
        .max(MIN_TARGET_PIXELS);
    let decoded = decode_to_peaks(path, pixels)?;
    Ok(WaveformPeaks {
        track_id,
        channels: decoded.channels,
        samples_per_pixel: decoded.samples_per_pixel,
        total_samples: decoded.total_samples,
        sample_rate: decoded.sample_rate,
    })
}

fn handle(track_id: TrackId, kind: PlaybackKind) -> PlaybackHandle {
    PlaybackHandle {
        id: uuid::Uuid::new_v4().to_string(),
        track_id,
        kind,
        duration_seconds: 180.0,
    }
}

pub struct DecodedPeaks {
    pub channels: Vec<Vec<f32>>,
    pub samples_per_pixel: u32,
    pub total_samples: u64,
    pub sample_rate: u32,
}

pub fn decode_to_peaks(path: &Path, target_pixels: u32) -> CommandResult<DecodedPeaks> {
    let file = std::fs::File::open(path).map_err(|e| CommandError::Io(e.to_string()))?;
    let mss = MediaSourceStream::new(Box::new(file), Default::default());
    let mut hint = Hint::new();
    if let Some(ext) = path.extension().and_then(|s| s.to_str()) {
        hint.with_extension(ext);
    }
    let probed = symphonia::default::get_probe()
        .format(&hint, mss, &FormatOptions::default(), &MetadataOptions::default())
        .map_err(|e| CommandError::Decode(e.to_string()))?;
    let mut format = probed.format;

    let track = format
        .tracks()
        .iter()
        .find(|t| t.codec_params.codec != CODEC_TYPE_NULL)
        .ok_or_else(|| CommandError::Decode("no decodable track".to_string()))?;
    let stream_track_id = track.id;
    let sample_rate = track.codec_params.sample_rate.unwrap_or(44_100);
    let channel_count = track
        .codec_params
        .channels
        .map(|c| c.count())
        .unwrap_or(2)
        .max(1);
    let total_frames = track.codec_params.n_frames.unwrap_or(0);

    let mut decoder = symphonia::default::get_codecs()
        .make(&track.codec_params, &DecoderOptions::default())
        .map_err(|e| CommandError::Decode(e.to_string()))?;

    let samples_per_pixel = if total_frames > 0 {
        ((total_frames as f64 / target_pixels as f64).ceil() as u32).max(1)
    } else {
        (sample_rate / 50).max(1)
    };

    let mut channel_peaks: Vec<Vec<f32>> =
        vec![Vec::with_capacity(target_pixels as usize); channel_count];
    let mut running_max: Vec<f32> = vec![0.0; channel_count];
    let mut window_frames: u64 = 0;
    let mut total_decoded_frames: u64 = 0;
    let mut sample_buf: Option<SampleBuffer<f32>> = None;

    loop {
        let packet = match format.next_packet() {
            Ok(p) => p,
            Err(SymphoniaError::IoError(e))
                if e.kind() == std::io::ErrorKind::UnexpectedEof =>
            {
                break;
            }
            Err(SymphoniaError::ResetRequired) => break,
            Err(e) => return Err(CommandError::Decode(e.to_string())),
        };
        if packet.track_id() != stream_track_id {
            continue;
        }
        let decoded: AudioBufferRef = match decoder.decode(&packet) {
            Ok(d) => d,
            Err(SymphoniaError::IoError(_)) => continue,
            Err(SymphoniaError::DecodeError(_)) => continue,
            Err(e) => return Err(CommandError::Decode(e.to_string())),
        };
        if sample_buf.is_none() {
            let spec = *decoded.spec();
            let duration = decoded.capacity() as u64;
            sample_buf = Some(SampleBuffer::<f32>::new(duration, spec));
        }
        let sbuf = sample_buf.as_mut().unwrap();
        sbuf.copy_interleaved_ref(decoded);
        let samples = sbuf.samples();
        let frames = samples.len() / channel_count.max(1);
        total_decoded_frames += frames as u64;

        for frame in 0..frames {
            for ch in 0..channel_count {
                let v = samples[frame * channel_count + ch].abs();
                if v > running_max[ch] {
                    running_max[ch] = v;
                }
            }
            window_frames += 1;
            if window_frames >= u64::from(samples_per_pixel) {
                for ch in 0..channel_count {
                    channel_peaks[ch].push(running_max[ch]);
                    running_max[ch] = 0.0;
                }
                window_frames = 0;
            }
        }
    }

    if window_frames > 0 {
        for ch in 0..channel_count {
            channel_peaks[ch].push(running_max[ch]);
        }
    }

    Ok(DecodedPeaks {
        channels: channel_peaks,
        samples_per_pixel,
        total_samples: total_decoded_frames,
        sample_rate,
    })
}
