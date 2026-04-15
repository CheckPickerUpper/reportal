//! One entry in a `.code-workspace` file's `folders` array.

use std::path::Path;

/// One entry in a `.code-workspace` file's `folders` array.
///
/// Reportal owns the entire `folders` array and writes each entry
/// with just an absolute path, so the editor derives the sidebar
/// label from the directory basename. Top-level fields outside
/// `folders` (settings, extensions, launch, tasks) are preserved
/// verbatim by the round-trip machinery in `CodeWorkspaceFile`,
/// but the internal shape of each folder entry is owned by reportal
/// — it is reportal's job to regenerate the array from the current
/// paths of the workspace's member repos.
#[derive(Debug, Clone)]
pub struct CodeWorkspaceFolderEntry {
    /// Absolute filesystem path of the folder to open.
    ///
    /// Stored as a UTF-8 string because JSON strings are UTF-8 and
    /// the `.code-workspace` file serializes this value directly.
    folder_absolute_path: String,
}

/// Accessors for a folder entry in a `.code-workspace` file.
impl CodeWorkspaceFolderEntry {
    /// Builds a folder entry from an absolute path.
    ///
    /// Uses `Path::display()` to produce a UTF-8 lossy rendering of
    /// the path, which is correct for JSON output because a JSON
    /// string must be UTF-8. On platforms where a path may contain
    /// non-UTF-8 bytes, the lossy rendering is the closest faithful
    /// representation available for the JSON encoding.
    #[must_use]
    pub fn from_absolute_path(absolute_folder_path: &Path) -> Self {
        Self {
            folder_absolute_path: absolute_folder_path.display().to_string(),
        }
    }

    /// Builds a folder entry from an already-resolved path string.
    ///
    /// Used by the load path when reading an existing
    /// `.code-workspace` file: the CST gives the string value of
    /// the `path` field, which is already a `String` and does not
    /// need to go through `Path::display()`.
    #[must_use]
    pub fn from_path_string(resolved_path_string: String) -> Self {
        Self {
            folder_absolute_path: resolved_path_string,
        }
    }

    /// The absolute filesystem path this folder entry points to.
    #[must_use]
    pub fn folder_path(&self) -> &str {
        &self.folder_absolute_path
    }
}
