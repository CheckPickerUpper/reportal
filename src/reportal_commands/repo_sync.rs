//! Pulls latest changes across all registered repos.

use crate::error::ReportalError;
use crate::reportal_config::{ReportalConfig, TagFilter};
use crate::terminal_style;
use owo_colors::OwoColorize;
use std::path::PathBuf;
use std::process::Command;

/// Whether a git command produced usable output or failed.
enum GitCommandOutcome {
    /// The command ran and produced stdout output.
    Output(String),
    /// The command failed to run (git not installed, not a repo, etc).
    SpawnFailed,
    /// The command ran but returned a non-zero exit code.
    NonZeroExit,
}

/// Whether a repo's working tree is clean enough to pull.
enum PullReadiness {
    /// The working tree is clean; safe to pull.
    Ready,
    /// The working tree has uncommitted changes; skip this repo.
    DirtyWorkingTree,
    /// Could not determine status (git unavailable, etc).
    Unknown,
}

/// What happened when we tried to pull a repo.
enum SyncOutcome {
    /// Pull succeeded with this output summary.
    Pulled(String),
    /// Skipped because the working tree was dirty.
    SkippedDirty,
    /// The repo directory does not exist on disk.
    Missing,
    /// Pull failed with this error message.
    PullFailed(String),
}

/// The result of syncing a single repo.
struct RepoSyncResult {
    /// The alias of the repo.
    alias: String,
    /// What happened during the sync attempt.
    outcome: SyncOutcome,
}

/// Parameters for running a git command inside a repo directory.
struct GitCommandSpec<'a> {
    /// The resolved filesystem path to run git in.
    repo_path: &'a PathBuf,
    /// The git subcommand and its arguments.
    git_subcommand_args: &'a [&'a str],
}

/// Runs a git command in a directory and returns the trimmed stdout.
fn run_git_command(git_command_spec: GitCommandSpec<'_>) -> GitCommandOutcome {
    let command_result = Command::new("git")
        .args(git_command_spec.git_subcommand_args)
        .current_dir(git_command_spec.repo_path)
        .output();

    match command_result {
        Ok(output) => match output.status.success() {
            true => GitCommandOutcome::Output(
                String::from_utf8_lossy(&output.stdout).trim().to_string(),
            ),
            false => GitCommandOutcome::NonZeroExit,
        },
        Err(_git_spawn_error) => GitCommandOutcome::SpawnFailed,
    }
}

/// Checks whether the working tree is clean enough to safely pull.
fn check_pull_readiness(repo_path: &PathBuf) -> PullReadiness {
    match run_git_command(GitCommandSpec {
        repo_path,
        git_subcommand_args: &["status", "--porcelain"],
    }) {
        GitCommandOutcome::Output(output) => match output.is_empty() {
            true => PullReadiness::Ready,
            false => PullReadiness::DirtyWorkingTree,
        },
        GitCommandOutcome::NonZeroExit => PullReadiness::Unknown,
        GitCommandOutcome::SpawnFailed => PullReadiness::Unknown,
    }
}

/// Runs `git pull` in the repo directory and returns the outcome.
fn pull_repo(repo_path: &PathBuf) -> SyncOutcome {
    let pull_result = Command::new("git")
        .args(["pull"])
        .current_dir(repo_path)
        .output();

    match pull_result {
        Ok(output) => match output.status.success() {
            true => {
                let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
                SyncOutcome::Pulled(stdout)
            }
            false => {
                let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
                SyncOutcome::PullFailed(stderr)
            }
        },
        Err(spawn_error) => SyncOutcome::PullFailed(spawn_error.to_string()),
    }
}

/// Syncs a single repo: checks if clean, then pulls.
fn sync_single_repo(alias: &str, repo_path: &PathBuf) -> RepoSyncResult {
    match repo_path.exists() {
        false => RepoSyncResult {
            alias: alias.to_string(),
            outcome: SyncOutcome::Missing,
        },
        true => {
            match check_pull_readiness(repo_path) {
                PullReadiness::Ready => {
                    let outcome = pull_repo(repo_path);
                    RepoSyncResult {
                        alias: alias.to_string(),
                        outcome,
                    }
                }
                PullReadiness::DirtyWorkingTree => RepoSyncResult {
                    alias: alias.to_string(),
                    outcome: SyncOutcome::SkippedDirty,
                },
                PullReadiness::Unknown => RepoSyncResult {
                    alias: alias.to_string(),
                    outcome: SyncOutcome::PullFailed("could not determine working tree state".to_string()),
                },
            }
        }
    }
}

/// Pulls latest changes for every repo matching the tag filter.
/// Skips repos with uncommitted changes (with a warning).
/// Prints a per-repo summary showing what happened.
pub fn run_sync(tag_filter: TagFilter) -> Result<(), ReportalError> {
    let loaded_config = ReportalConfig::load_from_disk()?;
    let matching_repos = loaded_config.repos_matching_tag_filter(&tag_filter);

    if matching_repos.is_empty() {
        return Err(ReportalError::NoReposMatchFilter);
    }

    let mut pulled_count: usize = 0;
    let mut skipped_count: usize = 0;
    let mut failed_count: usize = 0;

    for (alias, repo) in &matching_repos {
        let resolved = repo.resolved_path();
        let result = sync_single_repo(alias, &resolved);

        match &result.outcome {
            SyncOutcome::Pulled(output) => {
                pulled_count += 1;
                match output.contains("Already up to date") {
                    true => {
                        println!(
                            "  {} {} up to date",
                            result.alias.style(terminal_style::ALIAS_STYLE),
                            "✓".style(terminal_style::SUCCESS_STYLE),
                        );
                    }
                    false => {
                        println!(
                            "  {} {} pulled",
                            result.alias.style(terminal_style::ALIAS_STYLE),
                            "↓".style(terminal_style::SUCCESS_STYLE),
                        );
                    }
                }
            }
            SyncOutcome::SkippedDirty => {
                skipped_count += 1;
                eprintln!(
                    "  {} {} skipped (dirty working tree)",
                    result.alias.style(terminal_style::ALIAS_STYLE),
                    "!".style(terminal_style::FAILURE_STYLE),
                );
            }
            SyncOutcome::Missing => {
                failed_count += 1;
                eprintln!(
                    "  {} {} missing on disk",
                    result.alias.style(terminal_style::ALIAS_STYLE),
                    "✗".style(terminal_style::FAILURE_STYLE),
                );
            }
            SyncOutcome::PullFailed(error_message) => {
                failed_count += 1;
                eprintln!(
                    "  {} {} pull failed: {}",
                    result.alias.style(terminal_style::ALIAS_STYLE),
                    "✗".style(terminal_style::FAILURE_STYLE),
                    error_message,
                );
            }
        }
    }

    println!();
    println!(
        "  {} pulled, {} skipped, {} failed",
        pulled_count.to_string().style(terminal_style::SUCCESS_STYLE),
        skipped_count.to_string().style(terminal_style::TAG_STYLE),
        failed_count.to_string().style(terminal_style::FAILURE_STYLE),
    );

    return Ok(());
}
