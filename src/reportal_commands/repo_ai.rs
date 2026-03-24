/// Fuzzy-selects a repo and launches an AI coding CLI in it.

use crate::error::ReportalError;
use crate::reportal_config::{RepoColor, ReportalConfig, TabTitle, TagFilter};
use crate::terminal_style::{self, TabColorAction, TerminalIdentity, TerminalIdentityParams};
use dialoguer::{theme::ColorfulTheme, FuzzySelect};
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

    let (selected_alias, selected_repo): (&str, &crate::reportal_config::RepoEntry) =
        match ai_params.direct_alias.is_empty() {
            false => {
                let found_repo = loaded_config.get_repo(ai_params.direct_alias)?;
                (ai_params.direct_alias, found_repo)
            }
            true => {
                let matching_repos =
                    loaded_config.repos_matching_tag_filter(&ai_params.tag_filter);

                match matching_repos.is_empty() {
                    true => return Err(ReportalError::NoReposMatchFilter),
                    false => {}
                }

                let display_labels: Vec<String> = matching_repos
                    .iter()
                    .map(|(alias, repo)| {
                        let mut label = alias.to_string();

                        match repo.aliases().is_empty() {
                            true => {}
                            false => {
                                let aliases_joined = repo.aliases().join(", ");
                                label.push_str(&format!(" ({aliases_joined})"));
                            }
                        }

                        match repo.description().is_empty() {
                            true => {}
                            false => {
                                label.push_str(&format!(" — {}", repo.description()));
                            }
                        }

                        return label;
                    })
                    .collect();

                let selected_index = FuzzySelect::with_theme(&ColorfulTheme::default())
                    .with_prompt(format!("Launch {tool_name} in"))
                    .items(&display_labels)
                    .interact_opt()
                    .map_err(|select_error| ReportalError::ConfigIoFailure {
                        reason: select_error.to_string(),
                    })?;

                match selected_index {
                    Some(chosen_index) => match matching_repos.get(chosen_index) {
                        Some((chosen_alias, chosen_repo)) => (chosen_alias.as_str(), *chosen_repo),
                        None => return Err(ReportalError::SelectionCancelled),
                    },
                    None => return Err(ReportalError::SelectionCancelled),
                }
            }
        };

    let resolved_title = match selected_repo.tab_title() {
        TabTitle::Custom(custom_title) => custom_title.to_string(),
        TabTitle::UseAlias => selected_alias.to_string(),
    };

    let tab_color_action = match selected_repo.repo_color() {
        RepoColor::Themed(hex_color) => {
            TabColorAction::SetColor(hex_color.as_osc_tab_color_sequence())
        }
        RepoColor::ResetToDefault => TabColorAction::Reset,
    };

    let identity = TerminalIdentity::new(TerminalIdentityParams {
        resolved_title,
        tab_color_action,
    });
    terminal_style::emit_terminal_identity_to_console(&identity);

    let resolved_repo_path = selected_repo.resolved_path();

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
