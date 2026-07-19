# Issue Plan Contract

<what-to-do>
Represent every coordinated issue write as one JSON Issue Plan and submit it once:

```bash
jakuta-agent-issue apply-plan --repo OWNER/REPO --plan-file <plan.json>
```

Use this exact shape:

```json
{
  "plan_key": "checkout-recovery",
  "references": {
    "existing-parent": 12
  },
  "issues": [
    {
      "key": "retry-failed-payment",
      "title": "Retry a failed payment",
      "body_file": "bodies/retry-failed-payment.md",
      "labels": ["feature"],
      "parent": "existing-parent",
      "blocked_by": []
    }
  ]
}
```

For an existing managed issue, add `"issue_number": 34`. Keys use lowercase
words separated by hyphens and stay stable across reruns. Body paths are
relative to the plan file. `parent` is a managed key, a key in `references`, or
`null`. Every `blocked_by` entry is likewise a managed or reference key.

Before applying a plan, make it complete:

- `title`, `body_file`, and `labels` are the exact desired issue state.
- `parent` is the exact desired native GitHub parent.
- `blocked_by` is the exact desired native GitHub blocker set.
- `references` names existing issues that supply relationships without becoming
  managed issues.
- Every managed body includes the Agent Attribution Marker exactly once.

When an existing issue enters a plan, read its current title, body, labels, and
native relationships first. Carry forward every value that should remain. Do
not submit a partial representation and expect omitted state to survive.

Use GitHub reads to inspect native relationships:

```bash
gh api --paginate repos/OWNER/REPO/issues/NUMBER/sub_issues
gh api --paginate repos/OWNER/REPO/issues/NUMBER/dependencies/blocked_by
gh api graphql -F owner=OWNER -F name=REPO -F number=NUMBER \
  -f query='query($owner:String!,$name:String!,$number:Int!){repository(owner:$owner,name:$name){issue(number:$number){parent{number}}}}'
```

Run one mutation command for the whole request. Read the structured result to
report the resolved issue numbers and URLs. A rerun uses the same plan and keys;
the command reconciles instead of creating duplicates.
</what-to-do>

<supporting-info>
The plan is the whole desired result, not a sequence of issue-sized operations.
Validation happens before GitHub writes, and the command owns creation, updates,
labels, native parent links, and native blocked-by links together.
</supporting-info>
