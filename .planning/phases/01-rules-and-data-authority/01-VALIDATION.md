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
| **Phase release command** | `pnpm verify && pnpm authority:verify-private` (explicit private-data gate added by Plan 08; default tests stay clean-clone-safe) |
| **Estimated runtime** | ~10 seconds |

---

## Sampling Rate

- **After every task commit:** Run the directly relevant `node --test tests/authority/<area>.test.ts` command plus `pnpm typecheck`.
- **After every plan wave:** Run `pnpm typecheck && pnpm lint && pnpm test`.
- **Before `$gsd-verify-work`:** The release command must pass with explicit network denial, independent primary/backup hashes, two clean private-input rebuilds, external root-hash validation, tamper copies, tracked/staged/package restricted-content scans, and an unchanged selected local revision.
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
| 01-06-02 | 06 | 4 | DATA-01, DATA-03 | T-01-26, T-01-27, T-01-30 | Fixed manual private-local path, `.local/authority/` Git exclusion, community-license audit, clean-room boundary, no art/PDF/public API, and future permission triggers are closed | policy/integration | `git check-ignore -q .local/authority/probe && node -e "const s=require('node:fs').readFileSync('docs/external-reuse-policy.md','utf8').toLowerCase();for(const x of ['manual browser','private','noncommercial','no redistribution','.local/authority/','git-ignored','no fetch','no scraping','no polling','public/network','art','pdf','full corpus','gpl-3.0','sadkinglabs/sorcery-registry','mit covers code only','reserved','no reusable license','clean-room','sha-256','backup','written publisher permission','separate plan'])if(!s.includes(x))process.exit(1)" && pnpm verify` | ❌ P06 | ⬜ pending |
| 01-07-01 | 07 | 5 | DATA-01, DATA-02, DATA-03 | T-01-31, T-01-32, T-01-33, T-01-34, T-01-35 | Manual full-response save plus an outside-Git byte-identical backup, exact hash, retrieval timestamp, and private/noncommercial/no-redistribution attestation release the local build without claiming legal permission | automated pre/postcheck + blocking human action | `git check-ignore -q .local/authority/inputs/official-2026-08-20/cards.raw.json && pnpm verify` before pause; after response run Plan 07's exact ordinary-file/JSON/containment/byte/SHA-256/tracked/staged check with `AUTHORITY_BACKUP_FILE` | N/A checkpoint | ⬜ pending |
| 01-08-01 | 08 | 6 | DATA-01, DATA-02, DATA-03 | T-01-36, T-01-37, T-01-38, T-01-39, T-01-40 | Plan 07 primary/backup bytes are independently rehashed; one official revision is built under ignored `.local/authority/`, externally revalidated, and represented in Git only by a safe receipt/README | integration/golden | `node -e "const fs=require('node:fs'),c=require('node:crypto'),cp=require('node:child_process');const r=JSON.parse(fs.readFileSync('data/authority/receipts/official-2026-08-20.json','utf8')),a=fs.readFileSync('.local/authority/inputs/official-2026-08-20/cards.raw.json'),b=fs.readFileSync(r.backupLocator),h=x=>'sha256:'+c.createHash('sha256').update(x).digest('hex');if(a.length!==r.rawByteLength||b.length!==a.length||h(a)!==r.rawSha256||h(b)!==r.rawSha256)process.exit(1);if(cp.execFileSync('git',['ls-files','.local/authority'],{encoding:'utf8'}).trim())process.exit(1);cp.execFileSync('pnpm',['authority:validate','--','--root','.local/authority/revisions/official-2026-08-20','--bundle','.local/authority/revisions/official-2026-08-20/bundle.json','--id',r.bundleId,'--hash',r.bundleRootHash],{stdio:'inherit',shell:process.platform==='win32'})"` | ❌ P08 | ⬜ pending |
| 01-08-02 | 08 | 6 | DATA-01, DATA-02, DATA-03 | T-01-36, T-01-37, T-01-38, T-01-39, T-01-40, T-01-41 | Independent primary/backup double build, no-network sentinels, byte-map comparison, external-hash validation, tamper copies, selected-revision immutability, and Git/staged/package leakage scans all pass | release/integration | `pnpm verify && pnpm authority:verify-private` | ❌ P08 | ⬜ pending |

*Status: ⬜ pending · ✅ green · ❌ red · ⚠️ flaky*

---

## Wave 0 Requirements

- [ ] `package.json`, `pnpm-lock.yaml`, `tsconfig.json`, and flat ESLint config with pinned TypeScript/Node scripts.
- [ ] `tests/authority/canonical-json.test.ts` with RFC-derived ordering, number, Unicode, array, and rejection vectors.
- [ ] `tests/authority/provenance.test.ts` with strict-envelope, reference, tamper, cycle, and path-confinement cases.
- [ ] `tests/authority/card-snapshot.test.ts` with license-safe synthetic valid, malformed, duplicate, and reordered fixtures.
- [ ] `tests/authority/bundle.test.ts` with current, superseded, ambiguous, authority-class, stored-source, manifest-only, input-lock, prohibited-media, write-once, atomic-failure, and offline fixtures.
- [ ] `tests/authority/fixtures/bundle-input/input-lock.json` with a synthetic exact file set, per-file SHA-256 values, and canonical input-root hash.

Plan 08 adds `tests/private-authority/private-revision.test.ts` outside the clean-clone default test glob after the real private revision exists. It is not a Wave 0 gap; the explicit `pnpm authority:verify-private` command must fail rather than skip when its primary, backup, receipt, revision, or attestation is absent.

---

## Manual-Only Verifications

| Behavior | Requirement | Why Manual | Test Instructions |
|----------|-------------|------------|-------------------|
| One complete official API response and durable byte-identical backup exist for private local use | DATA-01, DATA-02 | The project is forbidden from automating acquisition; only the user can save through their browser and select private backup storage | Save `https://api.sorcerytcg.com/api/cards` unchanged to the fixed ignored path, copy exact bytes to an outside-Git durable locator, provide UTC retrieval time, and attest private/noncommercial/no redistribution/manual browser/no automation/no art-or-PDF/no legal conclusion; halt without Plans 07/08 summaries if incomplete |
| External reuse audit confirms no copied GPL, unlicensed, or publisher-reserved source/assets/data | DATA-03 | Independent clean-room provenance requires a human attestation | Review changed files against `docs/external-reuse-policy.md`; record audited repository revisions and sign-off; do not treat community card data as a licensed corpus |

---

## Validation Sign-Off

- [ ] All tasks have `<automated>` verification or Wave 0 dependencies.
- [ ] Sampling continuity: no three consecutive tasks without automated verification.
- [ ] Wave 0 covers every missing test reference.
- [ ] No watch-mode flags.
- [ ] Feedback latency remains below 30 seconds.
- [ ] Release test explicitly denies fetch/http/https/net access.
- [ ] The fixed primary and outside-Git durable backup are ordinary non-symlink files with matching independently computed byte length/SHA-256 and the complete private-use attestation.
- [ ] Two clean rebuilds, one from each exact private copy, are path- and byte-identical.
- [ ] Independently captured/recomputed expected hashes validate both candidates and the selected local revision; no candidate self-hash is trusted.
- [ ] Stored-byte, canonical artifact, source-reference, and cycle tamper copies are rejected; the selected local revision's before/after byte map is identical.
- [ ] Tracked, staged, and package dry-run scans prove Git/package contents contain no official raw/normalized corpus, private revision, card art, publisher PDF, or copied reference/community material.
- [ ] Default `pnpm test` remains synthetic and clean-clone-safe; only `pnpm authority:verify-private` requires local private evidence and it fails rather than skips when absent.

**Approval:** pending
