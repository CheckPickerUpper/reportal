//! Detects git remote URLs from local repo directories.

use std::process::Command;
use crate::terminal_style;

/// Whether a git remote was found in the target directory.
pub enum GitRemoteDetection {
    /// A remote URL was successfully read from git.
    Found(String),
    /// The directory has no configured origin remote.
    NoOriginConfigured,
    /// Git command failed to execute (git not installed or not a repo).
    GitUnavailable,
}

/// Detects the git remote URL by running `git remote get-url origin` in the directory.
pub fn detect_git_remote(directory_path: &str) -> GitRemoteDetection {
    let expanded_directory = shellexpand::tilde(directory_path);
    let detection_result = Command::new("git")
        .args(["remote", "get-url", "origin"])
        .current_dir(expanded_directory.as_ref())
        .output();

    match detection_result {
        Ok(command_output) => if command_output.status.success() {
            let remote_url = String::from_utf8_lossy(&command_output.stdout).trim().to_owned();
            if remote_url.is_empty() { GitRemoteDetection::NoOriginConfigured } else { GitRemoteDetection::Found(remote_url) }
        } else { GitRemoteDetection::NoOriginConfigured },
        Err(git_spawn_error) => {
            terminal_style::write_stderr(&format!("  git not available: {git_spawn_error}\n"));
            GitRemoteDetection::GitUnavailable
        }
    }
}
