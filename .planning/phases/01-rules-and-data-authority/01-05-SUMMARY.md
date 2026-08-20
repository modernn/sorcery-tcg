---
phase: 01-rules-and-data-authority
plan: "05"
subsystem: authority-data
tags: [offline-import, atomic-publish, input-lock, deterministic-validation, sha-256]
dependency-graph:
  requires:
    - phase: 01-03
      provides: strict canonical authority artifacts and provenance schemas
    - phase: 01-04
      provides: confined recursive authority bundle validation
  provides:
    - exact independently verified durable input-lock importer
    - same-directory atomic write-once authority revisions
    - offline deterministic validation CLI with honest evidence modes
  affects: [01-06, 01-07, 01-08]
tech-stack:
  added: []
  patterns:
    - verify exact input-root identity and every raw byte before parsing
    - fully validate a same-parent temporary candidate before atomic rename
    - distinguish stored-byte rehash evidence from manifest-binding evidence
key-files:
  created:
    - .gitattributes
    - src/commands/import-authority.ts
    - src/commands/validate-authority.ts
    - tests/authority/fixtures/bundle-input/cards.raw.json
    - tests/authority/fixtures/bundle-input/formats.json
    - tests/authority/fixtures/bundle-input/sources.json
  modified:
    - package.json
    - src/authority/validate-bundle.ts
    - tests/authority/bundle.test.ts
    - tests/authority/card-snapshot.test.ts
    - tests/authority/fixtures/README.md
    - tests/authority/fixtures/bundle-input/input-lock.json
key-decisions:
  - "The durable input lock covers the command's fixed local inputs, and its independently supplied root is verified before content parsing."
  - "Stored source paths resolve from the selected revision and only approved raw card bytes may occupy the fixed raw/cards.raw.json path."
  - "Validation success explicitly reports stored-bytes-rehashed separately from manifest-binding-verified."
  - "Byte-locked fixture JSON is pinned to LF in .gitattributes for cross-platform hash stability."
requirements-completed: [DATA-01, DATA-02]
metrics:
  duration: 42 min
  completed: 2026-08-20
---

# Phase 1 Plan 5: Local Authority Commands Summary

**Offline, byte-locked authority imports that validate completely before atomic publication, plus read-only validation with explicit stored-versus-manifest evidence.**

## Performance

- **Duration:** 42 min
- **Started:** 2026-08-20T17:03:22Z
- **Completed:** 2026-08-20T17:45:02Z
- **Tasks:** 2
- **Files modified:** 12

## Accomplishments

- Added a developer-only importer that accepts only the documented local paths, verifies an exact durable file lock and independently computed input-root hash before parsing, and reuses the shared normalization, artifact, and bundle-validation modules.
- Made publication write-once and failure-safe by building in a bounded same-parent temporary directory, validating the complete candidate, and performing one atomic rename without overwrite.
- Added an offline read-only validation command with canonical output, sorted diagnostics, expected ID/hash binding, and evidence that does not overstate verification of absent manifest-only bytes.
- Replaced every Plan 01-05 authority todo with security and determinism assertions, including byte tampering, companion/reference tampering, ambiguity, atomic failure, overwrite refusal, no mutation, and disabled-network execution.

## Task Commits

Each task was committed atomically:

1. **Task 1: Build write-once revisions atomically from local inputs** - `cf159fd` (`feat`)
2. **Task 2: Provide deterministic offline validation command** - `4519b26` (`feat`)

## Files Created/Modified

- `.gitattributes` - Pins byte-locked fixture JSON to LF so reviewed hashes survive Windows checkouts.
- `src/commands/import-authority.ts` - Local-only input-lock verification, canonical candidate construction, recursive validation, and atomic publication.
- `src/commands/validate-authority.ts` - Thin offline validator CLI with deterministic success and diagnostic records.
- `src/authority/validate-bundle.ts` - Validates imported revision companions, exact file sets, aggregate limits, normalized payloads, and stored bytes.
- `package.json` - Adds `authority:import` and `authority:validate` developer scripts.
- `tests/authority/bundle.test.ts` - Covers deterministic builds, write-once/atomic behavior, storage policy, tampering, ambiguity, read-only validation, and offline execution.
- `tests/authority/card-snapshot.test.ts` - Proves the independently expected root is rejected before raw-card normalization.
- `tests/authority/fixtures/bundle-input/*` - Reviewed source, format, raw-card, and exact input-lock fixture set.

## Decisions Made

- Keep the command contract intentionally narrow: three fixed data inputs plus the durable lock, with no URL, acquisition, repair, bypass, or generic copy mode.
- Resolve stored source paths relative to the selected immutable revision, matching the bundle validator's trust boundary.
- Publish manifest-only inputs as canonical provenance bindings without copying or claiming to rehash absent upstream bytes.
- Emit candidate input and bundle roots before recursive validation while withholding publication until validation and atomic rename succeed.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] Corrected stored-source path resolution**

- **Found during:** Task 1
- **Issue:** Stored source paths were initially resolved from the authority root, but bundle paths are relative to the selected revision directory.
- **Fix:** Resolve and confine stored paths from the revision containing `bundle.json`.
- **Files modified:** `src/authority/validate-bundle.ts`
- **Commit:** `cf159fd`

**2. [Rule 2 - Missing Critical Functionality] Enforced imported-revision shape and companion binding**

- **Found during:** Task 1
- **Issue:** Recursive validation needed to reject extra/missing imported files and independently verify canonical companion payloads and aggregate byte/file bounds.
- **Fix:** Added exact revision file-set, canonical artifact/source/format/card companion, and aggregate resource validation for imported bundles.
- **Files modified:** `src/authority/validate-bundle.ts`
- **Commit:** `cf159fd`

**3. [Rule 2 - Missing Critical Functionality] Stabilized byte-locked fixtures across checkouts**

- **Found during:** Task 1
- **Issue:** Repository `core.autocrlf` could change reviewed JSON bytes and invalidate the durable input root on Windows.
- **Fix:** Added a narrow `.gitattributes` rule pinning only the locked fixture JSON to LF.
- **Files modified:** `.gitattributes`
- **Commit:** `cf159fd`

**4. [Rule 2 - Missing Critical Functionality] Closed the remaining Plan 01-05 pre-parse boundary todo**

- **Found during:** Task 2
- **Issue:** The Wave 0 card-snapshot suite still contained a Plan 01-05 todo proving an incorrect independent root is rejected before normalization.
- **Fix:** Replaced it with an importer boundary test using invalid raw bytes and a wrong expected root.
- **Files modified:** `tests/authority/card-snapshot.test.ts`
- **Commit:** `4519b26`

## Issues Encountered

- The Windows sandbox helper and direct patch helper rejected repository writes. Exact unified diffs were applied with command-scoped `git apply`; all resulting files were subsequently typechecked, linted, tested, and reviewed by Git diff checks.

## Authentication Gates

None.

## Known Stubs

None. The modified source and tests contain no placeholder, TODO, FIXME, skipped, or todo behavior for this plan.

## User Setup Required

None. Both commands operate entirely on approved local files.

## Verification Results

- `pnpm typecheck` - passed
- `pnpm lint` - passed
- `pnpm test` - 63 passed, 0 failed, 0 skipped, 0 todo
- Targeted authority suites - 31 passed, 0 failed
- Real CLI smoke: local import followed by ID/hash-bound offline validation - passed
- Source scan for `fetch`, `node:http`, `node:https`, and `node:net` in the command/validation path - no matches
- `git diff --check` - passed

## Next Phase Readiness

- Plans 01-06 through 01-08 can consume immutable, deterministic revisions without adding any acquisition path to runtime automation.
- Official authenticity, permission review, and publication remain deliberately deferred to their planned policy and human-review gates.

## Self-Check: PASSED

- All declared created files exist.
- Task commits `cf159fd` and `4519b26` are present in repository history.

