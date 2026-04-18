//! Creates a new workspace and writes its `.code-workspace` file.

use crate::cli_args::WorkspaceArgsCreateParts;
use crate::error::ReportalError;
use crate::reportal_commands::workspace_operations::WorkspaceRegenerator;
use crate::reportal_config::{ReportalConfig, WorkspaceRegistrationBuilder};
use crate::terminal_style;
use owo_colors::OwoColorize;

/// Registers a new workspace with the given members and writes the
/// initial `.code-workspace` file from the current member paths.
///
/// The save-then-regenerate ordering is required so the regenerator
/// reads the post-insert registry state. Regenerating before save
/// would make the new workspace unreachable via `get_workspace` and
/// the file would never be produced.
///
/// # Errors
///
/// Returns [`ReportalError::ValidationFailure`] from the builder if
/// the name or member list is empty,
/// [`ReportalError::WorkspaceAlreadyExists`] if a workspace with
/// that name is already registered,
/// [`ReportalError::WorkspaceHasDanglingRepo`] if any member alias
/// is not registered, or the file I/O errors the regeneration path
/// surfaces.
pub fn run_workspace_create(
    create_parts: &WorkspaceArgsCreateParts,
) -> Result<(), ReportalError> {
    let mut loaded_config = ReportalConfig::load_or_initialize()?;

    let validated_registration = WorkspaceRegistrationBuilder::start(
        create_parts.workspace_name().to_owned(),
    )
    .repo_aliases(create_parts.repo_aliases().to_vec())
    .workspace_description(create_parts.description().to_owned())
    .workspace_file_path(create_parts.custom_file_path().to_owned())
    .workspace_aliases(create_parts.workspace_aliases().to_vec())
    .build()?;

    loaded_config.add_workspace(validated_registration)?;
    loaded_config.save_to_disk()?;

    let regenerator = WorkspaceRegenerator::for_config(&loaded_config);
    let written_file_path = regenerator.regenerate_workspace_file(create_parts.workspace_name())?;

    terminal_style::print_success(&format!(
        "Created workspace {} at {}",
        create_parts.workspace_name().style(terminal_style::ALIAS_STYLE),
        written_file_path.display().to_string().style(terminal_style::PATH_STYLE),
    ));
    Ok(())
}
