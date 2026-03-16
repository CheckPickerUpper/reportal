# RePortal

A fast, cross-platform CLI tool for navigating and managing development repositories. Written in Rust.

## Plugins

Install the local marketplace and perfect-rustacean plugin:

```
/plugin marketplace add C:\Users\ozzyi\dev\personal-projects\ai-lab\claude-plugins
/plugin install perfect-rustacean@ai-lab-local
```

## Build & Run

```
cargo build
cargo run -- <command>
```

## Architecture

- `src/main.rs` — CLI entry point, clap command definitions
- `src/config.rs` — TOML config parsing, repo structs
- `src/commands/` — one file per subcommand (jump, open, list, etc.)
- Config lives at `~/.reportal/config.toml`

## Spec

See `SPEC.md` for full feature spec and roadmap.
