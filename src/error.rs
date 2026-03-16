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

    /// Config already exists when trying to initialize a new one.
    #[error("Config already exists at {config_path}")]
    ConfigAlreadyExists {
        /// Path to the existing config file.
        config_path: String,
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
}
