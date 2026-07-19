#!/usr/bin/env node
/**
 * Resolve the issue-body template for a Work Type, letting a repo override the
 * plugin default.
 *
 * @why A venture repo often has its own issue format. This is the single lookup
 * both guided-issue-filing and manage-announcement use, so the override rule
 * cannot drift between them: a committed repo-local template at
 * <repo>/.github-coordination/templates/<work-type>.md wins; otherwise the
 * plugin's bundled templates/ default is used.
 *
 * CLI: `node resolve-template.js <work-type>` prints the resolved absolute path
 * on stdout and the source (RepoOverride | PluginDefault) on stderr. Exits 2 on
 * an unknown Work Type so a caller never silently files against the wrong shape.
 */

"use strict";

const fs = require("fs");
const path = require("path");
const { execFileSync } = require("child_process");

// Work Type -> template filename. parent uses parent-issue.md; the rest match.
const WORK_TYPE_FILE = {
  bug: "bug.md",
  "ui-bug": "ui-bug.md",
  "ux-issue": "ux-issue.md",
  feature: "feature.md",
  task: "task.md",
  parent: "parent-issue.md",
  announcement: "announcement.md",
};

// A venture drops its own templates here, committed and shared with the repo.
const REPO_OVERRIDE_DIR = path.join(".github-coordination", "templates");

function repoRoot() {
  try {
    return execFileSync("git", ["rev-parse", "--show-toplevel"], {
      encoding: "utf8",
      stdio: ["ignore", "pipe", "ignore"],
    }).trim();
  } catch {
    // @why Outside a git repo there is no override location; fall back to the
    // plugin default rather than failing.
    return null;
  }
}

/**
 * @why Returns a tagged outcome (UnknownWorkType | RepoOverride | PluginDefault)
 * so the CLI and any future caller branch on the source explicitly instead of
 * guessing from a bare path string.
 */
function resolveTemplate(workType) {
  const file = WORK_TYPE_FILE[workType];
  if (!file) return { kind: "UnknownWorkType", workType };

  const pluginRoot = path.resolve(__dirname, "..");
  const root = repoRoot();
  if (root) {
    const override = path.join(root, REPO_OVERRIDE_DIR, file);
    if (fs.existsSync(override)) return { kind: "RepoOverride", path: override };
  }
  return { kind: "PluginDefault", path: path.join(pluginRoot, "templates", file) };
}

function main() {
  const workType = process.argv[2];
  const outcome = resolveTemplate(workType);
  if (outcome.kind === "UnknownWorkType") {
    process.stderr.write(
      "resolve-template: unknown Work Type '" + String(workType) + "'. " +
        "Known: " + Object.keys(WORK_TYPE_FILE).join(", ") + "\n"
    );
    process.exit(2);
  }
  process.stderr.write(outcome.kind + "\n");
  process.stdout.write(outcome.path + "\n");
}

if (require.main === module) main();

module.exports = { resolveTemplate, WORK_TYPE_FILE, REPO_OVERRIDE_DIR };
