---
phase: 01-rules-and-data-authority
reviewed: 2026-08-26T19:36:29Z
depth: standard
files_reviewed: 35
files_reviewed_list:
  - data/authority/README.md
  - data/authority/receipts/official-2026-08-20.json
  - docs/authority-precedence.md
  - docs/external-reuse-policy.md
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
  - tests/private-authority/repository-boundary.test.ts
findings:
  critical: 9
  warning: 3
  info: 0
  total: 12
status: issues_found
---

# Phase 1: Code Review Report

**Reviewed:** 2026-08-26T19:36:29Z  
**Depth:** standard  
**Files Reviewed:** 35  
**Status:** issues_found

## Summary

The authority pipeline has strong hashing and filesystem checks, but nine defects undermine its fail-closed and lossless contracts. In addition to collector, precedence, provenance, and boundary failures, normalization drops game-critical fields, card IDs change with each revision-qualified source ID, and source derivation cycles are accepted.

## Critical Issues

### CR-01: Changelog extraction can fail on current markup or select a hidden date

**Classification:** BLOCKER  
**File:** `scripts/collect-private-authority.ps1:147-153,309-337`  
**Issue:** `Get-NormalizedVisibleText` strips tags but retains script, style, comment, and head text. `Get-ChangelogDate` then takes the first ISO or full-English-month date anywhere after a raw marker. This both caused the reported live failure at line 323 and can silently select a hidden build/metadata date instead of the first visible changelog entry. The synthetic test fixture at `tests/authority/private-authority-collector.test.ts:29-40` cannot detect either failure mode.

**Fix:** Remove non-visible elements before extracting text and bind the date to the first changelog entry, preferably a unique `<time datetime>` element. Test the exact current publisher structure, a hidden earlier date, an ambiguous pair, and the publisher's actual date spelling; reject ambiguity.

### CR-02: Direct collector output leaks private roots and the Drive locator

**Classification:** BLOCKER  
**File:** `scripts/collect-private-authority.ps1:197-218,845-887,960-963`  
**Issue:** Transport errors interpolate the full request URI, including the private Drive file ID/query. On success the core returns the complete lock object, and the direct script invocation emits that object through PowerShell's output stream. That object contains `primaryRoot`, `backupRoot`, and `privateLocatorEvidence`, contradicting the policy that private absolute locators exist only in the ignored lock and allowing terminal/transcript logs to retain them.

**Fix:** Sanitize shared transport errors to a fixed source label and approved host without path/query. Suppress the lock object in the direct wrapper and emit only a constant success message. Add canary roots/locators and assert neither stdout nor stderr contains them.

### CR-03: Superficial source checks can publish a partial corpus as complete authority

**Classification:** BLOCKER  
**File:** `scripts/collect-private-authority.ps1:274-306,442-472`  
**Issue:** HTML authority is accepted from a doctype, title marker, and absence of challenge words. Card authority is accepted from any nonempty JSON array whose members merely have a nonblank `name`; the happy fixture is one such card. A publisher shell/error page or a truncated-but-valid one-card response can therefore be hashed, backed up, and locked as the complete seven-source authority set. `verifyPrivateSourceSet` only checks that card JSON has an object/array root (`src/authority/private-source-set.ts:442-458`), so it does not close this gap.

**Fix:** Validate the card payload with the production official API adapter before publication and enforce the pinned revision's reviewed cardinality/baseline invariants. Add source-specific structural markers for HTML. Test one-card, duplicate-printing-ID, missing-field, title-only, and partial-shell rejection.

### CR-04: Partial supersession silently defeats an unrelated viable contender

**Classification:** BLOCKER  
**File:** `src/authority/validate-bundle.ts:258-277`  
**Issue:** Any record with a nonempty `supersedes` array receives rank 100. If A supersedes B while unrelated C remains applicable, B is removed and A silently outranks C even though no relationship resolves A versus C. This violates `docs/authority-precedence.md:30-41`, which requires ambiguous or otherwise unresolved official conflicts to return `unsupported` rather than guess.

**Fix:** Resolve supersession as a graph. A unique winner exists only when exactly one viable node remains after applying all valid supersession edges (or when the remaining peers are uniquely resolved by the documented non-supersession rank/date rules). Add A→B plus unrelated C, competing superseders, and transitive/cyclic tests.

### CR-05: Manifest-only evidence is reported as verified without verifying its locator or procedure

**Classification:** BLOCKER  
**File:** `src/authority/schemas.ts:205-208,270-291`; `src/authority/validate-bundle.ts:838-841`  
**Issue:** The schema requires a locator or procedure hash syntactically, but never binds a `urn:sha256:*` locator to `source.byteHash` and never resolves or verifies `acquisitionProcedureHash`. Validation nevertheless returns every manifest-only source as `manifestBindingsVerified`, and the command emits `verification: manifest-binding-verified`. The tamper test at `tests/authority/provenance.test.ts:510-539` only catches stale outer hashes; recomputing the bundle hash makes the mismatched locator/procedure pass.

**Fix:** Require a URN locator digest to equal `byteHash`. Define and verify the acquisition-procedure binding when that alternative is used. For HTTPS-only declarations that cannot be checked offline, report an honest state such as `manifest-declaration-bound` rather than `verified`, and add recomputed-hash negative tests.

### CR-06: The private-boundary gate misses most copied excerpts and normalized derivatives

**Classification:** BLOCKER  
**File:** `scripts/verify-private-authority-boundary.ts:90-94,116-133,155-162`  
**Issue:** Each private source contributes only three 32-byte markers: start, midpoint, and end. A committed excerpt from any other offset, or a normalized/re-encoded derivative, passes unless it independently triggers the exact-file or artwork checks. Private-locator scanning likewise checks only native and slash-normalized bytes, so JSON/TypeScript-escaped Windows paths and case variants evade the gate. The tests deliberately use the midpoint marker and a plain-text locator (`tests/authority/private-authority-boundary.test.ts:175-186`) and therefore prove only the sampled cases.

**Fix:** Replace three-position sampling with a deterministic comprehensive fingerprint strategy over the prohibited source and normalized corpus. Scan native, slash-normalized, JSON-escaped, and Windows case-folded locator forms. Add arbitrary-offset, reordered JSON, partial-card, encoded-derivative, JSON-string, and TypeScript-string negatives across committed, staged, and packaged surfaces.

### CR-07: Official API normalization drops avatar life and elemental thresholds

**Classification:** BLOCKER  
**File:** `src/authority/official-card-api-adapter.ts:62-70,136-154`; `src/authority/schemas.ts:90-120`  
**Issue:** The strict adapter validates `guardian.life` and all four `guardian.thresholds`, then `adaptCard` omits them. `RawCard` and `NormalizedCard` cannot represent either field. The snapshot is therefore not lossless and lacks authoritative inputs needed for avatar life and threshold-based casting/deck logic.

**Fix:** Add typed life and four-element threshold fields to raw and normalized cards, copy them in `adaptCard` and `normalizeCard`, and add nonzero-threshold/avatar-life preservation assertions.

### CR-08: Project card IDs change when the authority revision changes

**Classification:** BLOCKER  
**File:** `src/authority/normalize-cards.ts:28-37`; `tests/private-authority/private-revision.test.ts:118-126`  
**Issue:** Every card stable ID hashes `source.sourceId`, while the real source-ID convention includes the acquisition revision. Identical publisher card IDs under the next authority revision therefore receive different project IDs, breaking saved decks, owned-collection mappings, longitudinal simulations, and update reconciliation.

**Fix:** Derive official card IDs from a revision-independent publisher namespace plus the publisher card ID. Keep the revision-qualified source ID only in `SourceRef`. Add a cross-revision stable-ID test.

### CR-09: Circular source derivations pass provenance validation

**Classification:** BLOCKER  
**File:** `src/authority/validate-bundle.ts:351-377`  
**Issue:** Source derivation validation rejects missing and self parents but never traverses the byte-hash graph. Two normalized/manual sources can name each other's byte hashes and pass, leaving circular provenance with no rooted evidence.

**Fix:** Build the source byte-hash derivation graph, reject duplicate parent hashes and cycles deterministically, and add a recomputed-hash two-source cycle fixture.

## Warnings

### WR-01: Collector subprocesses can hang indefinitely

**Classification:** WARNING  
**File:** `scripts/collect-private-authority.ps1:743-766`; `tests/authority/private-authority-collector.test.ts:55-75`  
**Issue:** The verifier has no deadline and drains stdout completely before stderr. A hung child never returns; a sufficiently chatty stderr can block the child while the parent waits on stdout. The test process helper also has no timeout, so this regression can hang the suite.

**Fix:** Read both streams concurrently, enforce a bounded wait, kill the child on expiry, and add the same timeout/cleanup behavior to `runPwsh`.


### WR-02: Public precedence resolution accepts impossible calendar dates

**Classification:** WARNING  
**File:** `src/authority/validate-bundle.ts:222-237`  
**Issue:** `effectiveAt` is checked only with `^\d{4}-\d{2}-\d{2}$`. Values such as `2026-99-99` are treated as valid and can resolve a winner, contrary to the documented requirement for a valid resolution date. Bundle callers currently receive schema-validated dates, but the exported resolver itself does not enforce its contract.

**Fix:** Perform semantic UTC date validation before lexical comparison and add impossible-day/month and leap-day tests.

### WR-03: Storage policy and importer disagree about normalized official card data

**Classification:** WARNING  
**File:** `docs/external-reuse-policy.md:63-75`; `src/commands/import-authority.ts:399-405`  
**Issue:** The policy says normalized official derivatives are forbidden from the built revision, while every import writes `cards.normalized.json`; the expected file is asserted at `tests/authority/bundle.test.ts:542-546`. The implementation and operating/legal boundary therefore cannot both be correct.

**Fix:** Make an explicit recorded decision: either amend the policy to permit normalized derivatives only inside the ignored private local revision, or keep the prohibition and change the revision format. Add a policy-boundary assertion matching that decision.

---

_Reviewed: 2026-08-26T19:36:29Z_  
_Reviewer: the agent (gsd-code-reviewer)_  
_Depth: standard_
