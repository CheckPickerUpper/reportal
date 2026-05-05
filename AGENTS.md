# reportal — agent notes

Rust CLI for registering and jumping between local git repos.

## Config location

The user config lives at `~/.reportal/config.toml`. The config directory is `~/.reportal/`. In code, the path is resolved by `config_file_path()` in `src/reportal_config/reportal_config_root.rs`, which returns `$HOME/.reportal/config.toml`.

Do not confuse reportal's config with `/home/ozzy/dev/repos.toml`. That file is an unrelated repo inventory (has a `[metadata]` table and `[[repos]]` array) and reportal never reads it.

## Adding a repo

Prefer `reportal add <absolute-path>`. It reads the git remote, assigns defaults, and handles alias collisions.

One caveat: `reportal add` requires an interactive TTY. Running it non-interactively (agents, scripts, CI) fails with `Config I/O error: IO error: not a terminal`.

For non-interactive contexts, edit `~/.reportal/config.toml` directly. Every `RepoEntry` field except `path` is `#[serde(default)]`, so the minimum entry is:

```toml
[repos.my-repo]
path = "~/dev/my-repo"
remote = "https://github.com/owner/my-repo.git"
```

Optional fields are `description`, `tags`, `aliases`, `title`, `color`, `commands`, and `shell_alias`. Aliases must be unique across all repos and workspaces. `color` is a hex `#RRGGBB` string or empty. Full schema in `src/reportal_config/repo_entry.rs`.

## Shell-alias export (`shell_alias = true`)

Per-entry opt-in on `[commands.*]`, `[repos.*]`, and `[workspaces.*]`. When set, `rep init <shell>` emits a top-level shell function for the entry (canonical key plus every declared alias for repos/workspaces). The function dispatches to the matching `rep` subcommand:

- repo entry → `cd "$(rep jump <name>)"; rep color`
- workspace entry → `cd "$(rep workspace jump <name>)"; rep color`
- global command entry → `rep run --cmd <name> "$@"`

Names that collide with the base integration's built-ins (`rj`, `ro`, `rjw`, `row`, `rw`, `rr`) or that contain characters outside `[A-Za-z0-9_-]` (or start with a digit) are silently skipped at emission time. Each emitted bash/zsh function is preceded by `unalias <name> 2>/dev/null` (and `Remove-Item Alias:<name> -ErrorAction SilentlyContinue` on PowerShell) so a pre-existing shell alias of the same name does not block the function definition. Schema in `src/reportal_config/shell_alias_export.rs`; emission in `src/reportal_commands/shell_alias_emit.rs`.

## System-command shadow rejection

`RepoRegistrationBuilder::build()` and `WorkspaceRegistrationBuilder::build()` reject any alias / canonical name that resolves to an existing executable on the user's `PATH`, regardless of whether `shell_alias = true` is set. The lookup walks `$PATH` directly (no `which` shell-out) via `src/system_executable_lookup.rs`. This means `rep add` will refuse aliases like `mc`, `train`, `git`, `ls` etc. on systems where those resolve to real binaries; pick a different name. Existing config entries that were hand-edited into TOML before this validation existed are NOT re-checked at load time, so legacy shadow aliases continue to work as `rj <alias>` targets.

## Workspaces

Workspaces live under `[workspaces.<name>]` in the same config and reference repos by alias. The generator writes `~/.reportal/workspaces/<name>.code-workspace` on every config save.

## Build and install

```sh
cargo build --release
cargo install --path .         # installs to ~/.cargo/bin/reportal
```

## Commands

`reportal --help` lists: init, list, jump, open, add, edit, remove, color, status, sync, doctor, ai, web, run, workspace.
