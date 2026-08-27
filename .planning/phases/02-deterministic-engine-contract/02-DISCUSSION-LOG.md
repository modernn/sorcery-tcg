# Phase 2: Deterministic Engine Contract - Discussion Log

> **Audit trail only.** Do not use as input to planning, research, or execution agents.
> Decisions are captured in CONTEXT.md — this log preserves the alternatives considered.

**Date:** 2026-08-26
**Phase:** 2-deterministic-engine-contract
**Areas discussed:** Hidden-information views, Legal-action shape, Stale and illegal rejection, Events and randomness

---

## Hidden-information views

| Question | Recommended and selected | Alternatives considered |
|----------|--------------------------|-------------------------|
| Opponent-hidden representation | Reveal-safe placeholders preserving legally public structure | Stable hidden handles; counts only |
| Seat observation packaging | One self-contained public + own-private + redacted-opponent snapshot | Separate public/private payloads; private lookups |
| Revealed card becoming hidden | Preserve identity only while officially trackable; erase after randomization or loss of trackability | Hide immediately; retain indefinitely |
| Omniscient access | Separate privileged replay/verification interface unavailable to competitors | Optional flag on normal observation; no privileged view |

**User's choice:** Accepted all recommended answers. Q1 was initially selected directly before the user requested smart-discuss review tables.
**Notes:** Hidden-information safety must hold across observations, legal choices, events, diagnostics, and random outcomes.

---

## Legal-action shape

| Question | Recommended and selected | Alternatives considered |
|----------|--------------------------|-------------------------|
| Complex choices | Staged engine-owned decision states with fully bound actions at each stage | Enumerate every combination; parameterized commands |
| Action identity | Deterministic opaque `actionId` bound to `stateVersion` plus typed descriptor | List index; human-readable semantic ID |
| Ordering | One canonical engine order for all consumers | Client-defined order; insertion order |
| Pass/progression | Explicit engine-issued choices; no client inference | Implicit client pass; automatic engine progression |

**User's choice:** Accepted all recommended answers.
**Notes:** Presentation grouping may differ, but identity and canonical order may not.

---

## Stale and illegal rejection

| Question | Recommended and selected | Alternatives considered |
|----------|--------------------------|-------------------------|
| Expected rejection result | Typed result with stable code, safe message, current version, and unchanged hash | Throw every rejection; boolean only |
| Stale/duplicate submissions | Exact-version rejection with no mutation or retry | Cached result; reinterpret against current state |
| Audit placement | Separate deterministic attempt log | Semantic game event; discard |
| Diagnostic detail | Specific stable observer-safe codes | One generic code; unrestricted internal details |

**User's choice:** Accepted all recommended answers.
**Notes:** Exceptions remain reserved for engine defects, not ordinary illegal choices.

---

## Events and randomness

| Question | Recommended and selected | Alternatives considered |
|----------|--------------------------|-------------------------|
| Event meaning | Ordered typed semantic outcomes with causal links | Raw state diffs; one large event per action |
| Accepted-action proof | Canonical receipt with sequence, identity, pre/post hashes, events, and next version | Per-event hashes; terminal hash only |
| Random evidence | Versioned PRNG plus privileged draw purpose/ordinal/domain/result/state-hash evidence | Seed only; public raw PRNG stream |
| Replay authority | Re-execute accepted actions and verify canonical receipts byte-for-byte | Event-sourced mutation replay; trust terminal state |

**User's choice:** Accepted all recommended answers.
**Notes:** Seat views redact hidden random results; privileged replay evidence remains complete.

---

## the agent's Discretion

- Exact TypeScript names, module boundaries, opaque-ID encoding, state-version representation, and versioned PRNG choice within the accepted contracts.

## Deferred Ideas

- Publisher authorization and any recurring catalog updater remain outside Phase 2.
