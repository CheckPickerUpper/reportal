/// Interactively registers a new repo in the RePortal config.
/// Supports both local paths and git URLs (clones first, then registers).
mod add_source;
mod alias_suggestion;
mod clone_destination;
mod clone_placement;
mod git_clone_operation;
mod git_remote_detection;
mod registration_context;
mod run;

pub use run::run_add;
