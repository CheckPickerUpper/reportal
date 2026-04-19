//! Creates a new workspace and materializes its on-disk directory
//! (symlinks + `.code-workspace` file).

use crate::cli_args::WorkspaceArgsCreateParts;
use crate::error::ReportalError;
use crate::reportal_commands::workspace_operations::WorkspaceRegenerator;
use crate::reportal_config::{ReportalConfig, WorkspaceRegistrationBuilder};
use crate::terminal_style;
use owo_colors::OwoColorize;
use std::path::PathBuf;

/// Registers a new workspace with the given members and
/// materializes the on-disk directory containing one symlink /
/// junction per member plus the `.code-workspace` file.
///
/// The save-then-regenerate ordering is required so the regenerator
/// reads the post-insert registry state. Regenerating before save
/// would make the new workspace unreachable via `get_workspace`.
///
/// The optional `--file-path` flag now specifies the workspace
/// **directory**, not the `.code-workspace` file path. If the user
/// passes a legacy value ending in `.code-workspace`, the parent
/// directory is used instead — so existing scripts that set the
/// flag continue to produce a usable workspace directory.
///
/// # Errors
///
/// Returns [`ReportalError::ValidationFailure`] from the builder if
/// the name or member list is empty,
/// [`ReportalError::WorkspaceAlreadyExists`] if a workspace with
/// that name is already registered,
/// [`ReportalError::WorkspaceHasDanglingRepo`] if any member alias
/// is not registered, or the file/link I/O errors the regeneration
/// path surfaces.
pub fn run_workspace_create(
    create_parts: &WorkspaceArgsCreateParts,
) -> Result<(), ReportalError> {
    let mut loaded_config = ReportalConfig::load_or_initialize()?;

    let reinterpreted_directory_path =
        reinterpret_custom_path_as_directory(create_parts.custom_file_path());

    let validated_registration = WorkspaceRegistrationBuilder::start(
        create_parts.workspace_name().to_owned(),
    )
    .repo_aliases(create_parts.repo_aliases().to_vec())
    .workspace_description(create_parts.description().to_owned())
    .workspace_file_path(reinterpreted_directory_path)
    .workspace_aliases(create_parts.workspace_aliases().to_vec())
    .build()?;

    loaded_config.add_workspace(validated_registration)?;
    loaded_config.save_to_disk()?;

    let regenerator = WorkspaceRegenerator::for_config(&loaded_config);
    let workspace_file_path =
        regenerator.regenerate_workspace_file(create_parts.workspace_name())?;
    let workspace_directory = regenerator.resolve_workspace_directory(create_parts.workspace_name())?;

    terminal_style::print_success(&format!(
        "Created workspace {} at {}",
        create_parts.workspace_name().style(terminal_style::ALIAS_STYLE),
        workspace_directory.display().to_string().style(terminal_style::PATH_STYLE),
    ));
    terminal_style::write_stdout(&format!(
        "   workspace file: {}\n",
        workspace_file_path.display().to_string().style(terminal_style::PATH_STYLE),
    ));
    Ok(())
}

/// Converts a user-supplied `--file-path` value into the workspace
/// directory path stored in config.
///
/// Pre-v0.15.2 the flag pointed at the `.code-workspace` file; we
/// interpret such values as the parent directory so existing
/// scripts continue to produce a working layout. Empty input
/// preserves the empty-string sentinel that means "use the default
/// `<default_workspace_root>/<name>/` location."
fn reinterpret_custom_path_as_directory(raw_path: &str) -> String {
    let trimmed = raw_path.trim();
    if trimmed.is_empty() {
        return String::new();
    }
    let candidate = PathBuf::from(trimmed);
    if candidate
        .extension()
        .is_some_and(|ext| ext.eq_ignore_ascii_case("code-workspace"))
    {
        return candidate
            .parent()
            .map_or_else(String::new, |parent_path| {
                parent_path.display().to_string()
            });
    }
    trimmed.to_owned()
}
