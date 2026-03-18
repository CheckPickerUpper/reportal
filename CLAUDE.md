# RePortal

A cross-platform CLI for jumping between, opening, and managing dev repos; written in Rust.

## Build & Install

```
cargo build
cargo install --path .
```

Produces two binaries: `reportal` and `rep` (short alias).

## Architecture

- `src/main.rs` — clap CLI entry point; dispatches to subcommands
- `src/error.rs` — `ReportalError` enum via thiserror
- `src/reportal_config.rs` — TOML config parsing, repo registry, builder, enums
- `src/terminal_style.rs` — centralized color palette (owo-colors)
- `src/reportal_commands/` — one file per subcommand:
  - `initialization.rs` — `rep init` (config + shell integration install)
  - `repo_listing.rs` — `rep list`
  - `repo_jump.rs` — `rep jump`
  - `repo_open.rs` — `rep open`
  - `repo_add.rs` — `rep add` (local paths + git URL cloning)
  - `repo_remove.rs` — `rep remove`
  - `repo_color.rs` — `rep color` (OSC terminal personalization for shell hooks)

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

Do NOT wait to be asked — this is part of shipping the feature.

## Internal Docs

`_docs/` is gitignored and contains:
- `SPEC.md` — full feature spec and roadmap
- `DEV_TOOLS.md` — developer setup guide
- `UX_IMPROVEMENTS.md` — UX polish checklist
