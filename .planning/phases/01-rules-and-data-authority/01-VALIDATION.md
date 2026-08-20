---
phase: 1
slug: rules-and-data-authority
status: draft
nyquist_compliant: true
wave_0_complete: true
created: 2026-08-20
updated: 2026-08-20
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
| **Phase release command** | `pnpm verify && pnpm authority:verify-private` (explicit private-data gate added by Plan 08; default tests stay clean-clone-safe) |
| **Estimated runtime** | default suite under 30 seconds; private gate under 60 seconds |

---

## Sampling Rate

- **After every task commit:** Run the directly relevant focused test/assertion plus `pnpm typecheck`.
- **After every plan wave:** Run `pnpm verify`.
- **Before `$gsd-verify-work`:** Run the phase release command with the complete private source set available; require full-set source verification, two independent rebuilds, exact CLI validation, tamper rejection, no-network execution, unchanged selected revision, and zero tracked/staged/package leakage.
- **Max feedback latency:** 60 seconds.

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
| 01-06-01 | 06 | 4 | DATA-01, DATA-03 | T-01-28, T-01-29 | Project precedence is reviewable, official-only, effective-dated, and fail-closed | policy/integration | `node -e "const s=require('node:fs').readFileSync('docs/authority-precedence.md','utf8').toLowerCase();for(const x of ['official rulebook','format','codex','faq','card update','effective date','scoped','superseded','contending','unsupported','community'])if(!s.includes(x))process.exit(1)" && pnpm test` | ❌ P06 | ⬜ pending |
| 01-06-02 | 06 | 4 | DATA-01, DATA-03 | T-01-26, T-01-27, T-01-30 | Complete manual source-set path, private Git exclusion, community audit, clean-room/no-leakage policy, and future permission triggers are closed | policy/integration | `git check-ignore -q .local/authority/probe && node -e "const s=require('node:fs').readFileSync('docs/external-reuse-policy.md','utf8').toLowerCase();for(const x of ['source set','rulebook','constructed','codex','faq','changelog','card update','manual browser','private','noncommercial','no redistribution','.local/authority/','git-ignored','no fetch','no scraping','no polling','public/network','art','pdf','full corpus','clean-room','sha-256','backup','written publisher permission','separate plan'])if(!s.includes(x))process.exit(1)" && pnpm verify` | ❌ P06 | ⬜ pending |
| 01-07-01 | 07 | 5 | DATA-01, DATA-02, DATA-03 | T-01-31, T-01-32, T-01-33 | Reusable realpath/platform-aware/dev-ino verifier rejects incomplete, linked, aliased, oversized, or mismatched primary/backup sets | unit/integration | `node --test tests/authority/private-source-set.test.ts && pnpm typecheck && pnpm verify` | ❌ P07 | ⬜ pending |
| 01-07-02 | 07 | 5 | DATA-01, DATA-02, DATA-03 | T-01-33, T-01-34, T-01-35 | Human manually supplies all seven official sources plus independent backup/metadata/attestation; postcheck recomputes the complete lock | blocking human action + automated postcheck | `node --input-type=module -e "import fs from 'node:fs';import cp from 'node:child_process';import {verifyPrivateSourceSet} from './src/authority/private-source-set.ts';const lock=JSON.parse(fs.readFileSync('.local/authority/locks/official-2026-08-20/source-set-lock.json','utf8')),v=await verifyPrivateSourceSet({primaryRoot:lock.primaryRoot,backupRoot:lock.backupRoot,repositoryRoot:'.',entries:lock.entries});if(v.entries.length!==7||v.sourceSetRootHash!==lock.sourceSetRootHash)process.exit(1);if(cp.execFileSync('git',['ls-files','.local/authority'],{encoding:'utf8'}).trim())process.exit(1)"` | N/A checkpoint | ⬜ pending |
| 01-08-01 | 08 | 6 | DATA-01, DATA-02, DATA-03 | T-01-36, T-01-37, T-01-38, T-01-39 | Both full source sets are reverified, independently derive identical bounded importer inputs, and build/validate the selected private revision with the exact CLI contract | integration/golden | `node --input-type=module -e "import fs from 'node:fs';import cp from 'node:child_process';import {identityHash} from './src/authority/hash.ts';import {verifyPrivateSourceSet} from './src/authority/private-source-set.ts';const lock=JSON.parse(fs.readFileSync('.local/authority/locks/official-2026-08-20/source-set-lock.json','utf8')),r=JSON.parse(fs.readFileSync('data/authority/receipts/official-2026-08-20.json','utf8')),v=await verifyPrivateSourceSet({primaryRoot:lock.primaryRoot,backupRoot:lock.backupRoot,repositoryRoot:'.',entries:lock.entries});if(v.entries.length!==7||v.sourceSetRootHash!==lock.sourceSetRootHash||r.sourceSetRootHash!==lock.sourceSetRootHash)process.exit(1);const b=JSON.parse(fs.readFileSync('.local/authority/revisions/official-2026-08-20/bundle.json','utf8'));if(b.identity.stableId!=='bundle:official-2026-08-20'||identityHash(b.identity)!==r.bundleRootHash)process.exit(1);const x=cp.spawnSync('pnpm',['authority:validate','--','--root','.local/authority/revisions/official-2026-08-20','--bundle','bundle.json','--id','bundle:official-2026-08-20','--hash',r.bundleRootHash],{stdio:'inherit',shell:process.platform==='win32'});process.exit(x.status??1)"` | ❌ P08 | ⬜ pending |
| 01-08-02 | 08 | 6 | DATA-01, DATA-02, DATA-03 | T-01-36, T-01-37, T-01-38, T-01-39 | Independent primary/backup full-set rebuilds, byte maps, derivation/reference tamper cases, and selected immutability all pass | private release/integration | `node --test tests/private-authority/private-revision.test.ts` | ❌ P08 | ⬜ pending |
| 01-08-03 | 08 | 6 | DATA-01, DATA-02, DATA-03 | T-01-40, T-01-41 | Fetch/http/https/net denial plus tracked/staged/package source/corpus/PDF/page/art/revision leakage scans pass | private boundary/integration | `pnpm verify && pnpm authority:verify-private` | ❌ P08 | ⬜ pending |

*Status: ⬜ pending · ✅ green · ❌ red · ⚠️ flaky*

---

## Wave 0 Requirements

- [x] `package.json`, `pnpm-lock.yaml`, `tsconfig.json`, and flat ESLint config with pinned TypeScript/Node scripts.
- [x] `tests/authority/canonical-json.test.ts` with RFC-derived ordering, number, Unicode, array, and rejection vectors.
- [x] `tests/authority/provenance.test.ts` with strict-envelope, reference, tamper, cycle, and path-confinement cases.
- [x] `tests/authority/card-snapshot.test.ts` with license-safe synthetic valid, malformed, duplicate, and reordered fixtures.
- [x] `tests/authority/bundle.test.ts` with current, superseded, ambiguous, authority-class, stored/manifest-only, input-lock, prohibited-media, write-once, atomic-failure, and offline fixtures.
- [x] `tests/authority/fixtures/bundle-input/input-lock.json` with the exact synthetic three-file lock, hashes, modes, and canonical input-root hash.

Plan 07 adds the generic clean-clone-safe `tests/authority/private-source-set.test.ts`. Plan 08 adds the two tests under `tests/private-authority/` outside the default glob. `pnpm authority:verify-private` must fail rather than skip when any private source set, lock, receipt, revision, or attestation is absent.

---

## Manual-Only Verifications

| Behavior | Requirement | Why Manual | Test Instructions |
|----------|-------------|------------|-------------------|
| Complete seven-file official source set and independent byte-identical backup exist for private local use | DATA-01, DATA-02 | Project acquisition is prohibited; only the user can save current official sources through the browser and select independent private backup storage | Save the rulebook PDF, Constructed page, Codex, FAQs, changelog, card-update page, and full API JSON at the seven fixed primary paths; copy to an outside-repository non-linked backup tree; provide every URL/date/media field and the exact scoped attestation; halt without Plan 07/08 summaries if incomplete |
| External reuse audit confirms no copied GPL, unlicensed, or publisher-reserved community material | DATA-03 | Independent clean-room provenance requires human review | Review changed files against `docs/external-reuse-policy.md`; record audited repository revisions/sign-off; never treat community card data as a licensed corpus |

---

## Validation Sign-Off

- [x] Wave 0 infrastructure and Plans 01-01 through 01-05 files/tests exist and are green.
- [ ] All remaining tasks have executable `<automated>` verification.
- [ ] No watch-mode flags; default and private feedback latency remain within the stated bounds.
- [ ] All seven primary/backup source files are ordinary, independent, path-contained, byte-identical, and metadata/hash complete.
- [ ] Two independently derived four-file importer inputs obey exact order/modes and 10,000,000-per/32,000,000-total bounds.
- [ ] Import stdout is used only for candidate input/bundle roots; fixed stable ID is independently checked.
- [ ] Validation uses `--root .local/authority/revisions/official-2026-08-20 --bundle bundle.json` plus external ID/root.
- [ ] Two full-set rebuilds are path/byte-identical; source-set/input/derivation/artifact/reference/cycle tamper cases fail.
- [ ] Selected local revision's before/after byte map is identical.
- [ ] Fetch/http/https/net denial stays active during real helper/import/validation.
- [ ] Tracked, staged, and package scans find no private absolute locator or official source/API/normalized/revision/art/PDF/page bytes.
- [ ] Default `pnpm verify` remains synthetic and clean-clone-safe; explicit `pnpm authority:verify-private` fails on missing private evidence.
- [ ] DATA-01/02/03 remain Pending until Plan 08 and phase verification pass.

**Approval:** pending
