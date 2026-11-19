//! Parse-merge-write for VSCode/Cursor `.code-workspace` files.
//!
//! A `.code-workspace` file is a JSON document with a `folders`
//! array plus optional user-authored fields like `settings`,
//! `extensions`, and `launch`. RePortal writes the derived folder
//! list and the configured title and title-bar colors. Other fields
//! and their comments are preserved across regeneration.

mod code_workspace_file;
mod code_workspace_folder_entry;
mod folder_parsing;

pub use code_workspace_file::CodeWorkspaceFile;
pub use code_workspace_folder_entry::CodeWorkspaceFolderEntry;
