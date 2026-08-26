---
phase: quick-260825-mhh-allow-one-explicitly-user-authorized-age
plan: "01"
subsystem: authority-data
tags: [powershell, one-shot-authorization, provenance, private-storage, sha256]
requires:
  - phase: 01-rules-and-data-authority
    provides: Fixed seven-source collector, source-set verifier, and private boundary policy
provides:
  - Two independently consumed fixed agent authorization references with no standing permission
  - One verified private seven-source official authority set and byte-identical independent backup
  - Git, index, and package leakage proof for the completed private source set
affects: [01-rules-and-data-authority, authority-import, provenance]
tech-stack:
  added: []
  patterns: [FileMode.CreateNew authorization consumption, receipt-last publication, lock-driven leakage verification]
key-files:
  created:
    - .planning/quick/260825-mhh-allow-one-explicitly-user-authorized-age/260825-mhh-SUMMARY.md
  modified:
    - scripts/collect-private-authority.ps1
    - tests/authority/private-authority-collector.test.ts
    - scripts/verify-private-authority-boundary.ts
    - tests/authority/private-authority-boundary.test.ts
    - docs/external-reuse-policy.md
    - .planning/phases/01-rules-and-data-authority/01-CONTEXT.md
    - .planning/phases/01-rules-and-data-authority/01-07-PLAN.md
key-decisions:
  - "Each exact agent authorization has its own immutable CreateNew consumption record; the original record was never reset or reused."
  - "The fresh retry reference quick-260825-mhh-retry-1 authorized exactly one production invocation after the PDF and changelog parser fixes."
  - "All official bytes, absolute private locators, and collector output remain outside Git and packages."
patterns-established:
  - "Closed authorization set: absent user-run evidence or one of two exact case-sensitive agent references; every other value fails before transport."
  - "Private acceptance: validate lock metadata before opening bytes, then independently verify both roots and run the Git/index/package content gate."
requirements-completed: [DATA-01, DATA-02, DATA-03]
duration: 3h 49m
completed: 2026-08-25
---

# Quick Task 260825-mhh: Authorized Private Authority Collection Summary

**A fresh, independently consumed one-shot authorization produced and verified the fixed seven-source private authority set and its byte-identical independent backup without leaking official content or private locators.**

## Performance

- **Duration:** 3h 49m
- **Started:** 2026-08-26T00:46:16Z
- **Completed:** 2026-08-26T04:35:01Z
- **Tasks:** 3
- **Tracked files modified:** 7

## Accomplishments

- Added a second exact, case-sensitive authorization reference, `quick-260825-mhh-retry-1`, with a distinct durable `FileMode.CreateNew` consumption record; the original `quick-260825-mhh` record remains consumed and unchanged.
- Invoked the production collector exactly once under the fresh reference with the required risk acknowledgment and no retry.
- Closed the verifier gap by requiring and propagating that acknowledgment through every exported collection entry before path resolution, authorization consumption, output creation, or transport; lock evidence now derives from the validated value.
- Independently accepted all seven metadata rows, ordinary-file identities, primary/backup byte equality, source-set root hash, rulebook evidence, private lock, and repository/package leakage boundary.
- Redacted one historical tracked occurrence of the non-normative rulebook locator discovered by the live leakage gate; the locator now remains only in ignored private evidence.

## Acquisition Receipt

- **Acquisition method:** `user-authorized-agent-run-one-shot-powershell`
- **Authorization reference:** `quick-260825-mhh-retry-1`
- **Primary label:** `.local/authority/inputs/official-2026-08-20/primary/`
- **Backup label:** `independent-private-backup-retry-1`
- **Backup locator fingerprint:** `sha256:28be9b6c8fbe3c5c3b417f6591f4da1ebd414542531e810cc7727ce5b5a9df57`
- **Source-set root hash:** `sha256:29111858fda613d6a75f4eb6681ca1ac8d3d389d87d03bf8431da86f7b229a6a`
- **Private-lock hash:** `sha256:5b6799c9c52c9f18fbfd2960f353592087d0dfcb4fd8e3e4b0714dc9b102cee9`
- **Standard rulebook observed filename:** `SorceryRulebook.pdf`
- **Derivation note:** The collector selected the PDF from the official December 2025 rulebook release page; its private locator is non-normative and retained only in the ignored lock.

## Verified Source Metadata

| Relative path | Official source URL | Retrieved at | Effective date | Media type | Bytes | SHA-256 |
|---|---|---|---|---|---:|---|
| `cards/cards.raw.json` | `https://api.sorcerytcg.com/api/cards` | `2026-08-26T03:54:14.290Z` | — | `application/json` | 1,409,089 | `sha256:55adfa29215ff81b764def80ff696e7925e51eaea7434c53e5fdc48bbbd45309` |
| `codex/changelog-current.html` | `https://curiosa.io/codex/changelog` | `2026-08-26T03:54:11.359Z` | `2026-05-19` | `text/html` | 629,757 | `sha256:434ae4b0abe1391bf5c3f32456ee962f0193bfedd9d0e1b368b13687e238610c` |
| `codex/codex-current.html` | `https://curiosa.io/codex` | `2026-08-26T03:54:10.866Z` | — | `text/html` | 494,736 | `sha256:143e1754d0c4ee91a18718755e9d250af2e525cc57ab54a117b30166552b309b` |
| `codex/faqs-current.html` | `https://curiosa.io/faqs` | `2026-08-26T03:54:11.008Z` | — | `text/html` | 1,137,176 | `sha256:f06813e4b000747713c8d7b891af0e8a575476e4031aa4cdeeb97707476a810d` |
| `formats/constructed-current.html` | `https://sorcerytcg.com/constructed` | `2026-08-26T03:54:10.405Z` | — | `text/html` | 33,825 | `sha256:b5c60cb777a423bd832b57b154b9cf704bdf00df0078d2660e98a2b70b91f721` |
| `rulebook/rulebook-current.pdf` | `https://sorcerytcg.com/news/sorcery-contested-realm-december-2025-rulebook-update` | `2026-08-26T03:54:10.256Z` | `2025-12-19` | `application/pdf` | 72,701,511 | `sha256:7d44d414bba5699dac7fb2e9db45a523f87f8d875ab466272773f17602d5ba10` |
| `updates/card-updates-2025.html` | `https://sorcerytcg.com/news/sorcery-contested-realm-card-updates-2025` | `2026-08-26T03:54:11.621Z` | `2025-11-25` | `text/html` | 79,225 | `sha256:49c1f891c60befc15427c250b850585628c5e266167a627c1ec578a61b09309a` |

## Operating Acknowledgment

```json
{
  "scope": "private-local-noncommercial",
  "noRedistributionReleaseHostingUploadOrArtwork": true,
  "apiTermsRobotsConflictAndPrivateUseRiskAccepted": true,
  "establishesLegalPermission": false,
  "stopOnBlockedStatusCaptchaOrPublisherObjection": true,
  "retryOrEvasion": false
}
```

This receipt records technical provenance and the user's narrow risk acceptance. Authorization and hashes establish neither publisher authenticity nor legal permission. The source set remains private, local, noncommercial, unshared, unpackaged, and artwork-free.

## Task Commits

1. **Task 1: Add and harden the one-shot authorization boundary** — `638d16d`, `2ddb50c`, `26063cd`, `5cdb9d3`, `5723c22`, `6b05b86`
2. **Task 2: Reconcile operating policy and planning contracts** — `9f9509f`, `5723c22`
3. **Task 3: Execute once and independently validate the private set** — `d67cbd6` (tracked leakage remediation; private collection outputs remain ignored and uncommitted)

## Verification

- Focused private collector suite: passed, including independent consumption of both exact references, PDF EOF variants, and official day-first changelog dates.
- Independent metadata and byte-set validation: passed for 7/7 exact sources; primary and backup entries and root hash match the lock.
- Private boundary gate: passed across Git HEAD, index, and actual `pnpm pack --dry-run --json` contents.
- Full repository verification: passed, 153 tests with zero failures.
- Git/index checks: `.local/authority` is ignored, untracked, unstaged, and absent from package contents.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] Accepted standard PDF EOF line endings**
- **Found during:** Initial authorized collection validation
- **Issue:** Valid PDFs ending with `%EOF` plus LF, CR, or CRLF were rejected.
- **Fix:** Preserved the PDF prefix check while accepting only no trailing byte or one standard line ending and rejecting arbitrary trailing bytes.
- **Files modified:** `scripts/collect-private-authority.ps1`, `tests/authority/private-authority-collector.test.ts`
- **Committed in:** `26063cd`

**2. [Rule 1 - Bug] Accepted the official changelog's day-first date**
- **Found during:** Initial authorized collection validation
- **Issue:** The official changelog used a valid day-first date form outside the parser's accepted formats.
- **Fix:** Added strict day-first parsing with impossible-date rejection.
- **Files modified:** `scripts/collect-private-authority.ps1`, `tests/authority/private-authority-collector.test.ts`
- **Committed in:** `5cdb9d3`

**3. [Rule 2 - Security] Removed a tracked private locator found by the live leakage gate**
- **Found during:** Task 3 post-collection boundary validation
- **Issue:** Historical tracked research contained the lock-matched non-normative rulebook locator.
- **Fix:** Redacted the single exact occurrence without weakening the locator gate; private evidence retains it under the ignored boundary.
- **Files modified:** `.planning/quick/260820-hhu-build-and-validate-a-user-run-powershell/260820-hhu-RESEARCH.md`
- **Committed in:** `d67cbd6`

**4. [Rule 1 - Bug] Required truthful risk acknowledgment on every exported collection path**
- **Found during:** Post-execution verifier review
- **Issue:** The direct script wrapper required `AcknowledgePrivateUseRisk`, but the exported production function omitted the switch while the core unconditionally recorded acceptance.
- **Fix:** Propagated the switch through both exported entry points into a shared pre-work core guard and derived the lock field from the validated value; added an offline no-request/no-output/no-consumption regression.
- **Files modified:** `scripts/collect-private-authority.ps1`, `tests/authority/private-authority-collector.test.ts`
- **Committed in:** `6b05b86`

**Total deviations:** 4 auto-fixed (3 bugs, 1 security boundary fix)

## Issues Encountered

- Two read-only preflight expressions failed before reaching the collector because PowerShell required date-preserving JSON parsing. The fresh authorization record remained absent, no network call occurred, and the corrected preflight then reached the one permitted collector invocation.
- The first post-run acceptance expression included two unsupported validator assumptions. Boolean-only diagnostics confirmed the artifacts were valid; the corrected exact plan contract passed without changing or recollecting any private data.

## User Setup Required

None. Private artifacts and the independent backup must remain in their current private locations and must not be moved into Git, packages, shared storage, or hosted services.

## Next Phase Readiness

- The ignored source-set lock, primary, and independent backup are ready for Plan 01-07's independent validation-only handoff.
- Broader agent acquisition, recurring updates, redistribution, artwork, public APIs, and commercial use remain blocked pending written publisher permission and a separate approved plan.

## Self-Check: PASSED

- Summary file exists and every referenced task commit resolves.
- The private source set remains ignored, untracked, unstaged, and outside package contents.
- The summary contains no consumption timestamp, private absolute locator, official source excerpt, normalized corpus, or artwork.

---
*Quick task: 260825-mhh-allow-one-explicitly-user-authorized-age*
*Completed: 2026-08-25*
