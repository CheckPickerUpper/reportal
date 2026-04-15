//! All error conditions that `RePortal` can encounter during operation.
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

    /// A command name was not found in global or repo-level config.
    #[error("Command '{command_name}' not found. Add it under [commands.{command_name}] or [repos.<alias>.commands]")]
    CommandNotFound {
        /// The command name that was looked up.
        command_name: String,
    },

    /// The user-defined command process failed to launch.
    #[error("Failed to run command: {reason}")]
    CommandLaunchFailure {
        /// The underlying OS error message.
        reason: String,
    },

    /// No commands are available (neither global nor repo-level).
    #[error("No commands configured. Add [commands.<name>] sections to your config")]
    NoCommandsAvailable,

    /// A workspace name was not found in the config.
    #[error("Workspace '{workspace_name}' not found in config")]
    WorkspaceNotFound {
        /// The workspace name that was looked up.
        workspace_name: String,
    },

    /// A workspace name already exists in the config.
    #[error("Workspace '{workspace_name}' already exists in config")]
    WorkspaceAlreadyExists {
        /// The workspace name that collided.
        workspace_name: String,
    },

    /// A workspace references a repo alias that is not registered.
    ///
    /// Detected during the post-parse validation pass on config load,
    /// which prevents shipping a config where `.code-workspace` files
    /// would be generated pointing at a non-existent repo.
    #[error("Workspace '{workspace_name}' references unknown repo '{missing_alias}'")]
    WorkspaceHasDanglingRepo {
        /// The workspace that contains the dangling reference.
        workspace_name: String,
        /// The repo alias that does not resolve to any registered repo.
        missing_alias: String,
    },

    /// Failed to read or write a `.code-workspace` file on disk.
    #[error("`.code-workspace` file I/O error for '{file_path}': {reason}")]
    CodeWorkspaceIoFailure {
        /// The path of the `.code-workspace` file that failed I/O.
        file_path: String,
        /// The underlying filesystem error message.
        reason: String,
    },

    /// A `.code-workspace` file exists but is not valid JSONC, or has
    /// a shape that the parse-merge-write path cannot round-trip.
    #[error("`.code-workspace` file at '{file_path}' is not valid JSONC: {reason}")]
    CodeWorkspaceParseFailure {
        /// The path of the `.code-workspace` file that failed to parse.
        file_path: String,
        /// The parse error detail from the JSONC deserializer.
        reason: String,
    },

    /// Refuses to remove a repo that is still a member of one or
    /// more workspaces. The user must edit those workspaces (or
    /// delete them) before the repo can be unregistered, because
    /// silently stripping the repo from every containing workspace
    /// would destroy user-declared membership without consent.
    #[error(
        "Repo '{alias}' is still a member of workspace(s): {affected_workspaces}. \
         Remove it from each workspace (rep workspace remove-repo) or delete them, then retry."
    )]
    RepoIsWorkspaceMember {
        /// The repo alias the user asked to remove.
        alias: String,
        /// Comma-separated list of workspace names that still contain the repo.
        affected_workspaces: String,
    },
}
