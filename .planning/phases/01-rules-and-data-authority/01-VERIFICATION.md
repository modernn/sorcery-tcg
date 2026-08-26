---
phase: 01-rules-and-data-authority
verified: 2026-08-26T19:52:33Z
status: gaps_found
score: 0/3 must-haves verified
overrides_applied: 0
gaps:
  - truth: "A developer can validate an immutable authority bundle offline and trust that it represents the exact applicable official authority."
    status: failed
    reason: "The collector can accept incomplete source responses or select hidden changelog dates, and precedence can silently select a partial superseder over an unrelated contender."
    artifacts:
      - path: "scripts/collect-private-authority.ps1"
        issue: "Visible-date extraction is not actually limited to visible changelog entries, success/error output can disclose private locators, and source completeness checks are superficial."
      - path: "src/authority/validate-bundle.ts"
        issue: "Partial supersession and impossible effectiveAt dates can resolve instead of failing closed."
    missing:
      - "Bind changelog extraction to the first unique visible entry and reject ambiguity."
      - "Sanitize collector output and validate complete source-specific structures before publication."
      - "Resolve supersession as a graph and semantically validate resolution dates."
  - truth: "A developer can build a complete, game-usable normalized card snapshot from the pinned official card bytes."
    status: failed
    reason: "The official adapter validates but discards avatar life and all elemental thresholds."
    artifacts:
      - path: "src/authority/official-card-api-adapter.ts"
        issue: "adaptCard omits guardian.life and guardian.thresholds."
      - path: "src/authority/schemas.ts"
        issue: "RawCard and NormalizedCard cannot represent life or elemental thresholds."
      - path: "src/authority/normalize-cards.ts"
        issue: "normalizeCard has no path for the discarded fields."
    missing:
      - "Represent and preserve avatar life and all four elemental thresholds with losslessness tests."
  - truth: "Every canonical artifact has stable logical identity and tamper-evident, acyclic provenance."
    status: failed
    reason: "Card IDs change with revision-qualified source IDs, source derivation cycles are accepted, and SHA-256 durable locators are not bound to the declared byte hash."
    artifacts:
      - path: "src/authority/normalize-cards.ts"
        issue: "Card stable IDs hash the revision-qualified sourceId."
      - path: "src/authority/validate-bundle.ts"
        issue: "Source derivation checks reject only self/missing parents, not multi-source cycles."
      - path: "src/authority/schemas.ts"
        issue: "A urn:sha256 locator whose digest differs from byteHash is accepted."
    missing:
      - "Mint card IDs from a revision-independent publisher namespace plus publisher card ID."
      - "Detect cycles and duplicate edges in the source byte-hash derivation graph."
      - "Bind SHA-256 locator digests to byteHash while retaining honest declaration-only states for other locator kinds."
  - truth: "The private-source boundary proves that protected source material and private locators cannot enter Git or packages."
    status: failed
    reason: "The boundary scanner samples only three source excerpts and misses escaped or case-varied locator representations; the collector can also print private lock fields."
    artifacts:
      - path: "scripts/verify-private-authority-boundary.ts"
        issue: "Only start/middle/end markers and native/slash locator spellings are scanned."
      - path: "scripts/collect-private-authority.ps1"
        issue: "Direct success and transport error paths can emit private roots or full request locators."
    missing:
      - "Cover arbitrary source excerpts/normalized representations and escaped/case-folded locator forms across history, index, worktree, and package surfaces."
      - "Emit only fixed sanitized production collector messages."
---

# Phase 1: Rules and Data Authority Verification Report

**Phase Goal:** Developers can identify and reproduce the exact official rules, format, card data, and provenance governing every later game.
**Verified:** 2026-08-26T19:52:33Z
**Status:** gaps_found
**Re-verification:** No — initial verification

## Goal Achievement

### Observable Truths

| # | Roadmap truth | Status | Evidence |
|---|---|---|---|
| 1 | A developer can validate an immutable authority bundle offline and see the pinned official source inventory, precedence, dates, URLs, and hashes. | FAILED | Offline validation and the safe seven-source receipt exist, but scripts/collect-private-authority.ps1:147-153, 289-337 and src/authority/validate-bundle.ts:208-287 do not prove complete/visible acquisition or fail-closed precedence. |
| 2 | A developer can build and validate the same versioned normalized card snapshot without live network access. | FAILED | Deterministic rebuild and no-network gates pass, but src/authority/official-card-api-adapter.ts:56-70,136-154 validates then drops life and thresholds, so the snapshot is reproducible but incomplete for gameplay. |
| 3 | Every canonical rule, card, format, deck, collection, behavior, and experiment artifact resolves to stable identity and complete provenance. | FAILED | The generic envelope exists, but card IDs change when source revision IDs change (src/authority/normalize-cards.ts:28-37), and source derivation cycles are not detected (src/authority/validate-bundle.ts:351-377). |

**Score:** 0/3 roadmap truths verified

No failed item is clearly assigned to a later roadmap phase. Later phases consume this authority layer; they do not replace its acquisition, identity, provenance, precedence, or privacy contracts.

### Required Artifacts

| Artifact | Expected | Status | Details |
|---|---|---|---|
| package.json / pnpm-lock.yaml | Exact one-package Node/TypeScript toolchain and runnable verification | VERIFIED | Node 24.19.0, pnpm 11.22.0, TypeScript 6.0.3, and the intended dependency set are pinned. |
| src/authority/canonical-json.ts / hash.ts | Bounded canonical identity and SHA-256 | VERIFIED | Substantive, imported by schemas/import/validation, and exact-vector tests pass. |
| src/authority/schemas.ts | Strict common artifact/source/card/format contracts | FAILED | Substantive and wired, but the card schema omits required gameplay fields and the manifest SHA locator is not semantically bound to byteHash. |
| src/authority/official-card-api-adapter.ts | Strict bounded official API adaptation | FAILED | Wired through normalizeCards, but it discards validated avatar life and thresholds. |
| src/authority/normalize-cards.ts | Pure deterministic, lossless card normalization | FAILED | Pure/offline and deterministic, but card identity is revision-dependent and the card model is incomplete. |
| src/authority/validate-bundle.ts | Sole fail-closed offline authority gate | FAILED | Substantive and used by both commands, but precedence, date, locator, and source-cycle gaps remain. |
| src/commands/import-authority.ts | Exact-lock, atomic, write-once importer | VERIFIED WITH DEPENDENCY GAPS | Exact-lock and atomic publication tests pass; output quality inherits adapter and validator defects. |
| src/commands/validate-authority.ts | Deterministic read-only validation command | VERIFIED WITH DEPENDENCY GAPS | Correctly delegates to validateAuthorityBundle, but labels and accepts the validator's incomplete manifest evidence. |
| src/authority/private-source-set.ts | Exact independent primary/backup verifier | VERIFIED | Opt-in private gate confirms both roots independently reproduce the selected revision; private content was not inspected by this verifier. |
| scripts/collect-private-authority.ps1 | Closed, fail-closed, non-disclosing one-shot collector | FAILED | Current day-first date parsing is fixed, but hidden-date selection, weak completeness checks, and output disclosure remain. |
| scripts/verify-private-authority-boundary.ts | Git/index/package leakage proof | FAILED | It scans all declared surfaces, but source and locator matching are materially incomplete. |
| data/authority receipt and README | Safe non-content selection and rebuild instructions | WARNING | Safe metadata exists, but docs/external-reuse-policy.md contradicts the importer's private normalized revision format. |

### Key Link Verification

| From | To | Via | Status | Details |
|---|---|---|---|---|
| package.json | tests/authority/*.test.ts | pnpm test | WIRED | Actual script is node --test tests/authority/*.test.ts; the SDK's escaped-pattern check was a false negative. |
| src/authority/hash.ts | src/authority/canonical-json.ts | identityHash calls canonicalJson | WIRED | Direct import and use confirmed. |
| src/authority/normalize-cards.ts | schemas, adapter, hash | strict parse/adapt/validate/create artifact | WIRED BUT LOSSY | Control flow is connected; life and thresholds disappear in the adapter/schema boundary. |
| src/commands/import-authority.ts | normalizeCards and validateAuthorityBundle | local import then candidate validation | WIRED | Lines 340-418 bind locked inputs, normalize, validate, and atomically rename. |
| src/commands/validate-authority.ts | validateAuthorityBundle | read-only CLI adapter | WIRED | Lines 76-119 call the validator and emit canonical success/diagnostics. |
| private source gate | importer inputs and selected revision | independent root verification and double build | WIRED | The 5-test private suite passes without network access. |

### Data-Flow Trace (Level 4)

| Artifact | Data | Source | Produces real data | Status |
|---|---|---|---|---|
| Official card snapshot | NormalizedCardSnapshot | Pinned private API bytes -> adaptOfficialCardApiSnapshot -> normalizeCards | Yes, but missing validated gameplay fields | HOLLOW/PARTIAL |
| Authority revision | Canonical bundle and companions | Exact input lock -> importAuthority -> validateAuthorityBundle -> atomic rename | Yes | FLOWING WITH VALIDATION GAPS |
| Safe receipt | Non-content source/revision metadata | Verified private source-set/revision evidence | Yes | FLOWING |

### Behavioral Spot-Checks

| Behavior | Command | Result | Status |
|---|---|---|---|
| Clean-clone/default verification | pnpm verify | 156 passed; 0 failed/skipped/todo | PASS |
| Opt-in private rebuild/isolation gate | pnpm authority:verify-private | 5 passed; 0 failed/skipped/todo | PASS |
| Partial supersession plus unrelated contender | In-memory resolveAuthorityPrecedence probe | Returned resolved with the partial superseder winning | FAIL |
| Semantic effectiveAt validation | In-memory resolver probe using 2026-99-99 | Returned resolved | FAIL |
| SHA locator binding | In-memory validateSourceRecord probe | Accepted a locator digest different from byteHash | FAIL |
| Official card field preservation | In-memory adapter probe | life=false; thresholds=false | FAIL |
| Cross-revision logical card identity | In-memory normalization probe | Identical publisher card received different stable IDs | FAIL |

### Probe Execution

No shell probes are declared by the Phase 1 plans and no conventional scripts/**/tests/probe-*.sh files exist. Behavioral commands above are the runnable verification surface.

### Requirements Coverage

| Requirement | Source plans | Status | Evidence |
|---|---|---|---|
| DATA-01 | 01-01, 01-04, 01-05, 01-06, 01-07, 01-08 | BLOCKED | Immutable/offline machinery exists, but source completeness, private output, precedence, and date failures prevent trusting the exact authority. |
| DATA-02 | 01-01, 01-03, 01-05, 01-07, 01-08 | BLOCKED | Rebuilds are deterministic and offline, but the normalized official model discards life and thresholds. |
| DATA-03 | 01-01, 01-02, 01-03, 01-04, 01-06, 01-07, 01-08 | BLOCKED | Generic canonical envelopes work, but card IDs are revision-dependent and source derivation can be cyclic. |

No orphaned Phase 1 requirement was found; DATA-01, DATA-02, and DATA-03 are all claimed by multiple plans.

### Code Review Finding Adjudication

| Finding | Verdict | Phase impact | Evidence |
|---|---|---|---|
| CR-01 changelog visibility/date extraction | REAL with stale historical subclaim | BLOCKER | Day-first parsing is now fixed and tested, but Get-NormalizedVisibleText keeps non-visible text and Get-ChangelogDate selects the first date after a raw marker without entry/ambiguity binding. |
| CR-02 collector output leaks private roots/locator | REAL | BLOCKER | Transport errors interpolate full URIs; the core and wrappers return the lock; direct invocation emits the returned object containing private fields. |
| CR-03 partial corpus accepted as complete | REAL | BLOCKER | Card validation accepts any non-empty array of named objects and HTML validation needs only signature/marker/no challenge. The selected card set later passes a strict 1,100-row gate, but acquisition completeness—especially HTML—is not established by the collector. |
| CR-04 partial supersession defeats unrelated contender | REAL | BLOCKER | A synthetic A->B plus unrelated C probe resolved to A because every superseder receives rank 100. |
| CR-05 manifest-only evidence overclaim | PARTIALLY REAL | BLOCKER for SHA locator binding | The generic HTTPS/procedure declaration is intentionally not dereferenced, so that part of the review overstates the contract. However, a urn:sha256 digest different from byteHash is accepted, contradicting the Plan 08 per-source SHA locator binding. |
| CR-06 boundary gate misses excerpts/escaped locators | REAL | BLOCKER | Only three 32-byte markers and native/slash locator spellings are checked. Existing tests select those same sampled/plain forms. |
| CR-07 adapter drops life and thresholds | REAL | BLOCKER | Adapter schema validates both, adaptCard omits both, and RawCard/NormalizedCard cannot carry them. |
| CR-08 card IDs change with authority revision | REAL | BLOCKER | stableHash includes revision-qualified source.sourceId; a direct probe confirmed different IDs for identical publisher card IDs. |
| CR-09 circular source derivation accepted | REAL | BLOCKER | Source checks reject self/missing parents only; DFS cycle detection traverses artifact parentRefs, not derivation.parentByteHashes. |
| WR-01 collector subprocesses can hang | REAL | WARNING | stdout and stderr are drained sequentially, no deadline exists, and the test helper has no timeout/kill path. |
| WR-02 impossible calendar dates resolve | REAL | WARNING, contract-significant | The exported resolver checks only YYYY-MM-DD shape; 2026-99-99 resolved in a direct probe. Schema-validated bundle callers mitigate, but the public resolver contract is still false. |
| WR-03 policy/importer normalized-data conflict | REAL | WARNING | Policy forbids normalized official derivatives from the built revision while importAuthority always writes cards.normalized.json to that private revision. |

### Anti-Patterns Found

No TBD, FIXME, or XXX debt markers; no skipped/todo tests; and no placeholder implementation was found in the Phase 1 source set. The blocking issues are behavioral contract failures, not stubs.

### Human Verification Required

None. The blocking failures are observable from source and read-only automated probes.

### Gaps Summary

Phase 1 has a strong mechanical foundation: the exact toolchain is pinned, canonicalization and hashing are substantive, local import is atomic/write-once, all 156 default tests pass, and the five private rebuild/no-network/leakage tests pass. Those green suites do not exercise several adversarial cases required by the phase goal.

The phase cannot pass until the reusable authority boundary is made fail-closed, the official card model preserves life and thresholds, logical card IDs survive authority refreshes, source derivation is acyclic, and private-data leakage checks/output are closed. These are Phase 1 inputs to every later engine and simulator result and are not safely deferrable.

---

_Verified: 2026-08-26T19:52:33Z_
_Verifier: the agent (gsd-verifier)_
