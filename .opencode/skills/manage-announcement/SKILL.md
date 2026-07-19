---
name: manage-announcement
description: "Create or update a repo-local announcement through one complete Issue Plan, preserving bot attribution and optionally pinning the resulting issue. Use when the user says 'post an announcement', 'announce the release', 'update the pinned notice', or runs /github-coordination:create-announcement or /github-coordination:update-announcement."
user-invocable: true
---
# Manage Announcement

<supporting-info>
An Announcement is a repository-local coordination record. Its issue content is
reconciled through the same bot-authored Issue Plan as every other issue write.
Pinning or publishing a release remains a separate action because it answers a
different user intent.
</supporting-info>

## Confirm the repository and writer

<what-to-do>
Resolve the repository and verify the issue writer:

```bash
gh repo view --json nameWithOwner -q .nameWithOwner
jakuta-agent-issue doctor --repo OWNER/REPO
```

Stop on an unresolved repository, unavailable GitHub auth, failed generated
credential setup, missing app installation, or missing issue permission. Repair
a missing CLI through the dotfiles `jakuta_agent_issue_cli` step. Keep issue
writes on the bot-authored plan path.
</what-to-do>

## Build the announcement body

<what-to-do>
For a new announcement, resolve the repository override or bundled template:

```bash
node .opencode/skill-assets/github-coordination/lib/resolve-template.js announcement
```

Fill the headline, details, affected surface, and links. Ask one grouped question
only for required facts that remain ambiguous. Append the Agent Attribution
Marker exactly once:

```bash
node -e "process.stdout.write(require('.opencode/skill-assets/github-coordination/lib/attribution.js').AGENT_ATTRIBUTION_MARKER)"
```

For an update, resolve the issue number from the user or this read:

```bash
gh issue list --state open --label announcement --json number,title
```

Read the current title, body, labels, and native relationships. Preserve the
marker and every desired value while changing only the announcement content the
user requested.
</what-to-do>

## Apply one complete plan

<what-to-do>
Read `.opencode/skill-assets/github-coordination/references/issue-plan.md`. Build one plan with one
managed announcement. A new announcement omits `issue_number`; an update
includes it. Set the exact labels to include `announcement`, with `parent: null`
and `blocked_by: []` because announcements are outside the work graph.

Submit it once:

```bash
jakuta-agent-issue apply-plan --repo OWNER/REPO --plan-file <plan.json>
```

Read the structured result for the resolved issue number and URL. If the user
wants a pinned repository notice, pin that issue after the plan succeeds:

```bash
gh issue pin <number>
```

For release notes, a release may also be published with `gh release create`;
the labeled announcement issue remains the coordination record.
</what-to-do>

## Report

<what-to-do>
Report the announcement URL, whether its content was created or reconciled, and
whether the separate pin or release action succeeded.
</what-to-do>
