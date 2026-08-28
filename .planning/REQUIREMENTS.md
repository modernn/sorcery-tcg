# Requirements: Sorcery Simulator

**Defined:** 2026-08-20
**Updated:** 2026-08-28
**Core value:** Simulation results must be reproducible and rules-correct enough that deck and model comparisons are trustworthy.

## Rules and data authority

- [x] **DATA-01**: Build an immutable authority bundle with pinned official sources, precedence, URLs, dates, and SHA-256 hashes.
- [x] **DATA-02**: Build and validate a versioned normalized card snapshot without live network access during a game or experiment.
- [x] **DATA-03**: Give every canonical artifact a stable ID, schema version, provenance record, and content hash.
- [ ] **DATA-04**: Import the supplied collection as exactly 223 copies across 149 canonical card names and report every mismatch.
- [ ] **DATA-05**: Import decklists without silent changes and report legality, inventory, card-data, and coverage failures.

## Deterministic engine contract

- [ ] **ENG-01**: Represent authoritative state as browser-safe JSON-compatible TypeScript data with no client-owned mutation path.
- [ ] **ENG-02**: Own a versioned seeded PRNG whose serializable state never depends on time, scheduling, or global randomness.
- [ ] **ENG-03**: Give each seat an observation that preserves public information and excludes opponent-private information.
- [ ] **ENG-04**: Return stable ordered legal actions and accept only a current `{stateVersion, actionId}`; rejection does not mutate state.
- [ ] **ENG-05**: Return ordered semantic events, canonical hashes, and exact replay evidence for every accepted action.
- [ ] **ENG-06**: Use the same observation/legal-action/step contract for humans, deterministic agents, models, replays, and simulations.

## Core game rules

- [ ] **RULE-01**: Enforce setup, zones, opening hands, mulligan, turns, draw choice, decking, Death's Door, and game end.
- [ ] **RULE-02**: Enforce the 5×4 realm, voids, sites, regions, adjacency, connection, placement, occupancy, and Avatar location.
- [ ] **RULE-03**: Enforce mana, thresholds, additional costs, targets, and payment ordering.
- [ ] **RULE-04**: Enforce summoning, tapping, movement, attack, defense, intercept, strikes, projectiles, damage, healing, and death.
- [ ] **RULE-05**: Implement Storyline ordering, player choices, immediate/triggered/delayed effects, replacements, and continuous modifiers.
- [ ] **RULE-06**: Give each supported core mechanic source-linked scenarios, invariants, and explicit unsupported outcomes.

## Cards and coverage

- [ ] **CARD-01**: Implement card behavior as typed handlers over engine-owned effect primitives.
- [ ] **CARD-02**: Track every relevant rule, keyword, card, token, ruling, and interaction as verified, implemented-unverified, unsupported, or not-applicable.
- [ ] **CARD-03**: Preflight transitive behavior dependencies and invalidate any game that dynamically reaches unsupported behavior.
- [ ] **CARD-04**: Verify all cards and transitive mechanics used by the owned decks and frozen v1 benchmark before ranking them.
- [ ] **CARD-05**: Report the smallest implementation increments that unlock the most ineligible owned and benchmark decks.

## Simulation and replay

- [ ] **SIM-01**: Run a complete non-interactive match between any two shared-contract competitors.
- [ ] **SIM-02**: Include deterministic random and scripted baseline competitors.
- [ ] **SIM-03**: Write an immutable manifest, action transcript, event JSONL, terminal hash, coverage evidence, and classified outcome for every game.
- [ ] **SIM-04**: Schedule predeclared seat-swapped seed blocks with fixed weights, caps, stopping, termination, and failure policies.
- [ ] **SIM-05**: Report counts, W/D/L, uncertainty, opponent and seat results, seat effect, game length, reliability, and eligibility.
- [ ] **SIM-06**: Replay from manifest and actions, verifying every hash and detecting engine/data/version mismatch.
- [ ] **SIM-07**: If workers are justified, produce identical per-job transcripts and reports with one or many workers.

## Deck field and provenance

- [ ] **DECK-01**: Preserve immutable normalized supplied decks and explicitly correct or reject the illegal 62-card Bone Engine list.
- [ ] **DECK-02**: Import attributable online decks with published pilot, event, result, date, ruleset, raw source, and hashes.
- [ ] **DECK-03**: Freeze a deduplicated archetype-labeled v1 benchmark with documented strata and weights.
- [ ] **DECK-04**: Distinguish equal-opponent results from evidence-weighted field results.

## AI competitors

- [ ] **MODEL-01**: Let provider-neutral model players see only the shared redacted observation and legal actions and return only an action ID.
- [ ] **MODEL-02**: Record model/config, prompt version, response or hash, action/fallback, classification, latency, tokens, cost, and failures.
- [ ] **MODEL-03**: Apply one predeclared deterministic fallback or forfeit policy to malformed, illegal, timed-out, or failed decisions.
- [ ] **MODEL-04**: Hold decks, opponents, seats, seeds, observations, actions, prompts, sampling, and budgets fixed for comparisons.
- [ ] **MODEL-05**: Replay recorded model action IDs without contacting the provider.
- [ ] **MODEL-06**: Keep the cited local rules adviser read-only and make any consulted game unranked.

## Collection-constrained optimization

- [ ] **OPT-01**: Generate only legal, owned, implementation-verified local deck mutations and record each hash and diff.
- [ ] **OPT-02**: Search on frozen development games and compare finalists on one untouched versioned holdout.
- [ ] **OPT-03**: Report exact swaps, inventory proof, matchup deltas, uncertainty, worst matchup, and representative replays.
- [ ] **OPT-04**: Allow a no-distinguishable-improvement result and never tune further on a consumed holdout.

## Browser human play

- [ ] **WEB-01**: Complete a local browser game against an automated or model competitor.
- [ ] **WEB-02**: Render the game from engine observations/events using only original/project-owned or user-supplied private art.
- [ ] **WEB-03**: Expose only engine-issued actions and never compute rules or effects independently.
- [ ] **WEB-04**: Inspect a recorded replay step by step.
- [ ] **WEB-05**: Provide keyboard equivalents, visible focus, non-color cues, and accessible action/rejection/event text.
- [ ] **WEB-06**: Produce identical hashes through headless and browser adapters for the same transcript.

## Verification and release

- [ ] **TEST-01**: Run unit, scenario, invariant, integration, replay, import, coverage, and end-to-end checks through one documented command.
- [ ] **TEST-02**: Reproduce deterministic-agent transcripts byte-for-byte in fresh processes and supported environments.
- [ ] **TEST-03**: Prove hidden opponent state cannot affect a seat's observation or legal actions until officially revealed.
- [ ] **TEST-04**: Mark no result ranked unless pinned-input, legality, coverage, execution, replay, design, and reporting gates pass.
- [ ] **TEST-05**: Account for every scheduled trial exactly once as completed, drawn, invalid, competitor failure, or infrastructure failure.

## Out of scope for v1

Interactive human CLI play, authoritative LLM rulings, balance-changing rules, silent approximations, perfect-play solving, general English-to-card execution, trading/pricing, model training, native mobile clients, public multiplayer, distributed simulation, and database infrastructure.

Phase ownership and status are maintained once in [ROADMAP.md](./ROADMAP.md).
