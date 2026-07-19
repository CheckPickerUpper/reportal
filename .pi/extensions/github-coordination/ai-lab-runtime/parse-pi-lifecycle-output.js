'use strict';

// Parse one canonical hook's stdout/stderr for the Pi lifecycle events
// (`session_start`, `input`, `turn_end`, `tool_result`, `session_before_compact`).
//
// `parse-hook-decision.js` answers only the binary allow/deny question the
// `tool_call` path needs. The lifecycle path needs more: a canonical hook for
// `SessionStart` or `UserPromptSubmit` typically emits no block at all, just an
// `additionalContext` / `systemMessage` string that has to reach the model.
// This module extracts every channel a canonical hook can speak on so the
// generated Pi extension can route each to the matching Pi mechanism:
//
//   blockReason      -> Pi `{ block: true }` / `{ cancel: true }` where the
//                       event supports it; otherwise surfaced via ctx.ui.notify
//   contextText      -> injected into the model via pi.sendMessage(...)
//   systemMessage    -> surfaced to the user via ctx.ui.notify
//
// The parser itself is shared with Codex; this Pi wrapper narrows the output to
// the three channels Pi's event handlers need.

// Repo-true path; the Pi compiler flattens `../../shared/lib/` to `./` when it
// vendors this file's require() closure into the flat ai-lab-runtime/ dir.
const { parseCanonicalLifecycleOutput } = require('./parse-canonical-lifecycle-output.js');

// Reduce a single canonical hook process result to the three Pi-relevant
// channels. `defaultReason` is used when the hook signals a block but gives no
// human-readable reason of its own.
function parsePiLifecycleOutput({ stdout, stderr, exitCode, defaultReason }) {
  const parsed = parseCanonicalLifecycleOutput({ stdout, stderr, exitCode, defaultReason });
  return {
    blockReasons: parsed.blockReasons,
    contextMessages: parsed.contextMessages,
    systemMessages: parsed.systemMessages,
  };
}

module.exports = { parsePiLifecycleOutput };
