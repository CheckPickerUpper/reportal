'use strict';

const fs = require('fs');
const pathModule = require('path');

function loadWriteToolSurface() {
  const candidatePaths = [
    pathModule.join(__dirname, 'write-tool-surface.json'),
    pathModule.join(__dirname, '../../../../../plugins/hook-lib/lib/write-tool-surface.json'),
  ];
  for (const candidatePath of candidatePaths) {
    if (fs.existsSync(candidatePath)) {
      return JSON.parse(fs.readFileSync(candidatePath, 'utf-8'));
    }
  }
  throw new Error('write-tool-surface.json not found');
}

function mcpWritePathFieldMap(writeToolSurface) {
  return new Map(writeToolSurface.mcp.map((spec) => [spec.name, spec.path_field]));
}

function nativeApplyPatchSurfaces(writeToolSurface, clientName) {
  const nativeSurfaces = Array.isArray(writeToolSurface.native) ? writeToolSurface.native : [];
  return nativeSurfaces.filter((spec) => (
    spec
    && spec.client === clientName
    && spec.kind === 'ApplyPatch'
  ));
}

function nativeApplyPatchToolNames(writeToolSurface, clientName) {
  const toolNames = new Set();
  for (const spec of nativeApplyPatchSurfaces(writeToolSurface, clientName)) {
    const rawToolNames = Array.isArray(spec.tool_names) ? spec.tool_names : [];
    for (const toolName of rawToolNames) {
      if (typeof toolName === 'string' && toolName.length > 0) toolNames.add(toolName);
    }
  }
  return toolNames;
}

function nativeApplyPatchTextFields(writeToolSurface, clientName) {
  const textFields = [];
  for (const spec of nativeApplyPatchSurfaces(writeToolSurface, clientName)) {
    const rawTextFields = Array.isArray(spec.text_fields) ? spec.text_fields : [];
    for (const fieldName of rawTextFields) {
      if (typeof fieldName === 'string' && fieldName.length > 0 && !textFields.includes(fieldName)) {
        textFields.push(fieldName);
      }
    }
  }
  return textFields;
}

function writeToolKindMap(writeToolSurface) {
  return new Map([
    ...writeToolSurface.builtin.map((name) => [name, name]),
    ...writeToolSurface.mcp.map((spec) => [spec.name, spec.kind]),
  ]);
}

function builtinWriteToolNames(writeToolSurface) {
  return new Set(writeToolSurface.builtin);
}

module.exports = {
  builtinWriteToolNames,
  loadWriteToolSurface,
  mcpWritePathFieldMap,
  nativeApplyPatchTextFields,
  nativeApplyPatchToolNames,
  writeToolKindMap,
};
