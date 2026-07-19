'use strict';

// Runs one plugin's canonical `hooks/*.js` script as a child process. Tool-event
// consumers call `runCanonicalHook` and get an allow|deny decision; lifecycle
// consumers call `runCanonicalHookProcess` and parse the raw stdout/stderr for
// their event-specific channels. Project narrowing is shared here so Codex,
// OpenCode, and Pi cannot drift on runtime-manifest semantics.

const childProcess = require('child_process');
const fs = require('fs');
const nodePath = require('path');

const { combineHookResult } = require('./parse-hook-decision.js');
const { projectAllowsPlugin } = require('./project-narrow.js');

const PROJECT_LOCAL_PLUGIN_SOURCE = 'project-local';
const DEFAULT_CLIENT_KEY = 'codex';
const DEFAULT_HOOK_TIMEOUT_MS = 5000;
const CLAUDE_PLUGIN_ROOT_TOKEN = '${CLAUDE_PLUGIN_ROOT}';

function pluginNameFromRoot(pluginRoot) {
  if (typeof pluginRoot !== 'string' || pluginRoot.length === 0) return null;
  const normalizedRoot = nodePath.normalize(pluginRoot);
  const basename = nodePath.basename(normalizedRoot);
  if (basename !== 'local' && !/^\d+\.\d+\.\d+(?:[-+].*)?$/.test(basename)) return basename;
  return nodePath.basename(nodePath.dirname(normalizedRoot)) || basename;
}

function resolvedPluginRoot(scriptPath, pluginRoot) {
  return typeof pluginRoot === 'string' && pluginRoot.length > 0
    ? pluginRoot
    : nodePath.join(scriptPath, '..', '..');
}

function resolvedCwd(cwd, claudePayload) {
  if (typeof cwd === 'string' && cwd.length > 0) return cwd;
  if (
    typeof claudePayload === 'object' &&
    claudePayload !== null &&
    typeof claudePayload.cwd === 'string' &&
    claudePayload.cwd.length > 0
  ) {
    return claudePayload.cwd;
  }
  return process.cwd();
}

function clientSpecificEnv(clientKey, pluginRoot) {
  if (clientKey === 'codex') return { CODEX_PLUGIN_ROOT: pluginRoot };
  if (clientKey === 'opencode') return { OPENCODE_PLUGIN_ROOT: pluginRoot };
  if (clientKey === 'pi') return { PI_PLUGIN_ROOT: pluginRoot };
  return {};
}

function scriptArgsWithPluginRoot(scriptArgs, pluginRoot) {
  if (!Array.isArray(scriptArgs)) return [];
  return scriptArgs.map((scriptArg) => {
    if (typeof scriptArg !== 'string') return String(scriptArg);
    if (scriptArg === CLAUDE_PLUGIN_ROOT_TOKEN) return pluginRoot;
    if (scriptArg.startsWith(`${CLAUDE_PLUGIN_ROOT_TOKEN}/`)) {
      return nodePath.join(pluginRoot, scriptArg.slice(CLAUDE_PLUGIN_ROOT_TOKEN.length + 1));
    }
    return scriptArg;
  });
}

// Result kinds:
//   { kind: 'skipped' }                        - script missing, or narrowed away
//   { kind: 'ran', stdout, stderr, exitCode }   - canonical hook executed
function runCanonicalHookProcess({
  scriptPath,
  scriptArgs,
  pluginRoot,
  pluginSource,
  claudePayload,
  cwd,
  timeoutMs,
  clientKey = DEFAULT_CLIENT_KEY,
}) {
  if (!fs.existsSync(scriptPath)) return { kind: 'skipped' };
  const root = resolvedPluginRoot(scriptPath, pluginRoot);
  const processCwd = resolvedCwd(cwd, claudePayload);

  if (pluginSource !== PROJECT_LOCAL_PLUGIN_SOURCE) {
    const pluginName = pluginNameFromRoot(root);
    if (pluginName && !projectAllowsPlugin(pluginName, processCwd, clientKey)) {
      return { kind: 'skipped' };
    }
  }

  const hookProcess = childProcess.spawnSync(process.execPath, [
    scriptPath,
    ...scriptArgsWithPluginRoot(scriptArgs, root),
  ], {
    input: JSON.stringify(claudePayload),
    encoding: 'utf8',
    timeout: typeof timeoutMs === 'number' && timeoutMs > 0 ? timeoutMs : DEFAULT_HOOK_TIMEOUT_MS,
    cwd: processCwd,
    env: {
      ...process.env,
      CLAUDE_PLUGIN_ROOT: root,
      ...clientSpecificEnv(clientKey, root),
    },
  });
  return {
    kind: 'ran',
    stdout: hookProcess.stdout || '',
    stderr: hookProcess.stderr || '',
    exitCode: typeof hookProcess.status === 'number' ? hookProcess.status : 0,
  };
}

function runCanonicalHook({
  scriptPath,
  scriptArgs,
  pluginRoot,
  pluginSource,
  claudePayload,
  defaultReason,
  timeoutMs,
  clientKey = DEFAULT_CLIENT_KEY,
}) {
  const processResult = runCanonicalHookProcess({
    scriptPath,
    scriptArgs,
    pluginRoot,
    pluginSource,
    claudePayload,
    timeoutMs,
    clientKey,
  });
  if (processResult.kind === 'skipped') return { kind: 'allow' };
  return combineHookResult({
    stdout: processResult.stdout,
    stderr: processResult.stderr,
    exitCode: processResult.exitCode,
    defaultReason,
  });
}

module.exports = { pluginNameFromRoot, runCanonicalHook, runCanonicalHookProcess, scriptArgsWithPluginRoot };
