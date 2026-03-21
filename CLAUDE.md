# RePortal

A cross-platform CLI for jumping between, opening, and managing dev repos; written in Rust.

## Build & Install

Published on **crates.io**. To install or update to the latest published version:

```
cargo install reportal --force
```

For local development builds:

```
cargo build
cargo install --path .
```

Produces two binaries: `reportal` and `rep` (short alias).

## Architecture

- `src/main.rs` — clap CLI entry point; dispatches to subcommands
- `src/error.rs` — `ReportalError` enum via thiserror
- `src/reportal_config/` — config module (split from monolith):
  - `mod.rs` — re-exports only
  - `reportal_config_root.rs` — `ReportalConfig` struct, load/save, queries, mutations
  - `repo_entry.rs` — `RepoEntry`, `RepoRegistrationBuilder`, `TabTitle`, `RepoColor`
  - `ai_tool_entry.rs` — `AiToolEntry` for AI CLI tool registry
  - `command_entry.rs` — `CommandEntry` for user-defined command registry
  - `global_settings.rs` — `ReportalSettings`, `PathVisibility`, `PathDisplayFormat`
  - `hex_color.rs` — `HexColor` validation, RGB extraction, OSC sequences
  - `tag_filter.rs` — `TagFilter` enum
- `src/terminal_style.rs` — centralized color palette (owo-colors)
- `src/reportal_commands/` — one file per subcommand:
  - `initialization.rs` — `rep init` (config + shell integration; writes sourced script file)
  - `repo_listing.rs` — `rep list`
  - `repo_jump.rs` — `rep jump`
  - `repo_open.rs` — `rep open`
  - `repo_selection.rs` — shared repo fuzzy-selection with color swatches
  - `terminal_identity_emit.rs` — OSC tab title and color emission after repo selection
  - `git_commands.rs` — shared git command execution (spawn + capture stdout)
  - `prompts/` — shared interactive prompt helpers:
    - `text_prompt.rs` — `TextPromptParams`, `prompt_for_text`, `parse_comma_separated_tags`
    - `color_prompt.rs` — `ColorPromptResult`, `prompt_for_color`
    - `color_edit_prompt.rs` — `ColorEditPromptParams`, `prompt_for_color_edit`
    - `color_edit_result.rs` — `ColorEditResult`
  - `path_display.rs` — shared path visibility output after repo selection
  - `repo_ai.rs` — `rep ai` (launch AI coding CLIs in repos)
  - `repo_run.rs` — `rep run` (run user-defined commands in repos)
  - `repo_web.rs` — `rep web` (open repo remote URL in browser)
  - `repo_add/` — `rep add` (local paths + git URL cloning):
    - `add_source.rs` — `AddSource` classification
    - `alias_suggestion.rs` — alias inference from paths and URLs
    - `clone_destination.rs` — `CloneDestination` placement prompts
    - `clone_placement.rs` — registered directory collection for placement
    - `git_clone_operation.rs` — `GitCloneOperation` execution
    - `git_remote_detection.rs` — `GitRemoteDetection` via `git remote`
    - `registration_context.rs` — `RegistrationContext` metadata collection
    - `run.rs` — `run_add` entry point
  - `repo_remove.rs` — `rep remove`
  - `repo_color.rs` — `rep color` (OSC terminal personalization for shell hooks)
  - `repo_edit.rs` — `rep edit` (field menu for editing individual repo metadata)
  - `repo_status.rs` — `rep status` (git status across all repos)
  - `repo_sync.rs` — `rep sync` (pull latest across repos)
  - `doctor.rs` — `rep doctor` (diagnose config, shell integration, repo paths)

Config lives at `~/.reportal/config.toml`.

## Plugins

Install the ai-lab marketplace then enable perfect-rustacean:

```
/plugin marketplace add C:\Users\ozzyi\dev\personal-projects\ai-lab
/plugin install perfect-rustacean@ai-lab-local
/plugin install style-enforcement@ai-lab-local
```

## Code Style

- Explicit `return` statements preferred
- No `bool` for domain state; use two-variant enums instead
- No `Option` params; use enums that name both states
- No `.unwrap()` or `.expect()`; propagate with `?` or `match`
- No positional args with >1 param; use a struct
- Private fields + accessors on all structs (smart constructors)
- `match` everywhere; no `if let` on custom enums
- `#[cfg(target_os = "windows")]` for platform-specific code, not runtime checks
- Unused code on the wrong platform gets `#[cfg(not(...))]`, not `#[allow(dead_code)]`

## Release Checklist

When implementing a feature, ALWAYS:
1. Bump version in `Cargo.toml`
2. Update `README.md` (commands table, config examples, roadmap)
3. Update `CLAUDE.md` architecture section if new files were added
4. Run `cargo install --path .` so the user can use the updated CLI immediately
5. Commit all changes
6. Create a git tag: `git tag vX.Y.Z`
7. Push commit and tag: `git push origin master --tags`

Do NOT wait to be asked; this is part of shipping the feature.

## Publishing (All Platforms)

Pushing a version tag triggers the full release pipeline via cargo-dist CI.

| Platform | Method | Automation |
|----------|--------|------------|
| GitHub Releases | cargo-dist builds binaries for Windows/macOS/Linux | Fully automatic on tag push |
| Homebrew (macOS/Linux) | cargo-dist pushes formula to `CheckPickerUpper/homebrew-tap` | Fully automatic on tag push |
| Scoop (Windows) | `checkver`/`autoupdate` in `CheckPickerUpper/scoop-reportal` bucket | Auto-updates via Scoop bots |
| Winget (Windows) | PR to `microsoft/winget-pkgs` via `wingetcreate update` | Manual; or CI job with `WINGET_PAT` |
| Crates.io | `cargo publish` | Manual (run after tag push) |

### What cargo-dist does on tag push

1. Cross-compiles for `x86_64-pc-windows-msvc`, `x86_64-apple-darwin`, `aarch64-apple-darwin`, `x86_64-unknown-linux-gnu`, `aarch64-unknown-linux-gnu`
2. Creates GitHub Release with `.zip` (Windows) and `.tar.gz` (Unix) archives
3. Generates shell and PowerShell installer scripts
4. Pushes updated Homebrew formula to the tap repo

### Config files

- `dist-workspace.toml` — cargo-dist config (targets, installers, homebrew tap)
- `.github/workflows/release.yml` — auto-generated by cargo-dist; do not edit manually
- `_docs/SPEC_DISTRIBUTION.md` — full distribution strategy and manifest examples

## Shell Integration Design

Shell integration uses a sourced-file pattern with two distinct mechanisms:

- **`rep init`** — one-time interactive setup. Creates config, detects the user's shell,
  adds a source line to their profile. Only needed once per machine.
- **`ensure_integration_file_current()`** — runs automatically before every `rep` command.
  Compares the version stamp in the integration file against the running binary. If they
  differ, it silently rewrites the file. This means new shell functions (like `rr`) are
  deployed automatically on binary update — no manual `rep init` re-run needed.

The integration file (`~/.reportal/integration.ps1` or `integration.sh`) is the only
thing that changes between versions. The source line in the shell profile is stable.

The prompt hook runs `rep color` AFTER the user's existing prompt (e.g. oh-my-posh)
so RePortal's tab title always wins over whatever the prompt tool sets. The PowerShell
hook captures the original prompt output, then emits OSC sequences, then returns the
prompt string. This ordering is critical — reversing it lets oh-my-posh overwrite
the tab title.

## Internal Docs

`_docs/` is gitignored and contains:
- `SPEC.md` — full feature spec and roadmap
- `DEV_TOOLS.md` — developer setup guide
- `UX_IMPROVEMENTS.md` — UX polish checklist
