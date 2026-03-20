# Spec: Dynamic Terminal Tab Color on Repo Jump

## Summary

When `rj <repo>` (or `reportal jump <repo>`) changes directory to a repo, reportal should also emit a Windows Terminal escape sequence to set the **tab color** to that repo's `color` field from the config.

## Motivation

When working across multiple repos in separate terminal tabs (often inside Claude Code, where oh-my-posh is not visible), there's no way to visually distinguish which tab is which. Terminal background colors don't scale — they clash with text. Tab colors are a small colored strip that never affects readability.

## Behavior

### On Jump

When a repo has a `color` field in `~/.reportal/config.toml`:

1. Parse the hex color (e.g., `#3D0000`) into RGB components
2. Emit the Windows Terminal OSC escape sequence to set the tab color:
   ```
   \x1b]6;1;bg;red;{r}\x07
   \x1b]6;1;bg;green;{g}\x07
   \x1b]6;1;bg;blue;{b}\x07
   ```
   Where `{r}`, `{g}`, `{b}` are the decimal RGB values (0-255).
3. Then proceed with the normal directory change.

### On Jump to Repo Without Color

If the target repo has no `color` field, **reset the tab color** to the terminal default:
```
\x1b]6;1;bg;default\x07
```

### On `reportal list`

No change to list behavior. Colors are already shown there (or could be — that's separate scope).

## Config

No new config fields. Uses the existing `color` field:

```toml
[repos.nro]
path = "~/dev/oja-gamez/ninja-revival-online"
color = "#3D0000"
```

## Edge Cases

- **Non-Windows-Terminal**: The escape sequences are Windows Terminal specific. On other terminals they'll be silently ignored (no-op). No feature detection needed — just emit them.
- **Invalid hex color**: If `color` is present but not a valid `#RRGGBB` hex string, skip emitting (don't crash).
- **SSH / WSL**: The escape sequences pass through to the host terminal. This should just work.

## Existing Repo Colors

```
nro        = #3D0000  (dark red)
ai-lab     = #003D00  (dark green)
venoble    = #E6B422  (golden yellow)
jakuta-admin = #9B59B6  (medium purple)
```

Repos without colors: reportal, otk, ozzy-website, ourhomii, claude-conspire, open-claw-dev.
