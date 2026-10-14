---
name: studio-review
description: Perform the studio workflow's independent two-pass review for a coordinated change, including reviewer freshness and exact target verification.
---

# Fresh review protocol

For consequential coordinated work, request reviewer 1 after the builder handoff, then correct findings and request reviewer 2 in a new, never-used Codex task on the exact fixed base/head or artifact hash set. Reviewers are read-only on the target and may write only in a separate report directory. Neither may be the author, planner, or a resumed former reviewer. Verify actual task IDs to establish freshness.

Review source semantics, architecture, privacy, scope, and the required checks. Report findings with severity, evidence, and affected path; do not fix implementation. Reviewer 1 may be resumed only to clarify correction feedback, never to serve as fresh reviewer 2. If any target changes after a review, that approval is stale and the changed target needs the required fresh passes. Preserve prior reviews as historical evidence; never relabel them as fresh.

Integration/admission requires both approvals on the exact final target plus required project checks. For docs/skills changes run `pnpm verify`; for Rust changes use the repository's locked format, check, clippy, and test gates. Do not invent extra tests unless requested or needed to prove a finding.

Source and license: [upstream attribution and MIT license](../studio-orchestrator/references/upstream.md).
