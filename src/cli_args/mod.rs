//! CLI argument types for the `RePortal` binary.
//!
//! Each subcommand's args live in their own file. Shared building
//! blocks (tag filter, repo selection) are composed via `#[command(flatten)]`.
mod ai_arguments;
mod color_arguments;
mod edit_arguments;
mod initialize_arguments;
mod jump_arguments;
mod list_arguments;
mod open_arguments;
mod prompt_arguments;
mod remove_arguments;
mod repository_selection_arguments;
mod reportal_cli;
mod run_arguments;
mod status_arguments;
mod sync_arguments;
mod tag_filter_arguments;
mod web_arguments;
mod workspace_arguments;
mod workspace_filter_arguments;

pub use ai_arguments::AiArguments;
pub use color_arguments::ColorArguments;
pub use edit_arguments::EditArguments;
pub use initialize_arguments::{InitializeArguments, InitializeShell};
pub use jump_arguments::JumpArguments;
pub use list_arguments::{ListArguments, ListArgumentsFilterParts};
pub use open_arguments::OpenArguments;
pub use prompt_arguments::{PromptArguments, PromptShell};
pub use remove_arguments::RemoveArguments;
pub use reportal_cli::{ReportalCli, ReportalCliSubcommand};
pub use run_arguments::RunArguments;
pub use status_arguments::StatusArguments;
pub use sync_arguments::SyncArguments;
pub use web_arguments::WebArguments;
pub use workspace_arguments::{
    WorkspaceArguments, WorkspaceArgumentsCreateParts, WorkspaceArgumentsDeleteParts, WorkspaceArgumentsMemberEditParts,
    WorkspaceArgumentsSubcommand,
};
