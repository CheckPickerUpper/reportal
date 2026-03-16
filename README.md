<div align="center">

<!-- Logo goes here -->
<!-- <img src="assets/logo.png" width="200" /> -->

# RePortal

**Jump between repos. Open in your editor. Stay in sync across machines.**

[![Crates.io](https://img.shields.io/crates/v/reportal.svg)](https://crates.io/crates/reportal)
[![License: MIT](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)
[![Rust](https://img.shields.io/badge/rust-1.78%2B-orange.svg)](https://www.rust-lang.org/)
[![Windows](https://img.shields.io/badge/platform-windows%20%7C%20macos%20%7C%20linux-lightgrey.svg)]()
[![PRs Welcome](https://img.shields.io/badge/PRs-welcome-brightgreen.svg)]()

</div>

---

RePortal is a single-binary CLI that keeps a registry of your dev repos and lets you fuzzy-jump between them, open them in your editor, and clone missing repos on a new machine.

## Install

```bash
cargo install reportal
```

This gives you both `reportal` and `rep` (short alias) commands.

From source:

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

| Command | What it does |
|---------|-------------|
| `rep init` | Creates config and installs `rj`/`ro` shell shortcuts |
| `rep list` | Shows all repos with path, description, tags, and whether it exists on disk |
| `rep list --tag work` | Filters repos by tag |
| `rep jump` | Fuzzy-select a repo, prints the path (used by `rj` shell function) |
| `rep open` | Fuzzy-select a repo, opens it in your editor |
| `rep open my-api` | Opens a repo directly by alias |
| `rep open --editor code` | Override the default editor |
| `rep add ~/dev/foo` | Register a local repo (auto-detects git remote, suggests alias) |
| `rep add https://github.com/org/repo.git` | Clone a repo and register it (asks where to place it) |
| `rep remove my-api` | Unregister a repo (does not delete files) |

## Shell integration

`rep init` automatically installs these shortcuts into your shell profile:

| Shortcut | What it does |
|----------|-------------|
| `rj` | Fuzzy-select a repo and `cd` into it |
| `ro` | Fuzzy-select a repo and open it in your editor |

Supports PowerShell, Bash, Zsh. Detected and installed during `rep init`.

You can also set them up manually:

**PowerShell:**
```powershell
function rj { Set-Location (rep jump) }
function ro { rep open }
```

**Bash / Zsh:**
```bash
rj() { cd "$(rep jump)"; }
ro() { rep open; }
```

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

## Roadmap

- [x] Config parsing
- [x] `init`, `list`, `jump`, `open`, `add`, `remove`
- [x] Shell integration auto-install (`rj`, `ro`)
- [x] Clone from URL with sibling/child placement
- [x] Auto-detect git remote on `add`
- [x] Colored output with themed fuzzy finder
- [x] `rep` short alias
- [x] Configurable path display (absolute/relative, show/hide)
- [ ] `status` — git status across all repos
- [ ] `sync` — pull latest across repos
- [ ] `dashboard` — rich overview with branches, dirty state, last commit
- [ ] `clone --all` — clone missing repos from config (machine sync)
- [ ] Shell completions
- [ ] Publish to crates.io

## Contributing

PRs welcome. Open an issue first for anything bigger than a typo fix.

## License

[MIT](LICENSE)
