//! Top-level CLI parser and subcommand dispatch enum.

use clap::{Parser, Subcommand};
use super::{
    AiArgs, ColorArgs, EditArgs, JumpArgs, ListArgs,
    OpenArgs, RemoveArgs, RunArgs, StatusArgs, SyncArgs, WebArgs, WorkspaceArgs,
};

/// A fast CLI tool for jumping between and managing your dev repos.
#[derive(Parser)]
#[command(name = "reportal", version, about)]
pub struct ReportalCli {
    /// The subcommand to execute.
    #[command(subcommand)]
    subcommand: ReportalCliSubcommand,
}

/// Accessor for the parsed subcommand.
impl ReportalCli {
    /// The subcommand the user invoked.
    pub fn into_subcommand(self) -> ReportalCliSubcommand {
        self.subcommand
    }
}

/// All available subcommands for the `RePortal` CLI.
#[derive(Subcommand)]
pub enum ReportalCliSubcommand {
    /// Set up config and shell integration (safe to re-run on updates)
    Init,
    /// List all registered repos with status and metadata
    #[command(alias = "l")]
    List(ListArgs),
    /// Fuzzy-select a repo and print its path (for shell cd integration)
    #[command(alias = "j")]
    Jump(JumpArgs),
    /// Fuzzy-select a repo and open it in your editor
    #[command(alias = "o")]
    Open(OpenArgs),
    /// Register a local repo in the config
    #[command(alias = "a")]
    Add {
        /// Filesystem path to the repo directory
        repo_path: String,
    },
    /// Edit a repo's description, tags, title, and color
    #[command(alias = "e")]
    Edit(EditArgs),
    /// Unregister a repo from the config (does not delete files)
    #[command(alias = "rm")]
    Remove(RemoveArgs),
    /// Emit terminal tab title and background color for a repo (for shell hooks)
    Color(ColorArgs),
    /// Show git status across all registered repos
    #[command(alias = "s")]
    Status(StatusArgs),
    /// Pull latest changes across all registered repos
    Sync(SyncArgs),
    /// Diagnose config, shell integration, and repo path issues
    Doctor,
    /// Launch an AI coding CLI in a repo
    Ai(AiArgs),
    /// Open a repo's remote URL in the browser
    #[command(alias = "w")]
    Web(WebArgs),
    /// Run a configured command in a repo
    #[command(alias = "r")]
    Run(RunArgs),
    /// Manage VSCode/Cursor `.code-workspace` definitions
    #[command(alias = "ws")]
    Workspace(WorkspaceArgs),
}
