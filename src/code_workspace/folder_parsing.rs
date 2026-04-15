//! CST-to-typed extraction for `.code-workspace` folder entries.
//!
//! Wraps the parsing functions in a `FolderParsingContext` that
//! carries the source file path so each method takes a single
//! non-`self` argument, keeping the parsing surface within the
//! project's param-count conventions while still producing
//! informative error messages that name the file.

use crate::code_workspace::code_workspace_folder_entry::CodeWorkspaceFolderEntry;
use crate::error::ReportalError;
use jsonc_parser::ParseOptions;
use jsonc_parser::cst::{CstNode, CstRootNode};
use std::path::Path;

/// Per-file context for parsing a `.code-workspace` document.
///
/// Holds the source file path so every parse error can report
/// the exact file that triggered it without each call site
/// having to thread the path through manually.
pub struct FolderParsingContext<'path_lifetime> {
    /// The `.code-workspace` file path being parsed, used only in
    /// error messages.
    source_file_path: &'path_lifetime Path,
}

/// Folder extraction methods that walk the JSONC CST and enforce
/// the VSCode/Cursor schema.
impl<'path_lifetime> FolderParsingContext<'path_lifetime> {
    /// Builds a parsing context for the given file path.
    #[must_use]
    pub fn for_file(source_file_path: &'path_lifetime Path) -> Self {
        Self { source_file_path }
    }

    /// Parses the given `.code-workspace` text as JSONC and returns
    /// the typed folder list extracted from its `folders` property.
    ///
    /// Extraction rules mirror the VSCode/Cursor schema: the root
    /// must be an object, `folders` must be an array, each element
    /// must be an object with a `path` property whose value is a
    /// string. Anything else is a hard parse error because a
    /// malformed entry indicates the file was not produced by a
    /// correct workspace generator and reportal must refuse to
    /// regenerate over it.
    ///
    /// # Errors
    ///
    /// Returns [`ReportalError::CodeWorkspaceParseFailure`] if the
    /// input is not valid JSONC, the top-level value is not an
    /// object, or any folder entry violates the schema.
    pub fn parse_folders_from_text(
        &self,
        file_contents: &str,
    ) -> Result<Vec<CodeWorkspaceFolderEntry>, ReportalError> {
        let cst_root = CstRootNode::parse(file_contents, &ParseOptions::default()).map_err(
            |parse_error| ReportalError::CodeWorkspaceParseFailure {
                file_path: self.source_file_path.display().to_string(),
                reason: parse_error.to_string(),
            },
        )?;
        let Some(root_object) = cst_root.object_value() else {
            return Err(ReportalError::CodeWorkspaceParseFailure {
                file_path: self.source_file_path.display().to_string(),
                reason: "top-level JSONC value must be an object".to_owned(),
            });
        };
        let Some(parsed_array) = root_object.array_value("folders") else {
            return Ok(Vec::new());
        };
        let mut folder_list = Vec::new();
        for folder_node in parsed_array.elements() {
            folder_list.push(self.extract_folder_entry(&folder_node)?);
        }
        Ok(folder_list)
    }

    /// Extracts one folder entry from a single CST node inside a
    /// `folders` array, enforcing the required
    /// object-with-string-path shape.
    ///
    /// Uses `let-else` early returns so every schema check flattens
    /// into a straight sequence instead of nesting, which is
    /// required because the excessive-nesting lint triggers quickly
    /// on this kind of deep descent through a CST.
    ///
    /// # Errors
    ///
    /// Returns [`ReportalError::CodeWorkspaceParseFailure`] if the
    /// node is not an object, has no `path` field, the field has
    /// no value, the value is not a string, or the string contains
    /// invalid escape sequences.
    pub fn extract_folder_entry(
        &self,
        folder_node: &CstNode,
    ) -> Result<CodeWorkspaceFolderEntry, ReportalError> {
        let Some(folder_object) = folder_node.as_object() else {
            return Err(ReportalError::CodeWorkspaceParseFailure {
                file_path: self.source_file_path.display().to_string(),
                reason: "each entry in `folders` must be an object".to_owned(),
            });
        };
        let Some(path_prop) = folder_object.get("path") else {
            return Err(ReportalError::CodeWorkspaceParseFailure {
                file_path: self.source_file_path.display().to_string(),
                reason: "each entry in `folders` must have a `path` field".to_owned(),
            });
        };
        let Some(path_value_node) = path_prop.value() else {
            return Err(ReportalError::CodeWorkspaceParseFailure {
                file_path: self.source_file_path.display().to_string(),
                reason: "`path` property has no value".to_owned(),
            });
        };
        let Some(path_string_lit) = path_value_node.as_string_lit() else {
            return Err(ReportalError::CodeWorkspaceParseFailure {
                file_path: self.source_file_path.display().to_string(),
                reason: "`path` value must be a string".to_owned(),
            });
        };
        let decoded_path = path_string_lit.decoded_value().map_err(|decode_error| {
            ReportalError::CodeWorkspaceParseFailure {
                file_path: self.source_file_path.display().to_string(),
                reason: format!("`path` value has invalid string escapes: {decode_error}"),
            }
        })?;
        Ok(CodeWorkspaceFolderEntry::from_path_string(decoded_path))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn parse(input_text: &str) -> Result<Vec<CodeWorkspaceFolderEntry>, ReportalError> {
        let dummy_file_path = PathBuf::from("test.code-workspace");
        let parsing_context = FolderParsingContext::for_file(&dummy_file_path);
        parsing_context.parse_folders_from_text(input_text)
    }

    #[test]
    fn extracts_folder_paths_in_declared_order() {
        let input_text = r#"{
            "folders": [
                { "path": "/alpha" },
                { "path": "/bravo" },
                { "path": "/charlie" }
            ]
        }"#;
        let parsed_folder_entries = parse(input_text).expect("valid input must parse");
        let folder_path_list: Vec<&str> = parsed_folder_entries
            .iter()
            .map(CodeWorkspaceFolderEntry::folder_path)
            .collect();
        assert_eq!(folder_path_list, vec!["/alpha", "/bravo", "/charlie"]);
    }

    #[test]
    fn missing_folders_property_yields_empty_vec() {
        let input_text = r#"{
            "settings": { "editor.fontSize": 14 }
        }"#;
        let parsed_folder_entries = parse(input_text).expect("non-folder objects must still parse");
        assert!(parsed_folder_entries.is_empty());
    }

    #[test]
    fn non_object_root_is_rejected() {
        let input_text = "[]";
        let parse_outcome = parse(input_text);
        assert!(
            matches!(parse_outcome, Err(ReportalError::CodeWorkspaceParseFailure { .. })),
            "array root must be rejected, got {parse_outcome:?}",
        );
    }

    #[test]
    fn folder_entry_without_path_is_rejected() {
        let input_text = r#"{
            "folders": [
                { "name": "no-path" }
            ]
        }"#;
        let parse_outcome = parse(input_text);
        assert!(
            matches!(parse_outcome, Err(ReportalError::CodeWorkspaceParseFailure { .. })),
            "missing path field must be rejected, got {parse_outcome:?}",
        );
    }

    #[test]
    fn folder_entry_with_non_string_path_is_rejected() {
        let input_text = r#"{
            "folders": [
                { "path": 42 }
            ]
        }"#;
        let parse_outcome = parse(input_text);
        assert!(
            matches!(parse_outcome, Err(ReportalError::CodeWorkspaceParseFailure { .. })),
            "non-string path must be rejected, got {parse_outcome:?}",
        );
    }

    #[test]
    fn jsonc_comments_in_source_do_not_break_parse() {
        let input_text = r#"{
            // top-level comment describing this workspace
            "folders": [
                // first repo
                { "path": "/alpha" }
            ]
        }"#;
        let parsed_folder_entries =
            parse(input_text).expect("JSONC comments must not break folder parse");
        assert_eq!(parsed_folder_entries.len(), 1);
        assert_eq!(parsed_folder_entries[0].folder_path(), "/alpha");
    }
}
