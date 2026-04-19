//! Global settings that apply across all repos, stored in the
//! `[settings]` section of `config.toml`.

use serde::{Deserialize, Serialize};
use crate::terminal_style;

/// How repo paths are displayed in output after selecting a repo.
#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum PathDisplayFormat {
    /// Full absolute path from root.
    Absolute,
    /// Path relative to the current working directory.
    Relative,
}

/// Formatting methods for converting absolute paths based on display preference.
impl PathDisplayFormat {
    /// Formats a path according to the configured display format.
    ///
    /// For absolute: returns the path as-is.
    /// For relative: computes the path relative to the current working directory.
    pub fn format_path(&self, absolute_path: &std::path::PathBuf) -> String {
        match self {
            PathDisplayFormat::Absolute => {
                absolute_path.display().to_string()
            }
            PathDisplayFormat::Relative => {
                let current_directory = std::env::current_dir();
                match current_directory {
                    Ok(working_directory) => {
                        let relative_result = pathdiff::diff_paths(absolute_path, &working_directory);
                        relative_result.map_or_else(
                            || absolute_path.display().to_string(),
                            |relative_path| relative_path.display().to_string(),
                        )
                    }
                    Err(cwd_read_error) => {
                        terminal_style::write_stderr(&format!("  Could not read working directory: {cwd_read_error}\n"));
                        absolute_path.display().to_string()
                    }
                }
            }
        }
    }
}

/// Whether to show the selected repo's path after jump/open.
#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum PathVisibility {
    /// Print the path after selection.
    Show,
    /// Do not print the path after selection.
    Hide,
}

/// Returns the default editor command when none is configured.
pub fn default_editor_command() -> String {
    "cursor".to_owned()
}

/// Returns the default path visibility (show).
pub fn default_path_visibility() -> PathVisibility {
    PathVisibility::Show
}

/// Returns the default path display format (absolute).
pub fn default_path_display_format() -> PathDisplayFormat {
    PathDisplayFormat::Absolute
}

/// Returns the default workspace root when none is configured.
///
/// Empty string means the runtime resolver falls back to the
/// computed default `<default_clone_root>/workspaces`, or
/// `~/dev/workspaces` when no clone root is set. Storing empty
/// here instead of a concrete path keeps the serialized config
/// stable across machines with different home directories.
pub fn default_workspace_root_value() -> String {
    String::new()
}

/// Global settings that apply across all repos, stored in config.toml.
#[derive(Debug, Deserialize, Serialize)]
pub struct ReportalSettings {
    /// Which editor command to use when opening repos.
    #[serde(default = "default_editor_command")]
    pub(crate) default_editor: String,
    /// Root directory for cloning new repos into.
    #[serde(default)]
    pub(crate) default_clone_root: String,
    /// Root directory under which workspace directories are
    /// materialized (one directory per registered workspace,
    /// containing a `.code-workspace` file and one symlink /
    /// junction per member repo).
    ///
    /// Empty falls back to `<default_clone_root>/workspaces` at
    /// resolution time, or `~/dev/workspaces` when no clone root
    /// is set. Supports `~` expansion like `default_clone_root`.
    #[serde(default = "default_workspace_root_value")]
    pub(crate) default_workspace_root: String,
    /// Whether to print the path after selecting a repo in jump/open.
    #[serde(default = "default_path_visibility")]
    pub(crate) path_on_select: PathVisibility,
    /// How to format paths when displayed: absolute or relative.
    #[serde(default = "default_path_display_format")]
    pub(crate) path_display_format: PathDisplayFormat,
    /// Which AI tool to launch by default when no --tool flag is given.
    #[serde(default)]
    pub(crate) default_ai_tool: String,
}

/// Provides sensible defaults for a fresh config with no `[settings]` section.
impl Default for ReportalSettings {
    fn default() -> Self {
        Self {
            default_editor: default_editor_command(),
            default_clone_root: String::new(),
            default_workspace_root: default_workspace_root_value(),
            path_on_select: default_path_visibility(),
            path_display_format: default_path_display_format(),
            default_ai_tool: String::new(),
        }
    }
}
