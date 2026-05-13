pub mod album;
pub mod audio;
pub mod dsp;
pub mod engine;
pub mod exports;
pub mod files;
pub mod jobs;
pub mod project;
pub mod settings;
pub mod types;

pub use types::*;

use std::sync::Arc;
use std::time::Duration;

use tauri::{Emitter, Manager};

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let player = Arc::new(audio::AudioPlayer::new());

    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .manage(player)
        .setup(|app| {
            let app_handle = app.handle().clone();
            let player_state = app.state::<Arc<audio::AudioPlayer>>().inner().clone();
            std::thread::spawn(move || {
                loop {
                    std::thread::sleep(Duration::from_millis(50));
                    let snap = player_state.snapshot();
                    if !snap.is_loaded {
                        continue;
                    }
                    let tick = PlaybackTick {
                        track_id: snap.track_id,
                        position_sec: snap.position_sec,
                        is_playing: snap.is_playing,
                        is_loaded: snap.is_loaded,
                        peak_dbfs: snap.peak_dbfs,
                        gr_low_db: snap.gr_low_db,
                        gr_mid_db: snap.gr_mid_db,
                        gr_high_db: snap.gr_high_db,
                        lufs_momentary: snap.lufs_momentary,
                        lufs_integrated: snap.lufs_integrated,
                    };
                    let _ = app_handle.emit("playback:tick", tick);
                }
            });
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            files::import_tracks,
            engine::analyze_tracks,
            engine::render_track_preview,
            engine::render_track_master,
            engine::render_album_master,
            engine::plan_album,
            engine::render_album_plan,
            audio::prepare_source_playback,
            audio::prepare_master_playback,
            audio::prepare_ab_preview,
            audio::prepare_waveform,
            audio::play_track,
            audio::play_master,
            audio::update_chain,
            audio::pause_playback,
            audio::resume_playback,
            audio::stop_playback,
            audio::seek_playback,
            audio::set_loop_region,
            exports::run_export_checks,
            exports::open_output,
            project::save_project,
            project::autosave_session,
            project::load_recent_session,
            project::load_project,
            settings::save_user_preset,
            settings::list_user_presets,
            settings::delete_user_preset,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
