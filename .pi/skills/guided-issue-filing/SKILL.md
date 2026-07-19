---
name: guided-issue-filing
description: "File or reconcile one or more GitHub issues from Work Type templates through one complete Issue Plan, with clarifying questions, native parent and blocker routing, evidence-backed acceptance criteria, and agent attribution. Use when the user says 'file an issue', 'open a bug', 'log a feature', 'track this task', 'turn these findings into issues', or runs a /github-coordination:file-*-issue or /github-coordination:to-issues command."
user-invocable: true
---
# Guided Issue Filing

<supporting-info>
This is the single filing flow behind every Work Type command and the batch
`/github-coordination:to-issues` command. GitHub is the authoritative
coordination record. One Issue Plan expresses the complete result; callers do
not assemble issue writes or relationship updates step by step.
</supporting-info>

## Confirm the repository and writer

<what-to-do>
Resolve the repository and verify the bot writer before drafting:

```bash
gh repo view --json nameWithOwner -q .nameWithOwner
jakuta-agent-issue doctor --repo OWNER/REPO
```

If the repository cannot be resolved or GitHub auth is unavailable, stop with
that blocker. If the CLI is missing, repair it through the dotfiles
`jakuta_agent_issue_cli` step. If `doctor` reports missing generated credentials,
repair the dotfiles machine environment and reopen the shell. If the GitHub App
is not installed or lacks issue write permission, surface that repository
blocker. All issue writes must remain on the bot-authored plan path.
</what-to-do>

## Draft the complete issue set

<what-to-do>
For each requested issue, resolve and read its Work Type template:

```bash
node .pi/skill-assets/github-coordination/lib/resolve-template.js <work-type>
```

A committed repository override at
`.github-coordination/templates/<work-type>.md` wins over the bundled template.
Fill every field supported by the user's input. Ask one grouped question only
when a required fact, Work Type, scope boundary, or routing choice remains truly
ambiguous. A batch gets one grouped clarification pass, not one interview per
issue.

Keep each acceptance criterion concrete and leave its evidence slot empty at
filing time. Append the Agent Attribution Marker exactly once:

```bash
node -e "process.stdout.write(require('.pi/skill-assets/github-coordination/lib/attribution.js').AGENT_ATTRIBUTION_MARKER)"
```

Write each finished body to a separate file beside the plan file.
</what-to-do>

## Route through GitHub's native graph

<what-to-do>
Read open parent candidates and relevant existing issue relationships before
building the plan:

```bash
gh issue list --state open --label parent --json number,title
```

If exactly one parent owns an issue, select it. If several plausibly own it,
ask once with the candidates and a standalone choice. Never invent a parent.
A parent Work Type is a root. Capture dependencies the user named as native
`blocked_by` relationships. Do not copy parents, children, or blockers into
issue-body fields or checklists.
</what-to-do>

## Apply one complete plan

<what-to-do>
Read `.pi/skill-assets/github-coordination/references/issue-plan.md` and build one Issue Plan
covering the whole request. A single-file command produces a one-issue plan; a
batch produces one multi-issue plan. Include the full desired labels: the Work
Type label for every issue and `parent` for a parent Work Type.

Submit the plan exactly once:

```bash
jakuta-agent-issue apply-plan --repo OWNER/REPO --plan-file <plan.json>
```

Use stable issue keys so rerunning the same plan reconciles the same records.
Read the structured result and report every issue URL, Work Type, native parent
or standalone status, and blocker set in one final block.
</what-to-do>

## Boundary

<what-to-do>
Keep branch creation, commits, pushes, and worktrees in `github-workflow`. If the
user also asks to start implementation, hand the filed issue set to that
workflow after this plan succeeds.
</what-to-do>
