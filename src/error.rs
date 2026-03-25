/// All error conditions that RePortal can encounter during operation.
#[derive(Debug, thiserror::Error)]
pub enum ReportalError {
    /// Config file does not exist at the expected location.
    #[error("Config not found at {config_path}\nRun 'reportal init' to create one.")]
    ConfigNotFound {
        /// Absolute path where the config was expected.
        config_path: String,
    },

    /// Config file exists but contains invalid TOML or schema.
    #[error("Failed to parse config: {reason}")]
    ConfigParseFailure {
        /// The parse error detail from the TOML deserializer.
        reason: String,
    },

    /// Failed to read or write the config file on disk.
    #[error("Config I/O error: {reason}")]
    ConfigIoFailure {
        /// The underlying filesystem error message.
        reason: String,
    },

    /// No repos matched the given filter criteria.
    #[error("No repos found matching filter")]
    NoReposMatchFilter,

    /// The user cancelled an interactive selection prompt.
    #[error("Selection cancelled")]
    SelectionCancelled,

    /// A repo alias was not found in the config.
    #[error("Repo '{alias}' not found in config")]
    RepoNotFound {
        /// The alias that was looked up.
        alias: String,
    },

    /// A repo alias already exists in the config.
    #[error("Repo '{alias}' already exists in config")]
    RepoAlreadyExists {
        /// The alias that collided.
        alias: String,
    },

    /// The editor process failed to launch.
    #[error("Failed to open editor: {reason}")]
    EditorLaunchFailure {
        /// The underlying OS error message.
        reason: String,
    },

    /// A registration field failed validation.
    #[error("Invalid {field}: {reason}")]
    ValidationFailure {
        /// Which field failed validation.
        field: String,
        /// Why the value was rejected.
        reason: String,
    },

    /// A color value in config is not valid `#RRGGBB` hex.
    #[error("Invalid color '{value}': expected #RRGGBB hex format")]
    InvalidColor {
        /// The malformed color string.
        value: String,
    },

    /// An AI tool name was not found in the config.
    #[error("AI tool '{tool_name}' not found in config. Add it under [ai_tools.{tool_name}]")]
    AiToolNotFound {
        /// The tool name that was looked up.
        tool_name: String,
    },

    /// The AI tool process failed to launch.
    #[error("Failed to launch AI tool: {reason}")]
    AiToolLaunchFailure {
        /// The underlying OS error message.
        reason: String,
    },

    /// No AI tools are configured.
    #[error("No AI tools configured. Add [ai_tools.<name>] sections to your config")]
    NoAiToolsConfigured,

    /// No default AI tool is set and none was specified via --tool.
    #[error("No default AI tool set. Use --tool <name> or set default_ai_tool in config")]
    NoDefaultAiTool,

    /// A repo has no remote URL configured and none could be detected from git.
    #[error("No remote URL for repo '{alias}'. Set the 'remote' field in config or add a git remote.")]
    NoRemoteUrl {
        /// The alias of the repo missing a remote.
        alias: String,
    },

    /// The browser failed to open a URL.
    #[error("Failed to open browser: {reason}")]
    BrowserLaunchFailure {
        /// The underlying OS error message.
        reason: String,
    },
}
