# Sorcery Simulator

## Purpose

Build a TypeScript application for rules-correct, deterministic Sorcery: Contested Realm games, reproducible deck and model evaluation, collection-constrained optimization, and later browser play over the same authoritative engine.

**Core value:** Simulation results must be reproducible and rules-correct enough that deck and model comparisons are trustworthy.

## Current milestone

- Phase 1 authority/data infrastructure is implemented and its review findings are resolved or explicitly deferred; see [the Phase 1 review](./phases/01-rules-and-data-authority/01-REVIEW.md) and [retrospective](./phases/01-rules-and-data-authority/01-SUMMARY.md).
- Phase 2, the deterministic engine contract, is in progress with a frozen JSON state and versioned seeded PRNG kernel. Locked decisions are in [Phase 2 context](./phases/02-deterministic-engine-contract/02-CONTEXT.md).
- [Requirements](./REQUIREMENTS.md) are the canonical product contract; [ROADMAP.md](./ROADMAP.md) owns phase order and status.

## Non-negotiable constraints

- TypeScript is used across the engine, simulator, agents, optimizer, and browser client.
- Official rules and card rulings are authoritative; unresolved behavior fails closed.
- The engine alone owns state and enumerates legal actions.
- A manifest plus seed reproduces deterministic-agent events byte-for-byte.
- Ranked results require verified coverage for every exercised mechanic.
- External source may be copied only when its license and obligations are accepted.
- Containers, if needed, use Podman.
- Changes leave the smallest meaningful automated check and one coherent verified commit.

## Private authority boundary

Official source bytes, locks, normalized snapshots, and built revisions remain ignored under `.local/authority/` and are never committed, packaged, shared, hosted, uploaded, or redistributed. No official artwork is acquired. Broader acquisition, recurring updates, sharing, hosting, public APIs, artwork, or commercial use requires written publisher permission and a separate plan. See [the external reuse policy](../docs/external-reuse-policy.md).

## Out of scope for v1

Interactive human CLI play, authoritative LLM rulings, marketplace/pricing features, native mobile clients, public matchmaking, and distributed simulation infrastructure.
