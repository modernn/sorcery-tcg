---
phase: 01-rules-and-data-authority
plan: "13"
subsystem: authority-boundary
tags: [typescript, node, privacy, canonical-json, deterministic-builds]

requires:
  - phase: 01-rules-and-data-authority
    provides: fixed verified fresh source roots, the Plan 15 v3 lock, immutable historical evidence, and the production private-boundary scanner
provides:
  - deterministic byte-identical authority candidates built only from the fixed fresh primary and backup roots
  - an ignored write-once official-2026-08-27-v3 private revision with a safe tracked selection receipt
  - complete release gates for default verification, fresh completeness, private authority, and repository boundaries
affects: [authority-runtime, rules-engine, card-data, phase-01-verification]

tech-stack:
  added: []
  patterns:
    - dual-root deterministic candidate construction with before-and-after source immutability checks
    - write-once ignored private revisions selected by content-free tracked receipts
    - current-schema verification for new revisions and direct identity verification for immutable historical revisions

key-files:
  created:
    - data/authority/receipts/official-2026-08-27-v3.json
    - .planning/phases/01-rules-and-data-authority/01-13-SUMMARY.md
  modified:
    - data/authority/README.md
    - tests/private-authority/private-revision.test.ts
    - tests/private-authority/repository-boundary.test.ts

key-decisions:
  - "The selected authority revision is the exact write-once bundle:official-2026-08-27-v3 revision; mutable aliases and stale IDs are rejected."
  - "Fresh primary and backup roots must independently produce byte-identical candidates without network access, source mutation, reacquisition, or historical fallback."
  - "Immutable historical revisions retain their original schema and are verified from their canonical identity rather than revalidated retroactively under the current schema."

patterns-established:
  - "Tracked receipts expose only safe provenance and selection metadata; private roots, locators, source bytes, and private-only fields stay ignored and local."
  - "Every selected private revision must pass the same production scanner used for reachable history, worktree, index, and package boundaries."

requirements-completed: [DATA-01, DATA-02, DATA-03]

duration: 36min
completed: 2026-08-27
---

# Phase 1 Plan 13: Final Authority Revision Summary

**Deterministic dual-root construction, write-once local selection, and complete leakage-gated release of the final private authority revision**

## Performance

- **Duration:** 36 min
- **Started:** 2026-08-28T01:37:17Z
- **Completed:** 2026-08-28T02:13:33Z
- **Tasks:** 3
- **Files modified:** 5 tracked files plus one ignored private revision

## Accomplishments

- Proved that the fixed fresh primary and backup roots independently build byte-identical candidates while network access is blocked and all source/evidence trees remain unchanged.
- Installed the exact `bundle:official-2026-08-27-v3` revision under the ignored write-once private boundary and selected it through a content-free tracked receipt and README contract.
- Rejected mismatched overwrites, stale revision IDs, mutable aliases, private receipt fields, and any fallback to historical or non-fresh roots.
- Passed the complete release sequence: default verification, focused fresh completeness, the full private authority gate, and the narrow production repository-boundary suite.

## Task Commits

All three tasks used RED/GREEN commits:

1. **Task 1 RED: final candidate build contract** - `f9124b8` (test)
2. **Task 1 GREEN: deterministic dual-root candidates** - `25bbe38` (feat)
3. **Task 2 RED: final revision selection contract** - `aa1fa99` (test)
4. **Task 2 GREEN: immutable v3 selection and safe receipt** - `7827109` (feat)
5. **Task 3 RED: final repository-boundary contract** - `0d5cf35` (test)
6. **Task 3 GREEN: complete final private release gates** - `367d335` (fix)

## Files Created/Modified

- `data/authority/README.md` - Selects the exact final revision while preserving the private-use, noncommercial, no-redistribution, artwork, upload, hosting, and public-API restrictions.
- `data/authority/receipts/official-2026-08-27-v3.json` - Records only safe content-free provenance and exact revision selection metadata.
- `tests/private-authority/private-revision.test.ts` - Proves dual-root determinism, write-once installation, exact selection, source immutability, and historical evidence preservation.
- `tests/private-authority/repository-boundary.test.ts` - Runs the production scanner against both the immutable historical evidence and the final selected revision.
- `.planning/phases/01-rules-and-data-authority/01-13-SUMMARY.md` - Records Plan 01-13 execution and release evidence without private content.

## Decisions Made

- Selected only the exact final v3 revision and root; no mutable alias, old-root fallback, retry, or reacquisition path was added.
- Kept all official source material and normalized corpus inside the ignored private boundary; committed only content-free receipt and policy metadata.
- Treated historical revisions as immutable published evidence and verified their stored canonical identity directly when later schema evolution made current-schema reconstruction inapplicable.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] Corrected a stale historical-revision regression after schema evolution**
- **Found during:** Task 3 full private authority gate
- **Issue:** A legacy test tried to rebuild and validate an immutable historical revision using the newer card-identity schema introduced after that revision was published.
- **Fix:** Preserved the historical bytes unchanged, retained deterministic reconstruction checks for the historical inputs, and verified the stored historical identity directly instead of applying the current schema retroactively.
- **Files modified:** `tests/private-authority/private-revision.test.ts`
- **Verification:** The corrected focused regression passed, then the complete four-command release gate passed.
- **Committed in:** `367d335`

---

**Total deviations:** 1 auto-fixed (1 Rule 1 bug)
**Impact on plan:** The correction restores the intended immutability contract without weakening current-schema validation, changing historical evidence, adding fallback inputs, or expanding acquisition scope.

## Issues Encountered

- The Windows sandbox setup-refresh helper rejected tracked patch calls. Each affected edit used the established exact-count-checked local fallback after the patch attempt failed; no broad replacement or unrelated file edit was performed.
- The initial complete private gate exposed the stale historical-schema assertion described above. After the focused correction passed, the entire four-command release sequence was restarted from the beginning.

## Authentication Gates

None.

## TDD Gate Compliance

- Task 1 RED `f9124b8` preceded GREEN `25bbe38` and proved the final candidate contract failed before deterministic dual-root construction was accepted.
- Task 2 RED `aa1fa99` preceded GREEN `7827109` and proved exact write-once selection and safe receipt requirements before installing the final revision.
- Task 3 RED `0d5cf35` preceded GREEN `367d335` and proved the final v3 production boundary scan was required before release.

## Verification

- `pnpm verify`: TypeScript typecheck and ESLint passed; 157 tests passed with 0 failures, skips, or todos.
- Focused fresh primary/backup completeness: 1 passed, 0 failed.
- `pnpm authority:verify-private`: 9 passed, 0 failed.
- Narrow repository-boundary suite: 4 passed, 0 failed.
- Plan frontmatter validation: valid with no missing fields.
- Plan structure validation: 3 tasks valid with no errors or warnings.
- The staged tracked tree remained unchanged across the complete release sequence.

## Known Stubs

None. No placeholder data, mock source, TODO, FIXME, empty UI value, or unwired runtime path was introduced.

## User Setup Required

None. The final revision is already installed locally under the ignored private boundary; no upload, sharing, hosting, push, or remote mutation was performed.

## Next Phase Readiness

- DATA-01, DATA-02, and DATA-03 now satisfy their Plan 13 release gates.
- All 15 Phase 1 plans have execution summaries; phase-level goal verification still belongs to the execute-phase verifier.
- Broader acquisition, artwork, sharing, hosting, redistribution, public/network APIs, and commercialization remain blocked pending written publisher permission and a separately approved plan.

---
*Phase: 01-rules-and-data-authority*
*Completed: 2026-08-27*

## Self-Check: PASSED

- All five tracked key files exist, and the ignored final revision validated by exact public revision identity during the release gate.
- All six RED/GREEN task commits exist in reachable Git history.
- Every task acceptance criterion and all four plan-level release commands passed against the final implementation.
- Historical official-2026-08-20 evidence remained unchanged.
- No private absolute path, locator, lock value, private-only hash, source excerpt, normalized corpus, PDF/page, artwork, or ignored revision content is included in this summary.
- No unplanned network endpoint, authentication path, schema trust boundary, or other threat surface was introduced beyond the plan threat register.
