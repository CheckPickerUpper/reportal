//! Pulls latest changes across all registered repos.

use crate::error::ReportalError;
use crate::reportal_commands::git_commands::{self, GitCommandOutcome, GitCommandParams};
use crate::reportal_config::{ReportalConfig, TagFilter};
use crate::terminal_style;
use owo_colors::OwoColorize;
use std::path::PathBuf;
use std::process::Command;

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

/// Checks whether the working tree is clean enough to safely pull.
fn check_pull_readiness(repo_path: &PathBuf) -> PullReadiness {
    match git_commands::run_git_command(&GitCommandParams {
        repo_path,
        git_subcommand_args: &["status", "--porcelain"],
    }) {
        GitCommandOutcome::Output(output) => if output.is_empty() { PullReadiness::Ready } else { PullReadiness::DirtyWorkingTree },
        GitCommandOutcome::NonZeroExit | GitCommandOutcome::SpawnFailed => PullReadiness::Unknown,
    }
}

/// Runs `git pull` in the repo directory and returns the outcome.
fn pull_repo(repo_path: &PathBuf) -> SyncOutcome {
    let pull_result = Command::new("git")
        .args(["pull"])
        .current_dir(repo_path)
        .output();

    match pull_result {
        Ok(output) => if output.status.success() {
            let stdout = String::from_utf8_lossy(&output.stdout).trim().to_owned();
            SyncOutcome::Pulled(stdout)
        } else {
            let stderr = String::from_utf8_lossy(&output.stderr).trim().to_owned();
            SyncOutcome::PullFailed(stderr)
        },
        Err(spawn_error) => SyncOutcome::PullFailed(spawn_error.to_string()),
    }
}

/// Syncs a single repo: checks if clean, then pulls.
fn sync_single_repo(alias: &str, repo_path: &PathBuf) -> RepoSyncResult {
    if !repo_path.exists() {
        return RepoSyncResult {
            alias: alias.to_owned(),
            outcome: SyncOutcome::Missing,
        };
    }
    let outcome = match check_pull_readiness(repo_path) {
        PullReadiness::Ready => pull_repo(repo_path),
        PullReadiness::DirtyWorkingTree => SyncOutcome::SkippedDirty,
        PullReadiness::Unknown => SyncOutcome::PullFailed("could not determine working tree state".to_owned()),
    };
    RepoSyncResult {
        alias: alias.to_owned(),
        outcome,
    }
}

/// Prints a styled status line for a single repo sync result.
fn print_sync_result(sync_result: &RepoSyncResult) {
    match &sync_result.outcome {
        SyncOutcome::Pulled(output) => {
            let marker = if output.contains("Already up to date") { "✓" } else { "↓" };
            let suffix = if output.contains("Already up to date") { "up to date" } else { "pulled" };
            terminal_style::write_stdout(&format!(
                "  {} {} {}
",
                sync_result.alias.style(terminal_style::ALIAS_STYLE),
                marker.style(terminal_style::SUCCESS_STYLE),
                suffix,
            ));
        }
        SyncOutcome::SkippedDirty => {
            terminal_style::write_stderr(&format!(
                "  {} {} skipped (dirty working tree)
",
                sync_result.alias.style(terminal_style::ALIAS_STYLE),
                "!".style(terminal_style::FAILURE_STYLE),
            ));
        }
        SyncOutcome::Missing => {
            terminal_style::write_stderr(&format!(
                "  {} {} missing on disk
",
                sync_result.alias.style(terminal_style::ALIAS_STYLE),
                "✗".style(terminal_style::FAILURE_STYLE),
            ));
        }
        SyncOutcome::PullFailed(error_message) => {
            terminal_style::write_stderr(&format!(
                "  {} {} pull failed: {}
",
                sync_result.alias.style(terminal_style::ALIAS_STYLE),
                "✗".style(terminal_style::FAILURE_STYLE),
                error_message,
            ));
        }
    }
}

/// Pulls latest changes for every repo matching the tag filter.
/// Skips repos with uncommitted changes (with a warning).
/// Prints a per-repo summary showing what happened.
pub fn run_sync(tag_filter: &TagFilter) -> Result<(), ReportalError> {
    let loaded_config = ReportalConfig::load_from_disk()?;
    let matching_repos = loaded_config.repos_matching_tag_filter(tag_filter);

    if matching_repos.is_empty() {
        return Err(ReportalError::NoReposMatchFilter);
    }

    let mut pulled_count: usize = 0;
    let mut skipped_count: usize = 0;
    let mut failed_count: usize = 0;

    for (alias, repo) in &matching_repos {
        let resolved = repo.resolved_path();
        let sync_result = sync_single_repo(alias, &resolved);
        print_sync_result(&sync_result);

        match &sync_result.outcome {
            SyncOutcome::Pulled(_) => pulled_count += 1,
            SyncOutcome::SkippedDirty => skipped_count += 1,
            SyncOutcome::Missing | SyncOutcome::PullFailed(_) => failed_count += 1,
        }
    }

    terminal_style::write_stdout("\n");
    terminal_style::write_stdout(&format!(
        "  {} pulled, {} skipped, {} failed\n",
        pulled_count.to_string().style(terminal_style::SUCCESS_STYLE),
        skipped_count.to_string().style(terminal_style::TAG_STYLE),
        failed_count.to_string().style(terminal_style::FAILURE_STYLE),
    ));

    Ok(())
}
