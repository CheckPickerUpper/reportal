//! Top-level `RePortal` config: load/save, repo queries, AI tool registry,
//! and settings mutation.

use crate::error::ReportalError;
use crate::reportal_config::ai_tool_entry::AiToolEntry;
use crate::reportal_config::alias_collision_query::AliasCollisionQuery;
use crate::reportal_config::command_entry::CommandEntry;
use crate::reportal_config::global_settings::{PathDisplayFormat, PathVisibility, ReportalSettings};
use crate::reportal_config::has_aliases::{HasAliases, resolve_canonical_key};
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

    /// Loads, parses, and validates the config from disk, creating
    /// a default config if none exists yet.
    ///
    /// The bootstrap-on-read behavior removes the "you must run
    /// `rep init` first" failure mode from every subcommand: a
    /// fresh install or a wiped home directory transparently gets
    /// a minimal working config on the first invocation, matching
    /// how tools like `zoxide` and `starship` behave.
    ///
    /// After successful TOML parsing, runs the workspace reference
    /// check and the alias-collision pass so no dangling member or
    /// ambiguous short name reaches command dispatch.
    ///
    /// # Errors
    ///
    /// Returns `ConfigIoFailure` if the file cannot be read or the
    /// default config cannot be written, `ConfigParseFailure` if
    /// the TOML is malformed, `WorkspaceHasDanglingRepo` if a
    /// workspace references an unknown repo alias, or
    /// `WorkspaceAliasConflict` if any workspace's name or alias
    /// collides with another entry.
    pub fn load_or_initialize() -> Result<Self, ReportalError> {
        let file_path = Self::config_file_path()?;
        if !file_path.exists() {
            let default_config = Self::create_default();
            default_config.save_to_disk()?;
            return Ok(default_config);
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
        parsed_config.validate_alias_collisions()?;
        Ok(parsed_config)
    }

    /// Validates that every workspace alias is globally unambiguous
    /// across both the workspace and repo namespaces.
    ///
    /// The resolver in `resolve_canonical_key` returns the first
    /// match it finds, so a config where two entries declare the
    /// same alias would produce silently order-dependent behavior.
    /// Rejecting the config at load time surfaces the ambiguity
    /// immediately instead of letting a command resolve `vn` to the
    /// wrong target.
    ///
    /// # Errors
    ///
    /// Returns [`ReportalError::WorkspaceAliasConflict`] at the
    /// first collision encountered.
    pub fn validate_alias_collisions(&self) -> Result<(), ReportalError> {
        for (workspace_name, workspace_entry) in &self.workspaces {
            self.check_workspace_canonical_name_repo_collision(workspace_name)?;
            for declared_alias in workspace_entry.aliases() {
                self.check_workspace_alias_collisions(AliasCollisionQuery::new(
                    workspace_name,
                    declared_alias,
                ))?;
            }
        }
        Ok(())
    }

    /// Rejects a workspace whose canonical name collides with a
    /// repo's canonical key or declared alias.
    ///
    /// # Errors
    ///
    /// Returns [`ReportalError::WorkspaceAliasConflict`] on collision.
    fn check_workspace_canonical_name_repo_collision(
        &self,
        workspace_name: &str,
    ) -> Result<(), ReportalError> {
        if self.repos.contains_key(workspace_name) {
            return Err(ReportalError::WorkspaceAliasConflict {
                workspace_name: workspace_name.to_owned(),
                conflicting_value: workspace_name.to_owned(),
                conflicting_entity_description: format!(
                    "repo '{workspace_name}' as its canonical key"
                ),
            });
        }
        for (repo_key, repo_entry) in &self.repos {
            if repo_entry.aliases().iter().any(|declared| declared == workspace_name) {
                return Err(ReportalError::WorkspaceAliasConflict {
                    workspace_name: workspace_name.to_owned(),
                    conflicting_value: workspace_name.to_owned(),
                    conflicting_entity_description: format!("repo '{repo_key}' as an alias"),
                });
            }
        }
        Ok(())
    }

    /// Rejects a workspace alias that collides with another
    /// workspace's canonical name, another workspace's alias, any
    /// repo's canonical key, or any repo's declared alias.
    ///
    /// # Errors
    ///
    /// Returns [`ReportalError::WorkspaceAliasConflict`] on collision.
    fn check_workspace_alias_collisions(
        &self,
        query: AliasCollisionQuery<'_>,
    ) -> Result<(), ReportalError> {
        let owning_workspace_name = query.owning_workspace_name();
        let candidate_alias = query.candidate_alias();
        for (other_workspace_name, other_workspace) in &self.workspaces {
            if other_workspace_name == owning_workspace_name {
                continue;
            }
            if other_workspace_name == candidate_alias {
                return Err(ReportalError::WorkspaceAliasConflict {
                    workspace_name: owning_workspace_name.to_owned(),
                    conflicting_value: candidate_alias.to_owned(),
                    conflicting_entity_description: format!(
                        "workspace '{other_workspace_name}' as its canonical name"
                    ),
                });
            }
            if other_workspace
                .aliases()
                .iter()
                .any(|declared| declared == candidate_alias)
            {
                return Err(ReportalError::WorkspaceAliasConflict {
                    workspace_name: owning_workspace_name.to_owned(),
                    conflicting_value: candidate_alias.to_owned(),
                    conflicting_entity_description: format!(
                        "workspace '{other_workspace_name}' as an alias"
                    ),
                });
            }
        }
        if self.repos.contains_key(candidate_alias) {
            return Err(ReportalError::WorkspaceAliasConflict {
                workspace_name: owning_workspace_name.to_owned(),
                conflicting_value: candidate_alias.to_owned(),
                conflicting_entity_description: format!(
                    "repo '{candidate_alias}' as its canonical key"
                ),
            });
        }
        for (repo_key, repo_entry) in &self.repos {
            if repo_entry
                .aliases()
                .iter()
                .any(|declared| declared == candidate_alias)
            {
                return Err(ReportalError::WorkspaceAliasConflict {
                    workspace_name: owning_workspace_name.to_owned(),
                    conflicting_value: candidate_alias.to_owned(),
                    conflicting_entity_description: format!("repo '{repo_key}' as an alias"),
                });
            }
        }
        Ok(())
    }

    /// Validates that every workspace's repo alias list references
    /// only registered repos.
    ///
    /// # Errors
    ///
    /// Returns [`ReportalError::WorkspaceHasDanglingRepo`] at the
    /// first dangling reference encountered.
    pub fn validate_workspace_references(&self) -> Result<(), ReportalError> {
        for (workspace_name, workspace) in &self.workspaces {
            for member_alias in workspace.repo_aliases() {
                if !self.repos.contains_key(member_alias) {
                    return Err(ReportalError::WorkspaceHasDanglingRepo {
                        workspace_name: workspace_name.to_owned(),
                        missing_alias: member_alias.to_owned(),
                    });
                }
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

    /// Resolves the default workspace root to an absolute path
    /// with `~` expanded, honoring the fallback chain:
    /// explicit `default_workspace_root` setting →
    /// `<default_clone_root>/workspaces` → `~/dev/workspaces`.
    ///
    /// # Errors
    ///
    /// Returns [`ReportalError::ConfigIoFailure`] if the home
    /// directory is needed for the final fallback and cannot be
    /// resolved.
    pub fn resolve_default_workspace_root(&self) -> Result<PathBuf, ReportalError> {
        let explicit_root = self.settings.default_workspace_root.trim();
        if !explicit_root.is_empty() {
            let expanded = shellexpand::tilde(explicit_root);
            return Ok(PathBuf::from(expanded.as_ref()));
        }
        let clone_root = self.settings.default_clone_root.trim();
        if !clone_root.is_empty() {
            let expanded = shellexpand::tilde(clone_root);
            return Ok(PathBuf::from(expanded.as_ref()).join("workspaces"));
        }
        let home_directory = resolve_home_directory()?;
        Ok(home_directory.join("dev").join("workspaces"))
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

    /// Looks up a repo by its canonical key or any of its declared
    /// aliases.
    ///
    /// # Errors
    ///
    /// Returns [`ReportalError::RepoNotFound`] if neither the
    /// canonical key nor any repo's alias list matches.
    pub fn get_repo(&self, alias_or_canonical: &str) -> Result<&RepoEntry, ReportalError> {
        let canonical_key = resolve_canonical_key(&self.repos, alias_or_canonical).ok_or_else(
            || ReportalError::RepoNotFound {
                alias: alias_or_canonical.to_owned(),
            },
        )?;
        self.repos
            .get(canonical_key)
            .ok_or_else(|| ReportalError::RepoNotFound {
                alias: alias_or_canonical.to_owned(),
            })
    }

    /// Returns a mutable reference to a repo, accepting either the
    /// canonical key or any of its declared aliases.
    ///
    /// # Errors
    ///
    /// Returns [`ReportalError::RepoNotFound`] if neither the
    /// canonical key nor any repo's alias list matches.
    pub fn get_repo_mut(
        &mut self,
        alias_or_canonical: &str,
    ) -> Result<&mut RepoEntry, ReportalError> {
        let canonical_key = resolve_canonical_key(&self.repos, alias_or_canonical)
            .ok_or_else(|| ReportalError::RepoNotFound {
                alias: alias_or_canonical.to_owned(),
            })?
            .to_owned();
        self.repos
            .get_mut(&canonical_key)
            .ok_or_else(|| ReportalError::RepoNotFound {
                alias: alias_or_canonical.to_owned(),
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
    #[must_use]
    pub fn workspaces_with_names(&self) -> Vec<(&String, &WorkspaceEntry)> {
        self.workspaces.iter().collect()
    }

    /// Resolves a user-supplied workspace name-or-alias to its
    /// canonical config key.
    ///
    /// Commands that derive file paths from the name (like
    /// `~/.reportal/workspaces/<name>.code-workspace`) must pass the
    /// canonical key, not the user's alias, otherwise `rep workspace
    /// show vn` would build `vn.code-workspace` instead of
    /// `venoble.code-workspace`.
    ///
    /// # Errors
    ///
    /// Returns [`ReportalError::WorkspaceNotFound`] if the query
    /// matches neither a canonical key nor any workspace's alias list.
    pub fn resolve_workspace_canonical_name(
        &self,
        alias_or_canonical: &str,
    ) -> Result<String, ReportalError> {
        resolve_canonical_key(&self.workspaces, alias_or_canonical)
            .map(str::to_owned)
            .ok_or_else(|| ReportalError::WorkspaceNotFound {
                workspace_name: alias_or_canonical.to_owned(),
            })
    }

    /// Looks up a workspace by canonical key or any declared alias.
    ///
    /// # Errors
    ///
    /// Returns [`ReportalError::WorkspaceNotFound`] if neither
    /// matches.
    pub fn get_workspace(
        &self,
        alias_or_canonical: &str,
    ) -> Result<&WorkspaceEntry, ReportalError> {
        let canonical_key = resolve_canonical_key(&self.workspaces, alias_or_canonical)
            .ok_or_else(|| ReportalError::WorkspaceNotFound {
                workspace_name: alias_or_canonical.to_owned(),
            })?;
        self.workspaces
            .get(canonical_key)
            .ok_or_else(|| ReportalError::WorkspaceNotFound {
                workspace_name: alias_or_canonical.to_owned(),
            })
    }

    /// Returns a mutable reference to a workspace, accepting either
    /// the canonical key or any declared alias.
    ///
    /// # Errors
    ///
    /// Returns [`ReportalError::WorkspaceNotFound`] if neither
    /// matches.
    pub fn get_workspace_mut(
        &mut self,
        alias_or_canonical: &str,
    ) -> Result<&mut WorkspaceEntry, ReportalError> {
        let canonical_key = resolve_canonical_key(&self.workspaces, alias_or_canonical)
            .ok_or_else(|| ReportalError::WorkspaceNotFound {
                workspace_name: alias_or_canonical.to_owned(),
            })?
            .to_owned();
        self.workspaces
            .get_mut(&canonical_key)
            .ok_or_else(|| ReportalError::WorkspaceNotFound {
                workspace_name: alias_or_canonical.to_owned(),
            })
    }

    /// Registers a new workspace from a validated builder result.
    ///
    /// Runs dangling-reference and alias-collision checks before
    /// accepting the new entry so an insert that would leave the
    /// config invalid is rejected at mutation time, not at the next
    /// load.
    ///
    /// # Errors
    ///
    /// Returns [`ReportalError::WorkspaceAlreadyExists`],
    /// [`ReportalError::WorkspaceHasDanglingRepo`], or
    /// [`ReportalError::WorkspaceAliasConflict`] depending on which
    /// invariant the new entry violates.
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
        self.check_workspace_canonical_name_repo_collision(&workspace_name)?;
        for declared_alias in workspace_entry.aliases() {
            self.check_workspace_alias_collisions(AliasCollisionQuery::new(
                &workspace_name,
                declared_alias,
            ))?;
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

    /// Returns all registered AI CLI executables with their names,
    /// for iteration.
    pub fn ai_cli_registry(&self) -> Vec<(&String, &AiToolEntry)> {
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
                default_workspace_root: "~/dev/workspaces".to_owned(),
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::reportal_config::workspace_member::WorkspaceMember;
    use crate::reportal_config::workspace_registration_builder::WorkspaceRegistrationBuilder;

    fn make_repo(aliases: Vec<String>) -> RepoEntry {
        RepoEntry {
            path: "C:/fake".to_owned(),
            description: String::new(),
            tags: Vec::new(),
            remote: String::new(),
            aliases,
            title: Default::default(),
            color: Default::default(),
            commands: BTreeMap::new(),
        }
    }

    fn make_workspace(
        member_repo_aliases: Vec<String>,
        declared_aliases: Vec<String>,
    ) -> Result<WorkspaceEntry, ReportalError> {
        let (_, entry) = WorkspaceRegistrationBuilder::start("placeholder".to_owned())
            .repo_aliases(member_repo_aliases)
            .workspace_aliases(declared_aliases)
            .build()?;
        Ok(entry)
    }

    fn config_with_repo_and_workspace(
        repo_canonical: &str,
        repo_aliases: Vec<String>,
        workspace_canonical: &str,
        workspace_aliases: Vec<String>,
    ) -> Result<ReportalConfig, ReportalError> {
        let mut config = ReportalConfig::create_default();
        config.repos.insert(repo_canonical.to_owned(), make_repo(repo_aliases));
        let workspace_entry =
            make_workspace(vec![repo_canonical.to_owned()], workspace_aliases)?;
        config.workspaces.insert(workspace_canonical.to_owned(), workspace_entry);
        Ok(config)
    }

    #[test]
    fn resolve_workspace_canonical_name_matches_alias() -> Result<(), ReportalError> {
        let config = config_with_repo_and_workspace(
            "app",
            vec![],
            "venoble",
            vec!["vn".to_owned(), "noble".to_owned()],
        )?;
        assert_eq!(config.resolve_workspace_canonical_name("vn")?, "venoble");
        assert_eq!(config.resolve_workspace_canonical_name("noble")?, "venoble");
        assert_eq!(config.resolve_workspace_canonical_name("venoble")?, "venoble");
        Ok(())
    }

    #[test]
    fn resolve_workspace_canonical_name_unknown_fails() -> Result<(), ReportalError> {
        let config = config_with_repo_and_workspace("app", vec![], "venoble", vec![])?;
        let outcome = config.resolve_workspace_canonical_name("ghost");
        assert!(matches!(outcome, Err(ReportalError::WorkspaceNotFound { .. })));
        Ok(())
    }

    #[test]
    fn get_workspace_resolves_via_alias() -> Result<(), ReportalError> {
        let config = config_with_repo_and_workspace(
            "app",
            vec![],
            "venoble",
            vec!["vn".to_owned()],
        )?;
        let resolved = config.get_workspace("vn")?;
        assert_eq!(resolved.repo_aliases(), vec!["app"]);
        Ok(())
    }

    #[test]
    fn get_workspace_mut_resolves_via_alias() -> Result<(), ReportalError> {
        let mut config = config_with_repo_and_workspace(
            "app",
            vec![],
            "venoble",
            vec!["vn".to_owned()],
        )?;
        let resolved_mut = config.get_workspace_mut("vn")?;
        resolved_mut.set_members(vec![
            WorkspaceMember::RegisteredRepo("app".to_owned()),
            WorkspaceMember::RegisteredRepo("worker".to_owned()),
        ]);
        let re_read = config.get_workspace("venoble")?;
        assert_eq!(re_read.repo_aliases(), vec!["app", "worker"]);
        Ok(())
    }

    #[test]
    fn get_repo_mut_now_resolves_via_alias() -> Result<(), ReportalError> {
        let mut config = ReportalConfig::create_default();
        config.repos.insert(
            "venoble-app".to_owned(),
            make_repo(vec!["vna".to_owned()]),
        );
        let resolved_mut = config.get_repo_mut("vna")?;
        resolved_mut.set_description("mutated".to_owned());
        assert_eq!(config.get_repo("venoble-app")?.description(), "mutated");
        Ok(())
    }

    #[test]
    fn alias_collision_workspace_alias_equals_peer_workspace_canonical_name(
    ) -> Result<(), ReportalError> {
        let mut config = ReportalConfig::create_default();
        config.repos.insert("app".to_owned(), make_repo(vec![]));
        config.workspaces.insert(
            "venoble".to_owned(),
            make_workspace(vec!["app".to_owned()], vec!["backend".to_owned()])?,
        );
        config.workspaces.insert(
            "backend".to_owned(),
            make_workspace(vec!["app".to_owned()], vec![])?,
        );
        let outcome = config.validate_alias_collisions();
        assert!(matches!(outcome, Err(ReportalError::WorkspaceAliasConflict { .. })));
        Ok(())
    }

    #[test]
    fn alias_collision_workspace_alias_equals_repo_canonical_key(
    ) -> Result<(), ReportalError> {
        let mut config = ReportalConfig::create_default();
        config.repos.insert("app".to_owned(), make_repo(vec![]));
        config.repos.insert("vn".to_owned(), make_repo(vec![]));
        config.workspaces.insert(
            "venoble".to_owned(),
            make_workspace(vec!["app".to_owned()], vec!["vn".to_owned()])?,
        );
        let outcome = config.validate_alias_collisions();
        assert!(matches!(outcome, Err(ReportalError::WorkspaceAliasConflict { .. })));
        Ok(())
    }

    #[test]
    fn alias_collision_workspace_alias_equals_repo_alias() -> Result<(), ReportalError> {
        let mut config = ReportalConfig::create_default();
        config.repos.insert(
            "venoble-app".to_owned(),
            make_repo(vec!["vna".to_owned()]),
        );
        config.workspaces.insert(
            "venoble".to_owned(),
            make_workspace(vec!["venoble-app".to_owned()], vec!["vna".to_owned()])?,
        );
        let outcome = config.validate_alias_collisions();
        assert!(matches!(outcome, Err(ReportalError::WorkspaceAliasConflict { .. })));
        Ok(())
    }

    #[test]
    fn alias_collision_workspace_name_equals_repo_canonical_key(
    ) -> Result<(), ReportalError> {
        let mut config = ReportalConfig::create_default();
        config.repos.insert("venoble".to_owned(), make_repo(vec![]));
        config.repos.insert("app".to_owned(), make_repo(vec![]));
        config.workspaces.insert(
            "venoble".to_owned(),
            make_workspace(vec!["app".to_owned()], vec![])?,
        );
        let outcome = config.validate_alias_collisions();
        assert!(matches!(outcome, Err(ReportalError::WorkspaceAliasConflict { .. })));
        Ok(())
    }

    fn make_inline_only_workspace() -> WorkspaceEntry {
        WorkspaceEntry {
            repos: vec![WorkspaceMember::InlinePath {
                path: "~/dev/inline-only".to_owned(),
            }],
            description: String::new(),
            path: String::new(),
            aliases: Vec::new(),
        }
    }

    #[test]
    fn inline_path_member_skips_dangling_repo_check() {
        let mut config = ReportalConfig::create_default();
        config.workspaces.insert("inline-only".to_owned(), make_inline_only_workspace());
        assert!(config.validate_workspace_references().is_ok());
    }

    #[test]
    fn inline_path_member_does_not_block_repo_removal() {
        let mut config = ReportalConfig::create_default();
        config.repos.insert("standalone".to_owned(), make_repo(vec![]));
        config.workspaces.insert("inline-only".to_owned(), make_inline_only_workspace());
        let containing = config.workspaces_containing_repo("standalone");
        assert!(
            containing.is_empty(),
            "inline-path-only workspace must not appear in the reverse index for an unrelated repo",
        );
    }

    #[test]
    fn clean_config_passes_alias_collision_validation() -> Result<(), ReportalError> {
        let config = config_with_repo_and_workspace(
            "app",
            vec!["a".to_owned()],
            "venoble",
            vec!["vn".to_owned()],
        )?;
        assert!(config.validate_alias_collisions().is_ok());
        Ok(())
    }
}
