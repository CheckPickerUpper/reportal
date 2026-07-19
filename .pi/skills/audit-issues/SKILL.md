---
name: audit-issues
description: "Audit GitHub issues against repository state or evidence-backed acceptance criteria, then reconcile confirmed issue changes through one complete Issue Plan."
user-invocable: true
disable-model-invocation: true
---
# Audit Issues

<supporting-info>
The first pass is read-only and returns one verdict per issue. A later confirmed
write is one coordinated plan, so a split, retitle, relabel, and relationship
change cannot land as separate half-finished operations.
</supporting-info>

## Read-only evidence pass

<what-to-do>
Do not close, edit, relabel, retitle, or comment until the read-only report is
complete and explicit user confirmation names the exact rows to mutate. An
initial request that already asks for a named batch update counts as confirmation
only for those named rows and changes.

Resolve the repository, trunk, issue set, and linked pull requests:

```bash
gh repo view --json nameWithOwner -q .nameWithOwner
git remote -v
git symbolic-ref --quiet refs/remotes/origin/HEAD
gh issue view <number> --json number,title,body,state,labels,comments,url
gh issue list --state open --limit 200 --json number,title,body,state,labels,updatedAt,url
gh pr list --state all --limit 200 --json number,title,body,state,mergedAt,headRefName,baseRefName,url
```

For every issue, inspect its title, body, labels, comments, native parent,
native children, native blocked-by relationships, acceptance criteria, linked
pull requests, merged commits, branches, and exact repository evidence. Use the
native relationship reads from
`.pi/skill-assets/github-coordination/references/issue-plan.md`; do not infer relationships from
body headings or checklists.

Gather exact implementation evidence:

```bash
git fetch --all --prune --quiet
git log --oneline --decorate --all --grep "<issue number or key>"
git branch --all --list "*<issue number or key>*"
rg -n "<exact feature term|symbol|path from issue>" .
git diff --stat origin/<trunk>...HEAD
git log --oneline -- <relevant paths>
```

Run tests or generated-surface checks named by the issue, pull request, changed
paths, or repository rules. When the issue names exact files, commands, errors,
screenshots, or artifacts, check those exact things. Otherwise label any
conservative search terms as derived.
</what-to-do>

## Evidence-close classification

<what-to-do>
For every acceptance criterion, assign exactly one evidence state:

- `PROVEN`: checked and backed by a followable pull request, URL, artifact,
  screenshot, command result, or log.
- `UNPROVEN`: checked but its evidence is empty, vague, placeholder text, or not
  independently verifiable.
- `OPEN`: unchecked.

An issue is closeable only when every criterion is `PROVEN`. Comments may explain
the work, but the Canonical Issue Body must reference the evidence.
</what-to-do>

## Verdicts

<what-to-do>
Assign exactly one verdict per issue:

```text
DONE · NOT_DONE · PARTIAL · STALE · SUPERSEDED · NEEDS_UPDATE
```

- `DONE`: every criterion has concrete proof.
- `NOT_DONE`: the requested outcome is absent or proof is missing.
- `PARTIAL`: some criteria have proof and some do not; name both groups.
- `STALE`: the described world no longer exists, but no replacement proves
  closure.
- `SUPERSEDED`: a named issue, pull request, commit, branch, or product decision
  replaces the issue.
- `NEEDS_UPDATE`: the issue stays open, but its title, body, labels, native
  relationships, or acceptance criteria are wrong or incomplete.

No `probably done`, `looks complete`, `close enough`, `maybe stale`, or free-form
verdicts. Never treat branch-name similarity, age, a closed but unmerged pull
request, a nearby implementation, worktree cleanliness, or memory as proof.
</what-to-do>

## Report

<what-to-do>
Render this read-only table before any write:

```text
ISSUE VERDICT AUDIT
Repo: <owner/repo>
Trunk: origin/<trunk>
Scope: <selector>

| Issue | Verdict | Reason | Evidence | Missing proof | Proposed issue action |
|---|---|---|---|---|---|
| #12 Title | DONE | every criterion is implemented | PR #44 merged; src/auth/passkey.ts:38; named test exits 0 | none | close with evidence comment |
| #13 Title | PARTIAL | CLI exists, docs missing | pyproject.toml:27 declares the script | docs criterion | comment + keep open |
| #14 Title | NOT_DONE | requested endpoint is absent | exact search returns no hit; no linked PR | proof the endpoint exists | keep open |
| #15 Title | STALE | premise was removed | commit a1b2c3d deleted the named path | named replacement | comment + keep open |
| #16 Title | SUPERSEDED | newer issue owns the work | #28 and merged PR #44 replace it | none | close with replacement comment |
| #17 Title | NEEDS_UPDATE | issue names the wrong public surface | source exposes only the blessed surface | none | reconcile title and criteria |
```

Allowed proposed actions are `close`, `comment`, `retitle`, `relabel`, `split`,
`update native relationships`, and `keep open`.

For evidence-close mode, append:

```text
EVIDENCE-CLOSE CHECK — #<number> <title>

PROVEN    (n)
  - <criterion> — <evidence>
UNPROVEN  (n)
  - <criterion> — <missing or unverifiable evidence>
OPEN      (n)
  - <criterion>

VERDICT: <CLOSEABLE | NOT_CLOSEABLE — m unproven, k open>
```
</what-to-do>

## Apply confirmed changes

<what-to-do>
After confirmation, divide the requested actions by intent:

1. For every split, new issue, title, body, label, parent, or blocker change,
   read the latest complete state and build one Issue Plan spanning every
   affected issue. Include unchanged desired values because plan state is exact.
2. Read `.pi/skill-assets/github-coordination/references/issue-plan.md` and submit the complete
   plan once:

   ```bash
   jakuta-agent-issue apply-plan --repo OWNER/REPO --plan-file <plan.json>
   ```

3. Only after the plan succeeds, perform confirmed comment and closure actions:

   ```bash
   gh issue comment <number> --body-file <evidence-summary.md>
   gh issue close <number>
   ```

Skip the plan only when the confirmed batch contains no issue creation, content,
label, parent, or blocker change. Do not close `PARTIAL`, `NOT_DONE`, `STALE`, or
`NEEDS_UPDATE`. Close `SUPERSEDED` only when the replacement is explicitly named
and linked in the confirmed comment.
</what-to-do>

## Invariants

<what-to-do>
Keep the first pass read-only, preserve missing proof as missing, use the closed
verdict set, require a concrete proposed action for every row, and send every
confirmed issue-state batch through one complete plan.
</what-to-do>
