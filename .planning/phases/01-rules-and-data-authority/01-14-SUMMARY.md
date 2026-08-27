---
phase: 01-rules-and-data-authority
plan: "14"
subsystem: authority-acquisition
tags: [powershell, node-test, offline-verification, privacy]

requires:
  - phase: 01-rules-and-data-authority
    provides: bounded private source verification and collector hardening from Plans 01-07 and 01-11
provides:
  - reusable source-specific content verification over an existing seven-item local authority set
  - mutually exclusive read-only existing-roots routing before transport and publication setup
  - synthetic proof of no transport, incomplete-root rejection, immutability, and fixed sanitized output
affects: [01-15-private-reacquisition, 01-13-phase-release-gate]

tech-stack:
  added: []
  patterns:
    - one shared content verifier for staged collection and read-only existing-root verification
    - fixed content-free production output at the private filesystem boundary

key-files:
  created:
    - .planning/phases/01-rules-and-data-authority/01-14-SUMMARY.md
  modified:
    - scripts/collect-private-authority.ps1
    - tests/authority/private-authority-collector.test.ts

key-decisions:
  - "Plan 14 certifies only the reusable verifier extraction and synthetic offline routing committed in 5aadff3 and 77c6000."
  - "The historical authority roots remain unchanged but did not satisfy the strengthened content contract; fresh evidence and proof belong to Plan 15."

patterns-established:
  - "Existing-root verification is read-only, mutually exclusive with acquisition parameters, and enters before transport or publication setup."
  - "Private-boundary failures emit fixed messages while detailed private values remain outside process output and summaries."

requirements-completed: []

duration: 4 min
completed: 2026-08-27
---

# Phase 1 Plan 14: Reusable Offline Private Authority Content Verifier Summary

**A shared seven-source content verifier with read-only existing-root routing, synthetic no-transport coverage, immutable inputs, and fixed non-disclosing output**

## Performance

- **Duration:** 4 min closeout verification
- **Started:** 2026-08-27T19:48:16Z
- **Completed:** 2026-08-27T19:51:44Z
- **Tasks:** 1
- **Files modified:** 3

## Accomplishments

- Extracted the collector's source-specific PDF, HTML, effective-date, and exact-card-corpus checks into `Assert-PrivateAuthorityContentSet` for reuse on local bytes.
- Added a mutually exclusive `-VerifyExistingRoots` route that structurally and semantically verifies an existing pair before any transport or publication setup and emits only fixed output.
- Proved the route with synthetic complete/incomplete roots, a throwing transport canary, before/after byte comparisons, and sanitized diagnostics.
- Kept DATA-01 pending: Plan 13 and phase verification must pass before DATA-01, DATA-02, or DATA-03 can be marked complete.

## Task Commits

Task 1 used the existing RED/GREEN commits and was not recommitted:

1. **RED: add failing offline verification regressions** - `5aadff3` (test)
2. **GREEN: add read-only existing-root verification** - `77c6000` (feat)

No other implementation commit is attributed to Plan 14.

## Files Created/Modified

- `scripts/collect-private-authority.ps1` - Reuses one content verifier and exposes the fixed read-only existing-roots route.
- `tests/authority/private-authority-collector.test.ts` - Covers offline routing, no transport, incomplete-root rejection, immutability, exclusivity, and fixed sanitized output with synthetic data.
- `.planning/phases/01-rules-and-data-authority/01-14-SUMMARY.md` - Records the safe-resume closeout and its explicit evidence boundary.

## Decisions Made

- Closed Plan 14 from the exact reachable RED/GREEN commits rather than rerunning or rewriting already-completed implementation.
- Treated the earlier historical evidence pair as nonconforming under the strengthened contract. Both roots remained byte-identical and unchanged, but neither is claimed to have passed.
- Left fresh acquisition and a real-root proof exclusively to Plan 15; this closeout performed no acquisition, retry, transport, publication, or private-root read.

## Deviations from Plan

None - plan executed exactly as revised.

## Issues Encountered

- The configured `pnpm` shim resolved to a missing local Corepack module. The focused synthetic tests passed normally, and the existing local TypeScript compiler then ran the project's exact `tsc --noEmit` typecheck script successfully. No dependency or source file changed.

## Authentication Gates

None.

## Verification

- Commit ancestry: `5aadff3` and `77c6000` are reachable from `HEAD`.
- Commit ownership: RED changes only `tests/authority/private-authority-collector.test.ts`; GREEN changes only that test and `scripts/collect-private-authority.ps1`.
- Focused synthetic command: 4 passed, 0 failed for offline mode, local-root, no-transport, incomplete-existing-root, and sanitized-output selection.
- Typecheck: the local implementation of `tsc --noEmit` passed with exit code 0.
- Stub scan: no `TODO`, `FIXME`, placeholder, coming-soon, or not-available markers were found in the Plan 14 implementation files.

## Historical Evidence Boundary

The historical roots did not pass the strengthened content contract. They remained byte-identical and unchanged after that failed proof. Plan 14 therefore makes no historical-root success claim and closes only the reusable verifier plus its synthetic offline regressions.

## User Setup Required

None for Plan 14. Plan 15 owns the separately authorized personal acquisition action.

## Next Phase Readiness

- Plan 15 can use the committed verifier with a fresh, fixed authorization and revision identity.
- Plan 13 remains dependent on successful Plan 15 evidence and phase-level verification.
- Publisher permission remains absent; all private-local, noncommercial, no-recurring, no-retry/evasion, no-artwork, no-redistribution, no-hosting, and no-upload limits remain unchanged.

---
*Phase: 01-rules-and-data-authority*
*Completed: 2026-08-27*

## Self-Check: PASSED

- The summary file and both implementation files exist.
- Exact RED/GREEN commits `5aadff3` and `77c6000` exist and are reachable.
- Focused synthetic regressions and the project typecheck script passed.
- No private absolute path, lock value, locator, excerpt, or source byte appears in this summary.
- DATA-01, DATA-02, and DATA-03 remain pending.
