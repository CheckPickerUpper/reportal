'use strict';

// Parse one canonical Claude hook child-process result into every non-binary
// channel generated runtimes need for lifecycle events. Codex and Pi route
// those channels differently, but the canonical stdout/stderr truth table is
// identical and belongs in one source.

function nonEmptyString(candidateText) {
  return typeof candidateText === 'string' && candidateText.trim().length > 0;
}

function appendText(textSegments, candidateText) {
  if (nonEmptyString(candidateText)) textSegments.push(candidateText.trim());
}

function parseStdoutJson(rawStdout) {
  const trimmed = String(rawStdout || '').trim();
  if (!trimmed) return { kind: 'empty' };
  try {
    const parsed = JSON.parse(trimmed);
    if (typeof parsed === 'object' && parsed !== null) return { kind: 'json', parsedPayload: parsed };
  } catch (jsonParseFailure) {
    return { kind: 'text', textPayload: trimmed };
  }
  return { kind: 'text', textPayload: trimmed };
}

function parseCanonicalLifecycleOutput({ stdout, stderr, exitCode, defaultReason }) {
  const outcome = {
    blockReasons: [],
    contextMessages: [],
    stopReasons: [],
    systemMessages: [],
    continueFalse: false,
  };

  if (exitCode === 2) {
    appendText(outcome.blockReasons, stderr || defaultReason);
    return outcome;
  }

  const parsed = parseStdoutJson(stdout);
  if (parsed.kind === 'text') {
    appendText(outcome.contextMessages, parsed.textPayload);
    return outcome;
  }
  if (parsed.kind !== 'json') {
    return outcome;
  }

  const canonicalHookJson = parsed.parsedPayload;
  if (canonicalHookJson.decision === 'block') {
    appendText(outcome.blockReasons, canonicalHookJson.reason || defaultReason);
  }
  if (canonicalHookJson.continue === false) {
    outcome.continueFalse = true;
    appendText(outcome.stopReasons, canonicalHookJson.stopReason || canonicalHookJson.reason || defaultReason);
    appendText(outcome.blockReasons, canonicalHookJson.stopReason || canonicalHookJson.reason || defaultReason);
  }
  appendText(outcome.systemMessages, canonicalHookJson.systemMessage);
  appendText(outcome.contextMessages, canonicalHookJson.additionalContext);

  const hookSpecificOutput = canonicalHookJson.hookSpecificOutput;
  if (typeof hookSpecificOutput === 'object' && hookSpecificOutput !== null) {
    appendText(outcome.contextMessages, hookSpecificOutput.additionalContext);
    if (hookSpecificOutput.permissionDecision === 'deny') {
      appendText(outcome.blockReasons, hookSpecificOutput.permissionDecisionReason || defaultReason);
    }
  }

  return outcome;
}

module.exports = {
  appendText,
  parseCanonicalLifecycleOutput,
  parseStdoutJson,
};
