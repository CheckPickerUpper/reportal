//! Step-by-step builder for registering a new repo.

use crate::error::ReportalError;
use crate::reportal_config::hex_color::HexColor;
use crate::reportal_config::repo_color::RepoColor;
use crate::reportal_config::repo_entry::RepoEntry;
use crate::reportal_config::tab_title::TabTitle;
use std::collections::BTreeMap;
use std::path::PathBuf;

/// Step-by-step builder for registering a new repo.
///
/// Validates that alias and path are non-empty and the path exists
/// on disk before allowing `build()` to produce a valid registration.
/// Validation runs at `build()` time rather than on each setter
/// because individual setters run mid-chain when the full
/// registration is not yet assembled, so rejecting then would be
/// rejecting an incomplete state rather than an invalid one.
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
    #[must_use]
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
    #[must_use]
    pub fn repo_path(mut self, filesystem_path: String) -> Self {
        self.repo_path = filesystem_path;
        self
    }

    /// Sets the human-readable description.
    #[must_use]
    pub fn repo_description(mut self, description_text: String) -> Self {
        self.repo_description = description_text;
        self
    }

    /// Sets the tags for filtering and grouping.
    #[must_use]
    pub fn repo_tags(mut self, tag_list: Vec<String>) -> Self {
        self.repo_tags = tag_list;
        self
    }

    /// Sets the git remote URL for cloning on other machines.
    #[must_use]
    pub fn repo_remote(mut self, remote_url: String) -> Self {
        self.repo_remote = remote_url;
        self
    }

    /// Sets a custom terminal tab title for this repo.
    #[must_use]
    pub fn repo_title(mut self, title_text: String) -> Self {
        self.repo_title = TabTitle::Custom(title_text);
        self
    }

    /// Sets the terminal background color for this repo.
    #[must_use]
    pub fn repo_color(mut self, hex_color: HexColor) -> Self {
        self.repo_color = RepoColor::Themed(hex_color);
        self
    }

    /// Validates all fields and produces a repo alias + entry pair.
    ///
    /// Rejects empty aliases, empty paths, and paths that do not
    /// exist on disk after tilde expansion. Constructs the
    /// `RepoEntry` via direct field assignment on the sibling
    /// module's `pub(super)` fields, which is correct because this
    /// builder IS the validation surface and the only production
    /// path that produces a new `RepoEntry`.
    ///
    /// # Errors
    ///
    /// Returns [`ReportalError::ValidationFailure`] with the field
    /// name and the specific reason for each rejection.
    pub fn build(self) -> Result<(String, RepoEntry), ReportalError> {
        if self.alias.trim().is_empty() {
            return Err(ReportalError::ValidationFailure {
                field: "alias".to_owned(),
                reason: "must not be empty".to_owned(),
            });
        }
        if self.repo_path.trim().is_empty() {
            return Err(ReportalError::ValidationFailure {
                field: "path".to_owned(),
                reason: "must not be empty".to_owned(),
            });
        }
        let expanded_path = shellexpand::tilde(&self.repo_path);
        let resolved = PathBuf::from(expanded_path.as_ref());
        if !resolved.exists() {
            return Err(ReportalError::ValidationFailure {
                field: "path".to_owned(),
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
