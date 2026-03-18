/// RePortal CLI entry point.
///
/// Parses command-line arguments via clap and dispatches
/// to the appropriate subcommand handler.

mod error;
mod reportal_commands;
mod reportal_config;
mod terminal_style;

use clap::{Parser, Subcommand};
use reportal_commands::{ColorCommandParams, JumpCommandParams, OpenCommandParams};
use reportal_config::TagFilter;

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
    /// Create a default config file at ~/.reportal/config.toml
    Init,
    /// List all registered repos with status and metadata
    #[command(alias = "l")]
    List {
        /// Filter repos by this tag
        #[arg(long)]
        tag: Option<String>,
    },
    /// Fuzzy-select a repo and print its path (for shell cd integration)
    #[command(alias = "j")]
    Jump {
        /// Jump directly to this alias (skip fuzzy finder)
        alias: Option<String>,
        /// Filter repos by this tag
        #[arg(long)]
        tag: Option<String>,
        /// Override the tab title for this session
        #[arg(long)]
        title: Option<String>,
    },
    /// Fuzzy-select a repo and open it in your editor
    #[command(alias = "o")]
    Open {
        /// Open this repo directly by alias (skip fuzzy finder)
        alias: Option<String>,
        /// Filter repos by this tag
        #[arg(long)]
        tag: Option<String>,
        /// Override the default editor command
        #[arg(long)]
        editor: Option<String>,
        /// Override the tab title for this session
        #[arg(long)]
        title: Option<String>,
    },
    /// Register a local repo in the config
    #[command(alias = "a")]
    Add {
        /// Filesystem path to the repo directory
        repo_path: String,
    },
    /// Unregister a repo from the config (does not delete files)
    #[command(alias = "rm")]
    Remove {
        /// Alias of the repo to remove
        alias: String,
    },
    /// Emit terminal tab title and background color for a repo (for shell hooks)
    Color {
        /// Look up this repo by alias instead of matching the current directory
        #[arg(long)]
        repo: Option<String>,
    },
}

fn main() {
    let parsed_cli = ReportalCli::parse();

    let command_result = match parsed_cli.subcommand {
        ReportalSubcommand::Init => reportal_commands::run_init(),
        ReportalSubcommand::List { tag } => {
            let tag_filter = match tag {
                Some(tag_value) => TagFilter::ByTag(tag_value),
                None => TagFilter::All,
            };
            reportal_commands::run_list(tag_filter)
        }
        ReportalSubcommand::Jump { alias, tag, title } => {
            let tag_filter = match tag {
                Some(tag_value) => TagFilter::ByTag(tag_value),
                None => TagFilter::All,
            };
            let direct_alias = match alias {
                Some(ref provided_alias) => provided_alias.as_str(),
                None => "",
            };
            let title_override = match title {
                Some(ref provided_title) => provided_title.as_str(),
                None => "",
            };
            reportal_commands::run_jump(JumpCommandParams {
                tag_filter,
                direct_alias,
                title_override,
            })
        }
        ReportalSubcommand::Open { alias, tag, editor, title } => {
            let tag_filter = match tag {
                Some(tag_value) => TagFilter::ByTag(tag_value),
                None => TagFilter::All,
            };
            let direct_alias = match alias {
                Some(ref provided_alias) => provided_alias.as_str(),
                None => "",
            };
            let editor_override = match editor {
                Some(ref provided_editor) => provided_editor.as_str(),
                None => "",
            };
            let title_override = match title {
                Some(ref provided_title) => provided_title.as_str(),
                None => "",
            };
            reportal_commands::run_open(OpenCommandParams {
                tag_filter,
                direct_alias,
                editor_override,
                title_override,
            })
        }
        ReportalSubcommand::Add { repo_path } => {
            reportal_commands::run_add(&repo_path)
        }
        ReportalSubcommand::Remove { alias } => {
            reportal_commands::run_remove(&alias)
        }
        ReportalSubcommand::Color { repo } => {
            let repo_alias = match repo {
                Some(ref provided_repo) => provided_repo.as_str(),
                None => "",
            };
            reportal_commands::run_color(ColorCommandParams {
                repo_alias,
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
