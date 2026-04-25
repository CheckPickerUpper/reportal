//! Configuration loading, saving, and repo registry for `RePortal`.
//!
//! The config file lives at `~/.reportal/config.toml` and stores
//! all registered repositories, AI tools, and global settings.

mod ai_tool_entry;
mod alias_collision_query;
mod command_entry;
mod global_settings;
mod has_aliases;
mod hex_color;
mod shell_alias_export;
mod repository_color;
mod repo_entry;
mod repo_registration_builder;
mod reportal_config_root;
mod tab_title;
mod tag_filter;
mod workspace_entry;
mod workspace_filter;
mod workspace_member;
mod workspace_registration_builder;

pub use command_entry::CommandEntry;
pub use global_settings::PathVisibility;
pub use has_aliases::HasAliases;
pub use hex_color::HexColor;
pub use repository_color::RepoColor;
pub use repo_entry::RepoEntry;
pub use repo_registration_builder::RepoRegistrationBuilder;
pub use reportal_config_root::ReportalConfig;
pub use shell_alias_export::ShellAliasExport;
pub use tab_title::TabTitle;
pub use tag_filter::TagFilter;
pub use workspace_entry::WorkspaceEntry;
pub use workspace_filter::WorkspaceFilter;
pub use workspace_member::{WorkspaceMember, WorkspaceMemberAliasLookup};
pub use workspace_registration_builder::WorkspaceRegistrationBuilder;
