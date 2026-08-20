---
phase: 01-rules-and-data-authority
plan: "04"
subsystem: authority-data
tags: [offline-validation, provenance-graph, sha-256, precedence, path-confinement]

requires:
  - phase: 01-rules-and-data-authority
    provides: Strict authority schemas, canonical JSON hashing, SourceRef binding, and stored-versus-manifest source contracts from Plan 02
provides:
  - Recursive bounded authority-bundle validation with confined real paths and cycle detection
  - Honest stored-byte rehashing versus manifest-only locator and procedure verification
  - Official-only precedence resolution with fail-closed ambiguity and storage-permission enforcement
affects: [01-05, 01-06, authority-data, runtime-bundle-selection]

tech-stack:
  added: []
  patterns:
    - Resolve and confine real paths before every authority file read
    - Bound file-handle reads, graph traversal, references, and diagnostics at the trust boundary
    - Preserve reviewed community provenance while excluding it from normative precedence

key-files:
  created:
    - src/authority/validate-bundle.ts
    - tests/authority/fixtures/provenance-valid.json
    - tests/authority/fixtures/provenance-cycle.json
    - tests/authority/fixtures/provenance-tampered.json
  modified:
    - tests/authority/provenance.test.ts
    - tests/authority/bundle.test.ts

key-decisions:
  - "Treat stored-byte rehashing and manifest-only reference verification as distinct successful evidence states."
  - "Resolve normative authority from official records only; equal-rank or unclear official outcomes remain unsupported."
  - "Require explicit approved license metadata before any stored source bytes may be accepted."

patterns-established:
  - "Offline trust gate: validate schema, canonical hashes, references, graph bounds, path confinement, and precedence in one authority-bundle boundary."
  - "Evidence honesty: stored bytes are rehashed locally, while manifest-only sources prove only their declared locator, procedure, hash, and SourceRef binding."
  - "Precedence safety: supersession and explicit scope can select an official record; unresolved ties fail closed instead of relying on array order."

requirements-completed: [DATA-01, DATA-03]

duration: 34min
completed: 2026-08-20
---

# Phase 1 Plan 4: Offline Authority Bundle Validation Summary

**Confined recursive bundle validation with bounded reads, tamper-evident provenance graphs, official-only precedence, and honest offline evidence reporting**

## Performance

- **Duration:** 34 min
- **Started:** 2026-08-20T16:19:56Z
- **Completed:** 2026-08-20T16:54:35Z
- **Tasks:** 2
- **Files modified:** 6

## Accomplishments

- Added a single offline validation boundary that strictly parses authority bundles, independently recomputes canonical artifact hashes, validates every SourceRef and parent edge, detects cycles, and caps bytes, files, graph depth, references, and diagnostics.
- Confined stored source paths by lexical and real-path checks before reading, rejecting traversal, absolute paths, missing targets, and Windows symlink or junction escapes; stored bytes are rehashed while manifest-only evidence is reported without claiming absent bytes were reverified.
- Added official-only precedence resolution for supersession, effective dates, card and format scopes, and authority rank, with ambiguous, equal-rank, mixed-topic, cyclic, or unclear outcomes rejected as unsupported.
- Enforced explicit approved license metadata before stored media or corpus bytes may be accepted, retained reviewed community provenance as non-normative evidence, and proved the validator has no network path.
- Replaced only Plan 01-04 contracts with active assertions while preserving the nine importer and publication contracts owned by Plan 01-05.

## Task Commits

Each TDD task was committed as a RED contract followed by its GREEN implementation:

1. **Task 1: Constrain paths and validate the provenance graph**
   - dd56839 — RED recursive validation, confinement, tampering, cycle, and diagnostic-bound contracts
   - af9906b — GREEN confined offline graph validator and synthetic provenance fixtures
2. **Task 2: Resolve official precedence and enforce offline storage policy**
   - 9ccdf59 — RED official precedence, ambiguity, offline, and storage-policy contracts
   - cedf262 — GREEN official-only resolver, storage gate, and bounded file reader

## Files Created/Modified

- src/authority/validate-bundle.ts - Central offline bundle validator, path confinement boundary, graph checker, storage policy, and official precedence resolver.
- tests/authority/provenance.test.ts - Active recursive validation, path escape, tamper, cycle, manifest-only, and bound diagnostics assertions.
- tests/authority/bundle.test.ts - Active precedence, ambiguity, community provenance, offline, and storage-policy assertions plus retained Plan 01-05 contracts.
- tests/authority/fixtures/provenance-valid.json - Minimal synthetic authority bundle descriptor for valid stored and manifest-only evidence.
- tests/authority/fixtures/provenance-cycle.json - Minimal synthetic cyclic provenance descriptor.
- tests/authority/fixtures/provenance-tampered.json - Minimal synthetic one-byte stored-source tamper descriptor.

## Decisions Made

- Successful stored evidence records a locally recomputed byte hash; successful manifest-only evidence records validated locator, procedure, declared hash, and SourceRef binding without implying the upstream bytes were present.
- Community and external references remain visible in provenance but can never win a normative resolution; only official records participate in precedence.
- Explicit supersession is evaluated before effective date, scoped overlays outrank general records only within their exact scope, and unresolved official ties or unclear applicability make the interaction unsupported.
- Stored source acceptance requires licenseStatus approved before any bytes are opened, keeping the permission decision ahead of the file-reading boundary.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 2 - Missing Critical] Bounded reads across file-size races**
- **Found during:** Task 2 (Resolve official precedence and enforce offline storage policy)
- **Issue:** A stat-then-readFile sequence could read beyond the configured byte cap if a file changed between the size check and allocation.
- **Fix:** Replaced the sequence with a bounded file-handle reader that reads at most the configured maximum plus one byte and emits the deterministic max-bytes diagnostic.
- **Files modified:** src/authority/validate-bundle.ts
- **Verification:** Targeted provenance and bundle tests, full test suite, typecheck, and lint pass.
- **Committed in:** cedf262

---

**Total deviations:** 1 auto-fixed (1 missing critical)
**Impact on plan:** The fix closes a trust-boundary resource-limit race without expanding scope or changing the authority model.

## Issues Encountered

- The Windows sandbox helper returned helper_unknown_error for local commands and the mandated patch editor; required local verification and exact unified git patches were run outside the malfunctioning sandbox with command-scoped approval.
- The installed apply_patch wrapper also failed with an access-denied sandbox error, so generated unified patches were applied through git apply and checked with git diff --check.
- An optional read-only helper agent could not start because the shared agent thread limit was already reached; execution continued sequentially as required.

## Authentication Gates

None.

## Known Stubs

- tests/authority/bundle.test.ts retains nine intentional test.todo contracts for input locking, immutable publication, atomic failure, repeatable builds, and published-copy tampering. Plan 01-05 owns those importer and publication behaviors, so they were preserved exactly as required.

These future contracts do not prevent Plan 01-04 from meeting its recursive validation, precedence, storage, and offline trust-gate goal. tests/authority/provenance.test.ts has no remaining todo or skipped contracts.

## User Setup Required

None - no external service, network acquisition, database, container, engine, GUI, or manual permission step is required.

## Verification Results

- node --test tests/authority/provenance.test.ts - PASS (19 active assertions, 0 todos)
- node --test tests/authority/provenance.test.ts tests/authority/bundle.test.ts - PASS (28 active assertions, 9 future Plan 01-05 todos, 0 failures)
- Targeted current, superseded, scoped, ambiguous, offline, storage, and media bundle suite - PASS (8 selected assertions, 0 failures)
- pnpm typecheck - PASS
- pnpm lint - PASS
- pnpm test - PASS (62 discovered contracts, 52 active assertions, 10 future-plan todos, 0 failures)
- Network-path scan - PASS; validate-bundle.ts has no node:http, node:https, node:net, or fetch call.
- Self-contained file and commit audit - PASS; all 6 plan files and all 4 task commits exist.

## Next Phase Readiness

- Plan 01-05 can use validateAuthorityBundle as the sole offline gate before immutable publication and can replace the nine retained importer and publication todos.
- Later runtime consumers can select only bundles whose graph, evidence state, storage permissions, and official precedence have passed this validator.
- No blockers remain for the next authority plan.

## Self-Check: PASSED

- All 6 created or modified implementation, test, and fixture files exist.
- Task commits dd56839, af9906b, 9ccdf59, and cedf262 exist in git history.
- Targeted and full verification commands passed after the final task commit.

---
*Phase: 01-rules-and-data-authority*
*Completed: 2026-08-20*
