---
phase: 01-rules-and-data-authority
plan: "08"
subsystem: authority-data
tags: [typescript, official-api-adapter, deterministic-import, provenance, private-boundary]
requires:
  - phase: 01-07
    provides: verified seven-source primary/backup evidence and closed one-shot authorization receipt
provides:
  - strict bounded adapter for the pinned 1,100-record official card API array
  - immutable offline authority revision selected by fixed stable ID and bundle root
  - independent double-build, tamper, no-network, Git/index, and package leakage gates
affects: [phase-02, simulator-catalog, deck-building, model-testing, local-gui]
tech-stack:
  added: []
  patterns:
    - preserve exact source bytes and bind provenance before deriving project records in memory
    - select local authority only by immutable stable ID plus independently verified root hash
    - keep private full-set verification outside the clean-clone default test glob
key-files:
  created:
    - src/authority/official-card-api-adapter.ts
    - data/authority/README.md
    - data/authority/receipts/official-2026-08-20.json
    - tests/private-authority/private-revision.test.ts
    - tests/private-authority/repository-boundary.test.ts
  modified:
    - src/authority/canonical-json.ts
    - src/authority/schemas.ts
    - src/authority/normalize-cards.ts
    - src/authority/validate-bundle.ts
    - tests/authority/card-snapshot.test.ts
    - package.json
key-decisions:
  - "Adapt the exact publisher API array through a strict bounded in-memory TypeScript adapter while hashing and retaining provenance for the unchanged original bytes."
  - "Treat derivation parent hashes as bundle-local integrity references: claimed parents must resolve to another source and cannot self-reference."
  - "Keep the real authority revision and full corpus private and ignored; commit only code, synthetic/default tests, safe non-content receipt metadata, and opt-in private gates."
patterns-established:
  - "Authority adaptation: validate the complete external shape strictly, reject unknown or malformed fields, and map deterministically without repairing or relabeling source bytes."
  - "Private release gate: stage the whole intended set, verify default tests plus private rebuild/no-network/leakage tests, then partition unchanged content into atomic commits."
requirements-completed: [DATA-01, DATA-02, DATA-03]
duration: 33 min
completed: 2026-08-26
---

# Phase 1 Plan 8: Official Authority Revision Summary

**A fixed offline authority bundle now derives 1,100 normalized card records from unchanged hash-bound official API bytes and is protected by independent rebuild, tamper, no-network, and repository/package leakage gates.**

## Performance

- **Duration:** 33 min continuation execution
- **Started:** 2026-08-26T18:42:39Z
- **Completed:** 2026-08-26T19:15:39Z
- **Tasks:** 3
- **Plan-owned files:** 12 including this summary

## Accomplishments

- Reverified both complete seven-source roots offline and independently derived byte-identical four-file importer trees.
- Added a strict bounded official-API adapter that preserves the original source hash while deterministically producing all 1,100 project card records.
- Built and validated the write-once local revision as `bundle:official-2026-08-20` with a fixed external root hash.
- Proved 43 evidence/source/input/artifact/reference tamper assertions fail closed while the selected four-file revision map remains unchanged.
- Proved importer, adapter, normalizer, and validator paths remain offline under throwing fetch/HTTP/HTTPS/net sentinels.
- Scanned reachable Git history, the worktree, index/staged blobs, and actual `pnpm pack --dry-run --json` contents with no private bytes, locators, publisher documents, artwork, corpus, or community-data leakage.

## Safe Build Receipt

- **Acquisition method:** `user-authorized-agent-run-one-shot-powershell`
- **Authorization reference:** `quick-260825-mhh-retry-1`
- **Consumed-record check:** passed against the exact five-field durable private record; its timestamp and locator are intentionally omitted.
- **Source count:** 7
- **Source-set root:** `sha256:29111858fda613d6a75f4eb6681ca1ac8d3d389d87d03bf8431da86f7b229a6a`
- **Primary importer input root:** `sha256:9fc2607b85d304c98e4cf0dbd7309459d655b9ec25ee106e4763ddfedcc54a16`
- **Backup importer input root:** `sha256:9fc2607b85d304c98e4cf0dbd7309459d655b9ec25ee106e4763ddfedcc54a16`
- **Primary importer four-file map:** `sha256:5b910b850f48b356a20146224c2593548c3f27e3b9fe9620e2b4f74b6c6aa1de`
- **Backup importer four-file map:** `sha256:5b910b850f48b356a20146224c2593548c3f27e3b9fe9620e2b4f74b6c6aa1de`
- **Selected and candidate stable ID:** `bundle:official-2026-08-20`
- **Selected and both candidate bundle roots:** `sha256:8d9719999c1a36595876cc1b045fc237fe9f5cc1e723950b8000a6e428e2a36d`
- **Selected revision before/after:** 4 files; identical path/length/hash map `sha256:33d8d8823c732d8f6031f3ee82ee3b88c9970db3a7e70dc864dc05b3557cf98d`
- **Operating boundary:** private/local/noncommercial; no publisher-permission claim, redistribution, public API, third-party upload, or artwork.

## Task Commits

Each task was committed atomically after the combined staged-set gate passed:

1. **Task 1: Materialize locked inputs and build the private revision** - `d9068bf` (feat)
2. **Task 2: Prove full-set deterministic rebuild integrity and tamper resistance** - `e1a41d1` (test)
3. **Task 3: Prove no network path and zero Git/package leakage** - `af11969` (test)

**Plan contract corrections:** `7da44ee`, `9825243` (docs)

## Files Created/Modified

- `src/authority/official-card-api-adapter.ts` - Strict bounded mapping for the audited official API array.
- `src/authority/canonical-json.ts` - Bounded canonical workload sized for the verified source.
- `src/authority/schemas.ts` - Nullable rarity and audited printing-slug grammar.
- `src/authority/normalize-cards.ts` - Root-shape dispatch while retaining legacy synthetic snapshot support and original-byte provenance.
- `src/authority/validate-bundle.ts` - Fail-closed derivation parent binding checks.
- `data/authority/README.md` - Local immutable-selection, rebuild, isolation, and permission-trigger instructions.
- `data/authority/receipts/official-2026-08-20.json` - Safe non-content source and revision receipt.
- `tests/authority/card-snapshot.test.ts` - Clean-clone adapter shape, rejection, bounds, and provenance coverage.
- `tests/private-authority/private-revision.test.ts` - Exact evidence, double-build, tamper, and immutability gate.
- `tests/private-authority/repository-boundary.test.ts` - No-network plus history/worktree/index/package boundary gate.
- `package.json` - Explicit `authority:verify-private` command outside default tests.

## Decisions Made

- The official API source is an array, so the adapter is isolated behind `normalizeCards`; the original project `{cards: ...}` shape remains supported for synthetic clean-clone tests.
- Official printing slugs remain source identifiers and nullable publisher rarity remains nullable; unsupported audited fields are strictly validated but are not invented into the project schema.
- Canonical parsing retains fixed node/string ceilings sized above the verified 1,100-record source rather than weakening trust-boundary validation.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] Corrected the plan contract for the actual official API source shape**
- **Found during:** Task 1
- **Issue:** The pinned source is a strict 1,100-record array while the importer accepted only the project wrapper and the old canonical node bound rejected the complete source.
- **Fix:** Approved narrow plan corrections admitted a strict bounded adapter and source-sized canonical ceilings without changing original bytes or provenance.
- **Files modified:** Plan 01-08, adapter, normalizer, schema, canonical parser, and card regression.
- **Verification:** 15 focused card tests and TypeScript validation pass.

**2. [Rule 2 - Missing Critical] Bound derivation parent hashes to bundle sources**
- **Found during:** Task 2
- **Issue:** A recomputed input lock could preserve a forged derived-parent hash because parent hashes previously had syntax and cardinality validation only.
- **Fix:** Claimed parent hashes must resolve to another source in the bundle; verbatim/self-parent claims fail closed.
- **Files modified:** Plan 01-08, validator, and private tamper regression.
- **Verification:** Affected validator suites pass 38/38 and the derived-parent tamper is rejected.

**3. [Rule 3 - Blocking] Finalized safe metadata after Windows child-process invocation failed**
- **Found during:** Task 1
- **Issue:** The offline build completed derivation but its local Node wrapper could not spawn `pnpm.cmd` on Windows.
- **Fix:** Ran the exact importer and validator commands directly, then finalized only the safe receipt/README from the already verified lock and revision without rerunning acquisition or import.
- **Files modified:** Safe README and receipt only; ignored private build state remained outside Git.
- **Verification:** Fixed stable ID, external bundle hash, and both complete source/input roots revalidated offline.

---

**Total deviations:** 3 auto-fixed (1 bug, 1 missing critical integrity check, 1 blocking platform issue)
**Impact on plan:** All changes were required for the real pinned source, fail-closed provenance, or offline Windows execution; no network, redistribution, artwork, or public API scope was added.

## Issues Encountered

- The first collector attempts had already produced durable consumed evidence and partial destinations; Plan 01-08 used only the verified completed retry evidence and never reran collection.
- The repository boundary scan is intentionally comprehensive and takes roughly 45 seconds because it opens every reachable, worktree, index, and package candidate blob.

## User Setup Required

None for the current machine. A clean clone cannot rebuild or run the opt-in private gate without the separately retained complete private source set and ignored lock/revision; that failure is intentional.

## Next Phase Readiness

- Phase 2 and later simulator work can consume the fixed local authority by stable ID and root hash with no runtime network dependency.
- Broader acquisition, sharing, hosting, public/network APIs, artwork, or commercial use remains blocked pending written publisher permission and a separate approved plan.

## Self-Check: PASSED

All plan-owned files and the selected ignored revision exist. Plan corrections and all three atomic task commits are present in Git history; the final metadata commit is intentionally created after this self-check.

---
*Phase: 01-rules-and-data-authority*
*Completed: 2026-08-26*
