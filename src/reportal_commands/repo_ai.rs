/// Fuzzy-selects a repo and launches an AI coding CLI in it.

use crate::error::ReportalError;
use crate::reportal_config::{ReportalConfig, TagFilter};
use crate::terminal_style;
use crate::reportal_commands::repo_selection::{self, SelectedRepoParams};
use crate::reportal_commands::terminal_identity_emit::{
    self, TerminalIdentityEmitParams,
};
use owo_colors::OwoColorize;
use std::process::Command;

/// All parameters needed to run the ai command.
pub struct AiCommandParams<'a> {
    /// Which repos to show in the fuzzy finder.
    pub tag_filter: TagFilter,
    /// If non-empty, skip the fuzzy finder and use this alias directly.
    pub direct_alias: &'a str,
    /// If non-empty, override the default AI tool.
    pub tool_override: &'a str,
}

/// Launches an AI coding CLI in the selected repo's directory.
///
/// Resolves the repo (fuzzy select or direct alias), resolves the AI tool
/// (--tool flag or config default), applies tab theming, then spawns the
/// AI CLI with stdin/stdout/stderr inherited for interactive passthrough.
pub fn run_ai(ai_params: AiCommandParams<'_>) -> Result<(), ReportalError> {
    let loaded_config = ReportalConfig::load_from_disk()?;

    let ai_tools = loaded_config.ai_tools_list();
    match ai_tools.is_empty() {
        true => return Err(ReportalError::NoAiToolsConfigured),
        false => {}
    }

    let tool_name = match ai_params.tool_override.is_empty() {
        false => ai_params.tool_override,
        true => {
            let default_tool = loaded_config.default_ai_tool();
            match default_tool.is_empty() {
                true => return Err(ReportalError::NoDefaultAiTool),
                false => default_tool,
            }
        }
    };
    let ai_tool = loaded_config.get_ai_tool(tool_name)?;

    let prompt_label = format!("Launch {tool_name} in");
    let selected = repo_selection::select_repo(SelectedRepoParams {
        loaded_config: &loaded_config,
        direct_alias: ai_params.direct_alias,
        tag_filter: &ai_params.tag_filter,
        prompt_label: &prompt_label,
    })?;

    terminal_identity_emit::emit_repo_terminal_identity(TerminalIdentityEmitParams {
        selected_alias: selected.repo_alias(),
        selected_repo: selected.repo_config(),
        title_override: "",
    });

    let resolved_repo_path = selected.repo_config().resolved_path();

    terminal_style::print_success(&format!(
        "Launching {} in {}",
        tool_name.style(terminal_style::ALIAS_STYLE),
        loaded_config.path_display_format().format_path(&resolved_repo_path)
            .style(terminal_style::PATH_STYLE),
    ));

    #[cfg(target_os = "windows")]
    let mut spawned_process = Command::new("cmd")
        .args(["/c", ai_tool.cli_command()])
        .args(ai_tool.launch_args())
        .current_dir(&resolved_repo_path)
        .stdin(std::process::Stdio::inherit())
        .stdout(std::process::Stdio::inherit())
        .stderr(std::process::Stdio::inherit())
        .spawn()
        .map_err(|spawn_error| ReportalError::AiToolLaunchFailure {
            reason: format!("{}: {spawn_error}", ai_tool.cli_command()),
        })?;

    #[cfg(not(target_os = "windows"))]
    let mut spawned_process = Command::new(ai_tool.cli_command())
        .args(ai_tool.launch_args())
        .current_dir(&resolved_repo_path)
        .stdin(std::process::Stdio::inherit())
        .stdout(std::process::Stdio::inherit())
        .stderr(std::process::Stdio::inherit())
        .spawn()
        .map_err(|spawn_error| ReportalError::AiToolLaunchFailure {
            reason: format!("{}: {spawn_error}", ai_tool.cli_command()),
        })?;

    spawned_process.wait().map_err(|wait_error| ReportalError::AiToolLaunchFailure {
        reason: format!("process exited unexpectedly: {wait_error}"),
    })?;

    return Ok(());
}
