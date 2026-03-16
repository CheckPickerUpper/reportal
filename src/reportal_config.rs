/// Configuration loading, saving, and repo registry for RePortal.
///
/// The config file lives at `~/.reportal/config.toml` and stores
/// all registered repositories with their metadata.

use crate::error::ReportalError;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::PathBuf;

/// Whether to filter repos by a specific tag or show all repos.
#[derive(Debug)]
pub enum TagFilter {
    /// Show every registered repo regardless of tags.
    All,
    /// Show only repos that carry this exact tag string.
    ByTag(String),
}

/// Top-level config structure deserialized from `config.toml`.
#[derive(Debug, Deserialize, Serialize)]
pub struct ReportalConfig {
    /// Global settings like default editor and clone root.
    #[serde(default)]
    settings: ReportalSettings,
    /// Map of alias to repo definition.
    #[serde(default)]
    repos: BTreeMap<String, RepoEntry>,
}

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
                return absolute_path.display().to_string();
            }
            PathDisplayFormat::Relative => {
                let current_directory = std::env::current_dir();
                match current_directory {
                    Ok(working_directory) => {
                        let relative_result = pathdiff::diff_paths(absolute_path, &working_directory);
                        match relative_result {
                            Some(relative_path) => return relative_path.display().to_string(),
                            None => return absolute_path.display().to_string(),
                        }
                    }
                    Err(cwd_read_error) => {
                        eprintln!("  Could not read working directory: {cwd_read_error}");
                        return absolute_path.display().to_string();
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

/// Global settings that apply across all repos, stored in config.toml.
#[derive(Debug, Deserialize, Serialize)]
pub struct ReportalSettings {
    /// Which editor command to use when opening repos.
    #[serde(default = "default_editor_command")]
    default_editor: String,
    /// Root directory for cloning new repos into.
    #[serde(default)]
    default_clone_root: String,
    /// Whether to print the path after selecting a repo in jump/open.
    #[serde(default = "default_path_visibility")]
    path_on_select: PathVisibility,
    /// How to format paths when displayed: absolute or relative.
    #[serde(default = "default_path_display_format")]
    path_display_format: PathDisplayFormat,
}

/// Step-by-step builder for registering a new repo.
///
/// Validates that alias and path are non-empty and the path exists
/// on disk before allowing `build()` to produce a valid registration.
pub struct RepoRegistrationBuilder {
    /// Alias collected from the user.
    alias: String,
    /// Filesystem path collected from the user.
    repo_path: String,
    /// Description collected from the user.
    repo_description: String,
    /// Tags collected from the user.
    repo_tags: Vec<String>,
    /// Git remote URL collected from the user.
    repo_remote: String,
}

/// A single registered repository with its metadata.
#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct RepoEntry {
    /// Filesystem path to the repo, may contain `~` for home.
    path: String,
    /// Human-readable description of what this repo is.
    #[serde(default)]
    description: String,
    /// Tags for filtering and grouping repos.
    #[serde(default)]
    tags: Vec<String>,
    /// Git remote URL for cloning on other machines.
    #[serde(default)]
    remote: String,
}

/// Returns the default editor command when none is configured.
fn default_editor_command() -> String {
    "cursor".to_string()
}

/// Returns the default path visibility (show).
fn default_path_visibility() -> PathVisibility {
    PathVisibility::Show
}

/// Returns the default path display format (absolute).
fn default_path_display_format() -> PathDisplayFormat {
    PathDisplayFormat::Absolute
}

/// Resolves the user home directory or returns an error if unavailable.
fn resolve_home_directory() -> Result<PathBuf, ReportalError> {
    dirs::home_dir().ok_or(ReportalError::ConfigIoFailure {
        reason: "Could not determine home directory".to_string(),
    })
}

impl Default for ReportalSettings {
    fn default() -> Self {
        Self {
            default_editor: default_editor_command(),
            default_clone_root: String::new(),
            path_on_select: default_path_visibility(),
            path_display_format: default_path_display_format(),
        }
    }
}

/// Chainable builder methods for constructing a validated repo registration.
impl RepoRegistrationBuilder {
    /// Starts building a registration with the given alias.
    pub fn start(alias: String) -> Self {
        Self {
            alias,
            repo_path: String::new(),
            repo_description: String::new(),
            repo_tags: Vec::new(),
            repo_remote: String::new(),
        }
    }

    /// Sets the filesystem path for this repo.
    pub fn repo_path(mut self, filesystem_path: String) -> Self {
        self.repo_path = filesystem_path;
        self
    }

    /// Sets the human-readable description.
    pub fn repo_description(mut self, description_text: String) -> Self {
        self.repo_description = description_text;
        self
    }

    /// Sets the tags for filtering and grouping.
    pub fn repo_tags(mut self, tag_list: Vec<String>) -> Self {
        self.repo_tags = tag_list;
        self
    }

    /// Sets the git remote URL for cloning on other machines.
    pub fn repo_remote(mut self, remote_url: String) -> Self {
        self.repo_remote = remote_url;
        self
    }

    /// Validates all fields and produces a repo alias + entry pair.
    ///
    /// Rejects empty aliases, empty paths, and paths that do not
    /// exist on disk after tilde expansion.
    pub fn build(self) -> Result<(String, RepoEntry), ReportalError> {
        if self.alias.trim().is_empty() {
            return Err(ReportalError::ValidationFailure {
                field: "alias".to_string(),
                reason: "must not be empty".to_string(),
            });
        }
        if self.repo_path.trim().is_empty() {
            return Err(ReportalError::ValidationFailure {
                field: "path".to_string(),
                reason: "must not be empty".to_string(),
            });
        }
        let expanded_path = shellexpand::tilde(&self.repo_path);
        let resolved = PathBuf::from(expanded_path.as_ref());
        if !resolved.exists() {
            return Err(ReportalError::ValidationFailure {
                field: "path".to_string(),
                reason: format!("{} does not exist", resolved.display()),
            });
        }
        let validated_entry = RepoEntry {
            path: self.repo_path,
            description: self.repo_description,
            tags: self.repo_tags,
            remote: self.repo_remote,
        };
        Ok((self.alias, validated_entry))
    }
}

/// Accessors and path resolution for a registered repository entry.
impl RepoEntry {
    /// Expands `~` in the stored path and returns the absolute filesystem path.
    pub fn resolved_path(&self) -> PathBuf {
        let expanded = shellexpand::tilde(&self.path);
        PathBuf::from(expanded.as_ref())
    }

    /// The raw path string as stored in config, before tilde expansion.
    pub fn raw_path(&self) -> &str {
        &self.path
    }

    /// Human-readable description of this repo's purpose.
    pub fn description(&self) -> &str {
        &self.description
    }

    /// Tags assigned to this repo for filtering and grouping.
    pub fn tags(&self) -> &[String] {
        &self.tags
    }

}

/// Loading, saving, querying, and mutating the RePortal config file.
impl ReportalConfig {
    /// Returns the directory where RePortal stores its config.
    pub fn config_directory() -> Result<PathBuf, ReportalError> {
        Ok(resolve_home_directory()?.join(".reportal"))
    }

    /// Returns the full path to the config TOML file.
    pub fn config_file_path() -> Result<PathBuf, ReportalError> {
        Ok(Self::config_directory()?.join("config.toml"))
    }

    /// Loads and parses the config from disk.
    ///
    /// Returns `ConfigNotFound` if the file does not exist,
    /// or `ConfigParseFailure` if the TOML is malformed.
    pub fn load_from_disk() -> Result<Self, ReportalError> {
        let file_path = Self::config_file_path()?;
        if !file_path.exists() {
            return Err(ReportalError::ConfigNotFound {
                config_path: file_path.display().to_string(),
            });
        }
        let toml_content = std::fs::read_to_string(&file_path).map_err(|io_error| {
            ReportalError::ConfigIoFailure {
                reason: io_error.to_string(),
            }
        })?;
        toml::from_str(&toml_content).map_err(|parse_error| ReportalError::ConfigParseFailure {
            reason: parse_error.to_string(),
        })
    }

    /// Serializes and writes the config to disk, creating the directory if needed.
    pub fn save_to_disk(&self) -> Result<(), ReportalError> {
        let file_path = Self::config_file_path()?;
        let parent_directory = Self::config_directory()?;
        if !parent_directory.exists() {
            std::fs::create_dir_all(&parent_directory).map_err(|io_error| {
                ReportalError::ConfigIoFailure {
                    reason: io_error.to_string(),
                }
            })?;
        }
        let serialized_toml =
            toml::to_string_pretty(self).map_err(|serialize_error| {
                ReportalError::ConfigIoFailure {
                    reason: serialize_error.to_string(),
                }
            })?;
        std::fs::write(&file_path, serialized_toml).map_err(|io_error| {
            ReportalError::ConfigIoFailure {
                reason: io_error.to_string(),
            }
        })?;
        Ok(())
    }

    /// Returns repos whose tags match the given filter, with their aliases.
    pub fn repos_matching_tag_filter(&self, tag_filter: &TagFilter) -> Vec<(&String, &RepoEntry)> {
        self.repos
            .iter()
            .filter(|(_, repo)| match tag_filter {
                TagFilter::All => true,
                TagFilter::ByTag(target_tag) => repo.tags.iter().any(|tag| tag == target_tag),
            })
            .collect()
    }

    /// Returns the configured default editor command.
    pub fn default_editor(&self) -> &str {
        &self.settings.default_editor
    }

    /// Whether to show the path after selecting a repo.
    pub fn path_on_select(&self) -> &PathVisibility {
        &self.settings.path_on_select
    }

    /// How to format displayed paths (absolute or relative).
    pub fn path_display_format(&self) -> &PathDisplayFormat {
        &self.settings.path_display_format
    }

    /// Looks up a repo by its alias. Returns `RepoNotFound` if not registered.
    pub fn get_repo(&self, alias: &str) -> Result<&RepoEntry, ReportalError> {
        self.repos.get(alias).ok_or_else(|| ReportalError::RepoNotFound {
            alias: alias.to_string(),
        })
    }

    /// Registers a new repo from a validated builder result.
    ///
    /// Returns `RepoAlreadyExists` if the alias is already taken.
    pub fn add_repo(&mut self, validated_registration: (String, RepoEntry)) -> Result<(), ReportalError> {
        let (repo_alias, repo_entry) = validated_registration;
        if self.repos.contains_key(&repo_alias) {
            return Err(ReportalError::RepoAlreadyExists {
                alias: repo_alias,
            });
        }
        self.repos.insert(repo_alias, repo_entry);
        Ok(())
    }

    /// Removes a repo by alias. Returns `RepoNotFound` if missing.
    pub fn remove_repo(&mut self, alias: &str) -> Result<RepoEntry, ReportalError> {
        self.repos.remove(alias).ok_or_else(|| ReportalError::RepoNotFound {
            alias: alias.to_string(),
        })
    }

    /// Creates a default empty config with sensible defaults for first-time setup.
    pub fn create_default() -> Self {
        Self {
            settings: ReportalSettings {
                default_editor: "cursor".to_string(),
                default_clone_root: "~/dev".to_string(),
                path_on_select: PathVisibility::Show,
                path_display_format: PathDisplayFormat::Absolute,
            },
            repos: BTreeMap::new(),
        }
    }
}
