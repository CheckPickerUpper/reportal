<div align="center">

<!-- Logo goes here -->
<!-- <img src="assets/logo.png" width="200" /> -->

# RePortal

**Jump between repos. Open in your editor. Stay in sync across machines.**

[![License: MIT](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)
[![Rust](https://img.shields.io/badge/rust-1.78%2B-orange.svg)](https://www.rust-lang.org/)
[![Windows](https://img.shields.io/badge/platform-windows%20%7C%20macos%20%7C%20linux-lightgrey.svg)]()
[![PRs Welcome](https://img.shields.io/badge/PRs-welcome-brightgreen.svg)]()

</div>

---

RePortal is a single-binary CLI that keeps a registry of your dev repos and lets you fuzzy-jump between them, open them in your editor, and clone missing repos on a new machine.

## Install

```bash
cargo install --path .
```

Or build from source:

```bash
git clone https://github.com/CheckPickerUpper/reportal.git
cd reportal
cargo build --release
```

The binary lands at `target/release/reportal`.

## Quick start

```bash
# Create the config file at ~/.reportal/config.toml
reportal init

# Register a repo
reportal add ~/dev/my-project

# List all registered repos
reportal list

# Fuzzy-select a repo and open it in Cursor
reportal open

# Fuzzy-select a repo and print the path (for cd)
reportal jump
```

## Commands

| Command | What it does |
|---------|-------------|
| `reportal init` | Creates `~/.reportal/config.toml` with defaults |
| `reportal list` | Shows all repos with path, description, tags, and whether the directory exists |
| `reportal list --tag work` | Filters repos by tag |
| `reportal jump` | Fuzzy-select a repo, prints the path to stdout |
| `reportal open` | Fuzzy-select a repo, opens it in your editor |
| `reportal open my-api` | Opens a repo directly by alias |
| `reportal open --editor code` | Override the default editor |
| `reportal add ~/dev/foo` | Registers a repo interactively (prompts for alias, description, tags) |
| `reportal remove my-api` | Unregisters a repo (does not delete files) |

## Shell integration

`reportal jump` prints a path to stdout. Wrap it in a shell function so `cd` happens in your current session:

**PowerShell:**
```powershell
function rj { Set-Location (reportal jump) }
```

**Bash / Zsh:**
```bash
rj() { cd "$(reportal jump)"; }
```

Add this to your shell profile and use `rj` to jump between repos.

## Config

Lives at `~/.reportal/config.toml`:

```toml
[settings]
default_editor = "cursor"
default_clone_root = "~/dev"

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

## Roadmap

- [x] Config parsing
- [x] `init`, `list`, `jump`, `open`, `add`, `remove`
- [ ] `status` — git status across all repos
- [ ] `sync` — pull latest across repos
- [ ] `dashboard` — rich overview with branches, dirty state, last commit
- [ ] `clone --all` — clone missing repos from config (machine sync)
- [ ] Shell completions (PowerShell, Bash, Zsh, Fish)

## Contributing

PRs welcome. Open an issue first for anything bigger than a typo fix.

## License

[MIT](LICENSE)
