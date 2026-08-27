# Phase 1: Rules and Data Authority - Context

**Gathered:** 2026-08-20
**Amended:** 2026-08-27 — Option 3 replaces new project acquisition with user-provided manual local files; LLM skills and original art are assigned to later phases
**Status:** Ready for planning

<domain>
## Phase Boundary

Establish the offline, immutable authority bundle and normalized card snapshot that identify the exact rules, format, card data, and provenance used by every later simulation. This phase defines identities, schemas, validation, source precedence, and clean-room reuse boundaries; it does not implement game rules, card behavior, deck importing, simulation, or UI.

</domain>

<decisions>
## Implementation Decisions

### Official authority and precedence
- **D-01:** Official Sorcery rulebooks, official format rules, the official Codex/FAQ, official card updates, and official card data are the only normative authorities. Community tools and deck sites may supply examples or provenance, never rules.
- **D-02:** The bundle must document an explicit precedence order and effective date. A later official clarification or erratum overrides older text; unresolved conflicts and ambiguous rulings are recorded as unsupported instead of guessed.
- **D-03:** Games and experiments never read mutable live authority data. They select one validated local bundle by ID and content hash.

### Snapshot and artifact identity
- **D-04:** Authority updates create a new immutable revision; existing revisions are never edited in place. Each revision records source URL, retrieval timestamp, effective date when available, media type, SHA-256, and derivation metadata.
- **D-05:** Normalized cards retain an official source identifier when available plus a stable project ID. Normalization is deterministic, schema-versioned, and reproducible from the pinned raw input.
- **D-06:** Rules, cards, formats, decks, collections, behaviors, and experiments use the same minimal provenance envelope: artifact kind, stable ID, schema version, content hash, and parent/source references.
- **D-07:** Canonical JSON uses one project-owned deterministic serialization routine before SHA-256 hashing. Validation reports exact paths and never silently repairs or drops malformed data.

### Distribution and reuse boundary
- **D-08 (amended 2026-08-27, manual provision):** The three completed official-2026-08-20 acquisition pairs and their locks/authorization records are immutable historical evidence only; none authorizes new transport. For official-2026-08-27-v3, the user manually downloads and saves exactly the fixed seven official non-artwork files into the ignored local inbox. Project code, agents, tests, schedulers, and tools do not acquire those sources over the network. Offline intake validates exact paths and complete content, copies them to a fresh repository-local primary and independent outside-repository backup, and writes the lock last with acquisitionMethod user-provided-manual-download and workflow-only authorizationReference phase-01-20260827-manual-provision-1. This reference grants no transport, publisher permission, or legal clearance. Raw sources, locks, normalized snapshots, and built revisions remain Git-ignored/private and are never committed, packaged, shared, hosted, redistributed, or uploaded. There is no artwork acquisition; Phase 9 may use only original/project-owned presentation art or user-supplied private local images.
- **D-09:** Contested Realms (GPL-3.0) and spells.bar/the playtest project (no reusable license found in the audited revision) are behavioral and UX references only. Do not copy their source, card implementations, assets, or data into this project.
- **D-10:** Phase 1 uses TypeScript and Node standard-library facilities first. Add a dependency only where runtime schema validation or deterministic normalization is materially safer than a small local implementation.
- **D-11 (amended 2026-08-27):** The operating scope remains private, local, and noncommercial. New project/agent acquisition, recurring updates, sharing, release, hosting, third-party upload, redistribution, public/network APIs, official artwork, and commercialization remain blocked pending written publisher permission and a separate plan. Later consumers query only the selected validated local JSON revision through a TypeScript module.

### the agent's Discretion
- Exact directory names, JSON field ordering, command names, and test file layout, provided the offline validation and immutable provenance requirements remain obvious.
- Exact safe committed receipt fields and non-sensitive backup label/fingerprint; absolute private source/backup locators stay only in the git-ignored source-set lock.

</decisions>

<canonical_refs>
## Canonical References

**Downstream agents MUST read these before planning or implementing.**

### Product and requirements
- `.planning/PROJECT.md` — Product boundary, TypeScript constraint, trust standard, and exclusions.
- `.planning/REQUIREMENTS.md` — DATA-01 through DATA-03 and downstream artifact expectations.
- `.planning/ROADMAP.md` § Phase 1 — Goal, success criteria, and required research topics.

### Prior research
- `.planning/research/SUMMARY.md` — Consolidated stack, architecture, feature, and risk recommendations.
- `.planning/research/STACK.md` — Minimal TypeScript/Node stack and dependency guidance.
- `.planning/research/ARCHITECTURE.md` — Authority bundle, canonical identity, engine boundary, and clean-room conclusions.
- `.planning/research/PITFALLS.md` — Trust, provenance, licensing, and reproducibility failure modes.

</canonical_refs>

<code_context>
## Existing Code Insights

### Reusable Assets
- No production code exists yet. The planning artifacts define the contracts to implement.

### Established Patterns
- Keep the initial project as one small pnpm TypeScript package.
- Prefer Node standard-library hashing, files, and tests; keep game/runtime consumers offline.
- Fail closed at validation and coverage boundaries.

### Integration Points
- Phase 2 will consume the authority bundle ID, normalized card snapshot, schemas, and canonical hashing routine.
- Later deck, collection, behavior, replay, and experiment artifacts must reuse the Phase 1 provenance envelope.

</code_context>

<specifics>
## Specific Ideas

- The simulator must ultimately be strong enough to play games, evaluate new AI models, compare attributable common online decks, and optimize the user's owned collection.
- Human play will be browser-based; there is no interactive human CLI requirement.

</specifics>

<deferred>
## Deferred Ideas

- Owned collection import and common online deck acquisition belong to Phase 6.
- Game-rule execution belongs to Phases 3 and 4.
- The versioned LLM player skill and cited read-only rules-adviser skill belong to Phase 7; adviser consultation is unranked and cannot mutate engine state. Browser human play and original/project-owned or user-supplied private local presentation art belong to Phase 9.

</deferred>

---

*Phase: 1-Rules and Data Authority*
*Context gathered: 2026-08-20*
