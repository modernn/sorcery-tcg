---
name: studio-plan-build
description: Plan and build coordinated changes when design risk, reversibility, or substantial research warrants distinct planning before implementation. Skip planning for fully specified mechanical fixes.
---

# Plan, then build

Use the project workflow's native Codex tasks, one role per task. Record the plan in the coordinator ledger: approved representations and helpers, affected callers, timing/ownership, proof pattern, compatibility effects, dependencies, and acceptance checks. Create a distinct planner task only for meaningful design risk or new representations; fully specified fixes go directly to a builder.

Pin the base commit and artifact hashes in each brief. Name read/write paths and keep worker ownership disjoint. A builder implements only its assigned product or operational-script files and returns changed paths, hashes, checks, unresolved issues, and a concise handoff. The coordinator alone edits shared catalog/feed/registry/admission/runtime qualification and integration records.

After build, obtain two fresh independent reviews, with reviewer 2 assigned to a new, never-used task of the exact final target through `studio-review`. Any target change invalidates approvals. Do not integrate or admit until both passes and required project checks succeed.

Source and license: [upstream attribution and MIT license](../studio-orchestrator/references/upstream.md).
