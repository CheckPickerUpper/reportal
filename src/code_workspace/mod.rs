//! Parse-merge-write for VSCode/Cursor `.code-workspace` files.
//!
//! A `.code-workspace` file is a JSON document with a `folders`
//! array plus optional user-authored fields like `settings`,
//! `extensions`, and `launch`. Reportal owns the `folders` array
//! because it is derived from the current paths of the member
//! repos; everything else is preserved untouched across writes so
//! regeneration cannot destroy settings the user set by hand.
//!
//! The round-trip invariant: reading a file and writing it back
//! without calling `set_folder_paths` must produce a byte-stable
//! document modulo whitespace normalization. Reading a file,
//! calling `set_folder_paths`, and writing it back must replace
//! only the `folders` field and leave every other key intact.

mod code_workspace_file;
mod code_workspace_folder_entry;
mod folder_parsing;

pub use code_workspace_file::CodeWorkspaceFile;
#[expect(unused_imports, reason = "consumed by later hooks — repo_edit path-change regeneration reads folder entries")]
pub use code_workspace_folder_entry::CodeWorkspaceFolderEntry;
