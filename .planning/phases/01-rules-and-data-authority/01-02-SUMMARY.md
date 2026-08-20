---
phase: 01-rules-and-data-authority
plan: "02"
subsystem: authority-data
tags: [canonical-json, sha-256, zod, provenance, json-pointer]

requires:
  - phase: 01-rules-and-data-authority
    provides: Pinned Node/TypeScript/Zod toolchain and authority test contracts from Plan 01
provides:
  - Bounded RFC 8785-derived canonical JSON and raw/canonical SHA-256 identity helpers
  - Duplicate-aware bounded authority JSON parsing with deterministic JSON-Pointer diagnostics
  - Strict stored/manifest-only source, card, format, artifact, and bundle contracts
affects: [01-03, 01-04, authority-data, deck-artifacts, experiment-artifacts]

tech-stack:
  added: []
  patterns:
    - Single canonical JSON identity boundary
    - Strict Zod trust-boundary schemas with no repair
    - Stable ID for logical identity and SHA-256 for immutable revision identity

key-files:
  created:
    - src/authority/canonical-json.ts
    - src/authority/hash.ts
    - src/authority/schemas.ts
  modified:
    - tests/authority/canonical-json.test.ts
    - tests/authority/provenance.test.ts

key-decisions:
  - "Canonical identity uses one bounded project-owned serializer; raw bytes and canonical identity documents are hashed separately."
  - "Source storage is a strict stored-versus-manifest-only union, and normative eligibility is derived only from authorityClass: official."
  - "Recursive graph validation, symlink confinement, stored-byte rehashing, and manifest reference binding remain owned by Plan 01-04."

patterns-established:
  - "Canonical boundary: validate JSON compatibility, work limits, cycles, Unicode, and duplicate keys before hashing."
  - "Diagnostic boundary: escape JSON-Pointer paths and sort by path, code, then message."
  - "Artifact envelope: stableId names the logical artifact while contentHash names the immutable canonical revision."

requirements-completed: [DATA-03]

duration: 28min
completed: 2026-08-20
---

# Phase 1 Plan 2: Canonical Identity and Provenance Contracts Summary

**Bounded canonical JSON and SHA-256 identity with strict, tamper-evident provenance envelopes for every authority artifact kind**

## Performance

- **Duration:** 28 min
- **Started:** 2026-08-20T15:12:21Z
- **Completed:** 2026-08-20T15:40:39Z
- **Tasks:** 2
- **Files modified:** 5

## Accomplishments

- Implemented one RFC 8785-derived canonical serializer with exact UTF-16 key ordering, ECMAScript numeric output, preserved Unicode/array order, fixed work caps, typed failures, cycle rejection, and duplicate-key detection before JSON parsing can erase evidence.
- Added Node SHA-256 helpers that distinguish raw-byte identity from canonical artifact identity and produce stable lowercase `sha256:` digests.
- Added strict Zod schemas and readonly TypeScript contracts for raw/normalized cards, formats, source records, identity documents, canonical artifacts, and authority bundles.
- Enforced mutually exclusive stored and manifest-only sources, confined lexical stored paths, reviewed HTTPS provenance, official-host allowlisting, and official-only normative eligibility.
- Added bounded UTF-8 parsing and deterministically sorted, escaped JSON-Pointer diagnostics with capped output and no coercion, trimming, defaults, inference, dropped fields, or repair.

## Task Commits

Each TDD task was committed as a RED contract followed by its GREEN implementation:

1. **Task 1: Lock canonical JSON and SHA-256 identity**
   - `1ff8d8b` — RED canonical identity contracts
   - `ec1f189` — GREEN canonical serializer and hash helpers
2. **Task 2: Define strict provenance and artifact envelopes**
   - `91bd0c4` — RED provenance/schema contracts
   - `f8e0d31` — GREEN strict schema and tamper-validation boundary

## Files Created/Modified

- `src/authority/canonical-json.ts` - Bounded canonical serialization plus duplicate-aware JSON text scanning.
- `src/authority/hash.ts` - Raw byte and canonical identity SHA-256 helpers.
- `src/authority/schemas.ts` - Shared strict source, card, format, artifact, bundle, parser, and diagnostic contracts.
- `tests/authority/canonical-json.test.ts` - Exact RFC-derived vectors and rejection/resource/hash assertions.
- `tests/authority/provenance.test.ts` - Exact source-mode, envelope, parser, diagnostic, authority, and tamper assertions.

## Decisions Made

- Kept canonicalization and duplicate-key scanning project-owned because ordinary `JSON.parse` erases duplicate keys and ordinary `JSON.stringify` cannot enforce the identity limits or rejection profile.
- Used the already-pinned Zod dependency only at untrusted schema boundaries; canonicalization, UTF-8 decoding, hashing, and tests use Node facilities.
- Encoded normative eligibility as `authorityClass === 'official'` instead of a caller-controlled boolean that community/reference records could misuse.
- Required every canonical identity to carry at least one parent or source reference, while allowing direct and derived artifacts to use the same envelope.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] Rejected terminal unpaired high surrogates**
- **Found during:** Task 1 GREEN verification
- **Issue:** End-of-string surrogate lookahead returned `NaN`, and numeric range comparisons alone did not reject it.
- **Fix:** Required an integer lookahead before accepting a low-surrogate pair.
- **Files modified:** `src/authority/canonical-json.ts`
- **Verification:** The exact Unicode rejection assertion and full canonical suite pass.
- **Committed in:** `ec1f189`

---

**Total deviations:** 1 auto-fixed (1 bug)
**Impact on plan:** The fix is required RFC-compatible input rejection at the shared identity boundary; no scope was added.

## Issues Encountered

- The built-in Windows `apply_patch` sandbox and its command wrapper both failed with the same helper/access errors recorded in Plan 01-01. Repository diffs were applied through `git apply` as the narrow patch-based fallback.
- Context7 was unavailable in both MCP tools and the local CLI, so the installed Zod 4 boundary was checked against official Zod 4 documentation before implementation.
- The GSD progress handler reported 25% but wrote `progress.percent: 0` in state frontmatter; the frontmatter was corrected to match the handler output and visible progress bar.

## Authentication Gates

None.

## Known Stubs

Five intentional `test.todo` contracts remain in `tests/authority/provenance.test.ts` for Plan 01-04:

- Missing/broken parent and source graph references
- Reference cycles
- Stored-source symlink escape
- Stored-source offline byte rehashing
- Manifest-only locator/hash/`SourceRef` binding tampering

They do not block this plan because Plan 01-04 owns filesystem resolution and recursive graph validation.

## Verification Results

- `node --test tests/authority/canonical-json.test.ts tests/authority/provenance.test.ts` - PASS (26 assertions, 5 planned Plan 01-04 todos)
- Task 2 targeted schema/envelope/source/authority/storage/diagnostic/duplicate/tamper pattern - PASS
- `pnpm typecheck` - PASS
- `pnpm lint` - PASS
- `pnpm test` - PASS (59 discovered contracts, 26 passing assertions, 33 future-plan todos)
- Identity call audit - PASS; ordinary `JSON.stringify` appears only inside the canonical serializer.
- Stub/threat scan - PASS; no unplanned network, authentication, filesystem, or schema trust surface was introduced.

## Next Phase Readiness

- Plan 01-03 can consume the strict raw/normalized card schemas, source metadata, canonical artifacts, and hash helpers without redefining identity.
- Plan 01-04 can consume the source/artifact/bundle contracts and replace the five remaining graph/filesystem todos.
- No blockers remain for the next planned authority work.

## Self-Check: PASSED

- All 5 created/modified implementation and test files exist.
- Task commits `1ff8d8b`, `ec1f189`, `91bd0c4`, and `f8e0d31` exist in git history.
- Exact plan verification plus full typecheck, lint, and authority tests passed after the final task commit.

---
*Phase: 01-rules-and-data-authority*
*Completed: 2026-08-20*

