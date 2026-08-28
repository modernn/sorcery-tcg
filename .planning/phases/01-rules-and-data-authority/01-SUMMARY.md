---
phase: 01-rules-and-data-authority
implemented: 2026-08-28
status: review_open
requirements: [DATA-01, DATA-02, DATA-03]
---

# Phase 1: Rules and Data Authority

Phase 1 established the offline authority and provenance layer consumed by every later phase. Implementation is present, but the current [code review](./01-REVIEW.md) reports open findings; resolve or explicitly accept the applicable findings before treating this boundary as closed.

## Delivered

- A pinned Node/TypeScript package with native TypeScript execution, `node:test`, strict type checking, and ESLint.
- Bounded canonical JSON and SHA-256 identity helpers.
- Strict artifact, source, card, format, and provenance schemas.
- Deterministic normalization of pinned official card bytes.
- Offline authority validation with path confinement, content hashes, references, precedence, and immutable revision selection.
- Atomic write-once import and read-only validation commands.
- Public synthetic tests plus opt-in private authority/repository-boundary checks.
- A private-local, no-redistribution authority boundary documented in [the external reuse policy](../../../docs/external-reuse-policy.md).

## Durable decisions

- Official rules, format documents, Codex/FAQ updates, card updates, and official card data are the only normative authorities.
- Games select an immutable local authority revision by ID and hash and never consult live mutable authority.
- Canonical artifacts carry stable identity, schema version, content hash, and provenance.
- Community simulators and datasets remain behavioral reference unless their licenses are explicitly accepted.
- Official source bytes, locks, normalized snapshots, and built revisions remain ignored/private under `.local/authority/`.
- New acquisition, recurring updates, sharing, hosting, upload, redistribution, public APIs, official artwork, or commercial use requires written publisher permission and a separate plan.

## Commands

- Default repository check: `pnpm verify`
- Private authority release check: `pnpm authority:verify-private` (requires the authorized ignored local inputs)

## Current evidence

- Safe public authority receipts remain under `data/authority/receipts/`.
- Rebuild and selection instructions remain in `data/authority/README.md`.
- The live review is [01-REVIEW.md](./01-REVIEW.md); it is the only retained Phase 1 gate report.
- Detailed execution plans, per-plan summaries, discussion logs, and superseded verification reports remain available in Git history.

## Next

Address the review findings that affect the Phase 2 boundary, then plan Phase 2 from [its locked context](../02-deterministic-engine-contract/02-CONTEXT.md).
