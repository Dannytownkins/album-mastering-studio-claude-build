pub mod audio;
pub mod engine;
pub mod exports;
pub mod files;
pub mod jobs;
pub mod project;
pub mod settings;
pub mod types;

pub use types::*;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .invoke_handler(tauri::generate_handler![
            files::import_tracks,
            engine::analyze_tracks,
            engine::render_track_preview,
            engine::render_track_master,
            engine::render_album_master,
            audio::prepare_source_playback,
            audio::prepare_master_playback,
            audio::prepare_ab_preview,
            audio::prepare_waveform,
            exports::run_export_checks,
            exports::open_output,
            project::save_project,
            project::autosave_session,
            project::load_recent_session,
            settings::save_user_preset,
            settings::list_user_presets,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
