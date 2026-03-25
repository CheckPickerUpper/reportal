/// Configuration loading, saving, and repo registry for RePortal.
///
/// The config file lives at `~/.reportal/config.toml` and stores
/// all registered repositories, AI tools, and global settings.

mod ai_tool_entry;
mod global_settings;
mod hex_color;
mod reportal_config_root;
mod repo_entry;
mod tag_filter;

pub use ai_tool_entry::AiToolEntry;
pub use global_settings::{PathDisplayFormat, PathVisibility, ReportalSettings};
pub use hex_color::HexColor;
pub use repo_entry::{RepoColor, RepoEntry, RepoRegistrationBuilder, TabTitle};
pub use reportal_config_root::ReportalConfig;
pub use tag_filter::TagFilter;
