'use strict';

// Translate a Pi extension `tool_call` event into the Claude-shaped hook payload
// the canonical `hooks/*.js` scripts already expect. This is the Pi peer of
// `map-codex-payload-to-claude.js` / `map-opencode-payload-to-claude.js`.
//
// Pi's write surface is small and concrete: the only built-in tools that mutate
// files are `write` ({ path, content }) and `edit` ({ path, edits: [{ oldText,
// newText }] }). Pi has no `apply_patch` and no MCP write tools, so this mapper
// has no patch parser and no MCP path-field table — it is a direct field rename
// from the two Pi tool shapes to the two matching Claude tool shapes:
//
//   Pi `write` { path, content }                  -> Claude `Write`  { file_path, content }
//   Pi `edit`  { path, edits:[{oldText,newText}] } -> Claude `MultiEdit` { file_path, edits:[{old_string,new_string}] }
//
// `MultiEdit` is the honest Claude analogue of Pi's `edit`: Pi's `edit` always
// carries an `edits` array, exactly like Claude's `MultiEdit`. The canonical
// hooks' `parseHookInput` normalizes `MultiEdit` to the same per-file scan as
// `Edit`, so picking `MultiEdit` loses nothing and avoids fabricating the
// single-edit `Edit` shape from a multi-edit source.

// These shared libs live at runtime/shared/lib in the repo, so this file and its
// tests resolve them directly. The Pi compiler flattens these `../../shared/lib/`
// specifiers to `./` when it vendors the require() closure into one flat
// ai-lab-runtime/ dir; the vendored set is derived from exactly these edges, so an
// added require can never ship without its file. Do not "simplify" the path — the
// repo layout needs it, and the compiler rewrites it for deployment.
const { hasCheckedExtension } = require('./checked-extensions.js');
const { canonicalFilePath } = require('./canonical-hook-path.js');
const { matcherAcceptsClaudeToolName } = require('./matcher-semantics.js');

function plainObject(candidate) {
  return (typeof candidate === 'object' && candidate !== null && !Array.isArray(candidate)) ? candidate : {};
}

// Pi `edit` carries `edits: [{ oldText, newText }]`. Claude `MultiEdit` carries
// `edits: [{ old_string, new_string }]`. Rename each pair; drop malformed rows.
function toClaudeEditList(piEdits) {
  if (!Array.isArray(piEdits)) return [];
  const claudeEdits = [];
  for (const piEdit of piEdits) {
    const piEditObject = plainObject(piEdit);
    claudeEdits.push({
      old_string: String(piEditObject.oldText || ''),
      new_string: String(piEditObject.newText || ''),
    });
  }
  return claudeEdits;
}

// Build the Claude-shaped PreToolUse payloads for one Pi `tool_call` event.
// Returns [] for non-write tool calls and for write paths whose extension is
// not in the checked set, so the canonical hook is never run pointlessly.
function payloadsForPiToolCall({ piToolName, piToolInput, projectRoot, sessionId }) {
  const toolInputObject = plainObject(piToolInput);
  const resolvedProjectRoot = String(projectRoot || process.cwd());

  function buildPayload(toolName, toolInput) {
    const p = {
      hook_event_name: 'PreToolUse',
      tool_name: toolName,
      tool_input: toolInput,
      cwd: resolvedProjectRoot,
    };
    if (sessionId) {
      p.session_id = String(sessionId);
    }
    return p;
  }

  if (piToolName === 'write') {
    const filePathText = String(toolInputObject.path || '');
    if (!hasCheckedExtension(filePathText)) return [];
    return [
      buildPayload('Write', {
        file_path: canonicalFilePath(resolvedProjectRoot, filePathText),
        content: String(toolInputObject.content || ''),
      }),
    ];
  }

  if (piToolName === 'edit') {
    const filePathText = String(toolInputObject.path || '');
    if (!hasCheckedExtension(filePathText)) return [];
    return [
      buildPayload('MultiEdit', {
        file_path: canonicalFilePath(resolvedProjectRoot, filePathText),
        edits: toClaudeEditList(toolInputObject.edits),
      }),
    ];
  }

  return [];
}

module.exports = {
  payloadsForPiToolCall,
  matcherAcceptsClaudeToolName,
};
