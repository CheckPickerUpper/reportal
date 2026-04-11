//! Checks whether the shell integration script file matches the running
//! binary version and silently rewrites it on mismatch. Called at the
//! top of `main()` before subcommand dispatch so that binary updates via
//! cargo, scoop, or brew take effect on the next shell session without
//! any manual steps.

use crate::reportal_config::ReportalConfig;

use super::detected_shell::{integration_file_path, DetectedShell};

/// Whether the integration file version matches, is outdated, or is missing.
pub(crate) enum IntegrationFileState {
    /// File exists and version matches the running binary.
    Current,
    /// File exists but was written by a different version.
    Outdated { file_version: String },
    /// File does not exist at the expected path.
    Missing,
}

/// Reads the integration script file and compares its version stamp
/// against the running binary version.
pub(crate) fn check_integration_file_state() -> IntegrationFileState {
    let script_path = match integration_file_path() {
        Ok(path) => path,
        Err(_path_error) => return IntegrationFileState::Missing,
    };

    let file_content = match std::fs::read_to_string(&script_path) {
        Ok(script_content) => script_content,
        Err(_read_error) => return IntegrationFileState::Missing,
    };

    let binary_version = env!("CARGO_PKG_VERSION");

    let Some(first_line) = file_content.lines().next() else {
        return IntegrationFileState::Outdated {
            file_version: String::from("empty"),
        };
    };

    if first_line.contains(binary_version) { IntegrationFileState::Current } else {
        let extracted_version = first_line
            .rsplit("— v")
            .next().map_or_else(|| String::from("unknown"), String::from);
        IntegrationFileState::Outdated {
            file_version: extracted_version,
        }
    }
}

/// Rewrites the integration script file if it is missing or was written
/// by a different binary version. Silently skipped if the config
/// directory does not exist (pre-init).
pub fn ensure_integration_file_current() {
    let config_directory = match ReportalConfig::config_directory() {
        Ok(directory) => directory,
        Err(_path_error) => return,
    };

    if !config_directory.exists() { return }

    match check_integration_file_state() {
        IntegrationFileState::Current => {}
        IntegrationFileState::Outdated { .. } | IntegrationFileState::Missing => {
            let script_path = match integration_file_path() {
                Ok(path) => path,
                Err(_path_error) => return,
            };

            #[cfg(target_os = "windows")]
            let integration_script = DetectedShell::powershell_integration_content();

            #[cfg(not(target_os = "windows"))]
            let integration_script = DetectedShell::bash_integration_content();

            let _write_result = std::fs::write(&script_path, &integration_script);
        }
    }
}
