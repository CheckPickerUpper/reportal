//! Top-level CLI parser and subcommand dispatch enum.

use clap::{Parser, Subcommand};
use super::{
    AiArguments, ColorArguments, EditArguments, InitializeArguments, JumpArguments, ListArguments,
    OpenArguments, PromptArguments, RemoveArguments, RunArguments, StatusArguments, SyncArguments,
    WebArguments, WorkspaceArguments,
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
    /// Print shell integration code for the given shell to stdout.
    ///
    /// Wire it into your rc file with a single eval line, the same
    /// pattern used by starship, zoxide, direnv, and mise. The binary
    /// never writes integration files to disk and never prompts.
    ///
    /// Zsh / Bash:
    ///
    ///     eval "$(rep init zsh)"
    ///     eval "$(rep init bash)"
    ///
    /// `PowerShell`:
    ///
    ///     Invoke-Expression (& rep init powershell | Out-String)
    #[command(verbatim_doc_comment)]
    Init(InitializeArguments),
    /// List all registered repos with status and metadata
    #[command(alias = "l")]
    List(ListArguments),
    /// Fuzzy-select a repo and print its path (for shell cd integration)
    #[command(alias = "j")]
    Jump(JumpArguments),
    /// Fuzzy-select a repo and open it in your editor
    #[command(alias = "o")]
    Open(OpenArguments),
    /// Register a local repo in the config
    #[command(alias = "a")]
    Add {
        /// Filesystem path to the repo directory
        repo_path: String,
    },
    /// Edit a repo's description, tags, title, and color
    #[command(alias = "e")]
    Edit(EditArguments),
    /// Unregister a repo from the config (does not delete files)
    #[command(alias = "rm")]
    Remove(RemoveArguments),
    /// Emit terminal tab title and background color for a repo (for shell hooks)
    Color(ColorArguments),
    /// Emit an ANSI-colored prompt badge for the current workspace or repo
    ///
    /// Output is wrapped with the target shell's non-printing-escape
    /// markers so it can be inlined inside a `PS1` (bash), `$PROMPT`
    /// (zsh), or `prompt` function (`PowerShell`) without breaking
    /// cursor math. Silent when the CWD matches neither a workspace
    /// directory nor a registered repo path.
    #[command(verbatim_doc_comment)]
    Prompt(PromptArguments),
    /// Show git status across all registered repos
    #[command(alias = "s")]
    Status(StatusArguments),
    /// Pull latest changes across all registered repos
    Sync(SyncArguments),
    /// Diagnose config, shell integration, and repo path issues
    Doctor,
    /// Launch an AI coding CLI in a repo
    Ai(AiArguments),
    /// Open a repo's remote URL in the browser
    #[command(alias = "w")]
    Web(WebArguments),
    /// Run a configured command in a repo
    #[command(alias = "r")]
    Run(RunArguments),
    /// Manage VSCode/Cursor `.code-workspace` definitions
    #[command(alias = "ws")]
    Workspace(WorkspaceArguments),
}
