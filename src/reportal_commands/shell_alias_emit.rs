//! Shell-function snippet generator for opted-in config entries.
//!
//! `rep init <shell>` calls into this module after emitting the
//! base integration script so users who set `shell_alias = true`
//! on a command, repo, or workspace get a top-level shell
//! function for that entry — `vna` instead of `rj vna`, `uc`
//! instead of `rep run --cmd uc` — without further setup.
//!
//! The emission is layered in its own module rather than folded
//! into `shell_integration` so the integration block (built-in
//! wrappers + auto-wire bookkeeping) and the per-config alias
//! block (driven by user opt-ins) can evolve on independent
//! release cadences.

use std::collections::HashSet;

use crate::cli_args::InitializeShell;
use crate::reportal_config::{
    CommandEntry, RepoEntry, ReportalConfig, ShellAliasExport, WorkspaceEntry,
};

/// Parameters for the shell-alias emission entry point.
///
/// Held as a struct rather than two positional arguments so the
/// call site at `run_initialize` reads as a single named binding
/// instead of an ambiguous `(shell, configuration)` pair.
pub struct ShellAliasEmissionParameters<'configuration> {
    /// Target shell whose function syntax to emit.
    pub target_shell: InitializeShell,
    /// Loaded configuration to scan for opted-in entries.
    pub configuration: &'configuration ReportalConfig,
}

/// Built-in shell function names emitted by the base integration
/// script. Listed here so opted-in user entries cannot clobber
/// the built-ins by reusing the same name — collisions are
/// silently skipped at emission time and the user keeps the
/// expected `rj`/`ro`/`rjw`/`row`/`rw`/`rr` behavior.
const BUILTIN_SHELL_FUNCTION_NAMES: &[&str] =
    &["rj", "ro", "rjw", "row", "rw", "rr"];

/// @why Builds the per-shell snippet that defines a top-level
/// function for every opted-in repo, workspace, and command so
/// users invoke their config entries directly from the prompt
/// without typing `rj`, `rjw`, or `rep run --cmd` first.
#[must_use]
pub fn shell_alias_export_snippet(
    parameters: &ShellAliasEmissionParameters<'_>,
) -> String {
    let exported_dispatch_entries =
        collect_exported_dispatch_entries(parameters.configuration);
    if exported_dispatch_entries.is_empty() {
        return String::new();
    }
    match parameters.target_shell {
        InitializeShell::Zsh | InitializeShell::Bash => {
            render_bash_snippet(&exported_dispatch_entries)
        }
        InitializeShell::Powershell => {
            render_powershell_snippet(&exported_dispatch_entries)
        }
    }
}

/// What the emitted shell function should dispatch to. Three
/// variants because each one expands to a different built-in
/// (`rep jump`, `rep workspace jump`, `rep run --cmd`) and the
/// emission helpers branch on this rather than re-deriving the
/// dispatch shape from a string tag.
enum ExportedDispatchKind {
    /// Repo jump: shell function runs `rep jump <name>` and cd's
    /// into the resolved path, applying repo color.
    RepositoryJump,
    /// Workspace jump: shell function runs `rep workspace jump
    /// <name>` and cd's into the workspace directory.
    WorkspaceJump,
    /// Configured command run: shell function runs `rep run
    /// --cmd <name>` after the user's repo selection prompt.
    ConfiguredCommandRun,
}

/// One emitted shell function: the name as the user typed it
/// (canonical key or an alias) and the dispatch kind that decides
/// which `rep` subcommand the function body invokes.
struct ExportedDispatchEntry {
    /// The shell-function name to define — exactly the canonical
    /// key or declared alias from config, validated to look like
    /// a usable shell identifier before reaching here.
    function_name: String,
    /// The argument value passed to the underlying `rep` command
    /// (`rep jump <argument>`, `rep workspace jump <argument>`,
    /// `rep run --cmd <argument>`). Always the exact key/alias
    /// the user typed, so dispatch resolves through the same
    /// alias machinery the rest of the CLI already uses.
    dispatch_argument: String,
    /// Which `rep` subcommand the emitted function body invokes.
    dispatch_kind: ExportedDispatchKind,
}

/// Collects every opted-in entry from the configuration and
/// converts each (canonical key + each declared alias) into one
/// `ExportedDispatchEntry`. Skips names that collide with the
/// base integration's built-in functions and names that are not
/// usable as bare shell identifiers.
fn collect_exported_dispatch_entries(
    configuration: &ReportalConfig,
) -> Vec<ExportedDispatchEntry> {
    let mut collected: Vec<ExportedDispatchEntry> = Vec::new();
    for (command_key, command_entry) in configuration.global_commands() {
        append_command_dispatch_entry(&mut collected, command_key, command_entry);
    }
    for (repository_canonical_key, repository_entry) in
        configuration.repos_with_aliases()
    {
        append_repository_dispatch_entries(
            &mut collected,
            repository_canonical_key,
            repository_entry,
        );
    }
    for (workspace_canonical_name, workspace_entry) in
        configuration.workspaces_with_names()
    {
        append_workspace_dispatch_entries(
            &mut collected,
            workspace_canonical_name,
            workspace_entry,
        );
    }
    deduplicate_by_function_name(collected)
}

/// Removes second-and-later occurrences of any function name so
/// each emitted shell function gets defined exactly once. Repos
/// can list their canonical key inside their own `aliases` array
/// (a no-op alias the rest of the CLI tolerates), and two
/// distinct entries opted-in with the same shell-function name
/// would otherwise produce two competing definitions; the first
/// wins so the iteration order — commands, repos, workspaces —
/// remains the published precedence.
fn deduplicate_by_function_name(
    raw_entries: Vec<ExportedDispatchEntry>,
) -> Vec<ExportedDispatchEntry> {
    let mut already_emitted_names: HashSet<String> = HashSet::new();
    let mut deduplicated: Vec<ExportedDispatchEntry> =
        Vec::with_capacity(raw_entries.len());
    for raw_entry in raw_entries {
        if already_emitted_names.contains(&raw_entry.function_name) {
            continue;
        }
        already_emitted_names.insert(raw_entry.function_name.clone());
        deduplicated.push(raw_entry);
    }
    deduplicated
}

/// Pushes a command's dispatch entry onto the collector when the
/// command is opted-in and its key is a usable shell identifier.
fn append_command_dispatch_entry(
    collector: &mut Vec<ExportedDispatchEntry>,
    command_key: &str,
    command_entry: &CommandEntry,
) {
    match command_entry.shell_alias_export() {
        ShellAliasExport::Disabled => {}
        ShellAliasExport::Enabled => {
            if is_emittable_function_name(command_key) {
                collector.push(ExportedDispatchEntry {
                    function_name: command_key.to_owned(),
                    dispatch_argument: command_key.to_owned(),
                    dispatch_kind: ExportedDispatchKind::ConfiguredCommandRun,
                });
            }
        }
    }
}

/// Pushes one dispatch entry per (canonical key + each declared
/// alias) for an opted-in repo, skipping names that are not
/// emittable shell identifiers.
fn append_repository_dispatch_entries(
    collector: &mut Vec<ExportedDispatchEntry>,
    repository_canonical_key: &str,
    repository_entry: &RepoEntry,
) {
    match repository_entry.shell_alias_export() {
        ShellAliasExport::Disabled => {}
        ShellAliasExport::Enabled => {
            push_one_dispatch_entry_if_emittable(
                collector,
                repository_canonical_key,
                repository_canonical_key,
                ExportedDispatchKind::RepositoryJump,
            );
            for declared_alias in
                crate::reportal_config::HasAliases::aliases(repository_entry)
            {
                push_one_dispatch_entry_if_emittable(
                    collector,
                    declared_alias,
                    declared_alias,
                    ExportedDispatchKind::RepositoryJump,
                );
            }
        }
    }
}

/// Pushes one dispatch entry per (canonical name + each declared
/// alias) for an opted-in workspace, skipping names that are not
/// emittable shell identifiers.
fn append_workspace_dispatch_entries(
    collector: &mut Vec<ExportedDispatchEntry>,
    workspace_canonical_name: &str,
    workspace_entry: &WorkspaceEntry,
) {
    match workspace_entry.shell_alias_export() {
        ShellAliasExport::Disabled => {}
        ShellAliasExport::Enabled => {
            push_one_dispatch_entry_if_emittable(
                collector,
                workspace_canonical_name,
                workspace_canonical_name,
                ExportedDispatchKind::WorkspaceJump,
            );
            for declared_alias in
                crate::reportal_config::HasAliases::aliases(workspace_entry)
            {
                push_one_dispatch_entry_if_emittable(
                    collector,
                    declared_alias,
                    declared_alias,
                    ExportedDispatchKind::WorkspaceJump,
                );
            }
        }
    }
}

/// Pushes a single dispatch entry onto the collector iff the
/// candidate function name is a usable shell identifier and does
/// not collide with the base integration's built-in functions.
fn push_one_dispatch_entry_if_emittable(
    collector: &mut Vec<ExportedDispatchEntry>,
    candidate_function_name: &str,
    dispatch_argument: &str,
    dispatch_kind: ExportedDispatchKind,
) {
    if !is_emittable_function_name(candidate_function_name) {
        return;
    }
    collector.push(ExportedDispatchEntry {
        function_name: candidate_function_name.to_owned(),
        dispatch_argument: dispatch_argument.to_owned(),
        dispatch_kind,
    });
}

/// Whether a candidate name is safe to emit as a bare shell
/// function identifier on every supported shell.
///
/// Allows ASCII letters, digits, the `_` character, and the `-`
/// character. zsh, bash, and `PowerShell` all accept this set as
/// function-name characters. The first character must not be a
/// digit. Names that collide with the base integration's
/// reserved built-ins are also rejected so user opt-ins cannot
/// accidentally shadow `rj`, `ro`, etc.
fn is_emittable_function_name(candidate_name: &str) -> bool {
    if candidate_name.is_empty() {
        return false;
    }
    if BUILTIN_SHELL_FUNCTION_NAMES.contains(&candidate_name) {
        return false;
    }
    let mut character_iterator = candidate_name.chars();
    let Some(first_character) = character_iterator.next() else {
        return false;
    };
    if !first_character.is_ascii_alphabetic() && first_character != '_' {
        return false;
    }
    for following_character in character_iterator {
        let is_allowed = following_character.is_ascii_alphanumeric()
            || following_character == '_'
            || following_character == '-';
        if !is_allowed {
            return false;
        }
    }
    true
}

/// Builds the bash/zsh emission block for a non-empty list of
/// dispatch entries.
///
/// `unalias <name>` is emitted on its own line BEFORE the
/// function definition (not chained with `;`) because zsh
/// expands aliases at parse time, before any preceding
/// statement on the same compound line has run. Putting
/// `unalias` and the `name() { ... }` definition on one line
/// would still trigger zsh's "defining function based on alias"
/// parse error and abort the whole `eval` block. Splitting them
/// onto two physical lines runs the unalias to completion before
/// the parser ever sees the function-definition token.
fn render_bash_snippet(exported_entries: &[ExportedDispatchEntry]) -> String {
    let mut rendered_block = String::from(
        "# RePortal user-config shell aliases — generated from `shell_alias = true` entries.\n",
    );
    for exported_entry in exported_entries {
        rendered_block.push_str(&format!(
            "unalias {name} 2>/dev/null\n",
            name = exported_entry.function_name,
        ));
        match exported_entry.dispatch_kind {
            ExportedDispatchKind::RepositoryJump => {
                rendered_block.push_str(&format!(
                    "function {name}() {{ cd \"$(rep jump {arg})\"; rep color; }}\n",
                    name = exported_entry.function_name,
                    arg = exported_entry.dispatch_argument,
                ));
            }
            ExportedDispatchKind::WorkspaceJump => {
                rendered_block.push_str(&format!(
                    "function {name}() {{ cd \"$(rep workspace jump {arg})\"; rep color; }}\n",
                    name = exported_entry.function_name,
                    arg = exported_entry.dispatch_argument,
                ));
            }
            ExportedDispatchKind::ConfiguredCommandRun => {
                rendered_block.push_str(&format!(
                    "function {name}() {{ rep run --cmd {arg} \"$@\"; }}\n",
                    name = exported_entry.function_name,
                    arg = exported_entry.dispatch_argument,
                ));
            }
        }
    }
    rendered_block
}

/// Builds the `PowerShell` emission block for a non-empty list of
/// dispatch entries. Uses `global:` scope so the functions remain
/// callable from interactive sessions regardless of how the
/// integration script was loaded.
///
/// Each function definition is preceded by a silent
/// `Remove-Item Alias:<name>` so a pre-existing PowerShell alias
/// (auto-loaded modules, the user's profile, etc.) does not
/// shadow the reportal function that the user explicitly opted
/// into.
fn render_powershell_snippet(exported_entries: &[ExportedDispatchEntry]) -> String {
    let mut rendered_block = String::from(
        "# RePortal user-config shell aliases — generated from `shell_alias = true` entries.\n",
    );
    for exported_entry in exported_entries {
        match exported_entry.dispatch_kind {
            ExportedDispatchKind::RepositoryJump => {
                rendered_block.push_str(&format!(
                    "Remove-Item Alias:{name} -ErrorAction SilentlyContinue; function global:{name} {{ Set-Location (rep jump {arg}); rep color }}\n",
                    name = exported_entry.function_name,
                    arg = exported_entry.dispatch_argument,
                ));
            }
            ExportedDispatchKind::WorkspaceJump => {
                rendered_block.push_str(&format!(
                    "Remove-Item Alias:{name} -ErrorAction SilentlyContinue; function global:{name} {{ Set-Location (rep workspace jump {arg}); rep color }}\n",
                    name = exported_entry.function_name,
                    arg = exported_entry.dispatch_argument,
                ));
            }
            ExportedDispatchKind::ConfiguredCommandRun => {
                rendered_block.push_str(&format!(
                    "Remove-Item Alias:{name} -ErrorAction SilentlyContinue; function global:{name} {{ rep run --cmd {arg} @args }}\n",
                    name = exported_entry.function_name,
                    arg = exported_entry.dispatch_argument,
                ));
            }
        }
    }
    rendered_block
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_empty_function_name() {
        assert!(!is_emittable_function_name(""));
    }

    #[test]
    fn rejects_builtin_collision() {
        assert!(!is_emittable_function_name("rj"));
        assert!(!is_emittable_function_name("rjw"));
        assert!(!is_emittable_function_name("rr"));
    }

    #[test]
    fn allows_hyphenated_repository_alias() {
        assert!(is_emittable_function_name("venoble-app"));
    }

    #[test]
    fn rejects_leading_digit() {
        assert!(!is_emittable_function_name("4chan"));
    }

    #[test]
    fn rejects_special_characters() {
        assert!(!is_emittable_function_name("foo bar"));
        assert!(!is_emittable_function_name("foo.bar"));
        assert!(!is_emittable_function_name("foo/bar"));
    }
}
