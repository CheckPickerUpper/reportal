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
