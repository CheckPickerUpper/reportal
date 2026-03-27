/// Repository entry types: `RepoEntry`, `RepoRegistrationBuilder`,
/// `TabTitle`, and `RepoColor`.

use crate::error::ReportalError;
use crate::reportal_config::command_entry::CommandEntry;
use crate::reportal_config::hex_color::HexColor;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::PathBuf;

/// Whether a repo has a custom tab title or falls back to its alias.
#[derive(Debug, Serialize, Clone)]
#[serde(untagged)]
pub enum TabTitle {
    /// No custom title configured; the repo alias is used instead.
    UseAlias,
    /// A custom title the user chose for this repo's terminal tab.
    Custom(String),
}

/// Defaults to using the repo alias as the tab title.
impl Default for TabTitle {
    fn default() -> Self {
        TabTitle::UseAlias
    }
}

/// Deserializes an empty string as `UseAlias`, non-empty as `Custom`.
impl<'de> Deserialize<'de> for TabTitle {
    fn deserialize<D: serde::Deserializer<'de>>(tab_title_deserializer: D) -> Result<Self, D::Error> {
        let raw: String = String::deserialize(tab_title_deserializer)?;
        match raw.is_empty() {
            true => return Ok(TabTitle::UseAlias),
            false => return Ok(TabTitle::Custom(raw)),
        }
    }
}

/// Whether a repo has a terminal background color configured.
#[derive(Debug, Serialize, Clone)]
#[serde(untagged)]
pub enum RepoColor {
    /// No color set; the terminal resets to its default background on jump.
    ResetToDefault,
    /// A specific background color applied via OSC 11 on jump.
    Themed(HexColor),
}

/// Defaults to resetting the terminal background color.
impl Default for RepoColor {
    fn default() -> Self {
        RepoColor::ResetToDefault
    }
}

/// Deserializes an empty string as `ResetToDefault`, valid hex as `Themed`.
impl<'de> Deserialize<'de> for RepoColor {
    fn deserialize<D: serde::Deserializer<'de>>(repo_color_deserializer: D) -> Result<Self, D::Error> {
        let raw: String = String::deserialize(repo_color_deserializer)?;
        match raw.is_empty() {
            true => return Ok(RepoColor::ResetToDefault),
            false => {
                let parsed = HexColor::parse(&raw).map_err(serde::de::Error::custom)?;
                return Ok(RepoColor::Themed(parsed));
            }
        }
    }
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
    /// Alternative names that can be used to jump to this repo directly.
    #[serde(default)]
    aliases: Vec<String>,
    /// Custom tab title shown in the terminal when jumping to this repo.
    #[serde(default)]
    title: TabTitle,
    /// Terminal background color applied via OSC 11 when jumping to this repo.
    #[serde(default)]
    color: RepoColor,
    /// Per-repo commands: same table format as global commands.
    #[serde(default)]
    commands: BTreeMap<String, CommandEntry>,
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

    /// Alternative names that can be used to jump to this repo directly.
    pub fn aliases(&self) -> &[String] {
        &self.aliases
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
    /// Custom tab title collected from the user.
    repo_title: TabTitle,
    /// Background color collected from the user.
    repo_color: RepoColor,
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
            repo_title: TabTitle::UseAlias,
            repo_color: RepoColor::ResetToDefault,
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

    /// Sets a custom terminal tab title for this repo.
    pub fn repo_title(mut self, title_text: String) -> Self {
        self.repo_title = TabTitle::Custom(title_text);
        self
    }

    /// Sets the terminal background color for this repo.
    pub fn repo_color(mut self, hex_color: HexColor) -> Self {
        self.repo_color = RepoColor::Themed(hex_color);
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
            aliases: Vec::new(),
            title: self.repo_title,
            color: self.repo_color,
            commands: BTreeMap::new(),
        };
        Ok((self.alias, validated_entry))
    }
}
