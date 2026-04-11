/// RePortal CLI entry point.
///
/// Parses command-line arguments via clap and dispatches
/// to the appropriate subcommand handler.

mod cli_args;
mod error;
mod reportal_commands;
mod reportal_config;
mod terminal_style;

use clap::Parser;
use cli_args::{ReportalCli, ReportalCliSubcommand};
use reportal_commands::{
    AiCommandParams, ColorCommandParams, EditCommandParams, JumpCommandParams,
    OpenCommandParams, RunCommandParams, WebCommandParams,
};

fn main() {
    reportal_commands::ensure_integration_file_current();

    let parsed_cli = ReportalCli::parse();

    let command_result = match parsed_cli.into_subcommand() {
        ReportalCliSubcommand::Init => reportal_commands::run_init(),
        ReportalCliSubcommand::List(list_args) => {
            reportal_commands::run_list(list_args.into_tag_filter())
        }
        ReportalCliSubcommand::Jump(jump_args) => {
            let (direct_alias, tag_filter, title_override) = jump_args.into_parts();
            reportal_commands::run_jump(JumpCommandParams {
                tag_filter,
                direct_alias: &direct_alias,
                title_override: &title_override,
            })
        }
        ReportalCliSubcommand::Open(open_args) => {
            let (direct_alias, tag_filter, editor_override, title_override) = open_args.into_parts();
            reportal_commands::run_open(OpenCommandParams {
                tag_filter,
                direct_alias: &direct_alias,
                editor_override: &editor_override,
                title_override: &title_override,
            })
        }
        ReportalCliSubcommand::Add { repo_path } => {
            reportal_commands::run_add(&repo_path)
        }
        ReportalCliSubcommand::Edit(edit_args) => {
            let (direct_alias, tag_filter) = edit_args.into_parts();
            reportal_commands::run_edit(EditCommandParams {
                tag_filter,
                direct_alias: &direct_alias,
            })
        }
        ReportalCliSubcommand::Remove(remove_args) => {
            reportal_commands::run_remove(remove_args.alias())
        }
        ReportalCliSubcommand::Color(color_args) => {
            let (repo_alias, mode) = color_args.into_parts();
            reportal_commands::run_color(ColorCommandParams {
                repo_alias: &repo_alias,
                mode,
            })
        }
        ReportalCliSubcommand::Status(status_args) => {
            reportal_commands::run_status(status_args.into_tag_filter())
        }
        ReportalCliSubcommand::Sync(sync_args) => {
            reportal_commands::run_sync(sync_args.into_tag_filter())
        }
        ReportalCliSubcommand::Doctor => reportal_commands::run_doctor(),
        ReportalCliSubcommand::Web(web_args) => {
            let (direct_alias, tag_filter) = web_args.into_parts();
            reportal_commands::run_web(WebCommandParams {
                tag_filter,
                direct_alias: &direct_alias,
            })
        }
        ReportalCliSubcommand::Run(run_args) => {
            let (direct_alias, tag_filter, direct_command) = run_args.into_parts();
            reportal_commands::run_run(RunCommandParams {
                tag_filter,
                direct_alias: &direct_alias,
                direct_command: &direct_command,
            })
        }
        ReportalCliSubcommand::Ai(ai_args) => {
            let (direct_alias, tag_filter, tool_override) = ai_args.into_parts();
            reportal_commands::run_ai(AiCommandParams {
                tag_filter,
                direct_alias: &direct_alias,
                tool_override: &tool_override,
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
