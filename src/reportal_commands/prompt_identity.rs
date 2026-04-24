//! Resolves the current working directory into a prompt-badge
//! identity (label + accent color) drawn from the configuration.
//!
//! Used by `rep prompt` to render the inline badge inside the
//! shell prompt, and by `rep color` to pick the right tab-strip
//! color when the user is inside a workspace directory that is
//! not itself a registered repository path.
//!
//! Resolution order is workspace-first, repository-second:
//!
//! 1. Longest-prefix match against every workspace's on-disk
//!    directory (the folder that holds the symlinks + the
//!    `.code-workspace` file). A workspace match produces the
//!    workspace's own title + color.
//! 2. If no workspace matches, longest-prefix match against every
//!    registered repository's resolved path. A repository match
//!    produces the repository's title + color.
//! 3. If neither matches, returns `None` — the caller decides
//!    whether to emit nothing (prompt hook) or reset chrome
//!    (explicit color call).
//!
//! Workspace-first is the intuitive rule: a user who `cd`s through
//! a workspace symlink has a PWD like
//! `~/dev/workspaces/venoble/app`, which is prefix-matched by the
//! workspace directory `~/dev/workspaces/venoble/` and therefore
//! identifies as "in the venoble workspace" rather than "in the
//! app repository". A user who `cd`s directly into the real
//! repository path bypasses the workspace dir and falls through
//! to the repository match, which is also the intuitive result.

use crate::error::ReportalError;
use crate::reportal_commands::workspace_operations::WorkspaceRegenerator;
use crate::reportal_config::{HexColor, ReportalConfig, TabTitle};
use std::path::Path;

/// Resolved identity for the prompt badge: the label to display
/// and the accent color to render it in.
///
/// Modeled as a concrete struct (rather than a tuple) so callers
/// can name the fields at the emit site and new rendering knobs
/// (icon glyph, badge shape) can be added later without breaking
/// the call-site signature.
pub struct PromptIdentity {
    /// Short human-readable label — e.g. `"🥷 nro"` or
    /// `"👑 ven"` — rendered inside the prompt and the editor
    /// title bar.
    pub display_label: String,
    /// Accent color applied to the label in the prompt and to
    /// the tab strip / editor title bar. `None` means no accent
    /// (fall back to the terminal's default foreground).
    pub accent_color: Option<HexColor>,
}

/// Resolver that walks a configuration's workspace directories
/// and repository paths to find the one whose longest-prefix
/// matches the given current working directory.
///
/// Constructed once with a borrowed configuration so each
/// resolution method has exactly one non-`self` parameter,
/// matching the project's `WorkspaceRegenerator` pattern for
/// configuration-backed helpers.
pub struct PromptIdentityResolver<'configuration_lifetime> {
    /// The loaded configuration supplying workspace and
    /// repository entries.
    configuration_registry: &'configuration_lifetime ReportalConfig,
}

/// Resolution methods that turn a working directory into the
/// matching workspace or repository identity.
impl<'configuration_lifetime> PromptIdentityResolver<'configuration_lifetime> {
    /// @why Binds the resolver to a loaded configuration once
    /// per command invocation so the two lookup passes
    /// (workspace, repository) share the same borrowed data and
    /// cannot observe a torn read across a config reload.
    #[must_use]
    pub fn for_configuration(
        configuration_registry: &'configuration_lifetime ReportalConfig,
    ) -> Self {
        Self {
            configuration_registry,
        }
    }

    /// @why Turns the current working directory into a concrete
    /// prompt badge identity so the callers (`rep prompt`,
    /// workspace-aware `rep color`) don't each re-implement the
    /// longest-prefix-match rule against workspaces then
    /// repositories.
    ///
    /// Returns `Ok(None)` when the CWD sits under neither a
    /// workspace directory nor a registered repository path.
    ///
    /// # Errors
    ///
    /// Returns [`ReportalError::ConfigIoFailure`] if the current
    /// working directory cannot be read, or any error surfaced
    /// by the workspace directory resolver (missing home
    /// directory).
    pub fn resolve_from_current_directory(
        &self,
    ) -> Result<Option<PromptIdentity>, ReportalError> {
        let current_directory = std::env::current_dir().map_err(|working_directory_error| {
            ReportalError::ConfigIoFailure {
                reason: working_directory_error.to_string(),
            }
        })?;
        if let Some(workspace_identity) = self.match_workspace_directory(&current_directory)? {
            return Ok(Some(workspace_identity));
        }
        Ok(self.match_repository_path(&current_directory))
    }

    /// Longest-prefix match against every workspace's on-disk
    /// directory. A match produces a [`PromptIdentity`] built
    /// from the workspace's title and color fields, with the
    /// workspace's canonical name as the fallback label.
    fn match_workspace_directory(
        &self,
        current_directory: &Path,
    ) -> Result<Option<PromptIdentity>, ReportalError> {
        let regenerator = WorkspaceRegenerator::for_config(self.configuration_registry);
        let mut best_match_length: usize = 0;
        let mut best_identity: Option<PromptIdentity> = None;

        for (workspace_name, workspace_entry) in
            self.configuration_registry.workspaces_with_names()
        {
            let workspace_directory = regenerator.resolve_workspace_directory(workspace_name)?;
            if !current_directory.starts_with(&workspace_directory) {
                continue;
            }
            let matched_path_length = workspace_directory.as_os_str().len();
            if matched_path_length <= best_match_length {
                continue;
            }
            best_match_length = matched_path_length;
            let resolved_display_label = match workspace_entry.tab_title() {
                TabTitle::Custom(custom_display_label) => custom_display_label.clone(),
                TabTitle::UseAlias => workspace_name.clone(),
            };
            best_identity = Some(PromptIdentity {
                display_label: resolved_display_label,
                accent_color: workspace_entry.workspace_color().themed_hex_color().cloned(),
            });
        }
        Ok(best_identity)
    }

    /// Longest-prefix match against every registered
    /// repository's resolved path. A match produces a
    /// [`PromptIdentity`] built from the repository's title and
    /// color fields, with the repository alias as the fallback
    /// label.
    fn match_repository_path(&self, current_directory: &Path) -> Option<PromptIdentity> {
        let mut best_match_length: usize = 0;
        let mut best_identity: Option<PromptIdentity> = None;

        for (repository_alias, repository_entry) in
            self.configuration_registry.repos_with_aliases()
        {
            let repository_path = repository_entry.resolved_path();
            if !current_directory.starts_with(&repository_path) {
                continue;
            }
            let matched_path_length = repository_path.as_os_str().len();
            if matched_path_length <= best_match_length {
                continue;
            }
            best_match_length = matched_path_length;
            let resolved_display_label = match repository_entry.tab_title() {
                TabTitle::Custom(custom_display_label) => custom_display_label.clone(),
                TabTitle::UseAlias => repository_alias.clone(),
            };
            best_identity = Some(PromptIdentity {
                display_label: resolved_display_label,
                accent_color: repository_entry.repo_color().themed_hex_color().cloned(),
            });
        }
        best_identity
    }
}
