//! Opt-in flag deciding whether a config entry (command, repo,
//! workspace) gets emitted as a top-level shell function by
//! `rep init <shell>`.
//!
//! Lives in its own module so every config entry type that can
//! opt into shell-alias export (`CommandEntry`, `RepoEntry`,
//! `WorkspaceEntry`) imports one definition instead of declaring
//! three parallel enums that would drift on variant additions.

use serde::{Deserialize, Deserializer, Serialize, Serializer};

/// Whether `rep init <shell>` should emit a top-level shell
/// function for the owning config entry.
///
/// Serialized to TOML as a bool so the human-editable config
/// remains `shell_alias = true` / `shell_alias = false`; the
/// enum form is the in-memory representation so the rest of
/// the codebase matches on named states instead of raw booleans.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum ShellAliasExport {
    /// Do not emit a shell function for this entry.
    #[default]
    Disabled,
    /// Emit a top-level shell function named after this entry's
    /// config key (and, for repos and workspaces, each declared
    /// alias) so the user can invoke the entry's action directly.
    Enabled,
}

/// Serializes as a plain TOML bool (`true` for `Enabled`,
/// `false` for `Disabled`) so config files stay human-readable.
impl Serialize for ShellAliasExport {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        match self {
            Self::Disabled => serializer.serialize_bool(false),
            Self::Enabled => serializer.serialize_bool(true),
        }
    }
}

/// Deserializes a plain TOML bool into the corresponding
/// variant so `shell_alias = true` in the user's config maps
/// to `Enabled` without requiring string tags.
impl<'de> Deserialize<'de> for ShellAliasExport {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let raw_bool_flag = bool::deserialize(deserializer)?;
        match raw_bool_flag {
            true => Ok(Self::Enabled),
            false => Ok(Self::Disabled),
        }
    }
}
