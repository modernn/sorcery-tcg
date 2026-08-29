# Roadmap: Sorcery Simulator

Requirements live in [REQUIREMENTS.md](./REQUIREMENTS.md). This file owns only phase order, dependencies, outcomes, and current status.

| Phase | Depends on | Requirements | Outcome | Status |
|---|---|---|---|---|
| 1. Rules and Data Authority | — | DATA-01–03 | Immutable offline authority, normalized cards, provenance, and private reuse boundary | Complete; [review resolved](./phases/01-rules-and-data-authority/01-REVIEW.md) |
| 2. Deterministic Engine Contract | 1 | ENG-01–06, TEST-02–03 | Authoritative JSON state, seeded randomness, scoped observations, legal actions, events, and replay receipts | Complete; [summary](./phases/02-deterministic-engine-contract/02-SUMMARY.md) |
| 3. Complete Core Game Rules | 2 | RULE-01–04, RULE-06 | Complete setup, turn, spatial, resource, movement, combat, damage, and ending rules | Pending |
| 4. Storyline, Card Effects, and Coverage | 3 | RULE-05, CARD-01–03, CARD-05 | Typed effects, Storyline ordering, explicit coverage, and fail-closed unsupported routes | Pending |
| 5. Replayable Simulator and Baseline Gauntlets | 4 | SIM-01–07, TEST-05 | Reproducible matches, baseline agents, replay, scheduling, and auditable reports | Pending |
| 6. Owned Collection and Competitive Deck Field | 5 | DATA-04–05, CARD-04, DECK-01–04, TEST-04 | Validated owned decks and a frozen attributable benchmark field | Pending |
| 7. AI Model Competitors | 6 | MODEL-01–06 | Provider-neutral legal-action players and a cited read-only rules adviser | Pending |
| 8. Collection-Constrained Deck Optimization | 7 | OPT-01–04 | Legal owned-card recommendations validated on an untouched holdout | Pending |
| 9. Browser Human Play and Replay | 8 | WEB-01–06, TEST-01 | Accessible local browser play and replay over the same engine | Pending |

## Current phase: 3

Phase 2 established the universal deterministic engine boundary and is complete. Its evidence and deferred real-rule work are recorded in [02-SUMMARY.md](./phases/02-deterministic-engine-contract/02-SUMMARY.md).

Phase 3 now implements actual Sorcery core rules behind that unchanged boundary. The first vertical slice is:

- deterministic match initialization and deck shuffling;
- opening hands and staged mulligan decisions;
- Avatar placement on the 5x4 realm;
- first-turn start sequencing, mandatory domain establishment, and the next draw choice;
- exact replay and hidden-zone verification through the shared contract.

## Backlog

### Phase 999.1: Batch Photo Collection Scanning and Count Reconciliation (BACKLOG)

**Goal:** Turn binder-page or tabletop photos into a reviewed collection list by detecting every card, identifying variant and finish with confidence, counting duplicates, and reconciling changes against the saved collection.
**Requirements:** TBD
**Plans:** 0 plans

Plans:
- [ ] TBD (promote with $gsd-review-backlog when ready)
