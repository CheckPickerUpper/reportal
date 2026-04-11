/// How the `rep color` subcommand was invoked, controlling both stdout
/// output and no-match behavior.

/// Invocation mode for the color subcommand.
///
/// `PromptHook` is used by the shell prompt function — it prints the
/// resolved title to stdout and preserves existing terminal state when
/// no repo matches the working directory.
///
/// `Explicit` is the default for direct `rep color` calls — it stays
/// silent on stdout and resets the tab color when no repo matches.
#[derive(Clone, clap::ValueEnum)]
pub enum ColorCommandMode {
    /// Shell prompt hook: prints title to stdout, preserves state on no-match.
    PromptHook,
    /// Direct invocation: silent stdout, resets color on no-match.
    Explicit,
}
