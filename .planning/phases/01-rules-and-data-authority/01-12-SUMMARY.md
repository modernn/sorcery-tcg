---
phase: 01-rules-and-data-authority
plan: "12"
subsystem: authority-boundary
tags: [typescript, node, privacy, git-history, packaging, canonical-json]

requires:
  - phase: 01-rules-and-data-authority
    provides: private source-set verification, immutable authority revisions, and the selected offline revision from Plans 01-08 through 01-11
provides:
  - one bounded production scanner for every protected source offset, semantic derivative, decoded string literal, and publication surface
  - case-aware private locator detection across native, normalized, JSON-escaped, and TypeScript-escaped forms
  - policy aligned with normalized official derivatives existing only in the ignored private offline revision
affects: [authority-release-gate, private-source-verification, phase-01-verification, offline-runtime]

tech-stack:
  added: []
  patterns:
    - one production candidate-inspection pipeline shared by history, worktree, index, and package enumeration
    - bounded non-evaluating string decoding with decoded bytes returned to the ordinary inspection pipeline
    - exact hashed public-provenance exemptions with negative extension tests

key-files:
  created:
    - .planning/phases/01-rules-and-data-authority/01-12-SUMMARY.md
  modified:
    - scripts/verify-private-authority-boundary.ts
    - tests/authority/private-authority-boundary.test.ts
    - tests/private-authority/repository-boundary.test.ts
    - docs/external-reuse-policy.md

key-decisions:
  - "Windows-shaped private locators use ASCII case-insensitive matching on every platform; POSIX locators remain exact and case-sensitive."
  - "Only exact audited public provenance literals are exempted from raw fingerprints; one-byte extensions and private prose remain blocking."
  - "Normalized official derivatives may exist only in the ignored private offline revision; raw publisher sources and all external publication remain prohibited."

patterns-established:
  - "Every safely decoded JSON or TypeScript string literal re-enters the same raw, normalized, semantic, and locator inspection path."
  - "Private gate diagnostics identify category, surface, and candidate path without printing protected bytes, roots, or locator values."

requirements-completed: [DATA-01, DATA-02, DATA-03]

duration: 13min
completed: 2026-08-27
---

# Phase 1 Plan 12: Comprehensive Private Authority Boundary Summary

**Every-offset and semantic private-content scanning across Git/package surfaces, encoded locator detection, and a policy-consistent ignored private revision**

## Performance

- **Duration:** 13 min continuation after the approved history-rewrite checkpoint
- **Started:** 2026-08-27T18:01:08Z
- **Completed:** 2026-08-27T18:14:22Z
- **Tasks:** 3
- **Files modified:** 5

## Accomplishments

- Replaced sampled leakage checks with bounded every-offset raw fingerprints, normalized visible-text fingerprints, canonical JSON subtree fingerprints, and decoded-literal recursion.
- Centralized reachable-history, worktree, index, and dry-run package enumeration in the production scanner; the real private test now delegates and checks only fixed output/status.
- Blocked native, slash/backslash, JSON-escaped, TypeScript-escaped, and mixed-case Windows private locators without weakening exact POSIX semantics or disclosing matched values.
- Corrected the private-use policy so `cards.normalized.json` may exist only inside the ignored private offline revision while raw source bytes and every external-use path remain prohibited.

## Task Commits

Tasks 1 and 2 used RED/GREEN commits; Task 3 was documentation-only:

1. **Task 1 RED: comprehensive boundary regressions** - `f2db1b3` (test)
2. **Task 1 GREEN: comprehensive private boundary scanner** - `9554130` (feat)
3. **Task 2 RED: locator encoding regressions** - `f4e9099` (test)
4. **Task 2 GREEN: encoded and case-aware locator detection** - `18acfe6` (feat)
5. **Task 3: ignored private derivative policy** - `9a644b7` (docs)

## Files Created/Modified

- `scripts/verify-private-authority-boundary.ts` - Owns bounded fingerprint construction, decoded-literal inspection, locator matching, and all four publication-surface enumerators.
- `tests/authority/private-authority-boundary.test.ts` - Proves arbitrary offsets, semantic/card derivatives, encodings, surfaces, case behavior, safe provenance, and content-free diagnostics.
- `tests/private-authority/repository-boundary.test.ts` - Delegates the real private repository gate to the production scanner and asserts only fixed output/status.
- `docs/external-reuse-policy.md` - Permits normalized official derivatives only inside the ignored private offline revision while retaining all external-use restrictions.
- `.planning/phases/01-rules-and-data-authority/01-12-SUMMARY.md` - Records Plan 01-12 execution, checkpoint resolution, and verification evidence.

## Decisions Made

- Used two rolling 32-byte fingerprints plus exact-window confirmation rather than retaining any sampled-offset strategy.
- Kept locator case folding ASCII-only and limited to Windows drive/UNC shapes; POSIX forms are never broadly folded.
- Kept the implemented private revision format and corrected policy wording instead of changing the importer or revision schema.
- Accepted a user-authorized local reachable-history rewrite only after a mirror rehearsal; no remote existed and no push or remote mutation occurred.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] Prevented safe public provenance and common-subtree false positives**
- **Found during:** Task 2 real repository gate after comprehensive Task 1 scanning
- **Issue:** Exact audited publisher titles/URLs in committed provenance and a common small elemental-threshold object overlapped protected-source fingerprints, causing safe public metadata to fail the same gate.
- **Fix:** Exempted only exact hash-bound public provenance literals, retained one-byte-extension negatives, and required 96 canonical bytes before a JSON subtree becomes a protected semantic record.
- **Files modified:** `scripts/verify-private-authority-boundary.ts`, `tests/authority/private-authority-boundary.test.ts`
- **Verification:** Exact public literals and the common fragment pass; one-byte extensions, reordered protected cards, and private prose fail; the real private gate passes.
- **Committed in:** `18acfe6`

---

**Total deviations:** 1 auto-fixed (1 Rule 1 bug)
**Impact on plan:** The fix removes false positives without sampling, broad allowlisting, private-negative weakening, dependency additions, or scope expansion.

## Issues Encountered

- The first real gate found an exact private locator in reachable historical planning blobs even though HEAD was redacted. Execution paused for an explicit decision. After the user authorized a local-only rewrite, the parent executor rehearsed it in a mirror, rewrote reachable local history, removed rewrite backup refs, verified connectivity, and confirmed the protected locator forms were absent before this continuation resumed.
- The Windows sandbox setup-refresh helper rejected normal patch calls while editing Task 3. The approved escalated Codex apply-patch mode applied the same minimal patch; no non-plan file was edited.

## Authentication Gates

None.

## TDD Gate Compliance

- Task 1 RED `f2db1b3` preceded GREEN `9554130` and captured failures for unsampled offsets, normalized/semantic derivatives, encoded content, and publication surfaces.
- Task 2 RED `f4e9099` preceded GREEN `18acfe6` and captured JSON/TypeScript/slash-normalized Windows locator and POSIX case-semantics failures.

## Verification

- Task 1 focused boundary suite: 15 passed, 0 failed; TypeScript typecheck passed.
- Task 2 focused locator/escaped/case-varied suite: 33 passed, 0 failed.
- Real delegated repository boundary gate after the approved history rewrite: 1 passed in 58.57 seconds.
- Exact Task 3 policy assertion: passed.
- `pnpm verify`: TypeScript typecheck and ESLint passed; 223 tests passed with 0 failures, skips, or todos.
- Final real delegated repository boundary gate: 1 passed in 62.35 seconds with only the fixed success line.

## Known Stubs

None. Empty arrays/strings reported by the mechanical scan are bounded local accumulators in the scanner/tests, not UI or runtime placeholder data.

## User Setup Required

None - no external service configuration, collection, upload, push, or remote change was performed.

## Next Phase Readiness

- CR-06 and WR-03 are closed with synthetic and real private-gate evidence.
- Plan 01-12 is ready for the remaining Phase 1 gap plans; no phase transition was performed.
- Broader acquisition, artwork, sharing, hosting, redistribution, public/network APIs, and commercialization remain blocked pending written publisher permission and a separate approved plan.

---
*Phase: 01-rules-and-data-authority*
*Completed: 2026-08-27*

## Self-Check: PASSED

- All five key files exist.
- All five RED/GREEN/task commits exist in reachable Git history.
- Task acceptance criteria and plan-level public/private verification commands passed against the final implementation.
- No unplanned network endpoint, authentication path, schema trust boundary, or other threat surface was introduced beyond the plan threat register.
