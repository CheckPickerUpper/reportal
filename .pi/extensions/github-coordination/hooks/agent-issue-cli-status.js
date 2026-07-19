#!/usr/bin/env node
/**
 * Surface the bot issue CLI setup before an agent reaches issue filing.
 *
 * @why The issue ledger depends on the Jakuta Review Bot author identity.
 * When `jakuta-agent-issue` is missing or cannot read its 1Password-backed
 * GitHub App fields, filing fails late and agents rediscover the same setup
 * gap. This hook prints the exact dotfiles repair path at session start while
 * leaving ordinary shell usage unblocked.
 */

"use strict";

const { spawnSync } = require("child_process");

function commandExists(commandName) {
  const result = spawnSync("sh", ["-lc", "command -v " + commandName], {
    encoding: "utf8",
    timeout: 3000,
  });
  return result.status === 0;
}

function isInsideGitRepo() {
  const result = spawnSync("git", ["rev-parse", "--is-inside-work-tree"], {
    encoding: "utf8",
    timeout: 3000,
  });
  return result.status === 0 && result.stdout.trim() === "true";
}

function warn(messageLines) {
  process.stderr.write(["github-coordination setup warning:", ...messageLines].join("\n") + "\n");
}

function runDoctor() {
  return spawnSync("jakuta-agent-issue", ["doctor"], {
    encoding: "utf8",
    timeout: 15000,
  });
}

function main() {
  if (!isInsideGitRepo()) {
    return;
  }
  if (!commandExists("jakuta-agent-issue")) {
    warn([
      "`jakuta-agent-issue` is not installed.",
      "Run `dotsync jakuta_agent_issue_cli`, then `jakuta-agent-issue doctor --repo OWNER/REPO`.",
    ]);
    return;
  }

  const doctorResult = runDoctor();
  if (doctorResult.status === 0) {
    return;
  }
  const doctorOutput = [doctorResult.stdout, doctorResult.stderr]
    .filter(Boolean)
    .join("")
    .trim();
  warn([
    "`jakuta-agent-issue doctor` did not pass.",
    doctorOutput || "No diagnostic output was produced.",
    "Repair path: `dotsync jakuta_agent_issue_cli`; if OP_SERVICE_ACCOUNT_TOKEN is missing, run `dotsync env` and open a new shell; if issues permission is missing, approve the Jakuta Review Bot GitHub App permission update.",
  ]);
}

try {
  main();
} catch (err) {
  process.stderr.write(
    "agent-issue-cli-status.js crashed: " + (err && err.stack ? err.stack : err) + "\n"
  );
}
