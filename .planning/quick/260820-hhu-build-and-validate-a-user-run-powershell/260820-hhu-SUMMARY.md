---
phase: quick-260820-hhu-build-and-validate-a-user-run-powershell
plan: "01"
subsystem: authority-data
tags: [powershell, node-test, loopback, provenance, sha256, private-storage]

requires:
  - phase: 01-rules-and-data-authority
    provides: strict seven-source private-set verifier and D-08 private operating boundary
provides:
  - fixed user-run one-shot PowerShell collector for exactly seven official sources
  - synthetic loopback transport/publication test suite
  - verifier-gated independent backup and receipt-last private lock publication
  - reconciled policy, outreach, Plan 01-07, and STATE decision wording
affects: [01-07, 01-08, authority-import, private-revision]

tech-stack:
  added: []
  patterns: [core-enforced fixed production manifest, loopback-only test seam, bounded manual redirects, verifier-gated receipt-last publication]

key-files:
  created:
    - scripts/collect-private-authority.ps1
    - tests/authority/private-authority-collector.test.ts
  modified:
    - docs/external-reuse-policy.md
    - docs/card-data-authorization-outreach.md
    - .planning/phases/01-rules-and-data-authority/01-07-PLAN.md
    - .planning/STATE.md

key-decisions:
  - "The production collector accepts only an absolute independent backup root and AcknowledgePrivateUseRisk; all source and transport settings remain fixed."
  - "The existing TypeScript verifyPrivateSourceSet contract owns canonical source-set identity at staging, final-tree, and lock-candidate gates."
  - "The D-08 exception permits only private user-run one-shot acquisition; written permission remains required for recurring, broader, artwork, hosted, redistributed, or commercial use."

patterns-established:
  - "Acquisition tests dot-source a production-inert script and supply only synthetic loopback descriptors through a production-inaccessible seam."
  - "Complete source trees move without overwrite, and the private lock is the last publication operation."

requirements-completed: []

duration: 29min
completed: 2026-08-20
---

# Quick Task 260820-hhu: User-Run Private Authority Collector Summary

**Fixed seven-source PowerShell collector with bounded loopback-tested transport, independent byte copies, three TypeScript verifier gates, and receipt-last private publication**

## Performance

- **Duration:** 29 min
- **Started:** 2026-08-20T21:32:35Z
- **Completed:** 2026-08-20T22:01:46Z
- **Tasks:** 3
- **Files modified:** 6

## Accomplishments

- Added a production-inert PowerShell 7 wrapper whose direct entry point exposes only `BackupRoot` and `AcknowledgePrivateUseRisk`, with exactly seven fixed official sources and no artwork or generic URL surface.
- Added 53 synthetic loopback checks covering strict rulebook selection, redirects, statuses, CAPTCHA/block evidence, timeouts, per-response and aggregate bounds, content validation, exact bytes, preflight, independent copies, verifier gates, no-overwrite behavior, and owned-output quarantine.
- Enforced the exact repository root, primary/lock paths, seven descriptors, timeouts, and aggregate limit inside the shared core before non-loopback transport; caller-supplied production configuration now fails with zero requests.
- Reconciled policy and Plan 01-07 around the risk-accepted D-08 one-shot exception while keeping broader acquisition, distribution, artwork, hosting, upload, and commercial use permission-gated.
- Narrowed the stale `STATE.md` stored-source decision so the one-shot exception is truthful without weakening license-status metadata or broader-use permission requirements.

## Task Commits

Each TDD and documentation slice was committed atomically:

1. **Task 1 RED: collector transport contract** - `315cbd1` (test)
2. **Task 1 GREEN: fixed bounded collector transport** - `bb7a9f6` (feat)
3. **Task 2 RED: collector publication contract** - `556b8bd` (test)
4. **Task 2 GREEN: verifier-gated private receipt** - `a47c7d5` (feat)
5. **Task 3: policy and Plan 01-07 reconciliation** - `b3d0377` (docs)
6. **Verification gap closure: fixed production boundary and missing regressions** - `9e039f8` (fix)
7. **Supported command surface: module-private collector helpers** - `302a685` (fix)

The orchestrator owns the uncommitted quick-task summary and `STATE.md` metadata commit.

## Files Created/Modified

- `scripts/collect-private-authority.ps1` - Fixed-manifest, bounded, no-retry collector with independent backup, three verifier gates, and private receipt-last publication.
- `tests/authority/private-authority-collector.test.ts` - Synthetic loopback integration, security, publication, and fault-quarantine coverage.
- `docs/external-reuse-policy.md` - Accepted one-shot risk boundary, private local-art boundary, and broader-use permission gate.
- `docs/card-data-authorization-outreach.md` - Outreach request preserved for recurring and broader permissions.
- `.planning/phases/01-rules-and-data-authority/01-07-PLAN.md` - Human checkpoint now runs and independently reverifies the collector-produced lock.
- `.planning/STATE.md` - Mandatory decision wording recognizes the narrow D-08 exception while retaining broader-use permission prerequisites.

## Decisions Made

- Kept production endpoints, paths, caps, redirect hosts, timeouts, and source markers inside one fixed accessor; only the loopback seam accepts temporary test inputs.
- Preserved exact response bytes and delegated canonical sorting/root hashing to `verifyPrivateSourceSet` rather than duplicating identity logic in PowerShell.
- Published backup, then primary, reverified final trees, reverified the lock candidate, and moved the lock last as the commit marker.
- Kept artwork outside the collector; a later GUI may use only separately user-supplied private local images under its own permission-reviewed task.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] Preserved verifier timestamps as strings across PowerShell JSON round trips**
- **Found during:** Task 2 publication verification
- **Issue:** PowerShell `ConvertFrom-Json` automatically converted UTC strings to `DateTime`, changing serialized entry bytes and therefore the candidate root hash.
- **Fix:** Used `-DateKind String` for verifier and lock-candidate JSON parsing.
- **Files modified:** `scripts/collect-private-authority.ps1`
- **Verification:** Staging, final-tree, and lock-candidate verifier results now match; focused and full suites pass.
- **Committed in:** `a47c7d5`

---

**Total deviations:** 1 auto-fixed bug
**Impact on plan:** The fix is required for deterministic receipt identity; scope is unchanged.

## Issues Encountered

- Git reported repository ownership mismatch under the execution account. All repository commands used the scoped `safe.directory=C:/src/sorcery-tcg` option; no global Git configuration was changed.

## Known Stubs

None.

## Verification

- `node --test tests/authority/private-authority-collector.test.ts` - 53/53 passed.
- `node --test tests/authority/private-source-set.test.ts` - 11/11 passed.
- Task 3 normalized policy assertion - passed.
- `pnpm verify` - 127/127 passed.
- `git diff --check` - passed.
- `.local/authority` tracked/staged scan - empty.
- No publisher endpoint or production collector invocation occurred.

## User Setup Required

Plan 01-07 remains blocked on the user personally choosing an absent absolute backup root outside the repository and running the documented production command with `-AcknowledgePrivateUseRisk`. The executor must then independently reverify the ignored lock before creating `01-07-SUMMARY.md` or releasing Plan 01-08.

## Next Phase Readiness

- The collector implementation and offline validation gates are ready.
- Plan 01-07 remains the blocking human-action checkpoint; no live official bytes, lock, or Plan 01-07 summary were created by this quick task.
- DATA-01/02/03 must remain pending at phase level until the user-run collection and Plans 01-07/01-08 complete.

## Self-Check: PASSED

All created/modified deliverables and all six task commits were verified present.

---
*Phase: quick-260820-hhu-build-and-validate-a-user-run-powershell*
*Completed: 2026-08-20*
