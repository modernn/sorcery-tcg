# Phase 2: Deterministic Engine Contract - Summary

**Completed:** 2026-08-28

## Delivered

- Frozen JSON-compatible authoritative state and versioned serializable seeded randomness.
- Shared structured legal-action, rejection, attempt-audit, semantic-event, random-draw, and receipt envelopes.
- Opaque actions bound to seat, state version, and typed descriptors in canonical order.
- Observer-safe rejections that preserve authoritative state, PRNG state, events, and accepted transcript.
- Seat-scoped hidden-information views and a separately privileged internal session/replay path.
- Manifest-seeded replay that re-executes accepted action IDs and compares canonical receipts.
- Fresh-process byte-identical full-transcript verification through the same step path used by the browser adapter.

## Evidence

- `src/engine/contract.ts`
- `src/engine/determinism.ts`
- `src/engine/demo-contract.ts`
- `tests/engine/contract.test.ts`
- `tests/engine/determinism.test.ts`
- `tests/engine/demo-contract.test.ts`
- `tests/engine/prototype-server.test.ts`
- `pnpm verify`: 181 passing tests at phase completion.

## Boundary

The local browser lab remains an explicitly synthetic fixture. Phase 2 does not claim Sorcery setup, spatial, resource, movement, combat, damage, or card-effect behavior. Phase 3 implements those rules behind the completed contract.
