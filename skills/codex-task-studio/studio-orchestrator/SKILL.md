---
name: studio-orchestrator
description: Coordinate bounded repository work through visible, user-owned Codex tasks, with durable plans, fresh reviews, and auditable handoffs. Use when the user asks for studio-style delegation, multi-step coordination, or a wake/queue workflow.
---

# Studio coordinator

Use native Codex tasks created with `create_thread`; do not substitute hidden subagents. Create visible tasks only when the current user authorizes task creation for the current goal; a skill or earlier goal's authorization does not authorize unrelated future work. Reconcile current chats first and reuse only the same unfinished job. Create a new task for each new job and every fresh review. Keep one role per task and keep briefs short: goal, base/artifact pins, owned read/write paths, constraints, dependencies, acceptance checks, stop condition, and handoff format. Tool allowlists may not be enforceable; state least-tool constraints in each brief and disclose that limitation.

The coordinator owns operational plans, ledgers, dispatch/integration records, shared catalog/feed/registry/admission/runtime qualification, and integration. Workers own only their assigned task files; delegate product code and operational scripts. Coordinate at most four bounded work items per cycle, subject to actual capacity, and dispatch all newly eligible cards without an arbitrary cap.

Read [role defaults](references/role-defaults.md) when assigning a task. Select only models actually available; set effort explicitly and raise it only for justified risk or ambiguity. Do not claim Claude equivalents, prices, or savings. For consequential review effort changes, first run blind representative review benchmarks and measure per reviewed delivered task, not launches.

For every accepted change require builder, fresh reviewer 1, correction as needed, then a different fresh reviewer 2 against the exact fixed base/head or artifact hashes. Reviewers are never authors or planners; they read the target and may write only a separate report directory. Verify freshness using actual task IDs. Reviewer 2 must use a new, never-used task. Reviewer 1 may be resumed for correction feedback only, never as fresh reviewer 2. A changed target invalidates both approvals. No integration or admission without the relevant two-pass evidence and required project checks. Preserve older reviews as historical evidence only.

Collect durable handoffs with exact hashes/revisions, checks, unresolved scope, and next action. Keep the builder chat and managed worktree available through dependent reviews, corrections, and delivery. Do not archive them until public changes are confirmed remotely and needed installation artifacts are preserved or recoverable. Then archive terminal tasks and the worktree after handoff, preserving needed ignored files; never archive active, awaiting-input, or coordinator tasks before goal completion. Use compact `wait_threads` snapshots and cursors, not repeated transcripts. Do not create an automation unless the user explicitly asks for cadence; a future scheduler needs live thread/process reconciliation and single-coordinator ownership, not a lease file alone.

Source and license: [upstream attribution and MIT license](references/upstream.md).
