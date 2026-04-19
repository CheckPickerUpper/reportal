//! CLI args for `rep workspace` and its subcommands.
//!
//! Exposes the subcommand tree that drives every operation on
//! registered VSCode/Cursor `.code-workspace` definitions:
//! listing, showing, creating, deleting, member editing, and
//! opening. Shared fields that would otherwise repeat across
//! variants (`workspace_name`, `repo_alias`) are extracted into
//! composable arg structs and flattened into each action's
//! payload so the variants never duplicate the field declaration.

use clap::{Args, Subcommand};

/// Arguments for the top-level `rep workspace` command.
#[derive(Args)]
pub struct WorkspaceArgs {
    /// The workspace action the user invoked.
    #[command(subcommand)]
    action: WorkspaceArgsSubcommand,
}

/// Consuming accessor for the workspace subcommand.
impl WorkspaceArgs {
    /// Extracts the action variant, consuming the parsed args.
    #[must_use]
    pub fn into_action(self) -> WorkspaceArgsSubcommand {
        self.action
    }
}

/// Positional `workspace_name` argument shared by every action
/// that targets a specific workspace.
///
/// Extracted so the identifier lives in exactly one place in the
/// type system, preventing the repeated-union-fields class of
/// drift where a rename on one variant leaves others out of sync.
#[derive(Args)]
pub struct WorkspaceArgsName {
    /// Name of the workspace being targeted.
    workspace_name: String,
}

/// Consuming accessor for the workspace name.
impl WorkspaceArgsName {
    /// Extracts the workspace name, consuming the parsed arg.
    #[must_use]
    pub fn into_name(self) -> String {
        self.workspace_name
    }
}

/// Positional `repo_alias` argument shared by member-editing
/// actions (`add-repo`, `remove-repo`).
#[derive(Args)]
pub struct WorkspaceArgsRepoAlias {
    /// Repo alias being added to or removed from the workspace.
    repo_alias: String,
}

/// Consuming accessor for the repo alias.
impl WorkspaceArgsRepoAlias {
    /// Extracts the repo alias, consuming the parsed arg.
    #[must_use]
    pub fn into_alias(self) -> String {
        self.repo_alias
    }
}

/// Payload for actions that target one workspace by name and
/// carry no other arguments: `show`, `open`, `rebuild`.
#[derive(Args)]
pub struct WorkspaceArgsNameOnly {
    /// The workspace this action targets.
    #[command(flatten)]
    target: WorkspaceArgsName,
}

/// Consuming accessor for the targeted workspace name.
impl WorkspaceArgsNameOnly {
    /// Extracts the targeted workspace name, consuming the args.
    #[must_use]
    pub fn into_workspace_name(self) -> String {
        self.target.into_name()
    }
}

/// Payload for `rep workspace delete`, which targets one workspace
/// by name and optionally purges the on-disk directory.
#[derive(Args)]
pub struct WorkspaceArgsDelete {
    /// The workspace this action targets.
    #[command(flatten)]
    target: WorkspaceArgsName,
    /// Also remove the on-disk workspace directory (symlinks and
    /// `.code-workspace` file). Member repos are not touched. The
    /// default behavior is to leave the directory in place so an
    /// accidental delete cannot destroy an open editor session.
    #[arg(long, default_value_t = false)]
    purge: bool,
}

/// Consuming accessor for the delete-action payload.
impl WorkspaceArgsDelete {
    /// Extracts the delete-action fields into a named parts struct,
    /// consuming the parsed args.
    #[must_use]
    pub fn into_parts(self) -> WorkspaceArgsDeleteParts {
        WorkspaceArgsDeleteParts {
            workspace_name: self.target.into_name(),
            purge: self.purge,
        }
    }
}

/// Owned named-field result of `WorkspaceArgsDelete::into_parts`.
pub struct WorkspaceArgsDeleteParts {
    /// The workspace to unregister.
    workspace_name: String,
    /// Whether to also delete the on-disk workspace directory.
    purge: bool,
}

/// Accessors for the delete-action parts.
impl WorkspaceArgsDeleteParts {
    /// The workspace name to unregister.
    #[must_use]
    pub fn workspace_name(&self) -> &str {
        &self.workspace_name
    }

    /// Whether the user opted into removing the on-disk directory.
    #[must_use]
    pub fn purge(&self) -> bool {
        self.purge
    }
}

/// Payload for member-editing actions: `add-repo`, `remove-repo`.
#[derive(Args)]
pub struct WorkspaceArgsMemberEdit {
    /// The workspace whose membership is being edited.
    #[command(flatten)]
    target: WorkspaceArgsName,
    /// The repo being added to or removed from the workspace.
    #[command(flatten)]
    member: WorkspaceArgsRepoAlias,
}

/// Consuming accessor for the target workspace + member repo.
impl WorkspaceArgsMemberEdit {
    /// Extracts the targeted workspace name and the repo alias,
    /// consuming the parsed args.
    #[must_use]
    pub fn into_parts(self) -> WorkspaceArgsMemberEditParts {
        WorkspaceArgsMemberEditParts {
            workspace_name: self.target.into_name(),
            repo_alias: self.member.into_alias(),
        }
    }
}

/// Owned tuple-equivalent returned from
/// `WorkspaceArgsMemberEdit::into_parts`, with named fields so
/// call sites never confuse the two strings.
pub struct WorkspaceArgsMemberEditParts {
    /// The workspace whose membership is being edited.
    workspace_name: String,
    /// The repo being added to or removed from the workspace.
    repo_alias: String,
}

/// Destructuring accessors for the member-edit parts.
impl WorkspaceArgsMemberEditParts {
    /// The workspace name this edit targets.
    #[must_use]
    pub fn workspace_name(&self) -> &str {
        &self.workspace_name
    }

    /// The repo alias being added to or removed from the workspace.
    #[must_use]
    pub fn repo_alias(&self) -> &str {
        &self.repo_alias
    }
}

/// Payload for `rep workspace create`, which declares a new
/// workspace with an initial member list, description, optional
/// custom file path, and optional short-name aliases.
#[derive(Args)]
pub struct WorkspaceArgsCreate {
    /// Name for the new workspace.
    #[command(flatten)]
    target: WorkspaceArgsName,
    /// Comma-separated list of repo aliases that belong to the
    /// workspace, in the order they should appear in the editor's
    /// sidebar.
    #[arg(long, value_delimiter = ',')]
    repos: Vec<String>,
    /// Human-readable description shown in `rep workspace list`.
    #[arg(long, default_value = "")]
    description: String,
    /// Explicit filesystem path for the workspace directory
    /// (the one that will contain the symlinks and the
    /// `.code-workspace` file). Defaults to
    /// `<default_workspace_root>/<name>/`. A legacy value
    /// pointing at a `.code-workspace` file is reinterpreted as
    /// the file's parent directory so pre-v0.15.2 invocations
    /// continue to work.
    #[arg(long, default_value = "")]
    file_path: String,
    /// Comma-separated short-name aliases that resolve to this
    /// workspace when passed to `rep workspace` subcommands.
    #[arg(long, value_delimiter = ',')]
    aliases: Vec<String>,
}

/// Consuming accessor for the create-action payload.
impl WorkspaceArgsCreate {
    /// Extracts all create-action fields into a named parts
    /// struct, consuming the parsed args.
    #[must_use]
    pub fn into_parts(self) -> WorkspaceArgsCreateParts {
        WorkspaceArgsCreateParts {
            workspace_name: self.target.into_name(),
            repo_aliases: self.repos,
            description: self.description,
            custom_file_path: self.file_path,
            workspace_aliases: self.aliases,
        }
    }
}

/// Owned named-field result of `WorkspaceArgsCreate::into_parts`.
///
/// Returned instead of a bare tuple so call sites never confuse
/// the string-typed fields.
pub struct WorkspaceArgsCreateParts {
    /// Name for the new workspace.
    workspace_name: String,
    /// Ordered list of repo aliases that belong to the workspace.
    repo_aliases: Vec<String>,
    /// Human-readable description of the workspace's purpose.
    description: String,
    /// Explicit filesystem path for the generated
    /// `.code-workspace` file, empty for the default location.
    custom_file_path: String,
    /// Short-name aliases that resolve to this workspace in
    /// `rep workspace` subcommands.
    workspace_aliases: Vec<String>,
}

/// Accessors for the create-action parts.
impl WorkspaceArgsCreateParts {
    /// The name for the new workspace.
    #[must_use]
    pub fn workspace_name(&self) -> &str {
        &self.workspace_name
    }

    /// The ordered list of repo aliases that belong to the
    /// workspace.
    #[must_use]
    pub fn repo_aliases(&self) -> &[String] {
        &self.repo_aliases
    }

    /// The human-readable description of the workspace's purpose.
    #[must_use]
    pub fn description(&self) -> &str {
        &self.description
    }

    /// The explicit filesystem path for the generated
    /// `.code-workspace` file, or empty string to use the default
    /// location.
    #[must_use]
    pub fn custom_file_path(&self) -> &str {
        &self.custom_file_path
    }

    /// The short-name aliases that resolve to this workspace in
    /// `rep workspace` subcommands.
    #[must_use]
    pub fn workspace_aliases(&self) -> &[String] {
        &self.workspace_aliases
    }
}

/// Subcommands under `rep workspace`.
#[derive(Subcommand)]
pub enum WorkspaceArgsSubcommand {
    /// List all registered workspaces with their member repos.
    #[command(alias = "ls")]
    List,
    /// Show a single workspace's details, including the resolved
    /// absolute paths of its member repos and the location of the
    /// workspace directory / `.code-workspace` file on disk.
    Show(WorkspaceArgsNameOnly),
    /// Create a new workspace and materialize its on-disk
    /// directory (symlinks + `.code-workspace` file) under
    /// `<default_workspace_root>/<name>/` unless a custom
    /// directory path is given via `--file-path`.
    Create(WorkspaceArgsCreate),
    /// Delete a workspace from the config. By default leaves the
    /// on-disk workspace directory in place. Pass `--purge` to
    /// also remove the directory (symlinks and `.code-workspace`
    /// file); member repos are never touched.
    #[command(alias = "rm")]
    Delete(WorkspaceArgsDelete),
    /// Add a repo alias to an existing workspace and regenerate
    /// its workspace directory (links + `.code-workspace` file).
    AddRepo(WorkspaceArgsMemberEdit),
    /// Remove a repo alias from an existing workspace and
    /// regenerate its workspace directory.
    RemoveRepo(WorkspaceArgsMemberEdit),
    /// Open a workspace in the configured default editor by
    /// running `<editor> <workspace-file-path>`. Without a
    /// workspace name, presents a fuzzy finder.
    Open(WorkspaceArgsOptionalNameOnly),
    /// Print the workspace's materialized directory path so the
    /// `rjw` shell wrapper can cd there. Without a workspace name,
    /// presents a fuzzy finder.
    Jump(WorkspaceArgsOptionalNameOnly),
    /// Rebuild the workspace's on-disk directory: recreate the
    /// member symlinks / junctions and regenerate the
    /// `.code-workspace` file from the current config. Idempotent;
    /// useful after renaming / moving a member repo or if the
    /// workspace directory was deleted by hand.
    Rebuild(WorkspaceArgsNameOnly),
}

/// Payload for actions that target a workspace by name but fall
/// back to fuzzy-select when no name is given — currently `jump`
/// and `open`.
///
/// Distinct from [`WorkspaceArgsNameOnly`] because `show`,
/// `delete`, and the member-edit actions must name an explicit
/// target — silently fuzzy-selecting a destructive action's
/// subject would violate the explicit-mutation rule.
#[derive(Args)]
pub struct WorkspaceArgsOptionalNameOnly {
    /// The workspace this action targets, or empty to present a
    /// fuzzy-select prompt.
    #[arg(default_value = "")]
    workspace_name: String,
}

/// Consuming accessor for the optional target name.
impl WorkspaceArgsOptionalNameOnly {
    /// Extracts the optional workspace name as a string that is
    /// empty when the user omitted the argument.
    #[must_use]
    pub fn into_optional_workspace_name(self) -> String {
        self.workspace_name
    }
}
