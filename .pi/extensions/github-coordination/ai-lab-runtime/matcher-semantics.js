'use strict';

const {
  loadWriteToolSurface,
  writeToolKindMap,
} = require('./write-tool-surface-loader.js');

const WRITE_TOOL_KINDS = writeToolKindMap(loadWriteToolSurface());

function matcherAcceptsClaudeToolName(matcher, claudeToolName) {
  if (matcher === null || matcher === undefined || matcher === '') return true;
  try {
    const re = new RegExp(`^(?:${matcher})$`);
    return matcherCandidateToolNames(claudeToolName).some((candidate) => re.test(candidate));
  } catch {
    return false;
  }
}

function matcherCandidateToolNames(claudeToolName) {
  const rawToolName = String(claudeToolName || '');
  const kind = WRITE_TOOL_KINDS.get(rawToolName);
  const candidates = [rawToolName];
  if (kind && kind !== rawToolName) candidates.push(kind);
  if (kind === 'MultiEdit' || rawToolName === 'MultiEdit') candidates.push('Edit');
  return [...new Set(candidates)];
}

module.exports = {
  matcherAcceptsClaudeToolName,
  matcherCandidateToolNames,
};
