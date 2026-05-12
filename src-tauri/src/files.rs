use crate::types::*;
use std::path::{Component, Path};

use symphonia::core::formats::FormatOptions;
use symphonia::core::io::MediaSourceStream;
use symphonia::core::meta::MetadataOptions;
use symphonia::core::probe::Hint;

#[tauri::command]
pub async fn import_tracks(paths: Vec<String>) -> CommandResult<Vec<ImportedTrack>> {
    paths.into_iter().map(|p| import_one(&p)).collect()
}

pub(crate) fn has_parent_dir_component(path: &Path) -> bool {
    path.components().any(|c| matches!(c, Component::ParentDir))
}

fn import_one(path_str: &str) -> CommandResult<ImportedTrack> {
    if path_str.is_empty() {
        return Err(CommandError::InvalidPath("empty path".to_string()));
    }
    let path = Path::new(path_str);
    if has_parent_dir_component(path) {
        return Err(CommandError::InvalidPath(format!(
            "path traversal not allowed: {path_str}"
        )));
    }
    let display_name = path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("Untitled")
        .to_string();
    let source_format = path
        .extension()
        .and_then(|s| s.to_str())
        .map(|s| s.to_lowercase())
        .unwrap_or_else(|| "unknown".to_string());

    let metadata = probe_metadata(path).unwrap_or_default();

    Ok(ImportedTrack {
        id: TrackId::new(),
        path: path_str.to_string(),
        display_name,
        source_format,
        duration_seconds: metadata.duration_seconds,
        sample_rate: metadata.sample_rate,
        channels: metadata.channels,
    })
}

#[derive(Default)]
struct TrackMetadata {
    duration_seconds: Option<f64>,
    sample_rate: Option<u32>,
    channels: Option<u16>,
}

fn probe_metadata(path: &Path) -> CommandResult<TrackMetadata> {
    let file = std::fs::File::open(path).map_err(|e| CommandError::Io(e.to_string()))?;
    let mss = MediaSourceStream::new(Box::new(file), Default::default());
    let mut hint = Hint::new();
    if let Some(ext) = path.extension().and_then(|s| s.to_str()) {
        hint.with_extension(ext);
    }
    let probed = symphonia::default::get_probe()
        .format(&hint, mss, &FormatOptions::default(), &MetadataOptions::default())
        .map_err(|e| CommandError::Decode(e.to_string()))?;
    let format = probed.format;
    let track = format
        .default_track()
        .ok_or_else(|| CommandError::Decode("no default track".to_string()))?;
    let params = &track.codec_params;
    let sample_rate = params.sample_rate;
    let channels = params.channels.map(|c| c.count() as u16);
    let duration = match (params.n_frames, params.sample_rate) {
        (Some(frames), Some(sr)) if sr > 0 => Some(frames as f64 / sr as f64),
        _ => None,
    };
    Ok(TrackMetadata {
        duration_seconds: duration,
        sample_rate,
        channels,
    })
}
