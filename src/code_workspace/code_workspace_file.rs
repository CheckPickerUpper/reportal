//! Parsed, mutable view of a `.code-workspace` file.

use crate::code_workspace::code_workspace_folder_entry::CodeWorkspaceFolderEntry;
use crate::code_workspace::folder_parsing::FolderParsingContext;
use crate::error::ReportalError;
use jsonc_parser::cst::{CstInputValue, CstObject, CstRootNode};
use jsonc_parser::ParseOptions;
use std::path::{Path, PathBuf};

/// Parsed, mutable view of a `.code-workspace` file.
///
/// Preserves the original JSONC text and edits the folder list and
/// configured editor identity in place so editor settings, other fields,
/// and their comments are preserved.
#[derive(Debug, Clone)]
pub struct CodeWorkspaceFile {
    /// Raw text of the `.code-workspace` file as it was last read.
    ///
    /// Empty string means the file did not exist on disk. On write,
    /// this text is re-parsed and mutated via the CST, so any
    /// comments or user-authored formatting inside it round-trip
    /// through regeneration untouched.
    original_file_text: String,
    /// The typed folder list reportal manages.
    ///
    /// Populated from the parsed CST on load and mutated directly
    /// by `set_folder_paths`. On write, this list is serialized
    /// into a new `folders` array via `jsonc-parser`'s typed
    /// `CstInputValue` builder API.
    folders: Vec<CodeWorkspaceFolderEntry>,
    identity: Option<WorkspaceEditorIdentity>,
}

#[derive(Debug, Clone)]
struct WorkspaceEditorIdentity {
    window_title: String,
    title_bar_color_hex: Option<String>,
}

/// Loading, mutation, and serialization for a `.code-workspace` file.
impl CodeWorkspaceFile {
    /// Constructs an empty `.code-workspace` document for the case
    /// where reportal is generating a workspace file that did not
    /// previously exist on disk.
    ///
    /// The original file text is left empty so that on write the
    /// CST is seeded from `"{}"` and the resulting file contains
    /// only a `folders` property.
    #[must_use]
    pub fn empty() -> Self {
        Self {
            original_file_text: String::new(),
            folders: Vec::new(),
            identity: None,
        }
    }

    /// Loads and parses an existing `.code-workspace` file from disk.
    ///
    /// If the file does not exist, returns an empty document so the
    /// caller can populate folders and write a fresh file. If the
    /// file exists, its text is stored verbatim and the `folders`
    /// property is extracted into the typed folder list, dropping
    /// any CST state once extraction is done — the original text
    /// is what the write path re-parses, not a retained CST.
    ///
    /// # Errors
    ///
    /// Returns [`ReportalError::CodeWorkspaceIoFailure`] if the file
    /// exists but cannot be read, or
    /// [`ReportalError::CodeWorkspaceParseFailure`] if the file is
    /// not valid JSONC or the `folders` property is malformed.
    pub fn load_or_empty(file_path: &Path) -> Result<Self, ReportalError> {
        if !file_path.exists() {
            return Ok(Self::empty());
        }
        let file_contents = std::fs::read_to_string(file_path).map_err(|io_error| {
            ReportalError::CodeWorkspaceIoFailure {
                file_path: file_path.display().to_string(),
                reason: io_error.to_string(),
            }
        })?;
        let folder_parser = FolderParsingContext::for_file(file_path);
        let extracted_folders = folder_parser.parse_folders_from_text(&file_contents)?;
        Ok(Self {
            original_file_text: file_contents,
            folders: extracted_folders,
            identity: None,
        })
    }

    /// The ordered folder entries currently stored in the document.
    #[must_use]
    pub(crate) fn folder_entries(&self) -> &[CodeWorkspaceFolderEntry] {
        &self.folders
    }

    /// @why Rewrites the folder list from absolute paths so the
    /// `.code-workspace` file stays in sync with the current
    /// workspace membership, preserving every other top-level
    /// field (user settings, extensions, launch configs, JSONC
    /// comments) through the parse-merge-write round-trip.
    pub fn set_folder_paths(&mut self, folder_absolute_paths: &[PathBuf]) {
        self.folders = folder_absolute_paths
            .iter()
            .map(|absolute_path| CodeWorkspaceFolderEntry::from_absolute_path(absolute_path))
            .collect();
    }

    /// @why Stamps a workspace-identity badge into the editor's
    /// `window.title` and `workbench.colorCustomizations`
    /// settings so opening the `.code-workspace` in VS Code or
    /// Cursor shows the same prompt identity the shell displays,
    /// giving one consistent signal across terminal and editor
    /// for which workspace the user is in.
    pub fn set_workspace_identity(
        &mut self,
        window_title: String,
        title_bar_color_hex: Option<String>,
    ) {
        self.identity = Some(WorkspaceEditorIdentity {
            window_title,
            title_bar_color_hex,
        });
    }

    /// Serializes the document back to JSONC and writes it to disk.
    ///
    /// Re-parses `original_file_text` (or `"{}"` when empty), uses
    /// the CST API to replace the `folders` property with a new
    /// array built from the typed folder list, serializes the
    /// mutated CST via its `Display` impl, and writes the result
    /// to `file_path`. Creates the parent directory if missing so
    /// that writing into the default location
    /// `~/.reportal/workspaces/` works on a first-run machine
    /// where that directory has not been created.
    ///
    /// # Errors
    ///
    /// Returns [`ReportalError::CodeWorkspaceIoFailure`] if the
    /// parent directory cannot be created or the file cannot be
    /// written, or
    /// [`ReportalError::CodeWorkspaceParseFailure`] if the seed
    /// text fails to parse as JSONC (which indicates internal
    /// corruption of the stored original text).
    pub fn write_to_disk(&self, file_path: &Path) -> Result<(), ReportalError> {
        ensure_parent_directory_exists(file_path)?;
        let seed_text: &str = if self.original_file_text.is_empty() {
            "{}"
        } else {
            &self.original_file_text
        };
        let cst_root =
            CstRootNode::parse(seed_text, &ParseOptions::default()).map_err(|parse_error| {
                ReportalError::CodeWorkspaceParseFailure {
                    file_path: file_path.display().to_string(),
                    reason: parse_error.to_string(),
                }
            })?;
        let root_object = cst_root.object_value_or_set();
        set_cst_property(
            &root_object,
            "folders",
            build_folders_cst_value(&self.folders),
        );
        if let Some(identity) = &self.identity {
            identity.write_to_settings(&root_object.object_value_or_set("settings"));
        }
        let serialized_text = cst_root.to_string();
        std::fs::write(file_path, serialized_text).map_err(|io_error| {
            ReportalError::CodeWorkspaceIoFailure {
                file_path: file_path.display().to_string(),
                reason: io_error.to_string(),
            }
        })?;
        Ok(())
    }
}

impl WorkspaceEditorIdentity {
    fn write_to_settings(&self, settings: &CstObject) {
        set_cst_property(
            settings,
            "window.title",
            CstInputValue::String(format!("{} — ${{activeEditorShort}}", self.window_title)),
        );

        const COLOR_CUSTOMIZATIONS: &str = "workbench.colorCustomizations";
        let accent = self.title_bar_color_hex.as_deref();
        let colors = match accent {
            Some(_) => settings.object_value_or_set(COLOR_CUSTOMIZATIONS),
            None => {
                let Some(existing_colors) = settings.object_value(COLOR_CUSTOMIZATIONS) else {
                    return;
                };
                existing_colors
            }
        };

        let foreground = accent.map(|_| "#ffffff");
        for (name, value) in [
            ("titleBar.activeBackground", accent),
            ("titleBar.activeForeground", foreground),
            ("titleBar.inactiveBackground", accent),
            ("titleBar.inactiveForeground", foreground),
        ] {
            match value {
                Some(color) => {
                    set_cst_property(&colors, name, CstInputValue::String(color.to_owned()));
                }
                None => {
                    if let Some(existing_color) = colors.get(name) {
                        existing_color.remove();
                    }
                }
            }
        }
    }
}

fn set_cst_property(object: &CstObject, name: &str, value: CstInputValue) {
    match object.get(name) {
        Some(property) => property.set_value(value),
        None => {
            object.append(name, value);
        }
    }
}

/// Creates the parent directory of `file_path` if it does not
/// already exist, so that writing a new `.code-workspace` file
/// into the default location `~/.reportal/workspaces/` works on
/// a first-run machine where that directory has not been created.
///
/// A path with no parent (root-level like `/` or `C:\`) is a
/// no-op because there is nothing to create.
///
/// # Errors
///
/// Returns [`ReportalError::CodeWorkspaceIoFailure`] if directory
/// creation fails.
fn ensure_parent_directory_exists(file_path: &Path) -> Result<(), ReportalError> {
    let Some(parent_directory) = file_path.parent() else {
        return Ok(());
    };
    if parent_directory.exists() {
        return Ok(());
    }
    std::fs::create_dir_all(parent_directory).map_err(|io_error| {
        ReportalError::CodeWorkspaceIoFailure {
            file_path: file_path.display().to_string(),
            reason: io_error.to_string(),
        }
    })
}

/// Builds a typed `CstInputValue::Array` from the given folder list
/// for insertion into the CST via `set_value` or `append`.
///
/// Each folder becomes a single-field object with a `path` string
/// entry. This is the one place reportal constructs JSON content
/// for the `.code-workspace` file, and it does so entirely via the
/// typed `CstInputValue` enum so no untyped JSON value passes
/// through reportal's code.
fn build_folders_cst_value(folder_entries: &[CodeWorkspaceFolderEntry]) -> CstInputValue {
    let array_elements: Vec<CstInputValue> = folder_entries
        .iter()
        .map(|entry| {
            CstInputValue::Object(vec![(
                "path".to_owned(),
                CstInputValue::String(entry.folder_path().to_owned()),
            )])
        })
        .collect();
    CstInputValue::Array(array_elements)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU64, Ordering};

    static TEST_FILE_COUNTER: AtomicU64 = AtomicU64::new(0);

    fn unique_temp_path() -> PathBuf {
        let counter_value = TEST_FILE_COUNTER.fetch_add(1, Ordering::Relaxed);
        let process_id = std::process::id();
        std::env::temp_dir().join(format!(
            "reportal-codeworkspace-test-{process_id}-{counter_value}.code-workspace"
        ))
    }

    fn write_fixture(file_path: &Path, file_contents: &str) -> Result<(), ReportalError> {
        std::fs::write(file_path, file_contents).map_err(|error| {
            ReportalError::CodeWorkspaceIoFailure {
                file_path: file_path.display().to_string(),
                reason: error.to_string(),
            }
        })
    }

    fn read_back(file_path: &Path) -> Result<String, ReportalError> {
        std::fs::read_to_string(file_path).map_err(|error| ReportalError::CodeWorkspaceIoFailure {
            file_path: file_path.display().to_string(),
            reason: error.to_string(),
        })
    }

    #[test]
    fn empty_doc_writes_only_folders_property() -> Result<(), ReportalError> {
        let workspace_file_path = unique_temp_path();
        let mut empty_doc = CodeWorkspaceFile::empty();
        empty_doc.set_folder_paths(&[PathBuf::from("/alpha"), PathBuf::from("/bravo")]);
        empty_doc.write_to_disk(&workspace_file_path)?;
        let written_text = read_back(&workspace_file_path)?;
        assert!(written_text.contains("\"folders\""), "missing folders key");
        assert!(written_text.contains("/alpha"), "missing /alpha entry");
        assert!(written_text.contains("/bravo"), "missing /bravo entry");
        let _ = std::fs::remove_file(&workspace_file_path);
        Ok(())
    }

    #[test]
    fn round_trip_preserves_unknown_top_level_fields() -> Result<(), ReportalError> {
        let workspace_file_path = unique_temp_path();
        let original_text = r#"{
            "folders": [
                { "path": "/old/path" }
            ],
            "settings": {
                "editor.fontSize": 14,
                "editor.tabSize": 2
            },
            "extensions": {
                "recommendations": ["rust-lang.rust-analyzer"]
            },
            "launch": {
                "version": "0.2.0",
                "configurations": []
            }
        }"#;
        write_fixture(&workspace_file_path, original_text)?;

        let mut loaded_doc = CodeWorkspaceFile::load_or_empty(&workspace_file_path)?;
        loaded_doc.set_folder_paths(&[PathBuf::from("/new/path")]);
        loaded_doc.write_to_disk(&workspace_file_path)?;

        let rewritten_text = read_back(&workspace_file_path)?;
        assert!(
            rewritten_text.contains("/new/path"),
            "folders array was not updated to /new/path. Rewritten:\n{rewritten_text}",
        );
        assert!(
            !rewritten_text.contains("/old/path"),
            "old folder path leaked through regeneration. Rewritten:\n{rewritten_text}",
        );
        assert!(
            rewritten_text.contains("editor.fontSize"),
            "settings key was stripped during regeneration. Rewritten:\n{rewritten_text}",
        );
        assert!(
            rewritten_text.contains("editor.tabSize"),
            "settings key was stripped during regeneration. Rewritten:\n{rewritten_text}",
        );
        assert!(
            rewritten_text.contains("rust-lang.rust-analyzer"),
            "extensions entry was stripped during regeneration. Rewritten:\n{rewritten_text}",
        );
        assert!(
            rewritten_text.contains("\"launch\""),
            "launch section was stripped during regeneration. Rewritten:\n{rewritten_text}",
        );
        let _ = std::fs::remove_file(&workspace_file_path);
        Ok(())
    }

    #[test]
    fn round_trip_preserves_jsonc_comments() -> Result<(), ReportalError> {
        let workspace_file_path = unique_temp_path();
        let original_text = r#"{
            // Workspace for the backend services
            "folders": [
                { "path": "/old/api" }
            ],
            // User-authored settings below
            "settings": {
                "editor.fontSize": 14
            }
        }"#;
        write_fixture(&workspace_file_path, original_text)?;

        let mut loaded_doc = CodeWorkspaceFile::load_or_empty(&workspace_file_path)?;
        loaded_doc.set_folder_paths(&[PathBuf::from("/new/api")]);
        loaded_doc.write_to_disk(&workspace_file_path)?;

        let rewritten_text = read_back(&workspace_file_path)?;
        assert!(
            rewritten_text.contains("Workspace for the backend services"),
            "top-level JSONC comment was stripped. Rewritten:\n{rewritten_text}",
        );
        assert!(
            rewritten_text.contains("User-authored settings below"),
            "settings JSONC comment was stripped. Rewritten:\n{rewritten_text}",
        );
        let _ = std::fs::remove_file(&workspace_file_path);
        Ok(())
    }

    #[test]
    fn workspace_identity_preserves_user_settings_and_nested_colors() -> Result<(), ReportalError> {
        let workspace_file_path = unique_temp_path();
        let original_text = r##"{
            // Workspace-level user note
            "folders": [
                { "path": "./old/api" }
            ],
            "settings": {
                // Keep the user's editor preference
                "editor.fontSize": 14,
                "window.title": "Old title",
                "workbench.colorCustomizations": {
                    // Keep the user's editor color
                    "editor.background": "#123456",
                    "titleBar.activeBackground": "#000000"
                }
            },
            "extensions": {
                "recommendations": ["rust-lang.rust-analyzer"]
            },
            "launch": {
                "version": "0.2.0",
                "configurations": []
            }
        }"##;
        write_fixture(&workspace_file_path, original_text)?;

        let mut loaded_doc = CodeWorkspaceFile::load_or_empty(&workspace_file_path)?;
        loaded_doc.set_folder_paths(&[PathBuf::from("./new/api")]);
        loaded_doc.set_workspace_identity("Backend".to_owned(), Some("#abcdef".to_owned()));
        loaded_doc.write_to_disk(&workspace_file_path)?;

        let rewritten_text = read_back(&workspace_file_path)?;
        assert!(
            rewritten_text.contains("./new/api"),
            "folder paths must still be regenerated. Rewritten:\n{rewritten_text}",
        );
        assert!(
            rewritten_text.contains("editor.fontSize"),
            "unrelated user settings must survive identity regeneration. Rewritten:\n{rewritten_text}",
        );
        assert!(
            rewritten_text.contains("editor.background"),
            "unrelated nested color customizations must survive identity regeneration. Rewritten:\n{rewritten_text}",
        );
        assert!(
            rewritten_text.contains("Workspace-level user note"),
            "top-level JSONC comments must survive identity regeneration. Rewritten:\n{rewritten_text}",
        );
        assert!(
            rewritten_text.contains("Keep the user's editor preference"),
            "settings comments must survive identity regeneration. Rewritten:\n{rewritten_text}",
        );
        assert!(
            rewritten_text.contains("rust-lang.rust-analyzer") && rewritten_text.contains("launch"),
            "unrelated top-level fields must survive identity regeneration. Rewritten:\n{rewritten_text}",
        );
        assert!(
            rewritten_text.contains("Backend — ${activeEditorShort}"),
            "RePortal must refresh the workspace title. Rewritten:\n{rewritten_text}",
        );
        assert!(
            rewritten_text.contains("#abcdef"),
            "RePortal must refresh its title-bar colors. Rewritten:\n{rewritten_text}",
        );
        assert!(
            !rewritten_text.contains("#000000"),
            "the previous RePortal-owned title-bar color must be replaced. Rewritten:\n{rewritten_text}",
        );
        let _ = std::fs::remove_file(&workspace_file_path);
        Ok(())
    }

    #[test]
    fn load_or_empty_on_missing_file_returns_empty_doc() -> Result<(), ReportalError> {
        let nonexistent_path = unique_temp_path();
        let loaded_doc = CodeWorkspaceFile::load_or_empty(&nonexistent_path)?;
        assert!(loaded_doc.folder_entries().is_empty());
        Ok(())
    }
}
