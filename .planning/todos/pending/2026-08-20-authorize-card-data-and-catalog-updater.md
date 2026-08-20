---
created: 2026-08-20T18:36:22.031Z
title: Authorize card data and catalog updater
area: planning
files:
  - .planning/phases/01-rules-and-data-authority/01-RESEARCH.md
  - .planning/phases/01-rules-and-data-authority/01-CONTEXT.md
  - docs/external-reuse-policy.md
---

## Problem

The private simulator needs a complete, current, reproducible Sorcery gameplay catalog. The official API is the best factual source and recommends intermittent polling and self-hosting, but the publisher Terms also prohibit automated access and systematic database creation without written permission. No audited community catalog currently grants a complete permissive data license: SadKingLabs' registry is technically strong but licenses code only, while other candidates are GPL-bound, unlicensed, stale, or incomplete.

The owner wants a reminder to contact relevant parties for authorization and to compare that route with a separate future catalog-maintenance tool. That possible tool would use teams of agents to collect, cross-check, normalize, and periodically update card/rules data every one or two weeks. Building it does not itself grant rights to source content, so authorization/provenance remains its first gate.

## Solution

1. Contact Erik's Curiosa at `community@sorcerytcg.com` and request written permission for agreed-rate API access, indefinite private caching, normalization, derivative machine-readable data, full gameplay fields/current text/updates/errata/FAQs, automated update checks, attribution, commercial/noncommercial scope, snapshot survival after revocation, and any redistribution terms. Treat artwork as excluded unless separately granted.
2. Contact SadKingLabs about licensing its registry-created stable IDs, mappings, slug/name history, corrections, schema, checksums, and export structure under CC0, CC BY 4.0, or ODC-By; confirm that publisher authorization separately covers underlying card content.
3. Record responses, permitted fields, rate limits, attribution, update obligations, and durable source/revision terms in `docs/external-reuse-policy.md` before enabling automated acquisition.
4. Compare authorization-first integration with a separate catalog-maintenance tool. The separate tool should exist only when recurring updates or multiple consumers justify it and should provide source/license registry, immutable raw snapshots, connectors, agent-assisted validation, deterministic normalization, semantic diffs, human approval, versioned publication, audit logs, and a weekly/biweekly scheduler.
5. Keep the simulator's current v1 path simple: manual browser save, Git-ignored private snapshot, offline TypeScript catalog/query module, no public HTTP API. If the separate tool is approved, create it in a different project/thread and publish signed/versioned local snapshots for this simulator to import.

## Decision Check

- Authorization route: lowest engineering cost and best authority, but response time is external and uncertain.
- Separate updater: materially more engineering and operations work; valuable only after authorized sources exist. It improves freshness and validation but cannot cure missing data rights.
- Recommended order: request authorization now, finish the private manual-import simulator path, then build the updater only after permission or a genuinely permissive source is secured.