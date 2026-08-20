---
phase: 01-rules-and-data-authority
plan: "03"
subsystem: authority-data
tags: [card-normalization, canonical-json, sha-256, zod, offline-determinism]

requires:
  - phase: 01-rules-and-data-authority
    provides: Canonical JSON, SHA-256 helpers, strict authority schemas, and provenance envelopes from Plan 02
provides:
  - Pure local-byte card normalization with deterministic project IDs
  - Lossless cardinality and exact-path failure behavior for malformed and duplicate records
  - Offline, provenance-bound, deeply frozen canonical card snapshot artifacts
affects: [01-04, 01-05, authority-data, deck-import, simulation-manifests]

tech-stack:
  added: []
  patterns:
    - Stable project IDs derived only from source identity fields
    - Whole-snapshot strict validation before and after normalization
    - Separate raw-byte, normalized-payload, and provenance-bound artifact identities

key-files:
  created:
    - src/authority/normalize-cards.ts
    - tests/authority/fixtures/cards-valid.json
    - tests/authority/fixtures/cards-malformed.json
    - tests/authority/fixtures/cards-duplicates.json
  modified:
    - src/authority/schemas.ts
    - tests/authority/card-snapshot.test.ts

key-decisions:
  - "Mint card stable IDs from source ID plus source card ID, never array position, display fields, or filesystem names."
  - "Keep the full artifact hash provenance-bound to the exact raw source byte hash while separately proving property-order-independent normalized payload identity."
  - "Validate cross-card stable-ID and printing-slug uniqueness in shared snapshot schemas instead of duplicating trust-boundary logic."

patterns-established:
  - "Normalizer boundary: bytes and strict SourceMetadata in; one canonical card-snapshot artifact out; no I/O or ambient inputs."
  - "Identity separation: raw source hash tracks exact bytes, payload hash tracks normalized semantics, artifact hash binds semantics to provenance."
  - "Mutation safety: validate and defensively copy, then recursively freeze the returned canonical artifact."

requirements-completed: [DATA-02, DATA-03]

duration: 21min
completed: 2026-08-20
---

# Phase 1 Plan 3: Deterministic Card Snapshot Normalization Summary

**Strict local-byte card normalization with stable IDs, exact failure diagnostics, provenance-bound hashes, and deeply frozen offline artifacts**

## Performance

- **Duration:** 21 min
- **Started:** 2026-08-20T15:51:17Z
- **Completed:** 2026-08-20T16:12:22Z
- **Tasks:** 2
- **Files modified:** 6

## Accomplishments

- Implemented normalizeCards as a pure transformation over bounded duplicate-aware JSON bytes and explicit strict source metadata.
- Preserved official source IDs and printing slugs, minted stable project IDs from source identity fields, sorted by stable ID using code-unit comparison, and enforced exact one-to-one cardinality.
- Rejected unknown or missing fields, invalid dates/hashes, raw-byte mismatches, duplicate JSON keys, duplicate derived IDs, and cross-card printing-slug collisions with deterministic JSON-Pointer diagnostics.
- Proved repeatability, property-order-independent normalized payload identity, raw-versus-semantic hash separation, offline operation with network/clock/random sentinels, and deep mutation safety.
- Added only compact independently authored synthetic fixtures; no publisher corpus, protected assets, GPL material, or unlicensed playtest content was introduced.

## Task Commits

Each TDD task was committed as a RED contract followed by its GREEN implementation:

1. **Task 1: Normalize strict card records without loss**
   - 21cbaca — RED strict normalization contracts and synthetic fixtures
   - 84825f2 — GREEN lossless normalizer and snapshot validators
2. **Task 2: Prove deterministic, offline snapshot identity**
   - 0411511 — RED determinism, identity-separation, offline, and mutation contracts
   - 0239e05 — GREEN deep-freeze return boundary

## Files Created/Modified

- src/authority/normalize-cards.ts - Pure byte-to-canonical-card-snapshot transformation with stable ordering and defensive freezing.
- src/authority/schemas.ts - Shared strict raw/normalized snapshot validators and cross-card printing-slug uniqueness.
- tests/authority/card-snapshot.test.ts - Eleven active DATA-02 assertions plus the retained Plan 01-05 input-root contract.
- tests/authority/fixtures/cards-valid.json - Two-card synthetic valid input.
- tests/authority/fixtures/cards-malformed.json - Synthetic unknown-field and missing-field failures.
- tests/authority/fixtures/cards-duplicates.json - Synthetic derived-ID and printing-slug collision cases.

## Decisions Made

- Stable card identity is the SHA-256 of canonical source ID and source card ID fields with a card prefix; name, rules text, array position, property order, and local path do not participate.
- Normalized cards are ordered by stable ID using direct code-unit comparison rather than locale-sensitive APIs.
- Exact raw-byte formatting changes the source hash and therefore the full provenance-bound artifact hash, while normalized payload bytes and payload hash remain identical. This preserves Plan 01-02's D-06 SourceRef contract.
- The normalizer retains only a SourceRef in the artifact; full reviewed source metadata remains owned by the source manifest rather than duplicated into every card snapshot.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 2 - Missing Critical] Added whole-snapshot validators to the shared schema boundary**
- **Found during:** Task 1 (Normalize strict card records without loss)
- **Issue:** Per-card validators could not report whole-payload paths or enforce cross-card printing-slug uniqueness without duplicating diagnostic logic inside the normalizer.
- **Fix:** Added shared raw and normalized snapshot validators plus one cross-card slug refinement.
- **Files modified:** src/authority/schemas.ts
- **Verification:** Malformed and duplicate fixture assertions pass with exact sorted paths; full typecheck and lint pass.
- **Committed in:** 84825f2

**2. [Rule 1 - Correctness] Preserved provenance-bound artifact identity for reordered raw bytes**
- **Found during:** Task 2 (Prove deterministic, offline snapshot identity)
- **Issue:** Requiring a full artifact hash to remain unchanged when raw JSON property order changes would contradict the required SourceRef binding to the exact raw byte hash.
- **Fix:** Proved reordered inputs produce identical normalized canonical payload bytes, payload hashes, stable IDs, and ordering; separately proved exact raw hashes and the full provenance-bound artifact hash change with raw bytes.
- **Files modified:** tests/authority/card-snapshot.test.ts
- **Verification:** Repeat, reorder, and hash-separation assertions pass.
- **Committed in:** 0411511

---

**Total deviations:** 2 auto-fixed (1 missing critical, 1 correctness)
**Impact on plan:** Both changes preserve the locked strict-validation and provenance contracts without adding acquisition, persistence, rules interpretation, or runtime scope.

## Issues Encountered

- The Windows sandbox helper failed for the mandated patch editor before writing. Exact unified diffs were applied through git apply, then checked with git diff --check.
- A first manually counted multi-file fixture diff was rejected as corrupt before applying. The replacement diff generated exact hunk counts and applied cleanly.
- The initial RED test had two truncated closing lines from an incorrect manual hunk count; it was corrected before the accepted RED run, which then failed for the intended missing implementation.

## Authentication Gates

None.

## Known Stubs

- tests/authority/card-snapshot.test.ts retains one intentional test.todo for changed input-lock roots. Plan 01-05 owns independent input-lock/root verification before normalization, so this future contract was left untouched as required.

This stub does not prevent Plan 01-03 from meeting its pure normalization and deterministic snapshot goal.

## User Setup Required

None - no external service configuration or acquisition permission was needed for synthetic local fixtures.

## Verification Results

- Task 1 targeted valid/malformed/unknown/duplicate/count/path suite - PASS (6 assertions)
- node --test tests/authority/card-snapshot.test.ts - PASS (11 active assertions, 1 future Plan 01-05 todo)
- pnpm typecheck - PASS
- pnpm lint - PASS
- pnpm test - PASS (61 discovered contracts, 37 active assertions, 24 future-plan todos, 0 failures)
- Purity scan - PASS; normalize-cards.ts has no network, clock, locale, randomness, filesystem, directory, or environment dependency.
- Fixture clean-room scan - PASS; fixtures are synthetic/minimal and contain no restricted external material.

## Next Phase Readiness

- Plan 01-04 can consume strict source/artifact contracts without changes to card normalization.
- Plan 01-05 can use normalizeCards after verifying its independent input lock/root and can replace the retained future todo.
- No blockers remain for the next authority plan.

## Self-Check: PASSED

- All 6 created/modified implementation, schema, test, and fixture files exist.
- Task commits 21cbaca, 84825f2, 0411511, and 0239e05 exist in git history.
- The targeted and full verification commands passed after the final task commit.

---
*Phase: 01-rules-and-data-authority*
*Completed: 2026-08-20*

