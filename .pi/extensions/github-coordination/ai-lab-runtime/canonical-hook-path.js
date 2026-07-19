'use strict';

// @why The canonical hook payload contract is exactly one slash-normalized
// `tool_input.file_path`, regardless of host OS or originating client. Node's
// `path.join()` emits backslashes on Windows, so a guard like
// `filePath.includes("/src/")` silently exempts real source files when a Codex,
// Pi, or OpenCode payload originates on Windows. Every client mapper routes its
// synthetic `file_path` through this single builder so no client hand-rolls a
// second normalization lane that can drift.

const { normalizePathSeparators } = require('./checked-extensions.js');

// A path is already absolute under the slash contract when it is POSIX-absolute
// (`/x`) or a Windows drive root with a forward slash (`C:/x`) after separator
// normalization. Both reach the canonical hook unchanged.
const ABSOLUTE_SLASH_PATH_PATTERN = /^(?:\/|[A-Za-z]:\/)/;

function canonicalFilePath(projectRoot, filePathText) {
  const normalizedFilePath = normalizePathSeparators(filePathText);
  if (ABSOLUTE_SLASH_PATH_PATTERN.test(normalizedFilePath)) {
    return normalizedFilePath;
  }
  const normalizedRoot = normalizePathSeparators(projectRoot).replace(/\/+$/, '');
  if (normalizedRoot.length === 0) return normalizedFilePath;
  return `${normalizedRoot}/${normalizedFilePath}`;
}

module.exports = { canonicalFilePath };
