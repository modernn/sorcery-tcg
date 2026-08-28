---
phase: 01-rules-and-data-authority
reviewed: 2026-08-28T02:41:27Z
depth: standard
files_reviewed: 42
files_reviewed_list:
  - .gitattributes
  - .gitignore
  - data/authority/README.md
  - data/authority/receipts/official-2026-08-20.json
  - data/authority/receipts/official-2026-08-27-v3.json
  - docs/authority-precedence.md
  - docs/external-reuse-policy.md
  - eslint.config.js
  - package.json
  - scripts/collect-private-authority.ps1
  - scripts/verify-private-authority-boundary.ts
  - src/authority/canonical-json.ts
  - src/authority/hash.ts
  - src/authority/normalize-cards.ts
  - src/authority/official-card-api-adapter.ts
  - src/authority/private-source-set.ts
  - src/authority/schemas.ts
  - src/authority/validate-bundle.ts
  - src/commands/import-authority.ts
  - src/commands/validate-authority.ts
  - tests/authority/bundle.test.ts
  - tests/authority/canonical-json.test.ts
  - tests/authority/card-snapshot.test.ts
  - tests/authority/fixtures/bundle-input/cards.raw.json
  - tests/authority/fixtures/bundle-input/formats.json
  - tests/authority/fixtures/bundle-input/input-lock.json
  - tests/authority/fixtures/bundle-input/sources.json
  - tests/authority/fixtures/cards-duplicates.json
  - tests/authority/fixtures/cards-malformed.json
  - tests/authority/fixtures/cards-valid.json
  - tests/authority/fixtures/provenance-cycle.json
  - tests/authority/fixtures/provenance-tampered.json
  - tests/authority/fixtures/provenance-valid.json
  - tests/authority/fixtures/README.md
  - tests/authority/private-authority-boundary.test.ts
  - tests/authority/private-authority-collector.test.ts
  - tests/authority/private-source-set.test.ts
  - tests/authority/provenance.test.ts
  - tests/private-authority/private-revision.test.ts
  - tests/private-authority/private-source-completeness.test.ts
  - tests/private-authority/repository-boundary.test.ts
  - tsconfig.json
findings:
  critical: 12
  warning: 5
  info: 0
  total: 17
status: resolved
---

# Phase 1: Code Review Report

**Reviewed:** 2026-08-28T02:41:27Z
**Depth:** standard
**Files Reviewed:** 42
**Status:** resolved

## Resolution

All critical findings and WR-01, WR-02, WR-04, and WR-05 were resolved in commits `5ca770d`, `3bd7878`, `50778d0`, and `df23e83`. Fresh verification passed typecheck, lint, 167 public tests, and all 8 private-local authority tests. WR-03 is deliberately deferred: the release gate continues rejecting all artwork until Phase 9 has concrete project-owned assets and a reviewed allowlist.

## Summary

The authority implementation has serious fail-closed gaps. Invalid specialized artifacts and unrooted derivations can be accepted, official card input can bypass the official API adapter, supersession can select older authority, trusted artifacts remain mutable after hashing, and several private-boundary paths can miss protected data. One opt-in verification test can also install missing real private state instead of detecting its absence.

The clean-clone-safe `pnpm verify` command passed during review. The ignored private suite was not run or read; its tracked test source was reviewed without accessing `.local/authority`.

## Narrative Findings (AI reviewer)

## Critical Issues

### CR-01: Generic bundles bypass specialized artifact schemas

**File:** `src/authority/validate-bundle.ts:359-365,689-695`
**Issue:** `validateGraph` validates every child with only `validateCanonicalArtifact`, whose payload is arbitrary JSON. The only format/card-snapshot payload validation is skipped entirely when `inputRootHash` is `null`. A hash-consistent artifact can therefore declare `artifactKind: "format"` or `"card-snapshot"` while carrying an invalid payload and still pass bundle validation.
**Fix:** Dispatch by artifact kind in `validateGraph` for every bundle: run `validateFormatArtifact` for formats and `validateNormalizedCardSnapshot` on card-snapshot payloads in addition to canonical hash validation. Add a generic-bundle regression for each specialized kind.

### CR-02: Canonical artifacts remain mutable after their hash is trusted

**File:** `src/authority/schemas.ts:648-663,694-701`
**Issue:** Creation and validation return mutable identities, payloads, arrays, and references; only the outer artifact created at line 698 is frozen. A caller can mutate `artifact.identity.payload` after hashing, leaving `contentHash` stale while the object still appears trusted. `normalizeCards` needs a separate deep freeze, which confirms the generic contract is unsafe.
**Fix:** Deep-freeze the validated/cloned identity before hashing and returning it, and deep-freeze artifacts returned by all validation functions. Add a regression that attempts nested mutation and then rechecks the hash.

### CR-03: Official card inputs can bypass the audited official API adapter

**File:** `src/authority/normalize-cards.ts:63-79`
**Issue:** Parser selection depends only on whether the JSON root is an array. An `official` source whose bytes use the internal `{cards:[...]}` shape bypasses `adaptOfficialCardApiSnapshot`, accepts arbitrary project-shaped records, and mints them in the `sorcerytcg` namespace. The tracked importer fixture exercises this path with official metadata (`tests/authority/fixtures/bundle-input/cards.raw.json:1-19` and `sources.json:3-20`).
**Fix:** Route by trusted source contract, not root shape: official Sorcery card sources must pass `adaptOfficialCardApiSnapshot`; generic raw snapshots must be non-official (or use an explicit separately validated source type). Require the importer card source to be the expected official API source and convert the synthetic official fixture to the audited API shape.

### CR-04: Set-like card fields are neither canonical nor fully validated

**File:** `src/authority/normalize-cards.ts:42,55`; `src/authority/schemas.ts:390-406`
**Issue:** `elements` and `printingSlugs` are copied in input order, and duplicate elements are accepted. Semantically identical cards with reordered element/printing sets produce different normalized payload hashes, while a card such as `elements: ["fire", "fire"]` is accepted. This breaks the phase's normalized semantic identity contract.
**Fix:** Reject duplicate elements and sort elements by a fixed project order; sort printing slugs during normalization after the existing duplicate check. Add order-invariance and duplicate-element regressions.

### CR-05: Derived sources can be accepted without any provenance parent

**File:** `src/authority/schemas.ts:246-249`; `src/authority/validate-bundle.ts:394-428`
**Issue:** `manual-transcription` and `normalized` sources may have an empty `parentByteHashes` array. Bundle validation rejects parents on `verbatim` sources but never requires a parent for derived methods. The shipped format fixture is an accepted unrooted official manual transcription at `tests/authority/fixtures/bundle-input/sources.json:23-39`.
**Fix:** Make derivation validation method-specific: `verbatim` requires exactly zero parents; `normalized` and `manual-transcription` require at least one resolvable parent. Add an `unrooted_derivation` regression and root the format fixture in an actual source record.

### CR-06: An older source can supersede and defeat newer official authority

**File:** `src/authority/validate-bundle.ts:263-315`
**Issue:** Supersession edges are checked for existence and cycles, but never for chronological direction. An older applicable source can name a newer source in `supersedes`; line 299 then removes the newer source and resolves the older one as the winner. This contradicts `docs/authority-precedence.md:23`, which says the newer record wins, and violates fail-closed resolution.
**Fix:** Reject every supersession edge unless the superseder has a strictly later effective date than its target (or add a separate explicit same-day ordering field). Add reverse-date and equal-date supersession regressions.

### CR-07: Path confinement is vulnerable to a check/open race

**File:** `src/authority/validate-bundle.ts:113-141,149-186,878-884`; `src/commands/import-authority.ts:149-173,194-208,322-330`
**Issue:** Paths are realpathed and checked inside the authority root, then reopened later by pathname without verifying the opened handle's identity. A local writer can replace the checked file with a symlink/reparse target between `realpath` and `open`, causing validation/import to read outside the configured root.
**Fix:** Open once, reject links/reparse aliases, compare `lstat`/`realpath` identity with `FileHandle.stat` before and after the bounded read, and fail if the pathname identity changes. Reuse the handle-identity pattern already implemented in `private-source-set.ts`.

### CR-08: The production boundary scanner trusts incomplete lock metadata

**File:** `scripts/verify-private-authority-boundary.ts:777-813`
**Issue:** The scanner checks only that roots are strings, entries is an array, and each listed entry has a path/hash. It never requires the exact seven paths or validates the complete source-set contract. An empty or partial lock builds incomplete fingerprint indexes and can let protected excerpts from omitted sources pass the release gate.
**Fix:** Reuse `verifyPrivateSourceSet` (including the exact seven-path allowlist and root checks) before building fingerprints, and scan only its validated sorted result. Add empty, partial, duplicate, and unknown-entry lock regressions.

### CR-09: Escaped static template literals evade private-content decoding

**File:** `scripts/verify-private-authority-boundary.ts:488-555,630-659`
**Issue:** The decoder recognizes only single- and double-quoted strings. Protected bytes or a private locator encoded with `\\x`/`\\u` escapes inside a static backtick template literal contain no raw match and are never decoded, so they can pass every publication surface.
**Fix:** Decode static backtick literals through the same bounded path, rejecting/interpreting interpolation conservatively, and add escaped template-literal cases for raw excerpts, semantic records, and locators on all four surfaces.

### CR-10: Protected content in publication paths is never scanned

**File:** `scripts/verify-private-authority-boundary.ts:574-582,663-693`
**Issue:** Candidate bytes are scanned, but `candidate.path` is checked only for the literal private directory and artwork extensions. A Git/package filename containing a protected excerpt or locator can be published undetected. If another check fails, `fail` also prints that unscanned path verbatim.
**Fix:** Inspect UTF-8 path bytes with the same raw/normalized/locator indexes before content inspection. If the path itself is sensitive, emit only category and surface with an opaque candidate identifier, never the path.

### CR-11: The supposedly bounded subprocess path can hang forever

**File:** `scripts/collect-private-authority.ps1:278-292`
**Issue:** On timeout, `Kill($true)` failures are swallowed and the code immediately calls parameterless `WaitForExit()`. A process that cannot be terminated blocks collection indefinitely. Output is also fully accumulated by `ReadToEndAsync` before the `MaximumOutputCharacters` truncation, so the advertised output bound is post-hoc.
**Fix:** Treat termination failure as a sanitized hard failure, use a short bounded second wait, and never call parameterless `WaitForExit` on the timeout path. Enforce the output limit while streaming and kill the process tree when it is exceeded.

### CR-12: A verification test installs missing real private authority state

**File:** `tests/private-authority/private-revision.test.ts:281-289,551-582,630-642`
**Issue:** The test permits the selected revision to be absent, then `installWriteOnce` renames the rebuilt candidate into the real ignored selected-revision path. Cleanup restores nothing when `selectedBefore` is `null`. The test can therefore repair and persist missing production evidence, masking the exact failure it should detect.
**Fix:** Require the selected revision to exist, compare its file map with the independently rebuilt temporary candidate, and never install/rename into real authority paths from a test. Keep installation in an explicit non-test command.

## Warnings

### WR-01: Duplicate source byte hashes make derivation traversal order-dependent

**File:** `src/authority/validate-bundle.ts:350-354,431-463`
**Issue:** Duplicate source `byteHash` values are allowed, but both `sourceIndexByHash` and traversal state are keyed only by hash. One source overwrites another in the index and later records with the same hash can have their derivation edges skipped, hiding a cycle or depth violation depending on array order.
**Fix:** Reject duplicate source byte hashes in a bundle, or model traversal nodes by source ID and reject ambiguous hash-to-source parent resolution.

### WR-02: The privacy-critical TypeScript scanner is not typechecked or linted

**File:** `eslint.config.js:3-5`; `tsconfig.json:16`
**Issue:** Both configurations include `src` and `tests` but omit `scripts/**/*.ts`. Consequently `pnpm verify` can pass while `scripts/verify-private-authority-boundary.ts` has type or lint defects.
**Fix:** Include `scripts/**/*.ts` in the TypeScript project and ESLint file patterns.

### WR-03: The boundary scanner rejects all artwork despite the written policy

**File:** `scripts/verify-private-authority-boundary.ts:679-682`; `docs/external-reuse-policy.md:67`
**Issue:** Every image extension or recognized image signature is rejected, including original/project-owned presentation art that the policy explicitly permits for Phase 9. This makes the release gate incompatible with a planned allowed asset class.
**Fix:** Scope rejection to publisher/private artwork evidence, or introduce a small reviewed project-owned asset allowlist before Phase 9. Do not blanket-reject all image files.

### WR-04: Boundary test subprocesses have no time or output bound

**File:** `tests/authority/private-authority-boundary.test.ts:28-41`; `tests/private-authority/repository-boundary.test.ts:225-258`
**Issue:** The async helper accumulates unlimited stdout/stderr and has no timeout/tree kill; both production `spawnSync` calls also omit `timeout`. A hung Git, pnpm, or scanner stalls the test suite indefinitely.
**Fix:** Reuse one bounded child helper with a finite timeout, incremental output cap, and process-tree termination; set `timeout` on synchronous calls.

### WR-05: Non-disclosure tests do not assert that matched content stays out of diagnostics

**File:** `tests/authority/private-authority-boundary.test.ts:332-337,390-394,474-480`
**Issue:** Negative cases check that roots and one host are absent from stderr, but do not assert that the actual matched excerpt, decoded record, or other distinctive protected content is absent. A regression that prints the offending content can still pass.
**Fix:** For each synthetic case, assert stderr excludes the exact candidate content and distinctive decoded fields while allowing only the fixed category, surface, and safe path/identifier.

---

_Reviewed: 2026-08-28T02:41:27Z_
_Reviewer: the agent (gsd-code-reviewer)_
_Depth: standard_
