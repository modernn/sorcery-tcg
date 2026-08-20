# Phase 1: Rules and Data Authority - Context

**Gathered:** 2026-08-20
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
- **D-08:** Store derived normalized data and source manifests in the repository. Do not bundle copyrighted rulebook PDFs or card images unless their redistribution terms clearly allow it; keep retrieval instructions and verified hashes instead.
- **D-09:** Contested Realms (GPL-3.0) and spells.bar/the playtest project (no reusable license found in the audited revision) are behavioral and UX references only. Do not copy their source, card implementations, assets, or data into this project.
- **D-10:** Phase 1 uses TypeScript and Node standard-library facilities first. Add a dependency only where runtime schema validation or deterministic normalization is materially safer than a small local implementation.

### the agent's Discretion
- Exact directory names, JSON field ordering, command names, and test file layout, provided the offline validation and immutable provenance requirements remain obvious.
- Whether raw official API responses can be committed after the source/licensing audit; otherwise commit only the normalized snapshot, manifest, and reproducible retrieval tooling.

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
- Model competitors belong to Phase 7; browser human play belongs to Phase 9.

</deferred>

---

*Phase: 1-Rules and Data Authority*
*Context gathered: 2026-08-20*
