---
name: studio-wake
description: Resume an explicitly requested studio workflow from its existing coordination ledger and latest journal entry, reconcile active tasks, and continue one coordinator-owned cycle.
---

# Resume a studio run

Resume from the existing checkpoint/coordination ledger and latest journal. Reconcile live Codex tasks before dispatch; do not create a parallel queue, duplicate authority, scheduler, or automation. A single coordinator owns shared mutable records and runtime qualification. Create new Codex tasks only when the current user authorizes task creation for this goal; an earlier goal's authorization does not carry forward. Create only necessary new tasks, one role per chat, respecting live capacity and dependencies. Dispatch all newly eligible items without arbitrary card caps.

Use the live account usage-limit tool when available. If usage or reset telemetry cannot be read, record it as unavailable/null; never estimate it. A usage guard must use real account limits and must not bypass limits or pause/clear/recreate the user's goal. Do not create scheduled automation unless explicitly requested with a cadence. Any future scheduler must reconcile live thread/process state and enforce one coordinator owner; a lease file alone is insufficient.

At close-out, record handoffs and next action. Keep builder chat/worktree available through dependent reviews, corrections, and remote delivery; archive only after needed installation artifacts are preserved or recoverable. Archive other terminal worker/reviewer tasks after their dependencies and delivery are complete. Leave active/user-input tasks and the coordinator available. Keep the user informed of material progress and stop for required user input.

Source and license: [upstream attribution and MIT license](../studio-orchestrator/references/upstream.md).
