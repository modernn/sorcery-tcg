---
name: studio-queue
description: Maintain the studio workflow's durable task queue, dependencies, ownership, and terminal handoffs without duplicating active work.
---

# Studio queue

Before dispatch, reconcile existing Codex chats and reuse only the same unfinished job. Each new job and fresh review gets a new visible user-owned task; one role per chat. Every brief pins goal, base/artifacts, owned paths, constraints, dependencies, acceptance checks, stop, and handoff. Workers own only their task files; the coordinator alone writes shared catalog/feed/registry/admission/runtime qualification and integration records.

Limit each coordination cycle to four bounded work items or actual lower capacity, and send every newly eligible item without an arbitrary cap. Respect dependencies and avoid duplicate dispatch. Keep queue and dispatch status in the durable coordinator ledger. On completion capture exact hashes/revisions, checks, unresolved scope, and next action. Keep a builder chat and worktree available until dependent reviews and corrections are complete, delivery is confirmed remotely, and needed installation artifacts are preserved or recoverable. Never archive active or awaiting-input work, or a builder whose target still has dependent work.

Source and license: [upstream attribution and MIT license](../studio-orchestrator/references/upstream.md).
