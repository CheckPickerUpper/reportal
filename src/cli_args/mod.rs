//! CLI argument types for the `RePortal` binary.
//!
//! Each subcommand's args live in their own file. Shared building
//! blocks (tag filter, repo selection) are composed via `#[command(flatten)]`.
mod ai_args;
mod color_args;
mod edit_args;
mod jump_args;
mod list_args;
mod open_args;
mod remove_args;
mod repo_selection_args;
mod reportal_cli;
mod run_args;
mod status_args;
mod sync_args;
mod tag_filter_args;
mod web_args;
mod workspace_args;
mod workspace_filter_args;

pub use ai_args::AiArgs;
pub use color_args::ColorArgs;
pub use edit_args::EditArgs;
pub use jump_args::JumpArgs;
pub use list_args::{ListArgs, ListArgsFilterParts};
pub use open_args::OpenArgs;
pub use remove_args::RemoveArgs;
pub use reportal_cli::{ReportalCli, ReportalCliSubcommand};
pub use run_args::RunArgs;
pub use status_args::StatusArgs;
pub use sync_args::SyncArgs;
pub use web_args::WebArgs;
pub use workspace_args::{
    WorkspaceArgs, WorkspaceArgsCreateParts, WorkspaceArgsMemberEditParts, WorkspaceArgsSubcommand,
};
