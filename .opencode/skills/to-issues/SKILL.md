---
name: to-issues
description: "Turn a supplied report, audit, or backlog slice into one coordinated issue plan with native parent and blocker relationships"
user-invocable: true
---

Treat the user's invocation text after `to-issues` as `$ARGUMENTS`.

# /to-issues

Run `github-coordination:guided-issue-filing` in batch mode with the user's
supplied findings. Treat the complete batch as one Issue Plan, even when it mixes
Work Types, creates new parent and child issues together, or updates existing
issues while adding splits.

Resolve all ambiguity in one grouped question, draft every Canonical Issue Body,
read the current state of every existing managed issue, express relationships in
GitHub's native parent and blocked-by graph, and invoke
`jakuta-agent-issue apply-plan` once for the whole mutation. Report the resolved
issue URLs and graph placement only after the complete plan succeeds.
