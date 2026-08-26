---
phase: 01-rules-and-data-authority
plan: "11"
subsystem: authority-acquisition
tags: [powershell, node, privacy, html-validation, subprocess-bounds]

requires:
  - phase: 01-rules-and-data-authority
    provides: strict official-card adapter and private source-set verifier from Plans 01-03 and 01-08
provides:
  - fail-closed source-specific completeness gates for the fixed seven-item authority set
  - visible, unique, first-entry-bound changelog effective dates
  - fixed transcript-safe production output and bounded concurrent child-process handling
affects: [authority-import, private-source-verification, phase-01-gate]

tech-stack:
  added: []
  patterns:
    - existing TypeScript adapters invoked through bounded Node bridges
    - fixed production messages with detailed diagnostics retained only by synthetic loopback seams

key-files:
  created: []
  modified:
    - scripts/collect-private-authority.ps1
    - tests/authority/private-authority-collector.test.ts

key-decisions:
  - "Production card acquisition requires exactly 1,100 adapted records; loopback descriptors declare their independent synthetic cardinality."
  - "Changelog dates are parsed only from the first visible article after hidden HTML and comments are removed."
  - "One bounded process helper drains both redirected pipes concurrently and kills the complete child tree on timeout."

patterns-established:
  - "Production wrappers discard sensitive internal results and emit only fixed success or failure text."
  - "Synthetic hang/canary regressions prove timeout and non-disclosure without reading private authority data."

requirements-completed: [DATA-01]

duration: 23min
completed: 2026-08-26
---

# Phase 1 Plan 11: Fixed Private Authority Collector Summary

**Complete seven-source acquisition with visible-date binding, strict 1,100-card adaptation, transcript-safe output, and bounded verifier subprocesses**

## Performance

- **Duration:** 23 min
- **Started:** 2026-08-26T20:36:52Z
- **Completed:** 2026-08-26T20:59:55Z
- **Tasks:** 2
- **Files modified:** 3

## Accomplishments

- Removed head, script, style, template, noscript, title, and comment content before binding exactly one semantic date to the first visible changelog entry.
- Required explicit visible body markers for each fixed HTML descriptor and delegated card validation to the existing strict adapter with a fixed production cardinality of 1,100.
- Suppressed production lock/error detail behind fixed messages while concurrently draining, bounding, timing out, and tree-killing verifier subprocesses.
- Added synthetic title-shell, partial-card, canary-disclosure, pipe-pressure, and hung-child regressions without contacting or reading private authority sources.

## Task Commits

Each task used RED/GREEN commits:

1. **Task 1 RED: source completeness regressions** - `75ed316` (test)
2. **Task 1 RED: strict synthetic card fixture** - `fcc0bfc` (test)
3. **Task 1 GREEN: fail-closed source acceptance** - `37811ea` (feat)
4. **Task 2 RED: output and timeout regressions** - `f59c7a1` (test)
5. **Task 2 GREEN: bounded sanitized collection** - `d1fc9ec` (feat)

## Files Created/Modified

- `scripts/collect-private-authority.ps1` - Enforces visible/source-specific completeness, strict card adaptation/count, sanitized production output, and bounded subprocess execution.
- `tests/authority/private-authority-collector.test.ts` - Provides independent synthetic structures, canary output probes, bounded PowerShell execution, and verifier-hang cleanup coverage.
- `.planning/phases/01-rules-and-data-authority/01-11-SUMMARY.md` - Records Plan 01-11 execution and verification evidence.

## Decisions Made

- Used the existing `adaptOfficialCardApiSnapshot` implementation rather than duplicating the official schema in PowerShell.
- Kept complete lock data available only through the synthetic loopback seam; the production wrapper discards it.
- Used the fixed 30-second production subprocess deadline and a one-second synthetic hang fault to keep the regression suite bounded.

## Deviations from Plan

None - plan executed exactly as written.

## Issues Encountered

- The normal Windows sandbox helper failed during file operations; the approved escalated Codex apply-patch runner preserved the required patch-only editing workflow.
- Repository-wide verification briefly observed incomplete parallel Plan 01-12 edits. No out-of-scope files were changed; the final retry passed after those edits settled.

## Authentication Gates

None.

## TDD Gate Compliance

- RED commits: `75ed316`, `fcc0bfc`, and `f59c7a1` each captured observed pre-implementation failures.
- GREEN commits: `37811ea` and `d1fc9ec` followed the corresponding regressions and made them pass.

## Verification

- Focused Task 1 collector selection: 20 passed, 0 failed.
- Full collector suite: 85 passed, 0 failed.
- Focused ESLint on the owned TypeScript test: passed.
- `pnpm verify`: typecheck and lint passed; 188 tests passed with 0 failures, skips, or todos.

## User Setup Required

None - no external service configuration or production collection was performed.

## Next Phase Readiness

- CR-01, CR-02, CR-03, and WR-01 are closed with runnable synthetic evidence.
- The fixed collector is ready for Phase 1 re-verification; all broader private-use and publisher-permission limits remain unchanged.

---
*Phase: 01-rules-and-data-authority*
*Completed: 2026-08-26*

## Self-Check: PASSED

- All three key files exist.
- All five RED/GREEN task commits exist in Git history.
- Final focused, collector-wide, and repository-wide verification claims were confirmed before close-out.
