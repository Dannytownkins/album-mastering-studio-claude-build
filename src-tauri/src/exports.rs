use crate::types::*;

#[tauri::command]
pub async fn run_export_checks(
    report: ExportReport,
    source_analysis: Option<AnalysisResult>,
    settings: Option<MasteringSettings>,
) -> CommandResult<Vec<QualityCheck>> {
    let mut checks = Vec::new();

    if report.measured_true_peak_dbtp > -0.1 {
        checks.push(QualityCheck {
            level: QualityLevel::Warning,
            code: "true_peak_high".to_string(),
            message: format!(
                "True peak is {:.2} dBTP. Some delivery platforms reject masters above -1.0 dBTP.",
                report.measured_true_peak_dbtp
            ),
        });
    } else if report.measured_true_peak_dbtp > -1.0 {
        // Streaming-headroom advisory for the narrow gray zone between the
        // critical -0.1 dBTP threshold above and the typical -1.0 dBTP
        // streaming ceiling below. Lossy codecs (AAC, MP3, Opus) can boost
        // decoded peaks by up to ~1 dB relative to the source true peak due
        // to quantization noise added inside the codec's spectral bands, so a
        // master at e.g. -0.5 dBTP can clip after AAC encoding on dense
        // pop/rock material. This is a headroom-based advisory, NOT an actual
        // codec simulation; a real Phase 6.x-bis could add an encode/decode
        // round-trip if codec QC ever needs to be more precise.
        checks.push(QualityCheck {
            level: QualityLevel::Warning,
            code: "streaming_headroom_low".to_string(),
            message: format!(
                "True peak is {:.2} dBTP. Within safe digital range, but lossy delivery (AAC, MP3, Opus) can overshoot by up to ~1 dB after encoding. Consider lowering the ceiling for comfortable streaming masters.",
                report.measured_true_peak_dbtp
            ),
        });
    }

    if report.measured_lufs > -8.0 {
        checks.push(QualityCheck {
            level: QualityLevel::Warning,
            code: "lufs_very_loud".to_string(),
            message: format!(
                "Integrated loudness is {:.1} LUFS. This is louder than typical streaming targets and may sound flat.",
                report.measured_lufs
            ),
        });
    }

    if report.measured_dynamic_range_lu < 5.0 {
        checks.push(QualityCheck {
            level: QualityLevel::Warning,
            code: "dynamic_range_low".to_string(),
            message: format!(
                "Dynamic range is {:.1} LU. Highly compressed material; verify by ear before exporting.",
                report.measured_dynamic_range_lu
            ),
        });
    }

    if report.bit_depth < 16 {
        checks.push(QualityCheck {
            level: QualityLevel::Critical,
            code: "bit_depth_low".to_string(),
            message: format!("Bit depth {} is below 16. Not suitable for delivery.", report.bit_depth),
        });
    }

    if !report.measured_lufs.is_finite() {
        checks.push(QualityCheck {
            level: QualityLevel::Critical,
            code: "non_finite_metering".to_string(),
            message: "LUFS measurement is not finite. Re-analyze before exporting.".to_string(),
        });
    }

    // Phase 12.2 — already-compressed source advisory. Fires when the SOURCE
    // material is dynamically squashed (DR < 6 LU) AND the user is asking for
    // moderate-to-heavy compression density (> 0.3) AND they haven't manually
    // overridden any per-band threshold (per-band overrides imply the user
    // knows what they're doing and the macro isn't blindly driving). Advisory
    // only — does not block export.
    if let (Some(analysis), Some(s)) = (source_analysis.as_ref(), settings.as_ref()) {
        let density = s.advanced.compression_density.unwrap_or(0.0);
        let no_per_band_threshold_overrides = s.advanced.compression_low_threshold_db.is_none()
            && s.advanced.compression_mid_threshold_db.is_none()
            && s.advanced.compression_high_threshold_db.is_none();
        if analysis.dynamic_range_lu < 6.0
            && density > 0.3
            && no_per_band_threshold_overrides
        {
            checks.push(QualityCheck {
                level: QualityLevel::Warning,
                code: "comp_density_on_compressed_source".to_string(),
                message: "Source appears already compressed (DR < 6 LU). Heavy compression may pump.".to_string(),
            });
        }
    }

    if checks.is_empty() {
        checks.push(QualityCheck {
            level: QualityLevel::Info,
            code: "export_ok".to_string(),
            message: "No issues detected in measured values.".to_string(),
        });
    }

    Ok(checks)
}

#[tauri::command]
pub async fn open_output(output_path: String) -> CommandResult<()> {
    if output_path.is_empty() {
        return Err(CommandError::InvalidPath("empty path".to_string()));
    }
    let path = std::path::Path::new(&output_path);
    if crate::files::has_parent_dir_component(path) {
        return Err(CommandError::InvalidPath(format!(
            "path traversal not allowed: {output_path}"
        )));
    }
    if !path.exists() {
        return Err(CommandError::Io(format!(
            "path does not exist: {output_path}"
        )));
    }

    #[cfg(target_os = "windows")]
    {
        // /select, opens Explorer at the parent folder with the file highlighted.
        std::process::Command::new("explorer")
            .arg("/select,")
            .arg(&output_path)
            .spawn()
            .map_err(|e| CommandError::Io(format!("failed to open Explorer: {e}")))?;
        return Ok(());
    }
    #[cfg(target_os = "macos")]
    {
        std::process::Command::new("open")
            .arg("-R")
            .arg(&output_path)
            .spawn()
            .map_err(|e| CommandError::Io(format!("failed to open Finder: {e}")))?;
        return Ok(());
    }
    #[cfg(all(not(target_os = "windows"), not(target_os = "macos")))]
    {
        let parent = path.parent().unwrap_or(path);
        std::process::Command::new("xdg-open")
            .arg(parent)
            .spawn()
            .map_err(|e| CommandError::Io(format!("failed to open file manager: {e}")))?;
        Ok(())
    }
}
