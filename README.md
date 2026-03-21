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

### From source

```bash
git clone https://github.com/CheckPickerUpper/reportal.git
cd reportal
cargo install --path .
```

## Quick start

```bash
# Set up config + shell shortcuts (rj, ro)
rep init

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
| `rep init` | | Creates config and installs shell integration (safe to re-run on updates) |
| `rep list` | `rep l` | Shows all repos with path, description, tags, and whether it exists on disk |
| `rep list --tag work` | | Filters repos by tag |
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
| `rep edit my-api` | `rep e my-api` | Interactively edit a repo's description, tags, title, and color |
| `rep add ~/dev/foo` | `rep a ~/dev/foo` | Register a local repo (auto-detects git remote, suggests alias) |
| `rep add https://github.com/org/repo.git` | | Clone a repo and register it (asks where to place it) |
| `rep remove my-api` | `rep rm my-api` | Unregister a repo (does not delete files) |
| `rep doctor` | | Diagnose config, shell integration, and repo path issues |

## Shell integration

`rep init` writes a standalone integration script to `~/.reportal/integration.ps1` (or `.sh`) and adds a single source line to your shell profile. The profile line never changes between versions; updating the binary is all you need.

| Shortcut | What it does |
|----------|-------------|
| `rj` | Fuzzy-select a repo and `cd` into it |
| `rj my-api` | Jump directly to a repo by alias |
| `ro` | Fuzzy-select a repo and open it in your editor |
| `ro my-api` | Open a repo directly by alias |

Supports PowerShell, Bash, Zsh. Detected and installed during `rep init`. Re-run `rep init` after major version updates to regenerate the integration file.

## Config

Lives at `~/.reportal/config.toml`:

```toml
[settings]
default_editor = "cursor"
default_clone_root = "~/dev"
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
| `path_on_select` | `show`, `hide` | `show` | Print path after picking a repo in jump/open |
| `path_display_format` | `absolute`, `relative` | `absolute` | Full path or relative to current directory |

### Per-repo fields

| Field | Required | Default | What it controls |
|-------|----------|---------|-----------------|
| `path` | yes | — | Filesystem path to the repo (supports `~`) |
| `description` | no | `""` | Shown in fuzzy finder and `rep list` |
| `tags` | no | `[]` | Filter repos with `--tag` |
| `remote` | no | `""` | Git remote URL for cloning on other machines |
| `aliases` | no | `[]` | Alternative names for direct jump (e.g. `rj ninja` instead of `rj nro`) |
| `title` | no | repo alias | Terminal tab title set via OSC 2 on jump/open |
| `color` | no | reset to default | Terminal background color (`#RRGGBB`) set via OSC 11 on jump/open |

## Terminal personalization

When you jump to or open a repo, RePortal automatically sets:

1. **Tab title** (OSC 2) — uses the `title` config field, falling back to the repo alias
2. **Background color** (OSC 11) — uses the `color` config field; repos without a color reset the terminal to its default

Both sequences go to stderr so they don't interfere with the path output that `rj` captures. Terminals that don't support these sequences silently ignore them.

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
- [x] Colored output with themed fuzzy finder
- [x] `rep` short alias
- [x] Configurable path display (absolute/relative, show/hide)
- [x] Per-repo terminal tab title and background color (OSC 2 / OSC 11)
- [x] `color` command for shell prompt hooks
- [x] `status` — git status across all repos
- [x] `sync` — pull latest across repos
- [x] `doctor` — diagnose config, shell integration, and repo path issues
- [ ] `dashboard` — rich overview with branches, dirty state, last commit
- [ ] `clone --all` — clone missing repos from config (machine sync)
- [ ] Shell completions
- [ ] Publish to crates.io

## Contributing

PRs welcome. Open an issue first for anything bigger than a typo fix.

## License

[MIT](LICENSE)
