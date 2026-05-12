use crate::types::*;

#[tauri::command]
pub async fn run_export_checks(report: ExportReport) -> CommandResult<Vec<QualityCheck>> {
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
