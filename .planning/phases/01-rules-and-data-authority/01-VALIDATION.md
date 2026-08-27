---
phase: 1
slug: rules-and-data-authority
status: draft
nyquist_compliant: true
wave_0_complete: true
created: 2026-08-20
updated: 2026-08-27
---

# Phase 1 — Validation Strategy

> Per-phase validation contract for feedback sampling during execution.

---

## Test Infrastructure

| Property | Value |
|----------|-------|
| **Framework** | `node:test` and `node:assert/strict` on Node 24 |
| **Config file** | none |
| **Quick run command** | `node --test tests/authority/*.test.ts` |
| **Full suite command** | `pnpm typecheck && pnpm lint && pnpm test` |
| **Phase release command** | `pnpm verify && node --test --test-name-pattern="fresh v3 primary and backup completeness" tests/private-authority/private-source-completeness.test.ts && pnpm authority:verify-private && node --test tests/private-authority/repository-boundary.test.ts` |
| **Estimated runtime** | default suite under 30 seconds; final private release gate under 180 seconds |

---

## Sampling Rate

- **After every task commit:** Run the directly relevant focused test/assertion plus `pnpm typecheck`.
- **After every plan wave:** Run `pnpm verify`.
- **Before `$gsd-verify-work`:** Run the phase release command with the complete private source set available; require full-set source verification, two independent rebuilds, exact CLI validation, tamper rejection, no-network execution, unchanged selected revision, and zero tracked/staged/package leakage.
- **Max feedback latency:** 180 seconds for the final private release gate; focused task checks remain under 60 seconds.

---

## Per-Task Verification Map

| Task ID | Plan | Wave | Requirement | Threat Ref | Secure Behavior | Test Type | Automated Command | File Exists | Status |
|---------|------|------|-------------|------------|-----------------|-----------|-------------------|-------------|--------|
| 01-01-01 | 01 | 0 | DATA-01, DATA-02, DATA-03 | T-01-SC | Exact Node/pnpm/TypeScript/Zod/ESLint graph and scripts are pinned | config/supply-chain | `node -e "if(process.versions.node!=='24.19.0')process.exit(1)" && pnpm --version \| node -e "let s='';process.stdin.on('data',d=>s+=d).on('end',()=>{if(s.trim()!=='11.22.0')process.exit(1)})" && pnpm install --frozen-lockfile && pnpm exec tsc --version && pnpm exec eslint --version` | ✅ present | ✅ green |
| 01-01-02 | 01 | 0 | DATA-01, DATA-02, DATA-03 | T-01-01, T-01-05 | Current authority contracts have no remaining `test.todo` and fixtures remain license-safe | contract | `node -e "const fs=require('node:fs');for(const f of fs.readdirSync('tests/authority').filter(x=>x.endsWith('.test.ts')))if(fs.readFileSync('tests/authority/'+f,'utf8').includes('test.todo'))process.exit(1)" && pnpm verify` | ✅ present | ✅ green |
| 01-02-01 | 02 | 1 | DATA-03 | T-01-06, T-01-09 | Canonical JSON rejects unsupported values and produces stable identity hashes | unit | `node --test tests/authority/canonical-json.test.ts && pnpm typecheck` | ✅ present | ✅ green |
| 01-02-02 | 02 | 1 | DATA-03 | T-01-06, T-01-08, T-01-10 | Strict envelopes separate stored/manifest sources and keep non-official provenance non-normative | unit | `node --test --test-name-pattern="schema\|envelope\|source\|authority\|storage\|diagnostic\|duplicate\|tamper" tests/authority/provenance.test.ts && pnpm typecheck` | ✅ present | ✅ green |
| 01-03-01 | 03 | 2 | DATA-02, DATA-03 | T-01-11, T-01-13, T-01-15 | Card normalization is strict, lossless, duplicate-aware, and clean-room safe | integration | `node --test --test-name-pattern="valid\|malformed\|unknown\|duplicate\|count\|path" tests/authority/card-snapshot.test.ts && pnpm typecheck` | ✅ present | ✅ green |
| 01-03-02 | 03 | 2 | DATA-02, DATA-03 | T-01-14 | Repeated property-reordered normalization is byte-identical and offline | integration/property | `node --test tests/authority/card-snapshot.test.ts && pnpm typecheck && pnpm lint` | ✅ present | ✅ green |
| 01-04-01 | 04 | 2 | DATA-01, DATA-03 | T-01-16, T-01-17, T-01-19 | Paths/graphs are bounded; stored bytes and manifest bindings are validated honestly | integration | `node --test tests/authority/provenance.test.ts && pnpm typecheck` | ✅ present | ✅ green |
| 01-04-02 | 04 | 2 | DATA-01, DATA-03 | T-01-18, T-01-20 | Only official authority can win; ambiguity and prohibited storage fail offline | integration/golden | `node --test --test-name-pattern="current\|superseded\|scoped\|ambiguous\|offline\|storage\|media" tests/authority/bundle.test.ts && pnpm typecheck && pnpm lint` | ✅ present | ✅ green |
| 01-05-01 | 05 | 3 | DATA-01, DATA-02 | T-01-21, T-01-22, T-01-23, T-01-24, T-01-25 | Import verifies exact input lock/root, emits candidate hashes, and atomically finalizes a write-once local revision | integration | `node --test --test-name-pattern="input\|root\|build\|byte-identical\|write-once\|atomic\|failure\|storage" tests/authority/bundle.test.ts && pnpm typecheck` | ✅ present | ✅ green |
| 01-05-02 | 05 | 3 | DATA-01, DATA-02 | T-01-21, T-01-24, T-01-25 | Validation CLI accepts no raw-input argument, reports stored rehash/manifest binding, and remains offline | integration | `node --test tests/authority/bundle.test.ts && pnpm verify` | ✅ present | ✅ green |
| 01-06-01 | 06 | 4 | DATA-01, DATA-03 | T-01-28, T-01-29 | Project precedence is reviewable, official-only, effective-dated, and fail-closed | policy/integration | `node -e "const s=require('node:fs').readFileSync('docs/authority-precedence.md','utf8').toLowerCase();for(const x of ['official rulebook','format','codex','faq','card update','effective date','scoped','superseded','contending','unsupported','community'])if(!s.includes(x))process.exit(1)" && pnpm test` | ✅ 01-06-SUMMARY.md | ✅ green |
| 01-06-02 | 06 | 4 | DATA-01, DATA-03 | T-01-26, T-01-27, T-01-30 | Fixed user-run one-shot source-set path, private Git exclusion, community audit, no-art/no-leakage policy, risk acknowledgment, and broader-use permission gates are closed | policy/integration | `node --test tests/authority/private-authority-collector.test.ts && pnpm verify` | ✅ 01-06-SUMMARY.md | ✅ green |
| 01-07-01 | 07 | 5 | DATA-01, DATA-02, DATA-03 | T-01-31, T-01-32, T-01-33 | Reusable realpath/platform-aware/dev-ino verifier rejects incomplete, linked, aliased, oversized, or mismatched primary/backup sets | unit/integration | `node --test tests/authority/private-source-set.test.ts && pnpm typecheck && pnpm verify` | ✅ 01-07-SUMMARY.md | ✅ green |
| 01-07-02 | 07 | 5 | DATA-01, DATA-02, DATA-03 | T-01-33, T-01-34, T-01-35 | User personally runs the fixed one-shot collector; postcheck requires its exact acquisition method, complete independent source set, acknowledgment, and recomputed lock | blocking human action + automated postcheck | `node --input-type=module -e "import fs from 'node:fs';import cp from 'node:child_process';import {verifyPrivateSourceSet} from './src/authority/private-source-set.ts';const p='.local/authority/locks/official-2026-08-20/source-set-lock.json',lock=JSON.parse(fs.readFileSync(p,'utf8'));if(lock.acquisitionMethod!=='user-run-one-shot-powershell')process.exit(1);const u='https://sorcerytcg.com/news/sorcery-contested-realm-december-2025-rulebook-update',rule=lock.entries.find(function(e){return e.relativePath==='rulebook/rulebook-current.pdf'}),a=lock.rulebookAcquisitionEvidence;if(!rule||rule.url!==u||!a||a.sourceUrl!==u||a.relativePath!==rule.relativePath||a.byteHash!==rule.byteHash||a.retrievedAt!==rule.retrievedAt||typeof a.observedFilename!=='string'||a.observedFilename.trim()===''||a.privateLocatorIsNormative!==false)process.exit(1);const v=await verifyPrivateSourceSet({primaryRoot:lock.primaryRoot,backupRoot:lock.backupRoot,repositoryRoot:'.',entries:lock.entries});if(v.sourceSetRootHash!==lock.sourceSetRootHash||v.entries.length!==7)process.exit(1);if(cp.execFileSync('git',['ls-files','.local/authority'],{encoding:'utf8'}).trim())process.exit(1);if(cp.execFileSync('git',['diff','--cached','--name-only'],{encoding:'utf8'}).split(/\r?\n/).some(x=>x.startsWith('.local/authority/')))process.exit(1)"` | ✅ 01-07-SUMMARY.md | ✅ green |
| 01-08-01 | 08 | 6 | DATA-01, DATA-02, DATA-03 | T-01-36, T-01-37, T-01-38, T-01-39 | Both full source sets are reverified, independently derive identical bounded importer inputs, and build/validate the selected private revision with the exact CLI contract | integration/golden | `node --input-type=module -e "import fs from 'node:fs';import cp from 'node:child_process';import {identityHash} from './src/authority/hash.ts';import {verifyPrivateSourceSet} from './src/authority/private-source-set.ts';const lock=JSON.parse(fs.readFileSync('.local/authority/locks/official-2026-08-20/source-set-lock.json','utf8')),r=JSON.parse(fs.readFileSync('data/authority/receipts/official-2026-08-20.json','utf8')),v=await verifyPrivateSourceSet({primaryRoot:lock.primaryRoot,backupRoot:lock.backupRoot,repositoryRoot:'.',entries:lock.entries});if(v.entries.length!==7||v.sourceSetRootHash!==lock.sourceSetRootHash||r.sourceSetRootHash!==lock.sourceSetRootHash)process.exit(1);const b=JSON.parse(fs.readFileSync('.local/authority/revisions/official-2026-08-20/bundle.json','utf8'));if(b.identity.stableId!=='bundle:official-2026-08-20'||identityHash(b.identity)!==r.bundleRootHash)process.exit(1);const x=cp.spawnSync('pnpm',['authority:validate','--','--root','.local/authority/revisions/official-2026-08-20','--bundle','bundle.json','--id','bundle:official-2026-08-20','--hash',r.bundleRootHash],{stdio:'inherit',shell:process.platform==='win32'});process.exit(x.status??1)"` | ✅ 01-08-SUMMARY.md | ✅ green |
| 01-08-02 | 08 | 6 | DATA-01, DATA-02, DATA-03 | T-01-36, T-01-37, T-01-38, T-01-39 | Independent primary/backup full-set rebuilds, byte maps, derivation/reference tamper cases, and selected immutability all pass | private release/integration | `node --test tests/private-authority/private-revision.test.ts && pnpm typecheck` | ✅ 01-08-SUMMARY.md | ✅ green |
| 01-08-03 | 08 | 6 | DATA-01, DATA-02, DATA-03 | T-01-40, T-01-41 | Fetch/http/https/net denial plus tracked/staged/package source/corpus/PDF/page/art/revision leakage scans pass | private boundary/integration | `pnpm verify && pnpm authority:verify-private` | ✅ 01-08-SUMMARY.md | ✅ green |
| 01-09-01 | 09 | 7 | DATA-02, DATA-03 | T-01-42, T-01-44 | Strict card contracts require lossless avatar life and four elemental thresholds | unit/schema | `node --test tests/authority/card-snapshot.test.ts && pnpm typecheck` | ✅ 01-09-SUMMARY.md | ✅ green |
| 01-09-02 | 09 | 7 | DATA-02, DATA-03 | T-01-42 | Official adaptation and normalization preserve all gameplay fields and freezing | integration | `node --test tests/authority/card-snapshot.test.ts tests/authority/provenance.test.ts && pnpm typecheck` | ✅ 01-09-SUMMARY.md | ✅ green |
| 01-09-03 | 09 | 7 | DATA-02, DATA-03 | T-01-43 | Official logical card IDs remain stable across authority revisions while SourceRefs differ | integration/property | `node --test tests/authority/card-snapshot.test.ts tests/authority/bundle.test.ts && pnpm verify` | ✅ 01-09-SUMMARY.md | ✅ green |
| 01-10-01 | 10 | 8 | DATA-01, DATA-03 | T-01-45 | Supersession graphs and semantic dates resolve only one justified official winner | unit/integration | `node --test tests/authority/bundle.test.ts && pnpm typecheck` | ✅ 01-10-SUMMARY.md | ✅ green |
| 01-10-02 | 10 | 8 | DATA-01, DATA-03 | T-01-46 | SHA locators bind byte hashes and command evidence distinguishes bytes from declarations | integration | `node --test tests/authority/provenance.test.ts tests/authority/bundle.test.ts && pnpm typecheck && pnpm lint` | ✅ 01-10-SUMMARY.md | ✅ green |
| 01-10-03 | 10 | 8 | DATA-01, DATA-03 | T-01-47 | Duplicate and cyclic source derivation edges fail deterministically | unit/integration | `node --test tests/authority/provenance.test.ts && pnpm verify` | ✅ 01-10-SUMMARY.md | ✅ green |
| 01-11-01 | 11 | 7 | DATA-01 | T-01-49 | Source-specific visible/date/card checks reject shells, truncation, ambiguity, and malformed staged inputs before publication | integration | `node --test tests/authority/private-authority-collector.test.ts && pnpm typecheck` | ✅ present | ✅ green |
| 01-11-02 | 11 | 7 | DATA-01 | T-01-50, T-01-51 | Production output is sanitized and child processes are concurrently drained and bounded | integration | `node --test tests/authority/private-authority-collector.test.ts && pnpm verify` | ✅ present | ✅ green |
| 01-12-01 | 12 | 7 | DATA-01, DATA-02, DATA-03 | T-01-53, T-01-56 | Every-offset and semantic fingerprints scan history, worktree, index, and package candidates | security/integration | `node --test tests/authority/private-authority-boundary.test.ts && pnpm typecheck` | ✅ 01-12-SUMMARY.md | ✅ green |
| 01-12-02 | 12 | 7 | DATA-01, DATA-02, DATA-03 | T-01-54, T-01-55 | Encoded/case-varied locators fail through the single production boundary scanner | security/private | `node --test tests/authority/private-authority-boundary.test.ts tests/private-authority/repository-boundary.test.ts` | ✅ 01-12-SUMMARY.md | ✅ green |
| 01-12-03 | 12 | 7 | DATA-01, DATA-02, DATA-03 | T-01-53 | Policy permits normalized data only in ignored private revisions and forbids external publication | policy | `pnpm verify` | ✅ 01-12-SUMMARY.md | ✅ green |
| 01-14-01 | 14 | 8 | DATA-01 | T-01-62, T-01-64, T-01-65 | Executed Plan 11 checks are reused by a no-transport, content-free offline mode without changing collector behavior; no historical-root pass is claimed | integration | `git merge-base --is-ancestor 5aadff3 HEAD && git merge-base --is-ancestor 77c6000 HEAD && node --test --test-name-pattern="offline mode|local root|no transport|incomplete existing|sanitized" tests/authority/private-authority-collector.test.ts && pnpm typecheck` | ✅ commits 5aadff3/77c6000 | ✅ green |
| 01-15-01 | 15 | 9 | DATA-01, DATA-02, DATA-03 | T-01-66, T-01-69, T-01-70 | New production network acquisition is disabled; the fixed manual inbox can be validated and imported only offline with sanitized output | security/integration | node --test --test-name-pattern="manual inbox\|manual intake\|offline\|no transport\|sanitized" tests/authority/private-authority-collector.test.ts && pnpm typecheck | ❌ P15 | ⬜ pending |
| 01-15-02 | 15 | 9 | DATA-01, DATA-02 | T-01-66, T-01-67, T-01-70 | The user saves exactly seven browser-downloaded files at fixed ignored paths; no project or agent transport occurs | blocking human action + automated postcheck | User replies manual-source-set-ready; Task 3 runs the offline importer and exact identity/content checks | ❌ P15 | ⬜ pending |
| 01-15-03 | 15 | 9 | DATA-01, DATA-02, DATA-03 | T-01-68, T-01-69 | Both manually provisioned fresh roots pass structural/content proof offline with identical unchanged before/after maps | private integration | node --test --test-name-pattern="fresh v3 primary and backup completeness" tests/private-authority/private-source-completeness.test.ts && pnpm verify && node --test tests/private-authority/repository-boundary.test.ts | ❌ P15 | ⬜ pending |
| 01-13-01 | 13 | 10 | DATA-01, DATA-02, DATA-03 | T-01-57, T-01-58 | Only Plan 15 fresh roots build byte-identical v3 candidates; all historical evidence is unchanged | private release/integration | `node --test --test-name-pattern="fresh v3 roots\|final v3 candidates\|historical evidence remains immutable" tests/private-authority/private-source-completeness.test.ts tests/private-authority/private-revision.test.ts` | ❌ P13 | ⬜ pending |
| 01-13-02 | 13 | 10 | DATA-01, DATA-02, DATA-03 | T-01-59 | The write-once v3 revision, safe receipt, and README bind exact ID/root selection | private release/integration | `node --test --test-name-pattern="official-2026-08-27-v3\|safe receipt\|write-once selection\|receipt root validates" tests/private-authority/private-revision.test.ts` | ❌ P13 | ⬜ pending |
| 01-13-03 | 13 | 10 | DATA-01, DATA-02, DATA-03 | T-01-60, T-01-61 | Default, fresh-root, full private, and narrow boundary gates pass with no network or leakage | private release/security | `pnpm verify && node --test --test-name-pattern="fresh v3 primary and backup completeness" tests/private-authority/private-source-completeness.test.ts && pnpm authority:verify-private && node --test tests/private-authority/repository-boundary.test.ts` | ❌ P13 | ⬜ pending |

*Status: ⬜ pending · ✅ green · ❌ red · ⚠️ flaky*

### Safe-Resume Status

- **Total plans:** 15
- **Complete (SUMMARY exists):** 13 — 01-01 through 01-12 plus 01-14
- **Incomplete (no SUMMARY):** exactly 01-15 and 01-13
- **Resume order:** execute the user-gated manual provision and offline proof in 01-15, then execute final v3 rebuild/release 01-13.
- **Historical integrity:** 01-14 owns only commits 5aadff3 and 77c6000; the historical roots failed the strengthened content contract but remained byte-identical and unchanged. Plan 15 owns only fresh manual intake/proof work.

---

## Wave 0 Requirements

- [x] `package.json`, `pnpm-lock.yaml`, `tsconfig.json`, and flat ESLint config with pinned TypeScript/Node scripts.
- [x] `tests/authority/canonical-json.test.ts` with RFC-derived ordering, number, Unicode, array, and rejection vectors.
- [x] `tests/authority/provenance.test.ts` with strict-envelope, reference, tamper, cycle, and path-confinement cases.
- [x] `tests/authority/card-snapshot.test.ts` with license-safe synthetic valid, malformed, duplicate, and reordered fixtures.
- [x] `tests/authority/bundle.test.ts` with current, superseded, ambiguous, authority-class, stored/manifest-only, input-lock, prohibited-media, write-once, atomic-failure, and offline fixtures.
- [x] `tests/authority/fixtures/bundle-input/input-lock.json` with the exact synthetic three-file lock, hashes, modes, and canonical input-root hash.

Plan 07 adds the generic clean-clone-safe `tests/authority/private-source-set.test.ts`. Plans 08 and 12-15 add or extend opt-in tests under `tests/private-authority/` outside the default glob. `pnpm authority:verify-private` must fail rather than skip when any private source set, lock, receipt, revision, completeness proof, or attestation is absent.

---

## Human-Gated Verifications

| Behavior | Requirement | Why Manual | Test Instructions |
|----------|-------------|------------|-------------------|
| One fresh v3 seven-file source set and independent byte-identical backup exist from the fixed ignored manual inbox | DATA-01, DATA-02 | Only the user downloads and places the files; project code and agents never invoke publisher endpoints | After Plan 15 Task 1 passes, save the seven files under their exact ignored paths and reply manual-source-set-ready; Task 3 performs offline import, content verification, and dual-root proof. |
| External reuse audit confirms no copied GPL, unlicensed, or publisher-reserved community material | DATA-03 | Independent clean-room provenance requires human review | Review changed files against `docs/external-reuse-policy.md`; record audited repository revisions/sign-off; never treat community card data as a licensed corpus |

---

## Validation Sign-Off

- [x] Wave 0 infrastructure and Plans 01-01 through 01-05 files/tests exist and are green.
- [ ] All remaining tasks have executable `<automated>` verification.
- [ ] No watch-mode flags; default and private feedback latency remain within the stated bounds.
- [ ] All seven primary/backup source files are ordinary, independent, path-contained, byte-identical, and metadata/hash complete.
- [ ] Two independently derived four-file importer inputs obey exact order/modes and 10,000,000-per/32,000,000-total bounds.
- [ ] Import stdout is used only for candidate input/bundle roots; fixed stable ID is independently checked.
- [ ] Plan 14 is summarized only from commits `5aadff3` and `77c6000`; no historical-root pass is claimed.
- [ ] The user provides exactly seven files in the fixed ignored manual inbox; offline intake records the exact manual method/provision reference and no project or agent transport occurs.
- [ ] Both fresh v3 roots pass the structural/shared-content verifier offline and retain identical before/after file maps.
- [ ] Final import uses `--input`, `--input-lock input-lock.json`, `--expected-input-root-hash`, `--output-root`, and `--revision-id official-2026-08-27-v3`.
- [ ] Final validation uses `--root .local/authority/revisions/official-2026-08-27-v3 --bundle bundle.json --id bundle:official-2026-08-27-v3 --hash &lt;receipt bundleRootHash&gt;`.
- [ ] Two full-set rebuilds are path/byte-identical; source-set/input/derivation/artifact/reference/cycle tamper cases fail.
- [ ] Both v3 candidates are byte-identical; all historical evidence maps are unchanged; the new selected revision is write-once.
- [ ] Fetch/http/https/net denial stays active during real helper/import/validation.
- [ ] Tracked, staged, and package scans find no private absolute locator or official source/API/normalized/revision/art/PDF/page bytes.
- [ ] Default `pnpm verify` remains synthetic and clean-clone-safe; the focused fresh v3 completeness test, full `pnpm authority:verify-private`, and narrow repository-boundary test all pass.
- [ ] DATA-01/02/03 remain Pending until Plan 13 and phase verification pass.

**Approval:** pending
