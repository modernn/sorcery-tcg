---
phase: 01-rules-and-data-authority
plan: "07"
subsystem: authority-data
tags: [private-source-verification, sha256, provenance, powershell, git-boundary]
requires:
  - phase: 01-06
    provides: official-only precedence, private-local policy, and Git/package exclusions
provides:
  - dependency-free cross-platform verification of an exact seven-file primary and independent backup source set
  - safe non-content receipt for the verified one-shot private authority collection
  - Git, index, and package leakage proof without committing official bytes or private locators
affects: [01-08, authority-import, provenance, private-revision]
tech-stack:
  added: []
  patterns:
    - realpath containment plus ordinary-file and dev/ino identity checks
    - deterministic path-sorted metadata and byte-hash evidence
    - validation-only recovery over ignored private evidence
key-files:
  created:
    - src/authority/private-source-set.ts
    - tests/authority/private-source-set.test.ts
    - .planning/phases/01-rules-and-data-authority/01-07-SUMMARY.md
  modified:
    - scripts/collect-private-authority.ps1
    - tests/authority/private-authority-collector.test.ts
    - scripts/verify-private-authority-boundary.ts
    - tests/authority/private-authority-boundary.test.ts
    - docs/external-reuse-policy.md
    - .planning/phases/01-rules-and-data-authority/01-CONTEXT.md
    - .planning/phases/01-rules-and-data-authority/01-07-PLAN.md
key-decisions:
  - "Accept the exact consumed quick-260825-mhh-retry-1 agent-run evidence pair while preserving every private-local, noncommercial, no-redistribution, no-upload, no-artwork, and no-retry/evasion limit."
  - "Treat hashes as byte-identity evidence only, not publisher authenticity or legal permission."
  - "Keep DATA-01, DATA-02, and DATA-03 pending until Plan 01-08 is corrected and completed."
patterns-established:
  - "Private verification: validate closed receipt metadata before opening bytes, then verify both complete roots and the repository/package boundary."
  - "Safe receipts: publish relative labels, public provenance metadata, and fingerprints only; never private locators, excerpts, or source bytes."
requirements-completed: []
requirements-progress: [DATA-01, DATA-02, DATA-03]
duration: 5 min
completed: 2026-08-26
---

# Phase 1 Plan 7: Private Source-Set Verification Summary

**A locked seven-source official authority set and independent byte-identical backup verified locally with deterministic metadata, identity, and leakage gates while all source bytes remain private.**

## Performance

- **Duration:** 5 min safe-resume closeout
- **Started:** 2026-08-26T04:56:15Z
- **Completed:** 2026-08-26T05:01:15Z
- **Tasks:** 2
- **01-07-owned tracked files:** 3 (implementation, focused tests, and this summary); Task 2 reused the verified quick-task collector and boundary commits

## Accomplishments

- Reverified the exact seven locked paths, fixed official metadata, bounded byte lengths, lowercase SHA-256 hashes, standard-rulebook evidence, and one closed acquisition method/reference pair without rerunning the collector or contacting publisher endpoints.
- Reverified distinct ordinary primary/backup files, independent roots, complete path sets, byte/hash equality, and the lock-matching canonical source-set root hash.
- Proved the private inputs and locators remain ignored, untracked, unstaged, and absent from actual `pnpm pack --dry-run --json` contents.
- Passed 90 focused collector/boundary/source-set tests and all 153 repository tests.

## Acquisition Receipt

- **Acquisition method:** `user-authorized-agent-run-one-shot-powershell`
- **Authorization reference:** `quick-260825-mhh-retry-1`
- **Primary label:** `.local/authority/inputs/official-2026-08-20/primary/`
- **Backup label:** `independent-private-backup-retry-1`
- **Backup locator fingerprint:** `sha256:28be9b6c8fbe3c5c3b417f6591f4da1ebd414542531e810cc7727ce5b5a9df57`
- **Source-set root hash:** `sha256:29111858fda613d6a75f4eb6681ca1ac8d3d389d87d03bf8431da86f7b229a6a`
- **Private-lock hash:** `sha256:5b6799c9c52c9f18fbfd2960f353592087d0dfcb4fd8e3e4b0714dc9b102cee9`
- **Standard rulebook observed filename:** `SorceryRulebook.pdf`
- **Derivation note:** The fixed collector selected the PDF from the official December 2025 rulebook release page. Its private locator is non-normative and remains only in ignored private evidence.

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

1. **Task 1: Implement the cross-platform private source-set verifier** — `45d59f8` (`feat`)
2. **Task 2: Independently verify the completed one-shot private collection** — completed across the audited collector/policy/debug sequence `513ac0c`, `a8c5884`, `638d16d`, `2ddb50c`, `9f9509f`, `26063cd`, `5cdb9d3`, `5723c22`, `d67cbd6`, `4598b07`, `386fdfa`, `6b05b86`, and `635f4f5`

Task 1's completed implementation and tests were recovered as one existing atomic commit. Safe-resume verification did not rewrite or recommit completed work.

## Files Created/Modified

- `src/authority/private-source-set.ts` — Exact-path metadata validation, root/file independence, bounded hashing, primary/backup equality, and deterministic source-set identity.
- `tests/authority/private-source-set.test.ts` — Cross-platform valid/failure coverage for metadata, bounds, JSON, traversal, roots, aliases, links, and byte mismatch.
- `scripts/collect-private-authority.ps1` — Closed acknowledged one-shot collection and receipt-last publication boundary used by the completed quick task; not invoked during this closeout.
- `scripts/verify-private-authority-boundary.ts` — Git HEAD, index, and actual package-content leakage gate.
- `tests/authority/private-authority-collector.test.ts` and `tests/authority/private-authority-boundary.test.ts` — Synthetic loopback and leakage regression coverage.
- `docs/external-reuse-policy.md`, `01-CONTEXT.md`, and `01-07-PLAN.md` — Exact closed evidence pairs and retained private-use limits.
- `.planning/phases/01-rules-and-data-authority/01-07-SUMMARY.md` — Safe non-content completion receipt.

## Decisions Made

- Accepted only the lock's exact `user-authorized-agent-run-one-shot-powershell` / `quick-260825-mhh-retry-1` evidence pair and its matching immutable five-field consumption record.
- Reused the independently verified private artifacts in place. No production collector, network request, retry, copy, or source mutation was performed.
- Left DATA-01, DATA-02, and DATA-03 pending because the phase gate remains Plan 01-08.

## Deviations from Plan

### Auto-fixed Issues Inherited from Task 2 Execution

**1. [Rule 1 - Bug] Accepted standard PDF EOF line endings**
- **Found during:** Task 2 initial authorized collection validation
- **Issue:** Valid PDFs ending with `%EOF` plus LF, CR, or CRLF were rejected.
- **Fix:** Retained the PDF prefix check while accepting only no trailing byte or one standard line ending and rejecting arbitrary trailing bytes.
- **Files modified:** `scripts/collect-private-authority.ps1`, `tests/authority/private-authority-collector.test.ts`
- **Committed in:** `26063cd`

**2. [Rule 1 - Bug] Accepted the official changelog's day-first date**
- **Found during:** Task 2 initial authorized collection validation
- **Issue:** The fixed official changelog used a valid day-first date outside the parser's accepted formats.
- **Fix:** Added strict day-first parsing with impossible-date rejection.
- **Files modified:** `scripts/collect-private-authority.ps1`, `tests/authority/private-authority-collector.test.ts`
- **Committed in:** `5cdb9d3`

**3. [Rule 2 - Security] Removed a tracked private locator**
- **Found during:** Task 2 post-collection boundary verification
- **Issue:** Historical tracked research contained the lock-matched non-normative rulebook locator.
- **Fix:** Redacted the one exact occurrence without weakening the locator gate; the private evidence retains it only under the ignored boundary.
- **Files modified:** `.planning/quick/260820-hhu-build-and-validate-a-user-run-powershell/260820-hhu-RESEARCH.md`
- **Committed in:** `d67cbd6`

**4. [Rule 1 - Bug] Required truthful risk acknowledgment on every production entry**
- **Found during:** Task 2 post-execution verification
- **Issue:** An exported production wrapper could omit the acknowledgment while the lock recorded it as accepted.
- **Fix:** Propagated the switch through every exported entry into a shared pre-work guard and derived receipt evidence from the validated value.
- **Files modified:** `scripts/collect-private-authority.ps1`, `tests/authority/private-authority-collector.test.ts`
- **Committed in:** `6b05b86`

**5. [Rule 1 - Tracking] Corrected the GSD progress frontmatter**
- **Found during:** Plan closeout tracking
- **Issue:** `state.update-progress` correctly reported and rendered 7/8 as 88%, but left the YAML `progress.percent` value at `0`.
- **Fix:** Set the frontmatter percentage to the handler's verified 88% result without changing phase status.
- **Files modified:** `.planning/STATE.md`
- **Committed in:** final plan metadata commit

---

**Total deviations:** 5 auto-fixes (3 inherited bugs, 1 inherited security boundary fix, 1 closeout tracking correction).
**Impact on plan:** Each fix was required for truthful, fail-closed evidence or internally consistent tracking; no recurring acquisition, artwork, redistribution, or broader network surface was added.

## Issues Encountered

- The Windows sandbox setup-refresh helper failed for read-only commands. Required local checks ran through command-scoped approved PowerShell execution.
- The same helper later blocked one `apply_patch` update; the exact two-hunk STATE/summary correction was applied with the requested bounded `git apply` fallback and checked with `git diff --check`.
- Task 1 used a combined implementation/test commit rather than separate RED/GREEN commits. The existing commit was independently audited, and its focused and full verification gates pass; safe-resume recovery did not redo it.

## Authentication Gates

None.

## Known Stubs

None. The scan found only intentional empty accumulators/null validation branches in substantive implementation and test code; no placeholder behavior blocks the plan goal.

## Verification Results

- Exact private lock/method/reference/acknowledgment/metadata/rulebook/source-set check — PASS (7/7 entries; lock, verifier result, and safe fingerprints match).
- `git check-ignore -q .local/authority/inputs/official-2026-08-20/primary/probe` — PASS.
- Tracked and staged `.local/authority` scans — PASS (empty).
- Focused collector, source-set, and private-boundary suites — PASS (90 tests, 0 failures).
- `node scripts/verify-private-authority-boundary.ts ...` — PASS against Git HEAD, index, and actual package inventory.
- `pnpm verify` — PASS (typecheck, lint, 153 tests, 0 failures/skips/todos).
- `01-08-SUMMARY.md` absence check — PASS; Plan 08 was not started.

## Next Phase Readiness

- The ignored lock, complete primary source set, independent backup, and safe receipt are ready for a corrected Plan 01-08 validation/build flow.
- **Blocker:** `01-08-PLAN.md` still hard-requires `acquisitionMethod: user-run-one-shot-powershell`, but the verified receipt is the accepted agent-run `quick-260825-mhh-retry-1` pair. Revise and validate Plan 01-08 against the current D-08/D-11 three-pair contract before executing it.
- DATA-01, DATA-02, and DATA-03 remain pending until Plan 01-08 and phase verification pass.

## Self-Check: PASSED

- The summary and Task 1 implementation/test files exist.
- Every Task 1/Task 2 commit listed above resolves in repository history.
- No official source byte, source excerpt, absolute private locator, normalized corpus, or artwork appears in this summary.
- All local acceptance and leakage checks passed without rerunning the collector or making network requests.

---
*Phase: 01-rules-and-data-authority*
*Completed: 2026-08-26*
