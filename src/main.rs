/// RePortal CLI entry point.
///
/// Parses command-line arguments via clap and dispatches
/// to the appropriate subcommand handler.

mod error;
mod reportal_commands;
mod reportal_config;
mod terminal_style;

use clap::{Args, Parser, Subcommand};
use reportal_commands::{AiCommandParams, ColorCommandParams, JumpCommandParams, OpenCommandParams, TitleOutput};
use reportal_config::TagFilter;

/// Shared --tag flag used by commands that filter repos.
#[derive(Args)]
struct TagFilterArgs {
    /// Filter repos by this tag
    #[arg(long)]
    tag: Option<String>,
}

/// Converts the optional tag CLI arg into the domain's `TagFilter` enum.
impl TagFilterArgs {
    fn into_tag_filter(self) -> TagFilter {
        match self.tag {
            Some(tag_value) => TagFilter::ByTag(tag_value),
            None => TagFilter::All,
        }
    }
}

/// Shared optional alias positional arg + --tag flag for repo selection.
#[derive(Args)]
struct RepoSelectionArgs {
    /// Jump directly to this alias (skip fuzzy finder)
    alias: Option<String>,
    #[command(flatten)]
    tag_filter: TagFilterArgs,
}

/// CLI args for `rep list`.
#[derive(Args)]
struct ListArgs {
    #[command(flatten)]
    filter: TagFilterArgs,
}

/// CLI args for `rep jump`.
#[derive(Args)]
struct JumpArgs {
    #[command(flatten)]
    selection: RepoSelectionArgs,
    /// Override the tab title for this session
    #[arg(long)]
    title: Option<String>,
}

/// CLI args for `rep open`.
#[derive(Args)]
struct OpenArgs {
    #[command(flatten)]
    selection: RepoSelectionArgs,
    /// Override the default editor command
    #[arg(long)]
    editor: Option<String>,
    /// Override the tab title for this session
    #[arg(long)]
    title: Option<String>,
}

/// CLI args for `rep edit`.
#[derive(Args)]
struct EditArgs {
    /// Alias of the repo to edit (skip top-level menu)
    alias: Option<String>,
}

/// CLI args for `rep remove`.
#[derive(Args)]
struct RemoveArgs {
    /// Alias of the repo to remove
    alias: String,
}

/// CLI args for `rep color`.
#[derive(Args)]
struct ColorArgs {
    /// Look up this repo by alias instead of matching the current directory
    #[arg(long)]
    repo: Option<String>,
    /// Print the resolved tab title to stdout (for shell integration)
    #[arg(long)]
    print_title: bool,
}

/// CLI args for `rep status`.
#[derive(Args)]
struct StatusArgs {
    #[command(flatten)]
    filter: TagFilterArgs,
}

/// CLI args for `rep sync`.
#[derive(Args)]
struct SyncArgs {
    #[command(flatten)]
    filter: TagFilterArgs,
}

/// CLI args for `rep ai`.
#[derive(Args)]
struct AiArgs {
    #[command(flatten)]
    selection: RepoSelectionArgs,
    /// Which AI tool to launch (overrides default_ai_tool setting)
    #[arg(long)]
    tool: Option<String>,
}

/// A fast CLI tool for jumping between and managing your dev repos.
#[derive(Parser)]
#[command(name = "reportal", version, about)]
struct ReportalCli {
    /// The subcommand to execute.
    #[command(subcommand)]
    subcommand: ReportalSubcommand,
}

/// All available subcommands for the RePortal CLI.
#[derive(Subcommand)]
enum ReportalSubcommand {
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
}

fn main() {
    reportal_commands::ensure_integration_file_current();

    let parsed_cli = ReportalCli::parse();

    let command_result = match parsed_cli.subcommand {
        ReportalSubcommand::Init => reportal_commands::run_init(),
        ReportalSubcommand::List(list_args) => {
            reportal_commands::run_list(list_args.filter.into_tag_filter())
        }
        ReportalSubcommand::Jump(jump_args) => {
            let direct_alias = match jump_args.selection.alias {
                Some(ref provided_alias) => provided_alias.as_str(),
                None => "",
            };
            let title_override = match jump_args.title {
                Some(ref provided_title) => provided_title.as_str(),
                None => "",
            };
            reportal_commands::run_jump(JumpCommandParams {
                tag_filter: jump_args.selection.tag_filter.into_tag_filter(),
                direct_alias,
                title_override,
            })
        }
        ReportalSubcommand::Open(open_args) => {
            let direct_alias = match open_args.selection.alias {
                Some(ref provided_alias) => provided_alias.as_str(),
                None => "",
            };
            let editor_override = match open_args.editor {
                Some(ref provided_editor) => provided_editor.as_str(),
                None => "",
            };
            let title_override = match open_args.title {
                Some(ref provided_title) => provided_title.as_str(),
                None => "",
            };
            reportal_commands::run_open(OpenCommandParams {
                tag_filter: open_args.selection.tag_filter.into_tag_filter(),
                direct_alias,
                editor_override,
                title_override,
            })
        }
        ReportalSubcommand::Add { repo_path } => {
            reportal_commands::run_add(&repo_path)
        }
        ReportalSubcommand::Edit(edit_args) => {
            let direct_alias = match edit_args.alias {
                Some(ref provided_alias) => provided_alias.as_str(),
                None => "",
            };
            reportal_commands::run_edit(direct_alias)
        }
        ReportalSubcommand::Remove(remove_args) => {
            reportal_commands::run_remove(&remove_args.alias)
        }
        ReportalSubcommand::Color(color_args) => {
            let repo_alias = match color_args.repo {
                Some(ref provided_repo) => provided_repo.as_str(),
                None => "",
            };
            let title_output = match color_args.print_title {
                true => TitleOutput::PrintToStdout,
                false => TitleOutput::Silent,
            };
            reportal_commands::run_color(ColorCommandParams {
                repo_alias,
                title_output,
            })
        }
        ReportalSubcommand::Status(status_args) => {
            reportal_commands::run_status(status_args.filter.into_tag_filter())
        }
        ReportalSubcommand::Sync(sync_args) => {
            reportal_commands::run_sync(sync_args.filter.into_tag_filter())
        }
        ReportalSubcommand::Doctor => reportal_commands::run_doctor(),
        ReportalSubcommand::Ai(ai_args) => {
            let direct_alias = match ai_args.selection.alias {
                Some(ref provided_alias) => provided_alias.as_str(),
                None => "",
            };
            let tool_override = match ai_args.tool {
                Some(ref provided_tool) => provided_tool.as_str(),
                None => "",
            };
            reportal_commands::run_ai(AiCommandParams {
                tag_filter: ai_args.selection.tag_filter.into_tag_filter(),
                direct_alias,
                tool_override,
            })
        }
    };

    match command_result {
        Ok(()) => {}
        Err(command_error) => {
            terminal_style::print_error(&command_error.to_string());
            std::process::exit(1);
        }
    }
}
