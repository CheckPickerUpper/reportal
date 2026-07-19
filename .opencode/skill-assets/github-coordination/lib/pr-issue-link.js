#!/usr/bin/env node
/**
 * Detects whether a `gh pr create` / `gh pr edit` command either closes a
 * linked issue or proves an exact development-to-production promotion.
 *
 * @why Ordinary pull requests implement an issue and must close it. A direct
 * development-to-production promotion implements no new issue, so it instead
 * carries the evidence that makes that release move reviewable. Kept pure
 * (string in, named outcome out) so the hook stays a thin adapter and the rule
 * is unit-checkable.
 */

"use strict";

const fs = require("fs");

// GitHub's closing keywords, followed by an issue ref (#123 or a full URL).
const CLOSING_REFERENCE =
  /\b(close|closes|closed|fix|fixes|fixed|resolve|resolves|resolved)\b\s*:?\s+(#\d+|https?:\/\/github\.com\/[^/\s]+\/[^/\s]+\/issues\/\d+)/i;

const PROMOTION_PROOF_LINES = [
  /^##\s+Promotion proof\s*$/im,
  /^- Development commit:\s+[0-9a-f]{7,40}\s*$/im,
  /^- Production commit:\s+[0-9a-f]{7,40}\s*$/im,
  /^- Production history:\s+\S.+$/im,
  /^- Required checks:\s+\S.+$/im,
  /^- Protection:\s+no force-push or bypass\.?\s*$/im,
];

/**
 * Classifies a shell command string.
 *
 * Outcome kinds:
 *   NotPrWrite          — not a `gh pr create`/`edit`; the hook ignores it.
 *   HasClosingReference — a closing keyword + issue ref was found in the body.
 *   HasPromotionProof    — direct development-to-production PR with release proof.
 *   BodyUnreadable      — a --body-file was named but could not be read.
 *   MissingPromotionProof — direct promotion missing its required proof body.
 *   MissingClosingRef   — a PR write with no resolvable closing reference.
 */
function classifyPrCommand(command) {
  const text = typeof command === "string" ? command : "";

  const isPrWrite = /\bgh\s+pr\s+(create|edit)\b/.test(text);
  if (!isPrWrite) return { kind: "NotPrWrite" };

  const bodyTexts = [];
  const inline = extractInlineBody(text);
  if (inline !== null) {
    bodyTexts.push({ source: "inline", text: inline });
    if (CLOSING_REFERENCE.test(inline)) {
      return { kind: "HasClosingReference", source: "inline" };
    }
  }

  const bodyFile = extractBodyFilePath(text);
  if (bodyFile !== null) {
    let fileText;
    try {
      fileText = fs.readFileSync(bodyFile, "utf8");
    } catch (err) {
      return { kind: "BodyUnreadable", path: bodyFile, error: String(err && err.message ? err.message : err) };
    }
    bodyTexts.push({ source: "body-file", text: fileText });
    if (CLOSING_REFERENCE.test(fileText)) {
      return { kind: "HasClosingReference", source: "body-file" };
    }
  }

  // Some callers put the closing ref in --title; honor it so a legitimately
  // linked PR is never blocked on a body/title technicality.
  const title = extractFlagValue(text, "title") || extractFlagValue(text, "t");
  if (title !== null && CLOSING_REFERENCE.test(title)) {
    return { kind: "HasClosingReference", source: "title" };
  }

  for (const bodyText of bodyTexts) {
    if (isDevelopmentToProductionPromotion(text) && hasPromotionProof(bodyText.text)) {
      return { kind: "HasPromotionProof", source: bodyText.source };
    }
  }

  if (isDevelopmentToProductionPromotion(text)) {
    return { kind: "MissingPromotionProof" };
  }

  return { kind: "MissingClosingRef" };
}

function isDevelopmentToProductionPromotion(text) {
  return (
    /\bgh\s+pr\s+create\b/.test(text) &&
    extractFlagValue(text, "base") === "production" &&
    extractFlagValue(text, "head") === "development"
  );
}

function hasPromotionProof(bodyText) {
  return PROMOTION_PROOF_LINES.every((requirement) => requirement.test(bodyText));
}

function extractInlineBody(text) {
  const long = extractFlagValue(text, "body");
  if (long !== null) return long;
  const short = extractFlagValue(text, "b");
  if (short !== null) return short;
  return null;
}

function extractBodyFilePath(text) {
  return extractFlagValue(text, "body-file") || extractFlagValue(text, "F");
}

/**
 * Pulls the value of --flag / -f from a command string, handling single
 * quotes, double quotes, and bare tokens. Returns null when the flag is absent.
 */
function extractFlagValue(text, flag) {
  const dash = flag.length === 1 ? "-" : "--";
  const head = dash + escapeForRegex(flag);
  const re = new RegExp(
    head + "(?:=|\\s+)(?:'([^']*)'|\"((?:[^\"\\\\]|\\\\.)*)\"|(\\S+))"
  );
  const m = text.match(re);
  if (!m) return null;
  if (m[1] !== undefined) return m[1];
  if (m[2] !== undefined) return m[2].replace(/\\(["\\])/g, "$1");
  return m[3];
}

function escapeForRegex(s) {
  return s.replace(/[.*+?^${}()|[\]\\]/g, "\\$&");
}

module.exports = { classifyPrCommand, CLOSING_REFERENCE };
