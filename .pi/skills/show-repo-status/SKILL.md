---
name: show-repo-status
description: "Show open issue trees, blockers, linked pull requests, announcements, and issues waiting on evidence from GitHub's native coordination state"
user-invocable: true
---

Treat the user's invocation text after `show-repo-status` as `$ARGUMENTS`.

# /show-repo-status — Repo Status View

Render one read-only summary from the GitHub Coordination Ledger. Every line must
trace to a GitHub read; this command never writes or invents state.

## Confirm and gather

```bash
gh repo view --json nameWithOwner -q .nameWithOwner
gh issue list --state open --label parent --json number,title
gh issue list --state open --limit 200 --json number,title,body,labels
gh pr list --state open --json number,title,headRefName,body
gh issue list --state open --label announcement --json number,title
```

For each open parent and issue, read native relationships:

```bash
gh api --paginate repos/OWNER/REPO/issues/NUMBER/sub_issues
gh api --paginate repos/OWNER/REPO/issues/NUMBER/dependencies/blocked_by
```

Bound network calls so an unavailable remote fails clearly instead of leaving
the view waiting indefinitely.

## Derive

- Build each parent/child tree from the native `sub_issues` results. Issue-body
  headings and task lists are content, never relationship evidence.
- List an open issue as blocked when its native `blocked_by` result contains an
  open blocker. Include closed blockers only when they explain a recent state
  transition.
- Parse open pull request bodies for closing references and group linked pull
  requests under their issue. List a pull request with no closing reference as
  unlinked without calling it defective.
- Mark an issue waiting on evidence when an acceptance criterion is unchecked or
  checked with an empty `Evidence:` slot.

## Render

```text
REPO STATUS — <owner/repo>

PARENT ISSUES (open)
  #12 Checkout rewrite
    ├─ #13 Cart persistence         open · linked PR #40
    ├─ #14 Payment retry            closed ✓
    └─ #15 Confirmation email       open · waiting on evidence (2 of 3 AC)

BLOCKED ISSUES (open)
  #18 Refund settlement  — blocked by #17 Payment provider contract

LINKED PRS (open)
  #40 Persist cart  →  closes #13

ANNOUNCEMENTS (open)
  #22 v0.9 release notes (pinned)

WAITING ON EVIDENCE
  #15 Confirmation email  — 2 of 3 criteria have no evidence

UNLINKED PRS (open)
  #41 Dependency refresh
```

Show only populated sections. An empty repository prints one line saying there
are no open coordination items.

## Boundary

This view is read-only. Filing and issue reconciliation go through one Issue
Plan. Pull requests remain in `/open-pull-request`; git sync and worktrees remain
in `github-workflow`.
