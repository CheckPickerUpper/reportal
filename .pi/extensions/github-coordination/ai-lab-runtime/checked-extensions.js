'use strict';

const pathModule = require('path');

const CHECKED_EXTENSIONS = new Set([
  '.adoc',
  '.css',
  '.dart',
  '.htm',
  '.html',
  '.hs',
  '.bash',
  '.cjs',
  '.cts',
  '.js',
  '.jsx',
  '.less',
  '.lhs',
  '.lua',
  '.luau',
  '.md',
  '.mdx',
  '.mjs',
  '.py',
  '.rst',
  '.sass',
  '.scss',
  '.sh',
  '.json',
  '.toml',
  '.ts',
  '.tsx',
  '.txt',
  '.vue',
  '.svelte',
  '.rs',
  '.sql',
  '.zsh',
]);

function normalizePathSeparators(filePathText) {
  return String(filePathText || '').replace(/\\/g, '/');
}

function hasCheckedExtension(filePathText) {
  return CHECKED_EXTENSIONS.has(
    pathModule.extname(normalizePathSeparators(filePathText)).toLowerCase(),
  );
}

module.exports = {
  CHECKED_EXTENSIONS,
  hasCheckedExtension,
  normalizePathSeparators,
};
