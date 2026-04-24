//! A single registered repository with its metadata and accessors.

use crate::reportal_config::command_entry::CommandEntry;
use crate::reportal_config::has_aliases::HasAliases;
use crate::reportal_config::repository_color::RepoColor;
use crate::reportal_config::tab_title::TabTitle;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::PathBuf;

/// A single registered repository with its metadata.
///
/// Fields are `pub(super)` rather than fully private so the
/// sibling `repo_registration_builder` module can construct a
/// `RepoEntry` after running its own validation, without forcing
/// a constructor-params indirection. Callers outside
/// `reportal_config` still only see the accessor and setter
/// surface, so Design B's invariants are preserved: no external
/// code can bypass validation to land a malformed entry.
#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct RepoEntry {
    /// Filesystem path to the repo, may contain `~` for home.
    pub(super) path: String,
    /// Human-readable description of what this repo is.
    #[serde(default)]
    pub(super) description: String,
    /// Tags for filtering and grouping repos.
    #[serde(default)]
    pub(super) tags: Vec<String>,
    /// Git remote URL for cloning on other machines.
    #[serde(default)]
    pub(super) remote: String,
    /// Alternative names that can be used to jump to this repo directly.
    #[serde(default)]
    pub(super) aliases: Vec<String>,
    /// Custom tab title shown in the terminal when jumping to this repo.
    #[serde(default)]
    pub(super) title: TabTitle,
    /// Terminal background color applied via OSC 11 when jumping to this repo.
    #[serde(default)]
    pub(super) color: RepoColor,
    /// Per-repo commands: same table format as global commands.
    #[serde(default)]
    pub(super) commands: BTreeMap<String, CommandEntry>,
}

/// Accessors and mutators for a registered repository entry.
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

    /// Git remote URL for cloning on other machines (may be empty).
    pub fn remote(&self) -> &str {
        &self.remote
    }

    /// The configured tab title preference for this repo.
    pub fn tab_title(&self) -> &TabTitle {
        &self.title
    }

    /// The configured terminal background color preference for this repo.
    pub fn repo_color(&self) -> &RepoColor {
        &self.color
    }

    /// Per-repo commands (same format as global commands).
    pub fn repo_commands(&self) -> &BTreeMap<String, CommandEntry> {
        &self.commands
    }

    /// Replaces the stored filesystem path.
    ///
    /// Takes the raw, unexpanded string because the config stores
    /// tilde-prefixed paths verbatim and the resolver in
    /// `resolved_path` handles expansion at query time. Callers
    /// that change a repo's path MUST regenerate every
    /// `.code-workspace` file that references this repo afterward,
    /// or Design B's invariant — that moving a repo updates every
    /// containing workspace — breaks and editor sessions open
    /// against stale locations.
    pub fn set_raw_path(&mut self, new_raw_path: String) {
        self.path = new_raw_path;
    }

    /// Replaces the description text.
    pub fn set_description(&mut self, new_description: String) {
        self.description = new_description;
    }

    /// Replaces the tag list.
    pub fn set_tags(&mut self, new_tags: Vec<String>) {
        self.tags = new_tags;
    }

    /// Replaces the tab title preference.
    pub fn set_tab_title(&mut self, new_title: TabTitle) {
        self.title = new_title;
    }

    /// Replaces the terminal background color preference.
    pub fn set_repo_color(&mut self, new_color: RepoColor) {
        self.color = new_color;
    }
}

impl HasAliases for RepoEntry {
    fn aliases(&self) -> &[String] {
        &self.aliases
    }
}
