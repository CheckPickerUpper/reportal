'use strict';

// Translates a Claude hook's child-process result (stdout JSON + exit code +
// stderr text) into an explicit allow|deny decision. The Claude protocol has
// two normal ways to deny: (a) print a JSON object with
// hookSpecificOutput.permissionDecision === "deny" or legacy decision ===
// "block"; (b) exit with code 2, in which case stderr carries the
// human-readable reason. Tool-call adapters can also opt into fail-closed
// handling for non-zero crash exits so a broken guard cannot silently allow a
// write.

const DEFAULT_DENY_REASON = 'Blocked by ai-lab plugin.';

function stringOrFallback(preferred, fallback) {
  return typeof preferred === 'string' && preferred.trim().length > 0
    ? preferred.trim()
    : fallback;
}

function denyDecisionFromHookSpecific(hookSpecificOutput, fallback) {
  if (typeof hookSpecificOutput !== 'object' || hookSpecificOutput === null) return null;
  if (hookSpecificOutput.permissionDecision !== 'deny') return null;
  return {
    kind: 'deny',
    reason: stringOrFallback(hookSpecificOutput.permissionDecisionReason, fallback),
  };
}

function denyDecisionFromLegacyShape(parsedPayload, fallback) {
  if (parsedPayload.decision !== 'block') return null;
  return {
    kind: 'deny',
    reason: stringOrFallback(parsedPayload.reason, fallback),
  };
}

function decisionFromHookStdout(stdout, fallback) {
  const trimmed = String(stdout || '').trim();
  if (!trimmed) return { kind: 'allow' };
  let parsed;
  try {
    parsed = JSON.parse(trimmed);
  } catch (jsonParseFailure) {
    return { kind: 'allow' };
  }
  if (typeof parsed !== 'object' || parsed === null) { return { kind: 'allow' }; }
  return (
    denyDecisionFromHookSpecific(parsed.hookSpecificOutput, fallback)
    || denyDecisionFromLegacyShape(parsed, fallback)
    || { kind: 'allow' }
  );
}

function decisionFromExitCode(exitCode, stderr, fallback, failClosedOnNonDenyExit = false) {
  if (exitCode === 2) return { kind: 'deny', reason: stringOrFallback(stderr, fallback) };
  if (failClosedOnNonDenyExit && exitCode !== 0) {
    return { kind: 'deny', reason: stringOrFallback(stderr, fallback) };
  }
  return { kind: 'allow' };
}

function combineHookResult({
  stdout,
  stderr,
  exitCode,
  defaultReason,
  failClosedOnNonDenyExit = false,
}) {
  const fallback = stringOrFallback(defaultReason, DEFAULT_DENY_REASON);
  const stdoutDecision = decisionFromHookStdout(stdout, fallback);
  if (stdoutDecision.kind === 'deny') return stdoutDecision;
  return decisionFromExitCode(exitCode, stderr, fallback, failClosedOnNonDenyExit);
}

module.exports = {
  DEFAULT_DENY_REASON,
  stringOrFallback,
  denyDecisionFromHookSpecific,
  denyDecisionFromLegacyShape,
  decisionFromHookStdout,
  decisionFromExitCode,
  combineHookResult,
};
