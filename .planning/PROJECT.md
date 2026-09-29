# Sorcery Simulator

## Purpose

Build a Rust application for rules-correct, deterministic Sorcery: Contested Realm games, reproducible deck evaluation, checkpoint search, and browser play over the same authoritative engine. TypeScript remains a thin ingestion/server/UI boundary.

**Core value:** Simulation results must be reproducible and rules-correct enough that deck and model comparisons are trustworthy.

## Active direction and historical plans

Current instructions live in [AGENTS.md](../AGENTS.md), commands in
[README.md](../README.md), and implementation/validation order in the
[shared-rule batch loop](../docs/shared-rule-behaviors.md#binding-and-validation-batch-loop).
The goal is complete released-card behavior through Codex-derived shared Rust rules,
with source-guarded contextual dependencies and native per-card proof before admission.

The phase requirements, roadmap, and summaries preserve earlier planning and evidence;
their checkboxes and phase order do not describe current completion or own the work
queue. Current coverage and blockers come from the refreshed private catalog/workbook.

## Non-negotiable constraints

- Rust owns the authoritative engine, simulator, deterministic agents, search, checkpoints, and replay.
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
