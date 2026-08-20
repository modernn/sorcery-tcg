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
| **Estimated runtime** | ~10 seconds |

---

## Sampling Rate

- **After every task commit:** Run the directly relevant `node --test tests/authority/<area>.test.ts` command plus `pnpm typecheck`.
- **After every plan wave:** Run `pnpm typecheck && pnpm lint && pnpm test`.
- **Before `$gsd-verify-work`:** Full suite must be green and two clean rebuilds must produce byte-identical authority artifacts.
- **Max feedback latency:** 30 seconds.

---

## Per-Task Verification Map

| Task ID | Plan | Wave | Requirement | Threat Ref | Secure Behavior | Test Type | Automated Command | File Exists | Status |
|---------|------|------|-------------|------------|-----------------|-----------|-------------------|-------------|--------|
| 01-01-01 | 01 | 1 | DATA-03 | T-01-01 | Canonical JSON rejects unsupported values and produces stable RFC-compatible bytes/hashes | unit | `node --test tests/authority/canonical-json.test.ts` | ❌ W0 | ⬜ pending |
| 01-01-02 | 01 | 1 | DATA-03 | Strict provenance envelopes reject unknown fields, tampering, broken references, cycles, and path escapes | unit/integration | `node --test tests/authority/provenance.test.ts` | ❌ W0 | ⬜ pending |
| 01-02-01 | 02 | 2 | DATA-02 | Pinned local input normalizes byte-identically and malformed, duplicate, or unknown records fail with exact paths | integration | `node --test tests/authority/card-snapshot.test.ts` | ❌ W0 | ⬜ pending |
| 01-03-01 | 03 | 2 | DATA-01 | Bundle build is write-once, validates all source fields and hashes offline, and reports ambiguous precedence as unsupported | integration/golden | `node --test tests/authority/bundle.test.ts` | ❌ W0 | ⬜ pending |
| 01-03-02 | 03 | 2 | DATA-01 | Storage policy blocks publisher PDFs, images, and unapproved raw corpora from publication | integration | `node --test tests/authority/bundle.test.ts` | ❌ W0 | ⬜ pending |

*Status: ⬜ pending · ✅ green · ❌ red · ⚠️ flaky*

---

## Wave 0 Requirements

- [ ] `package.json`, `pnpm-lock.yaml`, `tsconfig.json`, and flat ESLint config with pinned TypeScript/Node scripts.
- [ ] `tests/authority/canonical-json.test.ts` with RFC-derived ordering, number, Unicode, array, and rejection vectors.
- [ ] `tests/authority/provenance.test.ts` with strict-envelope, reference, tamper, cycle, and path-confinement cases.
- [ ] `tests/authority/card-snapshot.test.ts` with license-safe synthetic valid, malformed, duplicate, and reordered fixtures.
- [ ] `tests/authority/bundle.test.ts` with current, superseded, ambiguous, prohibited-media, write-once, atomic-failure, and offline fixtures.

---

## Manual-Only Verifications

| Behavior | Requirement | Why Manual | Test Instructions |
|----------|-------------|------------|-------------------|
| Publisher permission covers any automated API retrieval and committed normalized derivatives | DATA-01, DATA-02 | The audited public terms do not grant these rights; only written permission can resolve scope | Record reviewer, date, allowed sources, caching/redistribution/image terms, attribution, and revocation expectations before enabling network acquisition or committing restricted derivatives |
| External reuse audit confirms no copied GPL or unlicensed source/assets/data | DATA-03 | Legal provenance and independent creation need human attestation | Review changed files against `docs/external-reuse-policy.md`; record audited repository revisions and sign-off |

---

## Validation Sign-Off

- [ ] All tasks have `<automated>` verification or Wave 0 dependencies.
- [ ] Sampling continuity: no three consecutive tasks without automated verification.
- [ ] Wave 0 covers every missing test reference.
- [ ] No watch-mode flags.
- [ ] Feedback latency remains below 30 seconds.
- [ ] Full suite passes with networking unavailable.
- [ ] Two clean rebuilds are byte-identical and tampering is rejected.

**Approval:** pending
