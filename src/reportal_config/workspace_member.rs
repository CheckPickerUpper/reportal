//! A single entry in a workspace's ordered member list.
//!
//! Members are either references to registered repos (the original
//! Design B shape, which gives the path-change reverse-index its
//! power) or raw filesystem paths declared inline (useful when the
//! user drives a workspace through reportal but does not want the
//! individual folders registered as top-level repos). The inline
//! variant gives up the automatic regeneration that repo renames
//! trigger — a deliberate tradeoff the user accepts when they
//! choose that form.

use serde::{Deserialize, Serialize};

/// A single member of a workspace's ordered folder list.
///
/// The untagged serde representation lets existing configs whose
/// `repos` array holds bare strings continue to load unchanged —
/// those literals resolve to `RegisteredRepo`. A new inline-table
/// form `{ path = "..." }` declares a filesystem path directly so
/// workspaces can contain folders that are not registered as
/// top-level repos.
#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(untagged)]
pub enum WorkspaceMember {
    /// Canonical key or alias of a registered repo, resolved to an
    /// absolute path through the repo registry at regeneration time.
    ///
    /// Backed by the string payload so TOML arrays of bare strings
    /// continue to parse into this variant without any shape change.
    RegisteredRepo(String),
    /// Literal filesystem path to a folder, bypassing the repo
    /// registry entirely.
    ///
    /// Chosen when the user wants a workspace folder but does not
    /// want the folder registered as a top-level repo. Gives up the
    /// reverse-index auto-regeneration that `rep edit`'s path field
    /// fires for alias-variant members.
    InlinePath {
        /// Raw path as written in config, with `~` expansion
        /// deferred to the regenerator so the config round-trips
        /// verbatim through save/load cycles.
        path: String,
    },
}

/// Classification accessors for workspace members.
impl WorkspaceMember {
    /// Returns the referenced repo alias when this member is a
    /// registered-repo reference, for use by the repo-rename
    /// reverse-index and the `rep remove` guard.
    ///
    /// Callers pattern-match on the returned enum rather than
    /// unwrapping, so the two member kinds never get confused at a
    /// call site.
    #[must_use]
    pub fn registered_repo_alias(&self) -> WorkspaceMemberAliasLookup<'_> {
        match self {
            Self::RegisteredRepo(alias) => {
                WorkspaceMemberAliasLookup::Matches(alias.as_str())
            }
            Self::InlinePath { .. } => WorkspaceMemberAliasLookup::NotARepoReference,
        }
    }
}

/// Outcome of asking a [`WorkspaceMember`] for its registered-repo
/// alias.
///
/// Named variants (not `Option<&str>`) because the absence has
/// domain meaning: the member is an inline path, which must not be
/// treated as a missing lookup — it is a different kind of member.
pub enum WorkspaceMemberAliasLookup<'borrow> {
    /// The member references a registered repo by the given alias.
    Matches(&'borrow str),
    /// The member is an inline filesystem path, so it has no
    /// registered-repo alias by design.
    NotARepoReference,
}
