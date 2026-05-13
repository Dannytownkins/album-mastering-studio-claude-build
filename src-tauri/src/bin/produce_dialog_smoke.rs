// One-shot binary that materializes a representative .ams.json project
// file at the Claude-build mirror of Codex's
// `test-output/tauri-project-dialogs-smoke/native-dialog-save-as.ams.json`
// path. Lets us hand a real artifact to anyone comparing project-file
// shapes across the two parallel builds, and exercises the
// `project::write_session_atomic` path end-to-end without spawning a
// Tauri window or a native dialog.

use std::collections::HashMap;
use std::path::Path;

use album_mastering_studio_lib::*;

fn main() {
    let track_id = TrackId("01-dialog-fixture".to_string());
    let track2_id = TrackId("02-dialog-fixture".to_string());

    let track1 = ImportedTrack {
        id: track_id.clone(),
        path: "test-output/tauri-project-dialogs-smoke/inputs/01_dialog_fixture.wav".to_string(),
        display_name: "01 Dialog Fixture".to_string(),
        source_format: "wav".to_string(),
        duration_seconds: Some(3.0),
        sample_rate: Some(44_100),
        channels: Some(2),
    };
    let track2 = ImportedTrack {
        id: track2_id.clone(),
        path: "test-output/tauri-project-dialogs-smoke/inputs/02_dialog_fixture.wav".to_string(),
        display_name: "02 Dialog Fixture".to_string(),
        source_format: "wav".to_string(),
        duration_seconds: Some(3.0),
        sample_rate: Some(44_100),
        channels: Some(2),
    };

    let default_settings = MasteringSettings {
        preset: Preset::Universal,
        intensity: 0.5,
        eq_low_db: 0.0,
        eq_low_mid_db: 0.0,
        eq_mid_db: 0.0,
        eq_high_db: 0.0,
        volume_match: false,
        input_gain_db: 0.0,
        output_gain_db: 0.0,
        delivery_profile: DeliveryProfile::default(),
        album: None,
        advanced: AdvancedSettings::default(),
    };

    let mut track_settings = HashMap::new();
    track_settings.insert(track_id.0.clone(), default_settings.clone());
    track_settings.insert(track2_id.0.clone(), default_settings.clone());

    let state = ProjectState {
        schema_version: 1,
        mode: ProjectMode::Track,
        tracks: vec![track1, track2],
        track_order: vec![track_id.clone(), track2_id.clone()],
        track_settings,
        album_intent: None,
        track_override_album: Vec::new(),
        last_saved_iso: Some("2026-05-13T00:00:00Z".to_string()),
    };

    let out_path = Path::new(
        "../test-output/tauri-project-dialogs-smoke/native-dialog-save-as.ams.json",
    );
    if let Some(parent) = out_path.parent() {
        std::fs::create_dir_all(parent).expect("create output dir");
    }
    project::write_session_atomic(out_path, &state).expect("write session");
    println!(
        "wrote {} bytes to {}",
        std::fs::metadata(out_path).map(|m| m.len()).unwrap_or(0),
        out_path.display()
    );
}
