---
phase: 01-rules-and-data-authority
plan: "15"
subsystem: authority-acquisition
tags: [powershell, node-test, offline-verification, privacy]

requires:
  - phase: 01-rules-and-data-authority
    provides: fixed manual inbox contract and reusable offline content verification
provides:
  - fresh seven-source v3 primary and independent backup roots imported entirely offline
  - immutable before-and-after evidence with exact fixed source identity coverage
  - production boundary proof that tracked, staged, packaged, and reachable history remain non-disclosing
affects: [01-13-phase-release-gate]

tech-stack:
  added: []
  patterns:
    - fixed ignored manual inbox with an offline-only importer
    - one shared content verifier with content-free process output
    - opt-in private completeness proof outside the clean-clone-safe default test glob

key-files:
  created:
    - tests/private-authority/private-source-completeness.test.ts
    - .planning/phases/01-rules-and-data-authority/01-15-SUMMARY.md
  modified:
    - scripts/collect-private-authority.ps1
    - scripts/verify-private-authority-boundary.ts
    - tests/authority/private-authority-collector.test.ts
    - tests/authority/private-authority-boundary.test.ts
    - tests/private-authority/repository-boundary.test.ts
    - docs/external-reuse-policy.md

key-decisions:
  - "The fixed seven-source set is user-provided through an ignored inbox and all project processing is offline."
  - "Private completeness is opt-in; the default verification remains synthetic and clean-clone-safe."
  - "DATA-01, DATA-02, and DATA-03 remain pending until Plan 13 and phase verification pass."

patterns-established:
  - "Private import failures are atomic and emit only fixed sanitized messages."
  - "Repository-boundary classification distinguishes generic local evidence labels from actual private locators."

requirements-completed: []

duration: 2h 11m
completed: 2026-08-27
---

# Phase 1 Plan 15: Offline Manual Authority Intake Summary

**Fresh seven-source v3 manual evidence with independent offline-verified roots, immutable before/after proofs, and zero Git or package leakage**

## Performance

- **Duration:** 2h 11m
- **Started:** 2026-08-27T23:07:47Z
- **Completed:** 2026-08-28T01:18:28Z
- **Tasks:** 3
- **Files modified:** 7 tracked implementation files plus this summary

## Accomplishments

- Replaced production transport with a fixed ignored manual inbox and an offline-only, atomic importer.
- Validated and imported exactly seven user-provided official non-artwork sources into fresh primary and independent backup roots without network access.
- Added opt-in proof of exact source identity coverage, root and lock immutability, shared PowerShell/TypeScript verification, and an in-process network sentinel.
- Proved the default suite remains synthetic and clean-clone-safe and the production boundary remains non-disclosing.
- Kept DATA-01, DATA-02, and DATA-03 pending for the final Plan 13 release gate.

## Task Commits

1. **RED: pin offline manual intake boundary** - `fad9049` (test)
2. **GREEN: replace acquisition with manual intake** - `916190d` (feat)
3. **Align current official-page validation markers** - `289ad5a` (fix)
4. **Accept the current date-only changelog heading** - `a5004a7` (fix)
5. **Prove fresh private-source completeness** - `76c6872` (test)
6. **Keep the generic private-locator label out of tracked source** - `cb52010` (fix)
7. **Distinguish local evidence from actual private locators** - `cc166a8` (fix)
8. **Raise the bounded decoded-string inspection ceiling** - `1995fe7` (fix)

## Files Created/Modified

- `scripts/collect-private-authority.ps1` - Imports and validates the fixed inbox entirely offline with atomic cleanup.
- `tests/authority/private-authority-collector.test.ts` - Covers manual intake, offline routing, exact validation, immutability, and sanitized failures.
- `tests/private-authority/private-source-completeness.test.ts` - Opt-in real-root completeness and no-network proof.
- `scripts/verify-private-authority-boundary.ts` - Preserves locator detection while allowing the exact generic local-evidence label.
- `tests/authority/private-authority-boundary.test.ts` - Adds the matching synthetic boundary regression.
- `tests/private-authority/repository-boundary.test.ts` - Keeps the release assertion on the single production boundary scanner.
- `docs/external-reuse-policy.md` - Records the manual, private-local reuse boundary.
- `.planning/phases/01-rules-and-data-authority/01-15-SUMMARY.md` - Records this content-free closeout.

## Decisions Made

- Kept user browser download separate from project code; all repository processing begins from the ignored fixed inbox and is offline.
- Reused the Plan 14 content verifier for both roots instead of creating a second validation path.
- Kept the real completeness test opt-in because clean clones intentionally lack private sources.

## Deviations from Plan

### Auto-fixed Issues

1. **Current official pages changed validation markers**
   - Updated the fixed structural markers without weakening source identity or content checks.

2. **The current changelog uses a date-only heading rather than an article wrapper**
   - Extended the existing bounded parser to accept that current official structure.

3. **An exact source duplicate existed at the repository root and in two internal tool refs**
   - Moved the worktree copy into ignored private holding and removed only the two isolated internal refs; the project branch was unchanged.

4. **The boundary scanner treated a generic provenance label as a private locator**
   - Narrowed the exemption to the exact assembled generic label and added a regression proving all real locator variants remain protected.

5. **The decoded-string work ceiling was below the reviewed bounded history workload**
   - Raised only that existing ceiling after measuring the bounded reachable input; byte ceilings and detection behavior were unchanged.

## Issues Encountered

- The first real intake failed safely on the changed changelog structure and left no partial targets. After the bounded parser fix, the same offline intake completed successfully.

## Authentication Gates

None. The user supplied the files through the local ignored inbox.

## Verification

- Plan frontmatter and task structure: valid, 3 tasks, no warnings.
- Focused manual/offline collector tests: 18 passed, 0 failed.
- Fresh-root completeness test: 1 passed, 0 failed.
- Full `pnpm verify`: 157 passed, 0 failed; typecheck and lint passed.
- Narrow repository-boundary test: passed.
- Production private-authority boundary: passed with fixed sanitized output.

## User Setup Required

None remaining for Plan 15.

## Next Phase Readiness

- Plan 13 can build and select the final immutable v3 authority revision from only these verified fresh roots.
- Broader reuse, recurring acquisition, redistribution, hosting, upload, artwork, and commercial use remain blocked pending written permission and a separate plan.

---
*Phase: 01-rules-and-data-authority*
*Completed: 2026-08-27*

## Self-Check: PASSED

- The summary and every tracked implementation file exist.
- All eight Plan 15 implementation commits exist and are reachable.
- Focused, full, private-completeness, and boundary gates passed.
- No private absolute path, lock value, locator, excerpt, source byte, or private hash appears in this summary.
- DATA-01, DATA-02, and DATA-03 remain pending.