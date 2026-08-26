---
phase: 01-rules-and-data-authority
plan: "10"
subsystem: authority-validation
tags: [typescript, zod, provenance, sha256, graph-validation]

requires:
  - phase: 01-rules-and-data-authority
    provides: canonical authority schemas, offline bundle validation, and refresh-stable card identity through Plan 01-09
provides:
  - fail-closed graph-based official supersession with strict calendar dates
  - SHA locator byte binding separated from declaration-only manifest evidence
  - duplicate-free acyclic source derivation validation
affects: [phase-02-engine-contract, phase-03-rules, phase-06-coverage]

tech-stack:
  added: []
  patterns: [bounded directed graph validation, evidence-strength-specific command labels]

key-files:
  created:
    - .planning/phases/01-rules-and-data-authority/01-10-SUMMARY.md
  modified:
    - src/authority/schemas.ts
    - src/authority/validate-bundle.ts
    - src/commands/validate-authority.ts
    - tests/authority/bundle.test.ts
    - tests/authority/provenance.test.ts

key-decisions:
  - "Supersession removes only explicit graph targets; it never grants a global precedence rank."
  - "Only matching SHA URNs prove a manifest byte binding; HTTPS locators and procedure hashes remain declarations."
  - "Source derivation uses the existing bounded graph limits and deterministic diagnostic ordering."

patterns-established:
  - "Fail-closed graph resolution: broken, cyclic, over-bounded, or non-unique authority graphs never select a winner."
  - "Evidence labels state only what offline validation actually proves."

requirements-completed: [DATA-01, DATA-03]

duration: 21 min
completed: 2026-08-26
---

# Phase 1 Plan 10: Fail-Closed Authority Boundary Summary

**Graph-correct supersession, semantically bound SHA evidence, and deterministic acyclic source derivation at the reusable offline authority boundary**

## Performance

- **Duration:** 21 min
- **Started:** 2026-08-26T21:42:47Z
- **Completed:** 2026-08-26T22:04:01Z
- **Tasks:** 3
- **Files modified:** 5

## Accomplishments

- Replaced supersession's global rank boost with bounded graph validation, explicit target elimination, unique-contender resolution, and strict UTC date round-tripping.
- Bound `urn:sha256` locators to source byte hashes and split command evidence into stored-byte, manifest-byte-binding, and declaration-only states.
- Rejected duplicate and cyclic source derivation edges while retaining deterministic paths, limits, legacy diagnostics, and valid rooted DAGs.

## Task Commits

Each TDD task was committed as RED then GREEN:

1. **Task 1: Resolve supersession as a graph and validate real dates**
   - `ee11f1d` - RED precedence/date regressions
   - `956d3e1` - GREEN graph resolver and strict date validation
2. **Task 2: Bind SHA locators and report honest manifest evidence**
   - `cd87969` - RED locator and evidence-state regressions
   - `94398b8` - GREEN schema, validator, and command evidence changes
   - `0e0f8b8` - exact invalid-hash diagnostic preservation
3. **Task 3: Reject duplicate and cyclic source derivation edges**
   - `7fd537c` - RED duplicate/cycle/DAG regressions
   - `37feb1f` - GREEN bounded derivation graph validation

## Files Created/Modified

- `.planning/phases/01-rules-and-data-authority/01-10-SUMMARY.md` - Execution record, verification evidence, and decisions.
- `src/authority/schemas.ts` - Matching SHA locator semantic refinement with stable diagnostics.
- `src/authority/validate-bundle.ts` - Supersession and derivation graph validation plus explicit manifest evidence arrays.
- `src/commands/validate-authority.ts` - Truthful deterministic evidence labels.
- `tests/authority/bundle.test.ts` - Supersession, calendar-date, and command evidence regressions.
- `tests/authority/provenance.test.ts` - Recomputed locator and derivation graph regressions.

## Decisions Made

- Explicit supersession is an edge that eliminates its target; normal scope, kind, and effective-date ranking applies only to the remaining viable records.
- Cyclic, broken, duplicate-ID, over-reference, and over-depth supersession graphs return existing fail-closed unsupported states with complete sorted evidence.
- SHA locators are byte evidence only when their digest equals `byteHash`; HTTPS locators and acquisition procedure hashes are bound declarations and are never dereferenced or claimed as executed.
- Source derivation is checked in the existing private `validateGraph` boundary rather than adding another public validator.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] Preserved exact diagnostics for syntactically invalid byte hashes**
- **Found during:** Task 3 full verification
- **Issue:** The new SHA locator refinement also emitted `locator_byte_hash_mismatch` when `byteHash` itself was malformed, adding a misleading secondary diagnostic to an established invalid-format case.
- **Fix:** Guarded semantic locator comparison behind the existing lowercase SHA-256 format check.
- **Files modified:** `src/authority/schemas.ts`
- **Verification:** The invalid source hash regression and recomputed mismatched-locator regression both pass.
- **Committed in:** `0e0f8b8`

---

**Total deviations:** 1 auto-fixed bug.
**Impact on plan:** The fix preserves stable diagnostics without weakening SHA locator binding or expanding scope.

## Issues Encountered

- The sandboxed patch helper could not refresh the external repository workspace. All edits were still made through the established Codex apply-patch runner using approved escalated execution.
- Parallel Plan 01-12 boundary edits remained outside this plan's ownership and were not modified.

## Authentication Gates

None.

## Known Stubs

None.

## User Setup Required

None - no external service configuration or package installation was required.

## Verification Results

- Task 1 focused precedence/date suite - PASS (4/4); complete bundle suite - PASS (23/23); typecheck - PASS.
- Task 2 focused locator/evidence suite - PASS (6/6); complete bundle and provenance suites - PASS (44/44); typecheck and lint - PASS.
- Task 3 focused derivation suite - PASS (4/4).
- `pnpm verify` - PASS: typecheck, lint, and 202/202 tests.
- Stub scan - PASS; no implementation placeholders or UI-flow stubs were introduced.
- Threat-surface scan - PASS; no endpoint, authentication, network, file-access, or schema trust boundary beyond the plan's threat register was introduced.

## Next Phase Readiness

- Later engine, coverage, and simulation phases can consume one fail-closed authority boundary whose graph and evidence claims are no stronger than the offline bytes prove.
- Plan 01-10 has no remaining blocker; Phase 1 can proceed through the remaining parallel gap closure and re-verification.

## Self-Check: PASSED

- All five plan-owned implementation/test files and this summary exist.
- RED/GREEN commits `ee11f1d`, `956d3e1`, `cd87969`, `94398b8`, `7fd537c`, and `37feb1f` exist, along with corrective commit `0e0f8b8`.
- Targeted acceptance gates and the complete 202-test project verification passed.

---
*Phase: 01-rules-and-data-authority*
*Completed: 2026-08-26*
