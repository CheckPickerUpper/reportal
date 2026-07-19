'use strict';

const fs = require('fs');
const nodePath = require('path');

function projectAllowsPlugin(pluginName, cwd, clientKey) {
  const runtimeManifestPath = findRuntimeManifestPath(cwd);
  if (runtimeManifestPath === null) { return true; }
  return runtimeManifestAllowsPlugin(runtimeManifestPath, pluginName, clientKey);
}

function projectIsAiLabManaged(cwd) {
  return findAiLabProjectRoot(cwd) !== null;
}

function findRuntimeManifestPath(cwd) {
  let searchDir = nodePath.resolve(typeof cwd === 'string' && cwd.length > 0 ? cwd : process.cwd());
  while (true) {
    const runtimeManifestPath = nodePath.join(searchDir, '.ai-lab', 'runtime-manifest.json');
    if (fs.existsSync(runtimeManifestPath)) { return runtimeManifestPath; }
    const parentDir = nodePath.dirname(searchDir);
    if (parentDir === searchDir) { return null; }
    searchDir = parentDir;
  }
}

function findAiLabProjectRoot(cwd) {
  let searchDir = nodePath.resolve(typeof cwd === 'string' && cwd.length > 0 ? cwd : process.cwd());
  while (true) {
    if (
      fs.existsSync(nodePath.join(searchDir, '.ai-lab', 'plugins.toml')) ||
      fs.existsSync(nodePath.join(searchDir, '.ai-lab', 'runtime-manifest.json'))
    ) {
      return searchDir;
    }
    const parentDir = nodePath.dirname(searchDir);
    if (parentDir === searchDir) { return null; }
    searchDir = parentDir;
  }
}

function runtimeManifestAllowsPlugin(runtimeManifestPath, pluginName, clientKey) {
  const parsed = readJsonObject(runtimeManifestPath);
  if (parsed === null) return true;
  const clients = parsed.clients;
  if (typeof clients !== 'object' || clients === null) return true;
  const rows = clients[clientKey];
  if (!Array.isArray(rows)) return true;
  return rows.some((row) => (
    typeof row === 'object' &&
    row !== null &&
    row.plugin === pluginName
  ));
}

function readJsonObject(jsonPath) {
  try {
    const parsed = JSON.parse(fs.readFileSync(jsonPath, 'utf8'));
    return typeof parsed === 'object' && parsed !== null ? parsed : null;
  } catch (error) {
    return null;
  }
}

module.exports = { findRuntimeManifestPath, projectAllowsPlugin, projectIsAiLabManaged };
