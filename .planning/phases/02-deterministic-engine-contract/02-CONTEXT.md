# Phase 2: Deterministic Engine Contract - Context

**Gathered:** 2026-08-26
**Status:** Ready for planning

<domain>
## Phase Boundary

Define the authoritative, deterministic game-state contract shared by every future client and competitor: JSON-compatible state, versioned seeded randomness, seat-scoped observations, canonical legal actions, mutation-safe stepping, semantic events, hashes, and exact replay receipts. This phase establishes the universal engine boundary but does not implement Sorcery setup, turns, spatial rules, combat, card effects, simulation scheduling, model providers, or browser UI.

</domain>

<decisions>
## Implementation Decisions

### Hidden-information views
- **D-01:** Opponent-hidden cards use reveal-safe placeholders that preserve only legally public counts, ordering, and revealed structure. Placeholders must not enable identity tracking through hidden-zone randomization.
- **D-02:** Each seat receives one self-contained observation containing all public state, that seat's complete private state, and redacted opponent state.
- **D-03:** A previously revealed card remains identified only while official rules and publicly observable movement keep it trackable. Randomization or loss of legal trackability erases that observer knowledge.
- **D-04:** Authoritative full state is available only through a separately named privileged replay/verification interface. Competitors and ordinary clients can never request an omniscient player observation.

### Legal-action shape
- **D-05:** Complex choices use staged engine-owned decision states. Every action offered at the current decision point is fully bound and immediately executable; clients never submit arbitrary parameters.
- **D-06:** Each action has a deterministic opaque `actionId` bound to the exact `stateVersion`, plus a typed structured descriptor clients can render or inspect.
- **D-07:** The engine defines one canonical legal-action order shared by every client and competitor. A UI may group actions for presentation without changing their identities or canonical sequence.
- **D-08:** Pass, decline, and similar choices are explicit engine-issued actions. An empty legal-action list means the state is terminal, explicitly unsupported, or defective; clients never infer an action.

### Stale and illegal rejection
- **D-09:** Expected invalid submissions return a typed rejection result with a stable reason code, observer-safe message, current `stateVersion`, and unchanged authoritative state hash. Thrown exceptions are reserved for engine defects.
- **D-10:** Every submission not bound to the exact current version, including a duplicate of an already accepted action, is rejected without mutation, cached success, reinterpretation, or automatic retry.
- **D-11:** Rejected attempts are recorded in a separate deterministic attempt/audit log. They do not enter the semantic game-event stream or the accepted-action replay transcript.
- **D-12:** Rejection codes are specific and stable, including cases such as `stale_version`, `unknown_action`, `wrong_seat`, and `terminal_state`, while messages and metadata must not reveal hidden information.

### Events and randomness
- **D-13:** Events are ordered, typed semantic game outcomes with explicit causal relationships. They are not raw state diffs, UI narration, or arbitrary mutation patches.
- **D-14:** Every accepted action returns a canonical receipt containing its sequence number and identity, pre/post authoritative state hashes, ordered events, and resulting `stateVersion`.
- **D-15:** The PRNG algorithm is versioned and its serializable state is authoritative. The privileged transcript records each deterministic draw's purpose, ordinal, domain, result, and pre/post PRNG-state hashes; seat-scoped views redact hidden outcomes.
- **D-16:** Replay starts from the pinned manifest and initial seed/state, re-executes accepted action IDs, and compares canonical receipts byte-for-byte. Recorded events are verified outputs, not mutations reapplied as authority.

### the agent's Discretion
- Exact TypeScript type names, module boundaries, opaque-ID encoding, and version-number representation, provided every locked contract above remains explicit and testable.
- Exact versioned PRNG algorithm and derivation scheme, provided it is injectable, serializable, platform-stable, and never uses wall time or global randomness.

</decisions>

<canonical_refs>
## Canonical References

**Downstream agents MUST read these before planning or implementing.**

### Product and phase contract
- `.planning/PROJECT.md` — Core value, authoritative-engine boundary, TypeScript constraint, and fail-closed policy.
- `.planning/REQUIREMENTS.md` — ENG-01 through ENG-06 plus TEST-02 and TEST-03 acceptance requirements.
- `.planning/ROADMAP.md` § Phase 2 — Phase goal, dependency, and success criteria.

### Upstream authority and identity decisions
- `.planning/phases/01-rules-and-data-authority/01-SUMMARY.md` — Canonical identity, provenance, immutability, offline selection, and reuse-boundary decisions consumed by the engine contract.
- `.planning/phases/01-rules-and-data-authority/01-REVIEW.md` — Current open findings at the Phase 1/Phase 2 boundary.
- `.planning/research/SUMMARY.md` — Native TypeScript, hashing, deterministic PRNG, engine boundaries, testing, and later worker guidance.

</canonical_refs>

<code_context>
## Existing Code Insights

### Reusable Assets
- `src/authority/canonical-json.ts` — Bounded canonical JSON serialization, duplicate-key detection, JSON Pointer diagnostics, and rejection of unsafe JavaScript values.
- `src/authority/hash.ts` — Project-standard `sha256:` and canonical `identityHash` helpers.
- `src/authority/schemas.ts` — Strict Zod boundary schemas, readonly artifact types, stable hashes, and structured diagnostics.
- `tests/authority/canonical-json.test.ts` — Existing `node:test` patterns for deterministic identity, malformed inputs, and bounded-work failures.

### Established Patterns
- Trust-boundary inputs are strict, bounded, and fail closed with stable path/code/message diagnostics.
- Canonical identity is derived from JSON-compatible values using one project-owned serializer and Node SHA-256.
- Public contracts use readonly TypeScript data and validate untrusted bytes once at the boundary.
- Tests use `node:test` and `node:assert/strict` with fixed deterministic fixtures.

### Integration Points
- Phase 2 consumes the selected immutable authority bundle and normalized card snapshot from Phase 1.
- Future deterministic, model, simulation, replay, and browser adapters all consume the observation/legal-action/step contract defined here.
- Phase 3 supplies actual Sorcery rules behind this contract without changing client authority.

</code_context>

<specifics>
## Specific Ideas

- The user requested smart-discuss proposals: the agent recommended answers in review tables and the user accepted each area.
- Reproducibility and hidden-information safety take priority over ergonomic shortcuts such as parameterized commands, inferred pass actions, or client-defined ordering.

</specifics>

<deferred>
## Deferred Ideas

### Reviewed Todos (not folded)
- `Authorize card data and catalog updater` — publisher authorization and recurring acquisition are outside the deterministic engine contract; keep pending for the authority/data lifecycle rather than Phase 2.

</deferred>

---

*Phase: 2-Deterministic Engine Contract*
*Context gathered: 2026-08-26*
