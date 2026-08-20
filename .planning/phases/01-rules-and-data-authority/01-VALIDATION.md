---
phase: 1
slug: rules-and-data-authority
status: draft
nyquist_compliant: true
wave_0_complete: false
created: 2026-08-20
---

# Phase 1 — Validation Strategy

> Per-phase validation contract for feedback sampling during execution.

---

## Test Infrastructure

| Property | Value |
|----------|-------|
| **Framework** | `node:test` and `node:assert/strict` on Node 24 |
| **Config file** | none — Wave 0 adds package scripts |
| **Quick run command** | `node --test tests/authority/*.test.ts` |
| **Full suite command** | `pnpm typecheck && pnpm lint && pnpm test` |
| **Phase release command** | `pnpm typecheck && pnpm lint && pnpm test && pnpm authority:verify-release` (added by Plan 08) |
| **Estimated runtime** | ~10 seconds |

---

## Sampling Rate

- **After every task commit:** Run the directly relevant `node --test tests/authority/<area>.test.ts` command plus `pnpm typecheck`.
- **After every plan wave:** Run `pnpm typecheck && pnpm lint && pnpm test`.
- **Before `$gsd-verify-work`:** The release command must pass with explicit network denial, two clean durable-input rebuilds, independent root-hash validation, tamper copies, restricted-content scans, and an unchanged published revision.
- **Max feedback latency:** 30 seconds.

---

## Per-Task Verification Map

| Task ID | Plan | Wave | Requirement | Threat Ref | Secure Behavior | Test Type | Automated Command | File Exists | Status |
|---------|------|------|-------------|------------|-----------------|-----------|-------------------|-------------|--------|
| 01-01-01 | 01 | 0 | DATA-01, DATA-02, DATA-03 | T-01-SC | Exact approved Node/pnpm/TypeScript/Zod/ESLint graph is pinned before source work | config/supply-chain | `node -e "if(process.versions.node!=='24.19.0')process.exit(1)" && pnpm --version \| node -e "let s='';process.stdin.on('data',d=>s+=d).on('end',()=>{if(s.trim()!=='11.22.0')process.exit(1)})" && pnpm install --frozen-lockfile && pnpm exec tsc --version && pnpm exec eslint --version` | ❌ W0 | ⬜ pending |
| 01-01-02 | 01 | 0 | DATA-01, DATA-02, DATA-03 | T-01-01, T-01-05 | All authority behavior has executable todo contracts and only license-safe fixtures | contract | `node --test tests/authority/*.test.ts && node -e "const fs=require('node:fs');for(const f of ['canonical-json','provenance','card-snapshot','bundle'])if(!fs.readFileSync('tests/authority/'+f+'.test.ts','utf8').includes('test.todo'))process.exit(1);JSON.parse(fs.readFileSync('tests/authority/fixtures/bundle-input/input-lock.json','utf8'))"` | ❌ W0 | ⬜ pending |
| 01-02-01 | 02 | 1 | DATA-03 | T-01-06, T-01-09 | Canonical JSON rejects unsupported values and produces stable identity hashes | unit | `node --test tests/authority/canonical-json.test.ts && pnpm typecheck` | ❌ W0 | ⬜ pending |
| 01-02-02 | 02 | 1 | DATA-03 | T-01-06, T-01-08, T-01-10 | Strict envelopes separate stored/manifest sources and keep non-official provenance non-normative | unit | `node --test --test-name-pattern="schema\|envelope\|source\|authority\|storage\|diagnostic\|duplicate\|tamper" tests/authority/provenance.test.ts && pnpm typecheck` | ❌ W0 | ⬜ pending |
| 01-03-01 | 03 | 2 | DATA-02, DATA-03 | T-01-11, T-01-13, T-01-15 | Card normalization is strict, lossless, duplicate-aware, and clean-room safe | integration | `node --test --test-name-pattern="valid\|malformed\|unknown\|duplicate\|count\|path" tests/authority/card-snapshot.test.ts && pnpm typecheck` | ❌ W0 | ⬜ pending |
| 01-03-02 | 03 | 2 | DATA-02, DATA-03 | T-01-14 | Repeated property-reordered normalization is byte-identical and offline | integration/property | `node --test tests/authority/card-snapshot.test.ts && pnpm typecheck && pnpm lint` | ❌ W0 | ⬜ pending |
| 01-04-01 | 04 | 2 | DATA-01, DATA-03 | T-01-16, T-01-17, T-01-19 | Paths/graphs are bounded; stored bytes are rehashed; manifest-only locator/hash/ref bindings are verified honestly | integration | `node --test tests/authority/provenance.test.ts && pnpm typecheck` | ❌ W0 | ⬜ pending |
| 01-04-02 | 04 | 2 | DATA-01, DATA-03 | T-01-18, T-01-20 | Only official authority can win; ambiguity and prohibited storage fail offline | integration/golden | `node --test --test-name-pattern="current\|superseded\|scoped\|ambiguous\|offline\|storage\|media" tests/authority/bundle.test.ts && pnpm typecheck && pnpm lint` | ❌ W0 | ⬜ pending |
| 01-05-01 | 05 | 3 | DATA-01, DATA-02 | T-01-21, T-01-22, T-01-23, T-01-24, T-01-25 | Import verifies exact durable input lock/root, emits trusted roots, and publishes atomically | integration | `node --test --test-name-pattern="input\|root\|build\|byte-identical\|write-once\|atomic\|failure\|storage" tests/authority/bundle.test.ts && pnpm typecheck` | ❌ W0 | ⬜ pending |
| 01-05-02 | 05 | 3 | DATA-01, DATA-02 | T-01-21, T-01-24, T-01-25 | Offline command reports stored-byte rehash vs manifest binding and never accepts raw input | integration | `node --test tests/authority/bundle.test.ts && pnpm typecheck && pnpm lint && pnpm test` | ❌ W0 | ⬜ pending |
| 01-06-01 | 06 | 4 | DATA-01, DATA-03 | T-01-28, T-01-29 | Project precedence is reviewable, official-only, effective-dated, and fail-closed | policy/integration | `node -e "const s=require('node:fs').readFileSync('docs/authority-precedence.md','utf8').toLowerCase();for(const x of ['official rulebook','format','codex','faq','card update','effective date','scoped','superseded','contending','unsupported','community'])if(!s.includes(x))process.exit(1)" && pnpm test` | ❌ P06 | ⬜ pending |
| 01-06-02 | 06 | 4 | DATA-01, DATA-03 | T-01-30 | Permission, durable-input, storage, and clean-room policy defaults closed | policy/integration | `node -e "const s=require('node:fs').readFileSync('docs/external-reuse-policy.md','utf8').toLowerCase();for(const x of ['permission-required','gpl-3.0','no reusable license','code','tests','assets','data','pdf','image','raw api','automated','normalized derivative','reviewer','attribution','revocation','independent','durable input','input-root','immutable','acquisition procedure','mismatch'])if(!s.includes(x))process.exit(1)" && pnpm verify` | ❌ P06 | ⬜ pending |
| 01-07-01 | 07 | 5 | DATA-01, DATA-02, DATA-03 | T-01-33, T-01-34, T-01-35 | Publication remains absent until written scope, clean-room attestation, and one exact durable input path are approved | automated precheck + blocking human | `node -e "const fs=require('node:fs');const s=fs.readFileSync('docs/external-reuse-policy.md','utf8').toLowerCase();if(!s.includes('permission-required'))process.exit(1);if(fs.existsSync('data/authority/official-2026-08-20'))process.exit(1)" && pnpm verify` | N/A checkpoint | ⬜ pending |
| 01-08-01 | 08 | 6 | DATA-01, DATA-02, DATA-03 | T-01-36, T-01-37, T-01-38, T-01-39, T-01-40 | Approved input/root is checked; captured import root is recomputed independently; publication follows conditional allowlist | integration/golden | `node --input-type=module -e "import fs from 'node:fs';import cp from 'node:child_process';import path from 'node:path';import {identityHash} from './src/authority/hash.ts';const policy=fs.readFileSync('docs/external-reuse-policy.md','utf8');const expected=/Published bundle root hash:\s*(sha256:[0-9a-f]{64})/i.exec(policy)?.[1];const inputRoot=/Approved input root hash:\s*(sha256:[0-9a-f]{64})/i.exec(policy)?.[1];const kind=/Durable input kind:\s*(repository-stored\|immutable-custodian\|approved-acquisition-procedure)/i.exec(policy)?.[1]?.toLowerCase();if(!expected\|\|!inputRoot\|\|!kind)process.exit(1);const root='data/authority/official-2026-08-20';const b=JSON.parse(fs.readFileSync(path.join(root,'bundle.json'),'utf8'));if(identityHash(b.identity)!==expected)process.exit(1);cp.execFileSync(process.execPath,['src/commands/validate-authority.ts','--root','data/authority','--bundle','official-2026-08-20/bundle.json','--id','official-2026-08-20','--hash',expected],{stdio:'inherit'});const found=[];for(const e of fs.readdirSync(root,{recursive:true,withFileTypes:true}))if(e.isFile())found.push(path.relative(root,path.join(e.parentPath,e.name)).replaceAll('\\\\','/'));const allowed=new Set(['bundle.json','sources.json','formats.json','cards.normalized.json']);if(kind==='repository-stored')allowed.add('raw/cards.raw.json');if(found.length!==allowed.size\|\|found.some(f=>!allowed.has(f)))process.exit(1)" && pnpm typecheck` | ❌ P08 | ⬜ pending |
| 01-08-02 | 08 | 6 | DATA-01, DATA-02, DATA-03 | T-01-36, T-01-37, T-01-38, T-01-39, T-01-40 | Explicit no-network double rebuild, byte compare, external-hash validation, tamper copies, restricted scan, and immutable published bytes all pass | release/integration | `pnpm typecheck && pnpm lint && pnpm test && pnpm authority:verify-release` | ❌ P08 | ⬜ pending |

*Status: ⬜ pending · ✅ green · ❌ red · ⚠️ flaky*

---

## Wave 0 Requirements

- [ ] `package.json`, `pnpm-lock.yaml`, `tsconfig.json`, and flat ESLint config with pinned TypeScript/Node scripts.
- [ ] `tests/authority/canonical-json.test.ts` with RFC-derived ordering, number, Unicode, array, and rejection vectors.
- [ ] `tests/authority/provenance.test.ts` with strict-envelope, reference, tamper, cycle, and path-confinement cases.
- [ ] `tests/authority/card-snapshot.test.ts` with license-safe synthetic valid, malformed, duplicate, and reordered fixtures.
- [ ] `tests/authority/bundle.test.ts` with current, superseded, ambiguous, authority-class, stored-source, manifest-only, input-lock, prohibited-media, write-once, atomic-failure, and offline fixtures.
- [ ] `tests/authority/fixtures/bundle-input/input-lock.json` with a synthetic exact file set, per-file SHA-256 values, and canonical input-root hash.

Plan 08 adds `tests/authority/official-revision.test.ts` after the real approved revision exists; it is not a Wave 0 gap and must fail rather than skip when its required approval/input environment is absent.

---

## Manual-Only Verifications

| Behavior | Requirement | Why Manual | Test Instructions |
|----------|-------------|------------|-------------------|
| Publisher permission and durable exact-byte input cover any acquisition, stored raw bytes, and committed normalized derivatives | DATA-01, DATA-02 | The audited public terms do not grant these rights, and only a human can approve the durable source/custodian/procedure | Record reviewer, date, allowed sources, caching/redistribution/image terms, attribution, revocation expectations, one durable input kind, exact locator/procedure, per-file hashes, and canonical input-root hash; halt without Plans 07/08 summaries if incomplete |
| External reuse audit confirms no copied GPL or unlicensed source/assets/data | DATA-03 | Legal provenance and independent creation need human attestation | Review changed files against `docs/external-reuse-policy.md`; record audited repository revisions and sign-off |

---

## Validation Sign-Off

- [ ] All tasks have `<automated>` verification or Wave 0 dependencies.
- [ ] Sampling continuity: no three consecutive tasks without automated verification.
- [ ] Wave 0 covers every missing test reference.
- [ ] No watch-mode flags.
- [ ] Feedback latency remains below 30 seconds.
- [ ] Release test explicitly denies fetch/http/https/net access.
- [ ] Two clean rebuilds from the approved durable input lock are path- and byte-identical.
- [ ] Independently captured/recomputed expected hashes validate the official and candidate bundles; no candidate self-hash is trusted.
- [ ] Stored-byte and manifest-only binding tamper copies are rejected; restricted-content scans pass; the published revision's before/after byte map is identical.

**Approval:** pending
