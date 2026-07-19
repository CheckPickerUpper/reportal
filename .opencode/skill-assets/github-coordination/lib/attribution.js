#!/usr/bin/env node
/**
 * Canonical attribution identity for github-coordination writes.
 *
 * @why One source for the reused dotfiles bot identity so every command, skill,
 * and hook stamps the same Bot Attribution label and the same visible Agent
 * Attribution Marker — inventing a second bot identity is exactly what the
 * coordination contract forbids.
 *
 * The dotfiles bot identity is the Jakuta Review Bot GitHub App
 * (manifests/dev-repos.toml -> jakuta-review-bot; app.yml "Jakuta Review Bot").
 * Agent-filed issue plans use `jakuta-agent-issue apply-plan`, so issue writes
 * are authored by the app bot. The visible marker remains the durable
 * provenance line for readers.
 */

"use strict";

// The reused dotfiles bot identity. Not the operator's personal git user.
const BOT_ATTRIBUTION_LABEL = "Jakuta Review Bot";
const BOT_ATTRIBUTION_SLUG = "jakuta-review-bot[bot]";

// The literal text that tells a human reader an agent authored this record.
// Must stay visible in the Canonical Issue Body, never hidden metadata alone.
const AGENT_ATTRIBUTION_MARKER =
  "> 🤖 **Agent-authored** — an agent wrote this coordination record. " +
  "Attributed to the " + BOT_ATTRIBUTION_LABEL + " (`" + BOT_ATTRIBUTION_SLUG + "`), " +
  "the reused dotfiles bot identity; no second bot identity is invented.";

/**
 * @why Appends the visible marker to a body only when it is not already there,
 * so re-running a guided flow over an existing body cannot stack duplicate
 * markers.
 */
function withAgentMarker(body) {
  const text = typeof body === "string" ? body : "";
  if (text.includes("an agent wrote this")) return text;
  return text.replace(/\s*$/, "") + "\n\n" + AGENT_ATTRIBUTION_MARKER + "\n";
}

module.exports = {
  BOT_ATTRIBUTION_LABEL,
  BOT_ATTRIBUTION_SLUG,
  AGENT_ATTRIBUTION_MARKER,
  withAgentMarker,
};
