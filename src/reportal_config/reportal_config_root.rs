//! Top-level `RePortal` config: load/save, repo queries, AI tool registry,
//! and settings mutation.

use crate::error::ReportalError;
use crate::reportal_config::ai_tool_entry::AiToolEntry;
use crate::reportal_config::command_entry::CommandEntry;
use crate::reportal_config::global_settings::{PathDisplayFormat, PathVisibility, ReportalSettings};
use crate::reportal_config::repo_entry::RepoEntry;
use crate::reportal_config::tag_filter::TagFilter;
use crate::reportal_config::workspace_entry::WorkspaceEntry;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::PathBuf;

/// Resolves the user home directory or returns an error if unavailable.
fn resolve_home_directory() -> Result<PathBuf, ReportalError> {
    dirs::home_dir().ok_or(ReportalError::ConfigIoFailure {
        reason: "Could not determine home directory".to_owned(),
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
    /// Map of command name to user-defined command definition.
    #[serde(default)]
    commands: BTreeMap<String, CommandEntry>,
    /// Map of workspace name to VSCode/Cursor workspace definition.
    ///
    /// Each entry is the single source of truth for one
    /// `.code-workspace` file, which reportal generates from the
    /// member repos' current paths so that moving a repo updates
    /// every workspace that contains it.
    #[serde(default)]
    workspaces: BTreeMap<String, WorkspaceEntry>,
}

/// Loading, saving, querying, and mutating the `RePortal` config file.
impl ReportalConfig {
    /// Returns the directory where `RePortal` stores its config.
    pub fn config_directory() -> Result<PathBuf, ReportalError> {
        Ok(resolve_home_directory()?.join(".reportal"))
    }

    /// Returns the full path to the config TOML file.
    pub fn config_file_path() -> Result<PathBuf, ReportalError> {
        Ok(Self::config_directory()?.join("config.toml"))
    }

    /// Loads, parses, and validates the config from disk.
    ///
    /// After successful TOML parsing, runs the workspace reference
    /// check so any workspace that points at a repo no longer
    /// registered is rejected at startup rather than silently
    /// producing broken `.code-workspace` files on a later regen.
    ///
    /// # Errors
    ///
    /// Returns `ConfigNotFound` if the file does not exist,
    /// `ConfigIoFailure` if the file cannot be read,
    /// `ConfigParseFailure` if the TOML is malformed, or
    /// `WorkspaceHasDanglingRepo` if a workspace references an
    /// unknown repo alias.
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
        let parsed_config: Self = toml::from_str(&toml_content).map_err(|parse_error| {
            ReportalError::ConfigParseFailure {
                reason: parse_error.to_string(),
            }
        })?;
        parsed_config.validate_workspace_references()?;
        Ok(parsed_config)
    }

    /// Validates that every workspace's repo alias list references
    /// only registered repos.
    ///
    /// Runs on every config load so a hand-edited TOML that drops a
    /// repo without cleaning up workspace membership is rejected at
    /// startup. Prevents the failure mode where `.code-workspace`
    /// files would later be regenerated pointing at a non-existent
    /// repo path. Uses the canonical repo registry keys only, not
    /// the repo `aliases` field, because workspace membership is
    /// stored against the canonical key.
    ///
    /// # Errors
    ///
    /// Returns [`ReportalError::WorkspaceHasDanglingRepo`] at the
    /// first dangling reference encountered, identifying both the
    /// workspace and the missing alias so the user can fix either
    /// side of the broken reference.
    pub fn validate_workspace_references(&self) -> Result<(), ReportalError> {
        let all_pairs = self.workspaces.iter().flat_map(|(workspace_name, workspace)| {
            workspace
                .repo_aliases()
                .iter()
                .map(move |member_alias| (workspace_name, member_alias))
        });
        for (workspace_name, member_alias) in all_pairs {
            if !self.repos.contains_key(member_alias) {
                return Err(ReportalError::WorkspaceHasDanglingRepo {
                    workspace_name: workspace_name.to_owned(),
                    missing_alias: member_alias.to_owned(),
                });
            }
        }
        Ok(())
    }

    /// Serializes and writes the config to disk, creating the directory if needed.
    pub fn save_to_disk(&self) -> Result<(), ReportalError> {
        let file_path = Self::config_file_path()?;
        let parent_directory = Self::config_directory()?;
        if !parent_directory.exists() {
            std::fs::create_dir_all(&parent_directory).map_err(|io_error| ReportalError::ConfigIoFailure {
                reason: io_error.to_string(),
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
        if let Some(found_repo) = self.repos.get(alias) {
            return Ok(found_repo);
        }
        let alias_match = self.repos.values()
            .find(|repo_entry| repo_entry.aliases().iter().any(|alt| alt == alias));
        alias_match.map_or_else(
            || Err(ReportalError::RepoNotFound {
                alias: alias.to_owned(),
            }),
            Ok,
        )
    }

    /// Returns a mutable reference to a repo by its primary key.
    /// Returns `RepoNotFound` if the alias is not a primary key.
    pub fn get_repo_mut(&mut self, alias: &str) -> Result<&mut RepoEntry, ReportalError> {
        self.repos.get_mut(alias).ok_or_else(|| ReportalError::RepoNotFound {
            alias: alias.to_owned(),
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
            alias: alias.to_owned(),
        })
    }

    /// Returns all registered workspaces with their names, for iteration.
    ///
    /// The list reflects the `BTreeMap` iteration order so display
    /// and tree rendering are deterministic across runs.
    #[must_use]
    pub fn workspaces_with_names(&self) -> Vec<(&String, &WorkspaceEntry)> {
        self.workspaces.iter().collect()
    }

    /// Looks up a workspace by name.
    ///
    /// # Errors
    ///
    /// Returns [`ReportalError::WorkspaceNotFound`] if the name has
    /// no matching workspace entry.
    pub fn get_workspace(&self, workspace_name: &str) -> Result<&WorkspaceEntry, ReportalError> {
        self.workspaces.get(workspace_name).ok_or_else(|| ReportalError::WorkspaceNotFound {
            workspace_name: workspace_name.to_owned(),
        })
    }

    /// Returns a mutable reference to a workspace by name.
    ///
    /// # Errors
    ///
    /// Returns [`ReportalError::WorkspaceNotFound`] if the name has
    /// no matching workspace entry.
    pub fn get_workspace_mut(
        &mut self,
        workspace_name: &str,
    ) -> Result<&mut WorkspaceEntry, ReportalError> {
        self.workspaces.get_mut(workspace_name).ok_or_else(|| ReportalError::WorkspaceNotFound {
            workspace_name: workspace_name.to_owned(),
        })
    }

    /// Registers a new workspace from a validated builder result.
    ///
    /// Runs the dangling-reference check before accepting the new
    /// entry so an insert that would leave the config invalid is
    /// rejected at mutation time, not at the next load.
    ///
    /// # Errors
    ///
    /// Returns [`ReportalError::WorkspaceAlreadyExists`] if the
    /// name is taken, or [`ReportalError::WorkspaceHasDanglingRepo`]
    /// if any of the new entry's repo aliases is not registered.
    pub fn add_workspace(
        &mut self,
        validated_registration: (String, WorkspaceEntry),
    ) -> Result<(), ReportalError> {
        let (workspace_name, workspace_entry) = validated_registration;
        if self.workspaces.contains_key(&workspace_name) {
            return Err(ReportalError::WorkspaceAlreadyExists {
                workspace_name,
            });
        }
        for member_alias in workspace_entry.repo_aliases() {
            if !self.repos.contains_key(member_alias) {
                return Err(ReportalError::WorkspaceHasDanglingRepo {
                    workspace_name,
                    missing_alias: member_alias.to_owned(),
                });
            }
        }
        self.workspaces.insert(workspace_name, workspace_entry);
        Ok(())
    }

    /// Removes a workspace by name.
    ///
    /// # Errors
    ///
    /// Returns [`ReportalError::WorkspaceNotFound`] if the name has
    /// no matching workspace entry.
    pub fn remove_workspace(
        &mut self,
        workspace_name: &str,
    ) -> Result<WorkspaceEntry, ReportalError> {
        self.workspaces.remove(workspace_name).ok_or_else(|| ReportalError::WorkspaceNotFound {
            workspace_name: workspace_name.to_owned(),
        })
    }

    /// Returns every workspace that contains the given repo alias.
    ///
    /// This is the reverse index that makes repo path changes
    /// correct: when `rep edit` changes a repo's path, every
    /// workspace containing that repo must regenerate its
    /// `.code-workspace` file so the folder entries stay aligned
    /// with reality. Without this lookup, path changes silently
    /// strand workspace files pointing at the old location.
    #[must_use]
    pub fn workspaces_containing_repo(
        &self,
        repo_alias: &str,
    ) -> Vec<(&String, &WorkspaceEntry)> {
        self.workspaces
            .iter()
            .filter(|(_, workspace)| workspace.contains_repo(repo_alias))
            .collect()
    }

    /// Returns the configured default AI tool name, if set.
    pub fn default_ai_tool(&self) -> &str {
        &self.settings.default_ai_tool
    }

    /// Looks up an AI tool by name. Returns `AiToolNotFound` if missing.
    pub fn get_ai_tool(&self, tool_name: &str) -> Result<&AiToolEntry, ReportalError> {
        self.ai_tools.get(tool_name).ok_or_else(|| ReportalError::AiToolNotFound {
            tool_name: tool_name.to_owned(),
        })
    }

    /// Returns all registered AI tools with their names.
    pub fn ai_tools_list(&self) -> Vec<(&String, &AiToolEntry)> {
        self.ai_tools.iter().collect()
    }

    /// Returns all globally registered commands with their names.
    pub fn global_commands(&self) -> &BTreeMap<String, CommandEntry> {
        &self.commands
    }

    /// Creates a default empty config with sensible defaults for first-time setup.
    pub fn create_default() -> Self {
        Self {
            settings: ReportalSettings {
                default_editor: "cursor".to_owned(),
                default_clone_root: "~/dev".to_owned(),
                path_on_select: PathVisibility::Show,
                path_display_format: PathDisplayFormat::Absolute,
                default_ai_tool: "claude".to_owned(),
            },
            repos: BTreeMap::new(),
            commands: BTreeMap::new(),
            workspaces: BTreeMap::new(),
            ai_tools: BTreeMap::from([
                ("claude".to_owned(), AiToolEntry::with_executable("claude".to_owned())),
                ("codex".to_owned(), AiToolEntry::with_executable("codex".to_owned())),
                ("aider".to_owned(), AiToolEntry::with_executable("aider".to_owned())),
            ]),
        }
    }
}
