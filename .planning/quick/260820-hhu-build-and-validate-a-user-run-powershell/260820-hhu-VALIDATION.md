---
quick_task: 260820-hhu-build-and-validate-a-user-run-powershell
status: passed
nyquist_compliant: true
created: 2026-08-20
updated: 2026-08-20
executed: 2026-08-20
---

# Quick Task 260820-hhu — Validation Strategy

The collector is validated only against a synthetic loopback server and temporary roots. Two guard tests invoke the direct entry point without a required argument and stop before transport; this quick task never completes a production collection or contacts publisher endpoints.

## Test Infrastructure

| Property | Value |
|---|---|
| Framework | Node 24 `node:test` + `node:assert/strict` |
| System under test | `pwsh -NoProfile -NonInteractive`, PowerShell 7+ |
| Network | `node:http` on `127.0.0.1` with an OS-assigned port; publisher URLs remain provenance strings only |
| Focused command | `node --test tests/authority/private-authority-collector.test.ts` |
| Existing verifier command | `node --test tests/authority/private-source-set.test.ts` |
| Full command | `pnpm verify` |
| Dependencies added | none |

## Requirement-to-Test Map

| Test ID | Requirement | Named behavior | Threats | Automated command | File | Status |
|---|---|---|---|---|---|---|
| QH-01 | DATA-01, DATA-03 | `production wrapper is inert when dot-sourced and exposes the exact fixed seven-source manifest` | T-QH-01, T-QH-07 | `node --test --test-name-pattern="wrapper|manifest|acknowledg" tests/authority/private-authority-collector.test.ts` | `tests/authority/private-authority-collector.test.ts` | passed |
| QH-02 | DATA-01 | `redirects and the standard rulebook fail closed` | T-QH-01 | `node --test --test-name-pattern="redirect|rulebook|annotated" tests/authority/private-authority-collector.test.ts` | same | passed |
| QH-03 | DATA-01, DATA-02 | `status block timeout size and content failures stop without retry` | T-QH-02, T-QH-03 | `node --test --test-name-pattern="401|403|429|captcha|block|timeout|size|content" tests/authority/private-authority-collector.test.ts` | same | passed |
| QH-04 | DATA-02 | `card JSON is strict nonempty and exact response bytes are preserved` | T-QH-02 | `node --test --test-name-pattern="card json|exact bytes" tests/authority/private-authority-collector.test.ts` | same | passed |
| QH-05 | DATA-01, DATA-03 | `loopback-only temporary destinations reject backup inside repository before requests` | T-QH-04, T-QH-05 | `node --test --test-name-pattern="temporary destination|backup.*repository|before request" tests/authority/private-authority-collector.test.ts` | same | passed |
| QH-06 | DATA-01, DATA-03 | `staged and final trees are independently verified and existing evidence is never overwritten` | T-QH-04, T-QH-05 | `node --test --test-name-pattern="staging|backup|no.overwrite|quarantine|verifier" tests/authority/private-authority-collector.test.ts` | same | passed |
| QH-07 | DATA-01, DATA-03 | `private lock records the fixed acquisition method acknowledgment hashes and rulebook evidence` | T-QH-02, T-QH-07 | `node --test --test-name-pattern="acquisitionMethod|receipt|acknowledg|sourceSetRootHash" tests/authority/private-authority-collector.test.ts` | same | passed |
| QH-08 | DATA-01, DATA-02, DATA-03 | `policy and Plan 01-07 describe the narrow exception without implying permission` | T-QH-06, T-QH-07 | Task 3 document assertion followed by `pnpm verify` | documentation/Plan 01-07 | passed |

## Required Assertions

- Direct script parameters are exactly `BackupRoot` and `AcknowledgePrivateUseRisk`; repository, primary, lock, URL, descriptor, transport, timeout, and size overrides are absent.
- Dot-sourcing exports exactly the three supported commands and performs no request or write; lower helpers are absent from the supported caller command surface.
- `Invoke-PrivateAuthorityCollectionForLoopbackTest` accepts temporary repository/primary/lock destinations and reduced limits only when every request and redirect host is loopback; the direct wrapper cannot dispatch it.
- Deliberate private-module invocation or modification/copying by the machine owner is outside the interface threat boundary; PowerShell module privacy is encapsulation, not an authorization sandbox.
- Equal, nested, containing, aliased, relative, or repository-contained backup roots fail before the loopback request counter changes.
- Happy-path receipt has `acquisitionMethod: user-run-one-shot-powershell` and is accepted by the existing `verifyPrivateSourceSet` using both final trees.
- Synthetic fixtures contain no publisher bytes or artwork, and the server fails the test if an unregistered/art route is requested.
- Failures produce no canonical lock; existing evidence is unchanged; cleanup/quarantine operates only on run-owned verified temporary paths.

## Sampling and Release Gate

- After Task 1: run QH-01 through QH-04 with the focused test file.
- After Task 2: run QH-01 through QH-07 plus `tests/authority/private-source-set.test.ts`.
- After Task 3: run the per-document policy assertion and `pnpm verify`.
- Create the quick-task summary only after all three tasks pass.
- The later live run is a blocking Plan 01-07 human action; it is not part of this validation strategy.

## Executed Evidence

| Gate | Result |
|---|---|
| Focused synthetic collector suite | `53/53` passed; zero publisher endpoints contacted |
| Private source-set verifier suite | `11/11` passed as part of the repository gate |
| Production configuration bypass regressions | Both wrapper and core overrides failed with `0` loopback requests |
| Supported command-surface regression | Exactly three commands exported; lower helpers absent from ordinary caller lookup |
| Aggregate byte-limit regression | Overflow stopped before the next descriptor; no primary, backup, or receipt published |
| Actual verifier-process failure regression | Node bridge exited nonzero after both trees moved; owned trees were quarantined and canonical lock remained absent |
| Policy/Plan normalized assertion | passed |
| `pnpm verify` | typecheck, lint, and `127/127` tests passed |

**Approval:** executed and passed
