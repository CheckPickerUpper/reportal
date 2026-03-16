/// RePortal CLI entry point.
///
/// Parses command-line arguments via clap and dispatches
/// to the appropriate subcommand handler.

mod commands;
mod error;
mod reportal_config;

use clap::{Parser, Subcommand};
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
    List {
        /// Filter repos by this tag
        #[arg(long)]
        tag: Option<String>,
    },
    /// Fuzzy-select a repo and print its path (for shell cd integration)
    Jump {
        /// Filter repos by this tag
        #[arg(long)]
        tag: Option<String>,
    },
    /// Fuzzy-select a repo and open it in your editor
    Open {
        /// Open this repo directly by alias (skip fuzzy finder)
        alias: Option<String>,
        /// Filter repos by this tag
        #[arg(long)]
        tag: Option<String>,
        /// Override the default editor command
        #[arg(long)]
        editor: Option<String>,
    },
    /// Register a local repo in the config
    Add {
        /// Filesystem path to the repo directory
        repo_path: String,
    },
    /// Unregister a repo from the config (does not delete files)
    Remove {
        /// Alias of the repo to remove
        alias: String,
    },
}

fn main() {
    let parsed_cli = ReportalCli::parse();

    let command_result = match parsed_cli.subcommand {
        ReportalSubcommand::Init => commands::run_init(),
        ReportalSubcommand::List { tag } => {
            let tag_filter = match tag {
                Some(tag_value) => TagFilter::ByTag(tag_value),
                None => TagFilter::All,
            };
            commands::run_list(tag_filter)
        }
        ReportalSubcommand::Jump { tag } => {
            let tag_filter = match tag {
                Some(tag_value) => TagFilter::ByTag(tag_value),
                None => TagFilter::All,
            };
            commands::run_jump(tag_filter)
        }
        ReportalSubcommand::Open { alias, tag, editor } => {
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
            commands::run_open(tag_filter, direct_alias, editor_override)
        }
        ReportalSubcommand::Add { repo_path } => {
            commands::run_add(&repo_path)
        }
        ReportalSubcommand::Remove { alias } => {
            commands::run_remove(&alias)
        }
    };

    match command_result {
        Ok(()) => {}
        Err(command_error) => {
            eprintln!("Error: {command_error}");
            std::process::exit(1);
        }
    }
}
