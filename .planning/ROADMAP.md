# Roadmap: Sorcery Simulator

Requirements live in [REQUIREMENTS.md](./REQUIREMENTS.md). This file owns only phase order, dependencies, outcomes, and current status.

| Phase | Depends on | Requirements | Outcome | Status |
|---|---|---|---|---|
| 1. Rules and Data Authority | — | DATA-01–03 | Immutable offline authority, normalized cards, provenance, and private reuse boundary | Implemented; [review findings open](./phases/01-rules-and-data-authority/01-REVIEW.md) |
| 2. Deterministic Engine Contract | 1 | ENG-01–06, TEST-02–03 | Authoritative JSON state, seeded randomness, scoped observations, legal actions, events, and replay receipts | Ready to plan |
| 3. Complete Core Game Rules | 2 | RULE-01–04, RULE-06 | Complete setup, turn, spatial, resource, movement, combat, damage, and ending rules | Pending |
| 4. Storyline, Card Effects, and Coverage | 3 | RULE-05, CARD-01–03, CARD-05 | Typed effects, Storyline ordering, explicit coverage, and fail-closed unsupported routes | Pending |
| 5. Replayable Simulator and Baseline Gauntlets | 4 | SIM-01–07, TEST-05 | Reproducible matches, baseline agents, replay, scheduling, and auditable reports | Pending |
| 6. Owned Collection and Competitive Deck Field | 5 | DATA-04–05, CARD-04, DECK-01–04, TEST-04 | Validated owned decks and a frozen attributable benchmark field | Pending |
| 7. AI Model Competitors | 6 | MODEL-01–06 | Provider-neutral legal-action players and a cited read-only rules adviser | Pending |
| 8. Collection-Constrained Deck Optimization | 7 | OPT-01–04 | Legal owned-card recommendations validated on an untouched holdout | Pending |
| 9. Browser Human Play and Replay | 8 | WEB-01–06, TEST-01 | Accessible local browser play and replay over the same engine | Pending |

## Current phase: 2

Phase 2 defines the universal engine boundary without implementing Sorcery setup, turns, spatial rules, combat, card effects, simulation scheduling, model providers, or UI. Its accepted decisions are in [02-CONTEXT.md](./phases/02-deterministic-engine-contract/02-CONTEXT.md).

Completion requires:

- deterministic JSON-compatible state and serializable seeded randomness;
- seat-scoped observations with hidden-information safety;
- stable state-version-bound legal actions and mutation-free rejection;
- canonical semantic event receipts and byte-exact action replay;
- one public observation/action/step contract for every client and competitor.

Plan Phase 2 only after the open Phase 1 review findings required by this boundary are resolved or explicitly accepted.
