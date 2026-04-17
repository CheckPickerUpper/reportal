//! Parameter struct for workspace-alias collision lookups.

/// Parameters for a single workspace-alias collision check run by
/// `ReportalConfig::check_workspace_alias_collisions`.
///
/// Named-field struct so the two `&str` arguments cannot be swapped
/// at call sites — swapping them would silently check the wrong
/// pair and pass every collision through. Lives in its own module
/// because the config root's one-primary-type-per-file convention
/// does not accept unrelated helper types as neighbors.
pub struct AliasCollisionQuery<'borrow> {
    /// Canonical name of the workspace whose alias is being
    /// validated; used so the scan can skip the owning entry
    /// itself when comparing against peers.
    owning_workspace_name: &'borrow str,
    /// The literal alias value being checked for collisions.
    candidate_alias: &'borrow str,
}

impl<'borrow> AliasCollisionQuery<'borrow> {
    /// Builds a new collision query from the owning workspace name
    /// and the alias value under test.
    ///
    /// Both arguments are borrowed for the lifetime of the enclosing
    /// validation pass — the query is consumed on use, so no
    /// allocation is performed.
    #[must_use]
    pub fn new(owning_workspace_name: &'borrow str, candidate_alias: &'borrow str) -> Self {
        Self {
            owning_workspace_name,
            candidate_alias,
        }
    }

    /// Canonical name of the workspace whose alias is being
    /// validated.
    #[must_use]
    pub fn owning_workspace_name(&self) -> &str {
        self.owning_workspace_name
    }

    /// The literal alias value being checked for collisions.
    #[must_use]
    pub fn candidate_alias(&self) -> &str {
        self.candidate_alias
    }
}
