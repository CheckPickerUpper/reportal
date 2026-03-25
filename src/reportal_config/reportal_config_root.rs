/// Top-level RePortal config: load/save, repo queries, AI tool registry,
/// and settings mutation.

use crate::error::ReportalError;
use crate::reportal_config::ai_tool_entry::AiToolEntry;
use crate::reportal_config::global_settings::{PathDisplayFormat, PathVisibility, ReportalSettings};
use crate::reportal_config::repo_entry::RepoEntry;
use crate::reportal_config::tag_filter::TagFilter;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::PathBuf;

/// Resolves the user home directory or returns an error if unavailable.
fn resolve_home_directory() -> Result<PathBuf, ReportalError> {
    dirs::home_dir().ok_or(ReportalError::ConfigIoFailure {
        reason: "Could not determine home directory".to_string(),
    })
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
    /// Map of tool name to AI CLI tool definition.
    #[serde(default)]
    ai_tools: BTreeMap<String, AiToolEntry>,
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
                TagFilter::ByTag(target_tag) => repo.tags().iter().any(|tag| tag == target_tag),
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

    /// Returns all registered repos with their aliases, for iteration.
    pub fn repos_with_aliases(&self) -> Vec<(&String, &RepoEntry)> {
        self.repos.iter().collect()
    }

    /// Looks up a repo by its primary key or any of its alternative aliases.
    /// Checks the primary key first, then walks all repos checking their
    /// `aliases` field. Returns `RepoNotFound` if no match is found.
    pub fn get_repo(&self, alias: &str) -> Result<&RepoEntry, ReportalError> {
        match self.repos.get(alias) {
            Some(found_repo) => return Ok(found_repo),
            None => {
                for (_primary_key, repo_entry) in &self.repos {
                    let has_matching_alias = repo_entry.aliases().iter().any(|alt| alt == alias);
                    match has_matching_alias {
                        true => return Ok(repo_entry),
                        false => {}
                    }
                }
                return Err(ReportalError::RepoNotFound {
                    alias: alias.to_string(),
                });
            }
        }
    }

    /// Returns a mutable reference to a repo by its primary key.
    /// Returns `RepoNotFound` if the alias is not a primary key.
    pub fn get_repo_mut(&mut self, alias: &str) -> Result<&mut RepoEntry, ReportalError> {
        self.repos.get_mut(alias).ok_or_else(|| ReportalError::RepoNotFound {
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

    /// Returns the configured default AI tool name, if set.
    pub fn default_ai_tool(&self) -> &str {
        &self.settings.default_ai_tool
    }

    /// Looks up an AI tool by name. Returns `AiToolNotFound` if missing.
    pub fn get_ai_tool(&self, tool_name: &str) -> Result<&AiToolEntry, ReportalError> {
        self.ai_tools.get(tool_name).ok_or_else(|| ReportalError::AiToolNotFound {
            tool_name: tool_name.to_string(),
        })
    }

    /// Returns all registered AI tools with their names.
    pub fn ai_tools_list(&self) -> Vec<(&String, &AiToolEntry)> {
        self.ai_tools.iter().collect()
    }

    /// Inserts or replaces an AI tool entry in the registry.
    /// Takes a (name, entry) pair, same shape as `ai_tools_list()` returns.
    pub fn set_ai_tool(&mut self, registration: (String, AiToolEntry)) {
        let (tool_name, tool_entry) = registration;
        self.ai_tools.insert(tool_name, tool_entry);
    }

    /// Removes an AI tool by name. Returns `AiToolNotFound` if missing.
    pub fn remove_ai_tool(&mut self, tool_name: &str) -> Result<AiToolEntry, ReportalError> {
        self.ai_tools.remove(tool_name).ok_or_else(|| ReportalError::AiToolNotFound {
            tool_name: tool_name.to_string(),
        })
    }

    /// Updates the default editor command in settings.
    pub fn set_default_editor(&mut self, editor_command: String) {
        self.settings.default_editor = editor_command;
    }

    /// Updates the default AI tool name in settings.
    pub fn set_default_ai_tool(&mut self, tool_name: String) {
        self.settings.default_ai_tool = tool_name;
    }

    /// Updates the default clone root path in settings.
    pub fn set_default_clone_root(&mut self, clone_root: String) {
        self.settings.default_clone_root = clone_root;
    }

    /// Returns the configured default clone root path.
    pub fn default_clone_root(&self) -> &str {
        &self.settings.default_clone_root
    }

    /// Creates a default empty config with sensible defaults for first-time setup.
    pub fn create_default() -> Self {
        Self {
            settings: ReportalSettings {
                default_editor: "cursor".to_string(),
                default_clone_root: "~/dev".to_string(),
                path_on_select: PathVisibility::Show,
                path_display_format: PathDisplayFormat::Absolute,
                default_ai_tool: "claude".to_string(),
            },
            repos: BTreeMap::new(),
            ai_tools: BTreeMap::from([
                ("claude".to_string(), AiToolEntry::with_executable("claude".to_string())),
                ("codex".to_string(), AiToolEntry::with_executable("codex".to_string())),
                ("aider".to_string(), AiToolEntry::with_executable("aider".to_string())),
            ]),
        }
    }
}
