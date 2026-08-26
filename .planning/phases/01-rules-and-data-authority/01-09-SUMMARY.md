---
phase: 01-rules-and-data-authority
plan: "09"
subsystem: authority-data
tags: [typescript, zod, card-normalization, stable-identity, provenance]
requires:
  - phase: 01-08
    provides: strict official API adaptation, deterministic card normalization, and immutable revision provenance
provides:
  - required lossless avatar life and four-element threshold contracts
  - exact official gameplay-field adaptation and deeply frozen normalization
  - revision-independent official card IDs with revision-specific SourceRefs and content hashes
affects: [phase-02, deck-import, collection-identity, simulation-manifests, authority-refresh]
tech-stack:
  added: []
  patterns:
    - strict required gameplay fields shared by raw and normalized card schemas
    - publisher-scoped logical identity separated from revision-scoped provenance
key-files:
  created: []
  modified:
    - src/authority/schemas.ts
    - src/authority/official-card-api-adapter.ts
    - src/authority/normalize-cards.ts
    - tests/authority/card-snapshot.test.ts
    - tests/authority/provenance.test.ts
    - tests/authority/fixtures/cards-valid.json
    - tests/authority/fixtures/cards-duplicates.json
    - tests/authority/fixtures/cards-malformed.json
    - tests/authority/fixtures/bundle-input/cards.raw.json
    - tests/authority/fixtures/bundle-input/sources.json
    - tests/authority/fixtures/bundle-input/input-lock.json
key-decisions:
  - "Require nullable nonnegative safe-integer life and an exact strict air/earth/fire/water threshold object without defaults, coercion, or repair."
  - "Hash official card identity from the fixed sorcerytcg publisher namespace and publisher card ID while retaining revision-qualified source identity only in provenance."
  - "Keep non-official card identity source-scoped instead of promoting synthetic or community records into the official namespace."
patterns-established:
  - "Lossless card boundary: validate complete gameplay values, defensively copy them at each transformation, then deep-freeze the canonical artifact."
  - "Identity separation: publisher identity is durable across refreshes; SourceRefs and artifact hashes remain bound to exact revision evidence."
requirements-completed: [DATA-02, DATA-03]
duration: 37 min
completed: 2026-08-26
---

# Phase 1 Plan 9: Lossless Card Data and Stable Identity Summary

**Official cards now retain avatar life and every elemental threshold exactly, while publisher-scoped card IDs survive authority revisions without weakening immutable provenance.**

## Performance

- **Duration:** 37 min
- **Started:** 2026-08-26T20:39:09Z
- **Completed:** 2026-08-26T21:16:17Z
- **Tasks:** 3
- **Files modified:** 12 including this summary

## Accomplishments

- Added required strict `life` and exact four-key `thresholds` fields to raw and normalized cards, with deterministic exact-path rejection for missing, negative, fractional, unsafe-integer, and unknown values.
- Preserved nullable/nonzero life and all threshold integers through the official API adapter and normalization, using defensive copies that are frozen with the canonical artifact.
- Stabilized official card IDs across revision-qualified source IDs using the fixed `sorcerytcg` namespace while preserving distinct SourceRefs and artifact content hashes.
- Migrated all synthetic card fixtures and rebuilt the bundle fixture's exact byte hashes and canonical input root with project hashing helpers.

## Task Commits

Each TDD task retained its RED contract and GREEN implementation:

1. **Task 1: Define required life and threshold contracts**
   - `4fba542` - RED gameplay schema regressions and complete synthetic fixtures
   - `20a0f08` - GREEN strict schemas plus constructor wiring required by typechecking
2. **Task 2: Preserve gameplay fields through adaptation and normalization**
   - `4b0de58` - RED official adapter, end-to-end, freeze, and schema-export regressions
   - `20a0f08` - GREEN shared adapter/normalizer implementation (shared with Task 1's coupled typecheck fix)
3. **Task 3: Stabilize official card IDs across authority revisions**
   - `ac12844` - RED cross-revision identity and non-official namespace regressions
   - `70e4bc4` - GREEN fixed publisher namespace and rebuilt byte locks

## Files Created/Modified

- `src/authority/schemas.ts` - Required lossless life and strict four-element threshold types and validators.
- `src/authority/official-card-api-adapter.ts` - Exact defensive mapping from audited guardian gameplay values.
- `src/authority/normalize-cards.ts` - Defensive gameplay copies and publisher-scoped official card identity.
- `tests/authority/card-snapshot.test.ts` - Schema, losslessness, freeze, identity, provenance-separation, and fixture-lock regressions.
- `tests/authority/provenance.test.ts` - Complete raw and normalized schema export examples.
- `tests/authority/fixtures/cards-valid.json` - Explicit valid nullable/nonzero life and threshold combinations.
- `tests/authority/fixtures/cards-duplicates.json` - Complete duplicate-case records retaining their intended diagnostics.
- `tests/authority/fixtures/cards-malformed.json` - Complete malformed records whose original defects remain isolated.
- `tests/authority/fixtures/bundle-input/cards.raw.json` - Complete byte-pinned synthetic card input.
- `tests/authority/fixtures/bundle-input/sources.json` - Updated exact card-source byte binding.
- `tests/authority/fixtures/bundle-input/input-lock.json` - Recomputed per-file hashes and canonical input root.

## Decisions Made

- Official logical identity uses only `{ namespace: "sorcerytcg", sourceCardId }`; display fields, ordering, and revision-qualified source IDs do not participate.
- Non-official logical identity continues to use its source ID as the namespace, preserving the prior synthetic/community isolation contract.
- Life and thresholds are required rather than optional or repaired; zero and null values are preserved exactly where allowed.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] Resolved the strict-schema constructor dependency before Task 1 typecheck**
- **Found during:** Task 1
- **Issue:** Making the card types required correctly caused TypeScript to reject the still-lossy adapter and normalizer constructors, while the plan scheduled that wiring in Task 2.
- **Fix:** Wrote and committed Task 2's failing losslessness regression first, then completed the adapter and normalizer copies in the shared GREEN commit so Task 1's mandated typecheck passed without casts or optional compatibility fields.
- **Files modified:** `src/authority/schemas.ts`, `src/authority/official-card-api-adapter.ts`, `src/authority/normalize-cards.ts`, and focused tests.
- **Verification:** Task 1 and Task 2 targeted suites plus `pnpm typecheck` passed.
- **Committed in:** `20a0f08`

**2. [Rule 1 - Correctness] Rebuilt locks from the exact LF bytes Git commits**
- **Found during:** Task 3
- **Issue:** Initial worktree hashes reflected CRLF checkout bytes while the byte-locked fixture attributes require LF, which would have made committed bytes disagree with the lock after checkout.
- **Fix:** Normalized the two changed byte-pinned inputs to LF, recomputed hashes from both worktree and staged/index bytes with project helpers, and verified they matched before recalculating the canonical root.
- **Files modified:** `tests/authority/fixtures/bundle-input/cards.raw.json`, `tests/authority/fixtures/bundle-input/sources.json`, `tests/authority/fixtures/bundle-input/input-lock.json`.
- **Verification:** Worktree/index hashes matched exactly; targeted lock tests and all bundle tests passed.
- **Committed in:** `70e4bc4`

---

**Total deviations:** 2 auto-fixed (1 blocking dependency, 1 correctness issue)
**Impact on plan:** Both fixes were required to keep the strict schema buildable and the committed fixture byte lock reproducible; no dependency, network, storage, UI, or database scope was added.

## Issues Encountered

- The new publisher namespace changed the stable-hash sort order, so the existing deterministic-order expectation was updated to the new stable-ID order.
- The default sandbox process setup failed in the shared workspace; approved escalated repository commands were used while all edits still went through the Codex apply-patch runner.
- Parallel Phase 1 gap work temporarily left unrelated boundary tests mid-edit. The final stable default suite passed after those changes reached public-green state.

## Authentication Gates

None.

## Known Stubs

None.

## User Setup Required

None - no external service configuration or package installation was required.

## Verification Results

- Task 1 targeted schema regression and `pnpm typecheck` - PASS.
- Task 2 official API/life/threshold/deep-freeze regressions, schema export regression, and `pnpm typecheck` - PASS.
- Task 3 cross-revision/stable-ID/input-lock regressions - PASS (5/5).
- Complete card and bundle suites - PASS (36/36).
- `pnpm verify` - PASS: typecheck, lint, and 188/188 default tests.
- Stub scan - PASS; no TODO, FIXME, placeholder, coming-soon, or unavailable markers in plan-owned files.
- Threat-surface scan - PASS; changes remain within the plan's schema, adapter, normalization, and identity trust boundaries.

## Next Phase Readiness

- Phase 2 and later deck, collection, replay, and simulation consumers can rely on complete card gameplay values and refresh-stable official IDs.
- Revision provenance remains immutable and content-bound; no blocker remains from CR-07 or CR-08.

## Self-Check: PASSED

- All 11 implementation, test, and fixture files modified by the plan exist.
- RED/GREEN commits `4fba542`, `4b0de58`, `20a0f08`, `ac12844`, and `70e4bc4` exist in Git history.
- Targeted acceptance gates, complete plan-owned suites, typecheck, lint, and all 188 default tests passed after the final production commit.

---
*Phase: 01-rules-and-data-authority*
*Completed: 2026-08-26*
