//! Configuration loading, saving, and repo registry for `RePortal`.
//!
//! The config file lives at `~/.reportal/config.toml` and stores
//! all registered repositories, AI tools, and global settings.

mod ai_tool_entry;
mod command_entry;
mod global_settings;
mod hex_color;
mod reportal_config_root;
mod repo_entry;
mod tag_filter;

pub use command_entry::CommandEntry;
pub use global_settings::PathVisibility;
pub use hex_color::HexColor;
pub use repo_entry::{RepoColor, RepoEntry, RepoRegistrationBuilder, TabTitle};
pub use reportal_config_root::ReportalConfig;
pub use tag_filter::TagFilter;
