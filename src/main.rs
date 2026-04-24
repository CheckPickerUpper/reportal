#![warn(missing_docs)]
#![forbid(unsafe_code)]

//! `RePortal` CLI entry point.
//!
//! Parses command-line arguments via clap and dispatches
//! to the appropriate subcommand handler.

mod cli_args;
mod code_workspace;
mod error;
mod reportal_commands;
mod reportal_config;
mod terminal_style;

use clap::Parser;
use cli_args::{ReportalCli, ReportalCliSubcommand};
use reportal_commands::{
    AiCommandParameters, ColorCommandParameters, EditCommandParameters, JumpCommandParameters,
    OpenCommandParameters, RunCommandParameters, WebCommandParameters,
};

fn main() {
    let parsed_cli = ReportalCli::parse();
    let subcommand = parsed_cli.into_subcommand();

    // First-run auto-wire: if the shell integration isn't already
    // sourced, silently append the fenced eval block to the user's rc
    // file so `cargo install reportal` users don't have to edit their
    // rc file by hand. Skipped for `rep init` because that subcommand
    // is consumed via `eval "$(rep init zsh)"`, where any stderr
    // noise would appear on every shell startup.
    if !matches!(subcommand, ReportalCliSubcommand::Init(_)) {
        reportal_commands::ensure_shell_integration_installed();
    }

    let command_result = match subcommand {
        ReportalCliSubcommand::Init(initialize_arguments) => {
            reportal_commands::run_initialize(initialize_arguments.shell());
            Ok(())
        }
        ReportalCliSubcommand::List(list_arguments) => {
            reportal_commands::run_list(&list_arguments.into_filter_parts())
        }
        ReportalCliSubcommand::Jump(jump_arguments) => {
            let (direct_alias, tag_filter, title_override) = jump_arguments.into_parts();
            reportal_commands::run_jump(&JumpCommandParameters {
                tag_filter,
                direct_alias: &direct_alias,
                title_override: &title_override,
            })
        }
        ReportalCliSubcommand::Open(open_arguments) => {
            let (direct_alias, tag_filter, editor_override, title_override) = open_arguments.into_parts();
            reportal_commands::run_open(&OpenCommandParameters {
                tag_filter,
                direct_alias: &direct_alias,
                editor_override: &editor_override,
                title_override: &title_override,
            })
        }
        ReportalCliSubcommand::Add { repo_path } => {
            reportal_commands::run_add(&repo_path)
        }
        ReportalCliSubcommand::Edit(edit_arguments) => {
            let (direct_alias, tag_filter) = edit_arguments.into_parts();
            reportal_commands::run_edit(&EditCommandParameters {
                tag_filter,
                direct_alias: &direct_alias,
            })
        }
        ReportalCliSubcommand::Remove(remove_arguments) => {
            reportal_commands::run_remove(remove_arguments.alias())
        }
        ReportalCliSubcommand::Color(color_arguments) => {
            let (repository_alias, mode) = color_arguments.into_parts();
            reportal_commands::run_color(&ColorCommandParameters {
                repository_alias: &repository_alias,
                mode,
            })
        }
        ReportalCliSubcommand::Prompt(prompt_arguments) => {
            reportal_commands::run_prompt(prompt_arguments.into_shell())
        }
        ReportalCliSubcommand::Status(status_arguments) => {
            reportal_commands::run_status(&status_arguments.into_tag_filter())
        }
        ReportalCliSubcommand::Sync(sync_arguments) => {
            reportal_commands::run_sync(&sync_arguments.into_tag_filter())
        }
        ReportalCliSubcommand::Doctor => {
            reportal_commands::run_doctor();
            Ok(())
        }
        ReportalCliSubcommand::Web(web_arguments) => {
            let (direct_alias, tag_filter) = web_arguments.into_parts();
            reportal_commands::run_web(&WebCommandParameters {
                tag_filter,
                direct_alias: &direct_alias,
            })
        }
        ReportalCliSubcommand::Run(run_arguments) => {
            let (direct_alias, tag_filter, direct_command) = run_arguments.into_parts();
            reportal_commands::run_run(&RunCommandParameters {
                tag_filter,
                direct_alias: &direct_alias,
                direct_command: &direct_command,
            })
        }
        ReportalCliSubcommand::Ai(ai_arguments) => {
            let (direct_alias, tag_filter, tool_override) = ai_arguments.into_parts();
            reportal_commands::run_ai(&AiCommandParameters {
                tag_filter,
                direct_alias: &direct_alias,
                tool_override: &tool_override,
            })
        }
        ReportalCliSubcommand::Workspace(workspace_arguments) => {
            reportal_commands::dispatch_workspace_subcommand(workspace_arguments)
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
