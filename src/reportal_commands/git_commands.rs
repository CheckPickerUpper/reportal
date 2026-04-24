//! Shared git command execution used by status, sync, and web.

use std::path::PathBuf;
use std::process::Command;

/// Whether a git command produced usable output or failed.
pub enum GitCommandOutcome {
    /// The command ran and produced trimmed stdout output.
    Output(String),
    /// The command failed to run (git not installed, not a repo, etc).
    SpawnFailed,
    /// The command ran but returned a non-zero exit code.
    NonZeroExit,
}

/// Parameters for running a git subcommand inside a specific repo directory.
/// Passed to `run_git_command()` as its single argument.
pub struct GitCommandParameters<'a> {
    /// The resolved filesystem path to run git in.
    pub repo_path: &'a PathBuf,
    /// The git subcommand and its arguments (e.g. `["status", "--porcelain"]`).
    pub git_subcommand_args: &'a [&'a str],
}

/// Spawns `git <args>` in the given repo directory and captures stdout.
///
/// Returns `Output(trimmed_stdout)` on zero exit, [`NonZeroExit`] on
/// non-zero exit, or `SpawnFailed` if the git binary couldn't be found.
/// Used by status, sync, and web to avoid duplicating the spawn+capture logic.
pub fn run_git_command(git_command_params: &GitCommandParameters<'_>) -> GitCommandOutcome {
    let command_result = Command::new("git")
        .args(git_command_params.git_subcommand_args)
        .current_dir(git_command_params.repo_path)
        .output();

    match command_result {
        Ok(output) => if output.status.success() { GitCommandOutcome::Output(
            String::from_utf8_lossy(&output.stdout).trim().to_owned(),
        ) } else { GitCommandOutcome::NonZeroExit },
        Err(_git_spawn_error) => GitCommandOutcome::SpawnFailed,
    }
}
