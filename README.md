<div align="center">

<!-- Logo goes here -->
<!-- <img src="assets/logo.png" width="200" /> -->

# RePortal

**Jump between repos. Open in your editor. Stay in sync across machines.**

[![Crates.io](https://img.shields.io/crates/v/reportal.svg)](https://crates.io/crates/reportal)
[![GitHub Release](https://img.shields.io/github/v/release/CheckPickerUpper/reportal)](https://github.com/CheckPickerUpper/reportal/releases/latest)
[![Homebrew](https://img.shields.io/badge/homebrew-tap-FBB040.svg)](https://github.com/CheckPickerUpper/homebrew-tap)
[![Scoop](https://img.shields.io/badge/scoop-bucket-5B5EA6.svg)](https://github.com/CheckPickerUpper/scoop-reportal)
[![License: MIT](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)
[![Rust](https://img.shields.io/badge/rust-1.78%2B-orange.svg)](https://www.rust-lang.org/)
[![Windows](https://img.shields.io/badge/platform-windows%20%7C%20macos%20%7C%20linux-lightgrey.svg)]()
[![PRs Welcome](https://img.shields.io/badge/PRs-welcome-brightgreen.svg)]()

</div>

---

RePortal is a single-binary CLI that keeps a registry of your dev repos and lets you fuzzy-jump between them, open them in your editor, and clone missing repos on a new machine.

## Install

All methods install both `reportal` and `rep` (short alias) binaries.

### Cargo (all platforms)

```bash
cargo install reportal
```

### Homebrew (macOS / Linux)

```bash
brew tap CheckPickerUpper/tap
brew install reportal
```

### Scoop (Windows)

```powershell
scoop bucket add reportal https://github.com/CheckPickerUpper/scoop-reportal
scoop install reportal
```

### Winget (Windows)

```powershell
winget install CheckPickerUpper.RePortal
```

### Shell installer (macOS / Linux)

```bash
curl --proto '=https' --tlsv1.2 -LsSf https://github.com/CheckPickerUpper/reportal/releases/latest/download/reportal-installer.sh | sh
```

### PowerShell installer (Windows)

```powershell
powershell -ExecutionPolicy Bypass -c "irm https://github.com/CheckPickerUpper/reportal/releases/latest/download/reportal-installer.ps1 | iex"
```

Both installers drop the binaries into a user-local directory and auto-append an idempotent `eval "$(rep init <shell>)"` block (or the PowerShell equivalent) to your shell rc file, so integration is wired up on the next shell start — no manual rc editing required.

### From source

```bash
git clone https://github.com/CheckPickerUpper/reportal.git
cd reportal
cargo install --path .
```

## Quick start

Shell integration is auto-wired the first time you run any `rep` subcommand: the binary detects that the integration isn't sourced yet, appends an idempotent fenced `eval` block to your rc file (`~/.zshrc`, `~/.bashrc`, or `$PROFILE`), and prints a single `source your-rc` hint. Subsequent runs are a no-op. If you prefer to wire it yourself — same pattern as `starship`, `zoxide`, `direnv`, and `mise` — add one of these lines manually:

**Zsh** (`~/.zshrc`):

```zsh
eval "$(rep init zsh)"
```

**Bash** (`~/.bashrc`):

```bash
eval "$(rep init bash)"
```

**PowerShell** (`$PROFILE`):

```powershell
Invoke-Expression (& rep init powershell | Out-String)
```

That's it — no disk writes beyond the one-shot rc append, no profile editing after setup, no prompts. The integration code is regenerated every shell session, so a `cargo install reportal` upgrade takes effect on the next terminal you open. The first time you run any `rep` subcommand, a default `~/.reportal/config.toml` is created for you.

```bash
# Register a local repo
rep add ~/dev/my-project

# Clone and register from a URL
rep add https://github.com/org/repo.git

# List all registered repos
rep list

# Jump to a repo (cd)
rj

# Open a repo in your editor
ro
```

## Commands

| Command | Alias | What it does |
|---------|-------|-------------|
| `rep init <shell>` | | Print shell integration code to stdout (zsh, bash, powershell) for `eval`-based loading |
| `rep list` | `rep l` | Shows repos grouped by workspace, with unassigned repos in a trailing section |
| `rep list --tag work` | | Filters repos by tag (composes with `--workspace`) |
| `rep list --workspace backend` | | Scopes the listing to one workspace (suppresses the unassigned section) |
| `rep jump` | `rep j` | Fuzzy-select a repo, prints the path (used by `rj` shell function) |
| `rep jump my-api` | `rep j my-api` | Jump directly to a repo by alias (also matches `aliases` field) |
| `rep jump --title "Debug"` | | Override the terminal tab title for this session |
| `rep open` | `rep o` | Fuzzy-select a repo, opens it in your editor |
| `rep open my-api` | `rep o my-api` | Opens a repo directly by alias |
| `rep open --editor code` | | Override the default editor |
| `rep open --title "Debug"` | | Override the terminal tab title for this session |
| `rep color` | | Emit tab title + background color for current directory (for shell hooks) |
| `rep color --repo my-api` | | Emit tab title + background color for a specific repo |
| `rep status` | `rep s` | Show git status (branch, dirty, upstream, last commit) across all repos |
| `rep status --tag work` | | Filter status by tag |
| `rep sync` | | Pull latest changes across all repos (skips dirty repos) |
| `rep sync --tag work` | | Pull only repos with this tag |
| `rep edit` | `rep e` | Fuzzy-select a repo, then pick a field to edit from a menu (loops until Done) |
| `rep add ~/dev/foo` | `rep a ~/dev/foo` | Register a local repo (auto-detects git remote, suggests alias) |
| `rep add https://github.com/org/repo.git` | | Clone a repo and register it (asks where to place it) |
| `rep remove my-api` | `rep rm my-api` | Unregister a repo (does not delete files) |
| `rep ai` | | Fuzzy-select a repo and launch the default AI coding CLI in it |
| `rep web` | `rep w` | Fuzzy-select a repo and open its remote URL in the browser |
| `rep web my-api` | `rep w my-api` | Open a repo's remote directly by alias |
| `rep ai my-api` | | Launch default AI tool directly in a repo by alias |
| `rep ai --tool codex` | | Launch a specific AI tool (overrides default) |
| `rep ai my-api --tool codex` | | Specific repo + specific tool |
| `rep run` | `rep r` | Fuzzy-select a repo and a configured command, then run it |
| `rep run my-api` | `rep r my-api` | Skip repo selection, fuzzy-select a command to run |
| `rep run --cmd test` | | Skip command selection, fuzzy-select a repo to run "test" in |
| `rep run my-api --cmd test` | | Run "test" directly in my-api (no fuzzy menus) |
| `rep workspace list` | `rep ws ls` | List all registered VSCode/Cursor workspaces with their directory path |
| `rep workspace show <name>` | | Show a workspace's description, directory path, and resolved member links |
| `rep workspace create <name> --repos a,b,c` | | Register a workspace and materialize its directory (symlinks + `.code-workspace`) |
| `rep workspace delete <name>` | `rep ws rm` | Unregister a workspace (leaves the directory on disk) |
| `rep workspace delete <name> --purge` | | Unregister AND remove the workspace directory (member repos untouched) |
| `rep workspace rebuild <name>` | | Recreate the workspace directory's symlinks and `.code-workspace` file from config |
| `rep workspace add-repo <name> <alias>` | | Add a repo to a workspace and rebuild its directory |
| `rep workspace remove-repo <name> <alias>` | | Remove a repo from a workspace and rebuild its directory |
| `rep workspace open <name>` | | Open a workspace in your default editor |
| `rep doctor` | | Diagnose config, shell integration, and repo path issues |

## Shell integration

`rep init <shell>` prints shell code to stdout; you load it with one `eval` line in your rc file. Nothing is ever written to disk, nothing is regenerated on upgrade, and nothing prompts the user — the same idiomatic pattern used by `starship`, `zoxide`, `direnv`, and `mise`. It works identically in TTY shells, CI runners, sandboxes, and WSL.

| Shortcut | What it does |
|----------|-------------|
| `rj` | Fuzzy-select a repo and `cd` into it |
| `rj my-api` | Jump directly to a repo by alias; falls through to a workspace alias if no repo matches (cd's into the workspace directory, where member repos appear as subdirs via symlink/junction) |
| `ro` | Fuzzy-select a repo and open it in your editor |
| `ro my-api` | Open a repo directly by alias; falls through to a workspace alias if no repo matches (opens the `.code-workspace` file) |
| `rjw my-ws` | Workspace-only jump — resolves the alias strictly against `[workspaces.*]` and cd's into the workspace directory |
| `row my-ws` | Workspace-only open — launches the editor on the `.code-workspace` file inside the workspace directory |
| `rw` | Fuzzy-select a repo and open it in the browser |
| `rw my-api` | Open a repo's remote directly by alias |
| `rr` | Fuzzy-select a repo and run a configured command in it |
| `rr my-api` | Skip repo selection, fuzzy-select a command |

Supports Zsh, Bash, and PowerShell. No re-init needed on upgrade — the `eval` in your rc file fetches fresh code from the newly-installed binary on every shell startup.

## Config

Lives at `~/.reportal/config.toml`:

```toml
[settings]
default_editor = "cursor"
default_clone_root = "~/dev"
default_workspace_root = "~/dev/workspaces"  # where rep materializes workspace dirs
path_on_select = "show"           # "show" or "hide" — print path after jump/open
path_display_format = "absolute"  # "absolute" or "relative"

[repos.my-api]
path = "~/dev/my-project/api"
description = "Backend API"
tags = ["work", "backend"]
remote = "git@github.com:org/api.git"
title = "API"              # custom terminal tab title (defaults to alias)
color = "#1a1a2e"          # terminal background color on jump (hex)

[repos.website]
path = "~/dev/personal/site"
description = "Personal website"
tags = ["personal", "frontend"]
```

| Setting | Values | Default | What it controls |
|---------|--------|---------|-----------------|
| `default_editor` | Any command | `cursor` | Editor for `rep open` |
| `default_clone_root` | Any path | `~/dev` | Where `rep add <url>` clones to |
| `default_workspace_root` | Any path | `<default_clone_root>/workspaces` (falls back to `~/dev/workspaces`) | Parent directory for materialized workspace directories |
| `path_on_select` | `show`, `hide` | `show` | Print path after picking a repo in jump/open |
| `path_display_format` | `absolute`, `relative` | `absolute` | Full path or relative to current directory |
| `default_ai_tool` | Any tool name | `claude` | Which AI CLI `rep ai` launches by default |

### Per-repo fields

| Field | Required | Default | What it controls |
|-------|----------|---------|-----------------|
| `path` | yes | — | Filesystem path to the repo (supports `~`) |
| `description` | no | `""` | Shown in fuzzy finder and `rep list` |
| `tags` | no | `[]` | Filter repos with `--tag` |
| `remote` | no | `""` | Git remote URL for cloning on other machines |
| `aliases` | no | `[]` | Alternative names for direct jump (e.g. `rj ninja` instead of `rj nro`) |
| `title` | no | repo alias | Terminal tab title on jump/open |
| `color` | no | reset to default | Terminal tab color (`#RRGGBB`) on jump/open |

### AI tools

Configure which AI coding CLIs are available for `rep ai`:

```toml
[ai_tools.claude]
command = "claude"
args = []

[ai_tools.codex]
command = "codex"
args = []

[ai_tools.aider]
command = "aider"
args = []
```

| Field | Required | Default | What it controls |
|-------|----------|---------|-----------------|
| `command` | yes | — | The executable to run |
| `args` | no | `[]` | Extra arguments passed on every launch |

The default config created on first `rep` invocation ships with claude, codex, and aider pre-registered.

### Workspaces

A workspace is a named group of repos that open together as one VSCode/Cursor window, **and** a real directory on disk containing one symlink (Unix) or directory junction (Windows) per member. RePortal owns the `[workspaces.<name>]` table as the single source of truth; the on-disk directory (symlinks + `.code-workspace` file) is a derived artifact generated from the member repos' current paths.

```
~/dev/workspaces/backend/
    backend.code-workspace    # folders[] uses relative "./api", "./worker", ...
    api      → ~/dev/services/api           (symlink on Unix, junction on Windows)
    worker   → ~/dev/services/worker
    db-migrations → ~/dev/db-migrations
```

`rjw backend` cd's into `~/dev/workspaces/backend/`, so `ls`, `claude`, `cargo test`, your IDE — everything sees every member repo as a subdirectory. The symlinks are transparent: `git`, editors, and build tools all resolve them to the real repo.

```toml
[settings]
default_workspace_root = "~/dev/workspaces"   # where workspace directories live

[workspaces.backend]
repos = [
    "api",                                    # registered-repo reference
    "worker",
    { path = "~/dev/db-migrations" },         # inline filesystem path (no repo registry entry needed)
]
description = "Jakuta backend services"
path = ""                                     # empty = <default_workspace_root>/backend/
aliases = ["be", "back"]                      # short names accepted by rep workspace <sub>
```

| Field | Required | Default | What it controls |
|-------|----------|---------|-----------------|
| `repos` | yes | — | Ordered list of members. Each entry is either a **bare string** (alias of a registered repo — gets the path-change reverse index) or an **inline table** `{ path = "..." }` (raw filesystem path — bypasses the repo registry entirely). Order matches the editor sidebar and the on-disk link listing. Bare-string members must resolve to a registered repo; inline-path members are not validated against the repo registry |
| `description` | no | `""` | Human-readable description shown in `rep workspace list` |
| `path` | no | `""` | Filesystem path for the workspace directory (the one containing the symlinks and the `.code-workspace` file). Empty falls back to `<default_workspace_root>/<name>/`. Pre-v0.15.2 configs may still store a `.code-workspace` file path here — those are auto-migrated on first `rep workspace jump` / `rebuild` |
| `aliases` | no | `[]` | Short names that resolve to this workspace in `rep workspace` subcommands. Must not collide with any repo name/alias or another workspace's name/alias — enforced at config load |

**Platform-specific linking is transparent.** On Unix the member entries are symlinks (`ln -s`); on Windows they are directory junctions (via the `junction` crate, no elevation required). `cd`, `ls`, and every build tool follow both transparently — the user doesn't need to know which form is used.

**Regeneration is automatic for registered-repo members.** When you change a repo's path via `rep edit`, every workspace whose `repos` field references it as a bare-string alias rebuilds its directory — stale symlinks get redirected at the new target, and the `.code-workspace` file is refreshed. Inline-path members (`{ path = "..." }`) are NOT tracked by the reverse index — moving an inline-path folder requires editing the `path` value in `~/.reportal/config.toml` yourself. User-authored fields inside the `.code-workspace` file (`settings`, `extensions`, `launch`, `tasks`, plus any JSONC comments) round-trip byte-stable across regeneration — RePortal only touches the `folders` array.

**If the workspace directory is missing or stale**, run `rep workspace rebuild <name>` to regenerate the symlinks and `.code-workspace` file from config. The command is idempotent and does not move or modify the member repos themselves — only the links inside the workspace directory.

**`rep workspace delete <name>` only removes the config entry by default.** Pass `--purge` to also delete the on-disk workspace directory (symlinks + `.code-workspace` file). Member repos are never touched; a purge just removes the thin directory of links.

**Removing a repo that's still a workspace member is refused** with a message listing the blocking workspaces. Remove the repo from each workspace first (or delete those workspaces), then retry. This keeps destructive membership changes explicit.

### Commands

Define reusable commands that can be run in any repo via `rep run`:

```toml
[commands]
test  = { command = "cargo test",          description = "Run tests" }
serve = { command = "npm run dev",         description = "Start dev server" }
build = { command = "cargo build --release", description = "Production build" }
```

| Field | Required | Default | What it controls |
|-------|----------|---------|-----------------|
| `command` | yes | — | The shell command to execute |
| `description` | no | `""` | Shown in the fuzzy picker alongside the command name |

Per-repo commands go under `[repos.<alias>.commands]` and can override global commands with the same name, or add repo-specific ones:

```toml
[repos.my-api.commands]
serve   = { command = "python manage.py runserver", description = "Start Django dev server" }
migrate = { command = "python manage.py migrate",   description = "Run database migrations" }
```

## Terminal personalization

When you jump to or open a repo, RePortal automatically sets:

1. **Tab title** — uses the `title` config field, falling back to the repo alias
2. **Tab color** — uses the `color` config field; repos without a color reset to the terminal default

The `--title` flag on `jump`/`open` lets you override the tab title for a single session without changing config.

### Shell hook for new terminals

Terminals opened directly into a repo (e.g. VS Code integrated terminal) won't go through `rj`, so they won't get the color/title automatically. Add `rep color` to your prompt to fix that:

**PowerShell:**
```powershell
function prompt { rep color 2>$null; "PS> " }
```

**Bash / Zsh:**
```bash
PROMPT_COMMAND='rep color 2>/dev/null'
```

`rep color` matches your current directory against registered repos (longest prefix wins) and emits the right sequences.

## Roadmap

- [x] Config parsing
- [x] `init`, `list`, `jump`, `open`, `add`, `remove`
- [x] Shell integration auto-install (`rj`, `ro`)
- [x] Clone from URL with sibling/child placement
- [x] Auto-detect git remote on `add`
- [x] Colored output with themed fuzzy finder (repo color swatches in `rep list` and fuzzy finder)
- [x] `rep` short alias
- [x] Configurable path display (absolute/relative, show/hide)
- [x] Per-repo terminal tab title and background color (OSC 2 / OSC 11)
- [x] `color` command for shell prompt hooks
- [x] `status` — git status across all repos
- [x] `sync` — pull latest across repos
- [x] `doctor` — diagnose config, shell integration, and repo path issues
- [x] `ai` — launch AI coding CLIs (Claude Code, Codex, aider) in any repo with configurable defaults
- [x] `web` — open a repo's remote URL in the browser (converts SSH remotes to HTTPS)
- [x] `run` — run configured commands in repos with fuzzy selection and per-repo overrides
- [x] `edit` UX overhaul — field menu (pick Path/Description/Tags/Title/Color individually, loop back)
- [x] VSCode/Cursor `.code-workspace` integration — `rep workspace` subcommands, owned config, auto-regenerate on repo path changes, JSONC round-trip preservation
- [ ] `config` — manage AI tools and global settings (`rep config ai-tools`, `rep config settings`)
- [x] Workspace-tree grouping in `rep list` (workspaces as the tree root, `--workspace` as a first-class filter composable with `--tag`)
- [x] Workspace aliases — short names that resolve to a workspace in `rep workspace` subcommands, with cross-namespace collision validation at config load
- [x] Inline-path workspace members — mix registered-repo references with raw filesystem paths in the same workspace so folders can belong to a workspace without being registered as top-level repos
- [x] Unified `rj` / `ro` — fall through to workspace aliases when no repo matches, so `rj venoble` cd's to the workspace's common-ancestor folder and `ro venoble` opens the workspace in the editor; `rjw` / `row` added as workspace-only variants
- [x] Unified fuzzy finder in `rj` / `ro` — workspaces render alongside repos with a `[workspace]` suffix, and `rjw` / `row` with no args fuzzy-select workspaces only
- [x] Idiomatic `rep init <shell>` — prints shell code to stdout for `eval`-based loading (starship/zoxide/direnv pattern), with lazy config bootstrap on first command
- [x] Workspaces as real directories — each workspace is materialized under `~/dev/workspaces/<name>/` with one symlink (Unix) or junction (Windows) per member repo plus the `.code-workspace` file inside; `rjw` cd's there so terminal and editor both see members as subdirs, and `rep workspace rebuild` / `delete --purge` manage the on-disk layout
- [ ] Interactive ratatui TUI absorbing `list` / `dash` with live git-status column
- [ ] `dashboard` — rich overview with branches, dirty state, last commit
- [ ] `clone --all` — clone missing repos from config (machine sync)
- [ ] Shell completions
- [x] Publish to crates.io

## Contributing

PRs welcome. Open an issue first for anything bigger than a typo fix.

## License

[MIT](LICENSE)
