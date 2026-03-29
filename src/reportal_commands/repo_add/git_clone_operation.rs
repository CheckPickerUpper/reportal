//! Git clone execution for remote repos.

use crate::error::ReportalError;
use crate::terminal_style;
use owo_colors::OwoColorize;
use std::path::Path;
use std::process::Command;

/// Parameters for cloning a git repo into a directory.
pub struct GitCloneOperation<'a> {
    /// The git URL to clone from.
    pub(super) git_url: &'a str,
    /// The directory to clone into.
    pub(super) target_directory: &'a Path,
}

/// Spawns `git clone <url>` in the target directory.
impl<'a> GitCloneOperation<'a> {
    /// Spawns `git clone` as a child process, waits for completion,
    /// and returns an error if git is unavailable or the clone fails.
    pub fn run_git_clone(&self) -> Result<(), ReportalError> {
        println!(
            "  {} {}",
            "Cloning:".style(terminal_style::LABEL_STYLE),
            self.git_url.style(terminal_style::PATH_STYLE),
        );

        let clone_result = Command::new("git")
            .args(["clone", self.git_url])
            .current_dir(self.target_directory)
            .status();

        match clone_result {
            Ok(exit_status) => match exit_status.success() {
                true => Ok(()),
                false => Err(ReportalError::ConfigIoFailure {
                    reason: format!("git clone exited with status {}", exit_status),
                }),
            },
            Err(git_spawn_error) => Err(ReportalError::ConfigIoFailure {
                reason: format!("Failed to run git: {}", git_spawn_error),
            }),
        }
    }
}
