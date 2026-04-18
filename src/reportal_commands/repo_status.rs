//! Shows git status across all registered repos in a table.

use crate::error::ReportalError;
use crate::reportal_commands::git_commands::{self, GitCommandOutcome, GitCommandParams};
use crate::reportal_config::{ReportalConfig, TagFilter};
use crate::terminal_style;
use comfy_table::{Cell, Table};
use owo_colors::OwoColorize;
use std::path::PathBuf;

/// The git state of a single repository.
struct RepoGitStatus {
    /// The alias this repo is registered under.
    alias: String,
    /// Whether the repo directory exists on disk.
    presence: RepoPresence,
}

/// Whether a registered repo exists on disk.
enum RepoPresence {
    /// The directory exists and git info was collected.
    Present(GitInfo),
    /// The directory does not exist.
    Missing,
}

/// Git metadata collected from a repo directory.
struct GitInfo {
    /// The current branch name, or "detached" if HEAD is detached.
    branch: String,
    /// Whether the working tree has uncommitted changes.
    working_tree: WorkingTreeState,
    /// How many commits ahead/behind the upstream tracking branch.
    upstream_delta: UpstreamDelta,
    /// The relative timestamp of the last commit.
    last_commit_age: String,
}

/// Whether the working tree is clean or has uncommitted changes.
enum WorkingTreeState {
    /// No uncommitted changes.
    Clean,
    /// Has modified, staged, or untracked files.
    Dirty,
}

/// How far ahead or behind the local branch is from its upstream.
enum UpstreamDelta {
    /// Local branch has commits ahead and/or behind upstream.
    Tracked(AheadBehindCounts),
    /// No upstream tracking branch configured.
    NoUpstream,
}

/// The number of commits ahead and behind upstream.
struct AheadBehindCounts {
    /// Commits on local not yet pushed.
    ahead: usize,
    /// Commits on remote not yet pulled.
    behind: usize,
}

/// Whether the rev-list count line could be split into ahead/behind.
enum RevListParse {
    /// Both counts were successfully parsed.
    Parsed(AheadBehindCounts),
    /// The output was not in the expected "N\tM" format.
    Malformed,
}

/// Whether a usize parse succeeded or the input was not a valid number.
enum UsizeParse {
    /// The string was a valid usize.
    Valid(usize),
    /// The string could not be parsed.
    Invalid,
}

/// Parameters for collecting git status from a single repo.
struct StatusCollectionParams<'a> {
    /// The alias of the repo being checked.
    alias: &'a str,
    /// The resolved filesystem path to the repo.
    repo_path: &'a PathBuf,
}

/// Wraps usize parsing to name both outcomes explicitly.
fn parse_usize(raw_text: &str) -> UsizeParse {
    match raw_text.parse::<usize>() {
        Ok(value) => UsizeParse::Valid(value),
        Err(_number_parse_error) => UsizeParse::Invalid,
    }
}

/// Reads the current branch name from a repo directory.
fn read_branch_name(repo_path: &PathBuf) -> String {
    match git_commands::run_git_command(&GitCommandParams {
        repo_path,
        git_subcommand_args: &["rev-parse", "--abbrev-ref", "HEAD"],
    }) {
        GitCommandOutcome::Output(branch) => if branch.is_empty() || branch == "HEAD" { "detached".to_owned() } else { branch },
        GitCommandOutcome::NonZeroExit => "unknown".to_owned(),
        GitCommandOutcome::SpawnFailed => "no-git".to_owned(),
    }
}

/// Checks whether the working tree has uncommitted changes.
fn read_working_tree_state(repo_path: &PathBuf) -> WorkingTreeState {
    match git_commands::run_git_command(&GitCommandParams {
        repo_path,
        git_subcommand_args: &["status", "--porcelain"],
    }) {
        GitCommandOutcome::Output(output) => if output.is_empty() { WorkingTreeState::Clean } else { WorkingTreeState::Dirty },
        GitCommandOutcome::NonZeroExit | GitCommandOutcome::SpawnFailed => WorkingTreeState::Dirty,
    }
}

/// Parses a "N\tM" rev-list count string into ahead/behind counts.
fn parse_rev_list_counts(raw_counts: &str) -> RevListParse {
    let parts: Vec<&str> = raw_counts.split('\t').collect();
    match (parts.first(), parts.get(1)) {
        (Some(ahead_str), Some(behind_str)) => {
            match (parse_usize(ahead_str), parse_usize(behind_str)) {
                (UsizeParse::Valid(ahead), UsizeParse::Valid(behind)) => {
                    RevListParse::Parsed(AheadBehindCounts { ahead, behind })
                }
                _ => RevListParse::Malformed,
            }
        }
        _ => RevListParse::Malformed,
    }
}

/// Reads ahead/behind counts relative to the upstream tracking branch.
fn read_upstream_delta(repo_path: &PathBuf) -> UpstreamDelta {
    match git_commands::run_git_command(&GitCommandParams {
        repo_path,
        git_subcommand_args: &["rev-list", "--left-right", "--count", "HEAD...@{upstream}"],
    }) {
        GitCommandOutcome::Output(counts_text) => match parse_rev_list_counts(&counts_text) {
            RevListParse::Parsed(counts) => UpstreamDelta::Tracked(counts),
            RevListParse::Malformed => UpstreamDelta::NoUpstream,
        },
        GitCommandOutcome::NonZeroExit | GitCommandOutcome::SpawnFailed => UpstreamDelta::NoUpstream,
    }
}

/// Reads the relative age of the last commit (e.g. "3 days ago").
fn read_last_commit_age(repo_path: &PathBuf) -> String {
    match git_commands::run_git_command(&GitCommandParams {
        repo_path,
        git_subcommand_args: &["log", "-1", "--format=%cr"],
    }) {
        GitCommandOutcome::Output(age) => age,
        GitCommandOutcome::NonZeroExit => "no commits".to_owned(),
        GitCommandOutcome::SpawnFailed => "no-git".to_owned(),
    }
}

/// Collects git status for a single repo by running git commands in its directory.
fn collect_repo_status(status_collection_params: &StatusCollectionParams<'_>) -> RepoGitStatus {
    if status_collection_params.repo_path.exists() {
        let branch = read_branch_name(status_collection_params.repo_path);
        let working_tree = read_working_tree_state(status_collection_params.repo_path);
        let upstream_delta = read_upstream_delta(status_collection_params.repo_path);
        let last_commit_age = read_last_commit_age(status_collection_params.repo_path);

        RepoGitStatus {
            alias: status_collection_params.alias.to_owned(),
            presence: RepoPresence::Present(GitInfo {
                branch,
                working_tree,
                upstream_delta,
                last_commit_age,
            }),
        }
    } else { RepoGitStatus {
        alias: status_collection_params.alias.to_owned(),
        presence: RepoPresence::Missing,
    } }
}

/// Formats the upstream delta into a string like "2↑ 1↓" or "synced".
fn format_upstream_delta(upstream_delta: &UpstreamDelta) -> String {
    match upstream_delta {
        UpstreamDelta::Tracked(counts) => {
            let mut parts: Vec<String> = Vec::new();
            match counts.ahead {
                0 => {}
                ahead_count => parts.push(format!("{ahead_count}↑")),
            }
            match counts.behind {
                0 => {}
                behind_count => parts.push(format!("{behind_count}↓")),
            }
            if parts.is_empty() { "synced".to_owned() } else { parts.join(" ") }
        }
        UpstreamDelta::NoUpstream => "no upstream".to_owned(),
    }
}

/// Collects git metadata (branch, dirty state, upstream delta, last commit)
/// from every repo matching the tag filter and prints a summary table.
/// Reports dirty and missing repo counts in a footer on stderr.
pub fn run_status(tag_filter: &TagFilter) -> Result<(), ReportalError> {
    let loaded_config = ReportalConfig::load_or_initialize()?;
    let matching_repos = loaded_config.repos_matching_tag_filter(tag_filter);

    if matching_repos.is_empty() {
        return Err(ReportalError::NoReposMatchFilter);
    }

    let statuses: Vec<RepoGitStatus> = matching_repos
        .iter()
        .map(|(alias, repo)| {
            let resolved = repo.resolved_path();
            collect_repo_status(&StatusCollectionParams {
                alias,
                repo_path: &resolved,
            })
        })
        .collect();

    let mut table = Table::new();
    table.set_header(vec!["Repo", "Branch", "Status", "Upstream", "Last Commit"]);

    for repo_status in &statuses {
        match &repo_status.presence {
            RepoPresence::Present(git_info) => {
                let status_text = match &git_info.working_tree {
                    WorkingTreeState::Clean => "clean".style(terminal_style::SUCCESS_STYLE).to_string(),
                    WorkingTreeState::Dirty => "dirty".style(terminal_style::FAILURE_STYLE).to_string(),
                };

                let upstream_text = format_upstream_delta(&git_info.upstream_delta);

                table.add_row(vec![
                    Cell::new(&repo_status.alias),
                    Cell::new(&git_info.branch),
                    Cell::new(&status_text),
                    Cell::new(&upstream_text),
                    Cell::new(&git_info.last_commit_age),
                ]);
            }
            RepoPresence::Missing => {
                table.add_row(vec![
                    Cell::new(&repo_status.alias),
                    Cell::new("—"),
                    Cell::new(
                        "missing"
                            .style(terminal_style::FAILURE_STYLE)
                            .to_string(),
                    ),
                    Cell::new("—"),
                    Cell::new("—"),
                ]);
            }
        }
    }

    terminal_style::write_stdout(&format!("{table}\n"));

    let dirty_count = statuses
        .iter()
        .filter(|status| match &status.presence {
            RepoPresence::Present(git_info) => match &git_info.working_tree {
                WorkingTreeState::Dirty => true,
                WorkingTreeState::Clean => false,
            },
            RepoPresence::Missing => false,
        })
        .count();

    let missing_count = statuses
        .iter()
        .filter(|status| match &status.presence {
            RepoPresence::Missing => true,
            RepoPresence::Present(_) => false,
        })
        .count();

    terminal_style::write_stdout("\n");
    match dirty_count {
        0 => {}
        count => {
            terminal_style::write_stderr(&format!(
                "  {} {} {} with uncommitted changes\n",
                "!".style(terminal_style::FAILURE_STYLE),
                count,
                match count {
                    1 => "repo",
                    _ => "repos",
                },
            ));
        }
    }
    match missing_count {
        0 => {}
        count => {
            terminal_style::write_stderr(&format!(
                "  {} {} {} not found on disk\n",
                "!".style(terminal_style::FAILURE_STYLE),
                count,
                match count {
                    1 => "repo",
                    _ => "repos",
                },
            ));
        }
    }

    Ok(())
}
