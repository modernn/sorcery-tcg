# Requirements: Sorcery Simulator

**Defined:** 2026-08-20
**Core Value:** Simulation results must be reproducible and rules-correct enough that deck and model comparisons are trustworthy.

## v1 Requirements

### Rules and Data Authority

- [ ] **DATA-01**: A developer can build an immutable authority bundle containing the pinned official rulebook, format rules, Codex/FAQ/card updates, precedence policy, source URLs, retrieval dates, and SHA-256 hashes.
- [ ] **DATA-02**: A developer can build and validate a versioned normalized card snapshot without live network access during a game or experiment.
- [ ] **DATA-03**: Every canonical card, rule, format, deck, collection, behavior, and experiment artifact has a stable ID, schema version, provenance record, and content hash.
- [ ] **DATA-04**: The application imports the supplied collection as exactly 223 copies across 149 canonical card names and reports every unknown, ambiguous, or mismatched entry.
- [ ] **DATA-05**: The application imports decklists without silently changing them and reports format legality, copy-limit, inventory, card-data, and implementation-coverage failures.

### Deterministic Engine Contract

- [ ] **ENG-01**: The engine represents complete authoritative game state as browser-safe, JSON-compatible TypeScript data with no I/O or client-owned mutation path.
- [ ] **ENG-02**: The engine owns a versioned seeded PRNG whose serializable state is included in game state and whose output never depends on wall time, process scheduling, or global randomness.
- [ ] **ENG-03**: Each seat receives a player-scoped observation that excludes opponent-private information while preserving all public information.
- [ ] **ENG-04**: The engine returns a stable ordered legal-action list and accepts only a current `{stateVersion, actionId}`; stale, forged, or illegal choices leave authoritative state unchanged.
- [ ] **ENG-05**: Every accepted action returns ordered semantic events, canonical hashes, and enough information for exact action-log replay.
- [ ] **ENG-06**: Human clients, deterministic agents, model agents, replays, and simulations all use the same public observation/legal-action/step contract.

### Core Game Rules

- [ ] **RULE-01**: The engine enforces official game setup, deck zones, opening hands, mulligan, first-player draw rule, turn phases, draw choice, decking loss, Death's Door, and game-end conditions.
- [ ] **RULE-02**: The engine enforces the 5×4 realm, voids and sites, surface and non-surface regions, adjacency/connection, site placement, occupancy, and Avatar location rules.
- [ ] **RULE-03**: The engine enforces mana generation and spending, elemental thresholds, additional costs, targeting requirements, and cost-payment ordering.
- [ ] **RULE-04**: The engine enforces summoning, summoning sickness, tapping/untapping, movement, Movement modifiers, airborne and regional movement, Move and Attack, Defend, Intercept, strikes, fights, projectiles, damage, healing, and unit death.
- [ ] **RULE-05**: The engine implements the official Storyline ordering model, active/non-active ordering, pending player choices, immediate/triggered/delayed effects, replacement effects, and continuous modifiers required by supported cards.
- [ ] **RULE-06**: Each supported core mechanic has source-linked scenario tests, invariant tests, and explicit unsupported outcomes for routes not yet implemented.

### Cards and Coverage

- [ ] **CARD-01**: Card behavior is implemented as typed handlers that request engine-owned effect primitives rather than mutating state directly or interpreting English during a game.
- [ ] **CARD-02**: Every rule, keyword, card, token, ruling, and known interaction has a versioned coverage status of `verified`, `implemented-unverified`, `unsupported`, or `not-applicable` with evidence links.
- [ ] **CARD-03**: Static matchup preflight follows transitive behavior dependencies, and any unsupported behavior reached dynamically invalidates the game without applying a partial mutation.
- [ ] **CARD-04**: All cards and transitive mechanics used by the supplied owned decks and the frozen v1 benchmark field are verified before those matchups can be ranked.
- [ ] **CARD-05**: A capability report identifies which smallest rule/card implementation increments unlock the most currently ineligible owned and benchmark decks.

### Simulation and Replay

- [ ] **SIM-01**: A non-interactive command can run a complete match between any two competitors that implement the shared action-selection contract.
- [ ] **SIM-02**: The project includes deterministic random and baseline scripted competitors used to test engine correctness independently of external models.
- [ ] **SIM-03**: Every scheduled game writes an immutable run manifest, action transcript, semantic event JSONL, terminal state hash, coverage evidence, and classified outcome.
- [ ] **SIM-04**: A gauntlet scheduler uses predeclared seat-swapped seed blocks, field weights, game cap, stopping rule, termination policy, and failure policy.
- [ ] **SIM-05**: Reports include scheduled/completed/invalid counts, W/D/L, uncertainty intervals, per-opponent and per-seat results, seat effect, game length, reliability failures, and coverage eligibility.
- [ ] **SIM-06**: Replay reconstructs a game from its manifest and action transcript, verifies every recorded hash, and detects any engine/data/version mismatch.
- [ ] **SIM-07**: When parallel workers are enabled after profiling, one-worker and multi-worker runs produce identical per-job transcripts and reports.

### Deck Field and Provenance

- [ ] **DECK-01**: The repository contains immutable normalized revisions of the supplied decks and an explicit correction or rejection for the illegal 62-card Bone Engine list.
- [ ] **DECK-02**: The application imports representative commonly played and competitive decks from attributable online sources while preserving author/pilot, event, placing/record when published, date, ruleset, raw source, and hashes.
- [ ] **DECK-03**: The v1 benchmark is a frozen, deduplicated, archetype-labeled field with documented strata and weights and never claims unsupported metagame share.
- [ ] **DECK-04**: Ranked field reports distinguish equal-opponent results from any evidence-weighted field result.

### AI Competitors

- [ ] **MODEL-01**: A versioned provider-neutral player skill and its adapters receive only the same redacted observation and legal-action list as other competitors and can return only an action ID.
- [ ] **MODEL-02**: Each model decision records model/configuration, prompt/template version, response or response hash, selected/fallback action, classification, latency, tokens, cost, and provider failure details.
- [ ] **MODEL-03**: Malformed, illegal, timed-out, or failed model decisions follow one predeclared deterministic fallback or forfeit policy and remain visible in reliability metrics.
- [ ] **MODEL-04**: Model experiments use the same decks, opponents, seats, seed blocks, observation schema, legal-action ordering, prompt, sampling settings, and budgets, and report deck, model, and model-by-deck effects separately.
- [ ] **MODEL-05**: Recorded model action IDs can be replayed without contacting the provider to reproduce engine outcomes exactly.
- [ ] **MODEL-06**: A versioned rules-adviser skill retrieves only from the selected local authority, cites its evidence, and returns explanation only; it cannot mutate state, invent legal actions, or make a consulted game ranked.

### Collection-Constrained Optimization

- [ ] **OPT-01**: The optimizer generates only format-legal, inventory-valid, implementation-verified local mutations of a starting deck and records every candidate hash and diff.
- [ ] **OPT-02**: Candidate search uses a frozen development field and seeds while final comparison uses one untouched, versioned holdout field and seed set.
- [ ] **OPT-03**: Recommendations compare the unchanged starting deck and finalist using the same deterministic pilot and include exact swaps, inventory proof, per-matchup deltas, uncertainty, worst matchup, and representative replays.
- [ ] **OPT-04**: The optimizer can conclude that no statistically distinguishable owned-card improvement was found and never adapts further search to a consumed holdout.

### Browser Human Play

- [ ] **WEB-01**: A human can start and complete a local browser game against an automated or model competitor without using an interactive CLI.
- [ ] **WEB-02**: The browser renders the realm, regions, zones, cards, public state, current Storyline, and forced choices from engine observations and engine events, using only original/project-owned presentation art or user-supplied private local images.
- [ ] **WEB-03**: The browser exposes only engine-issued legal actions, targets, paths, defenses, reactions, and choices and never computes rules or card effects independently.
- [ ] **WEB-04**: A human can load and inspect a recorded replay with step-by-step state, event, action, and eligibility information.
- [ ] **WEB-05**: All game interactions have keyboard equivalents, visible focus, non-color-only state cues, and accessible action/rejection/event text.
- [ ] **WEB-06**: Replaying the same action transcript through headless and browser adapters produces identical engine event and terminal-state hashes.

### Verification and Release Gates

- [ ] **TEST-01**: Unit, scenario, invariant, integration, replay, import, coverage, and end-to-end tests run through one documented command and are required before commits claiming a completed phase.
- [ ] **TEST-02**: Deterministic-agent games with the same pinned manifest reproduce byte-identical canonical transcripts in fresh processes and supported environments.
- [ ] **TEST-03**: Hidden-information tests prove that changing opponent-private state cannot change the active player's observation or legal choices unless official rules reveal that information.
- [ ] **TEST-04**: No result is marked ranked unless all pinned-input, legality, verified-coverage, safe-execution, replay, experimental-design, and reporting gates pass.
- [ ] **TEST-05**: Every scheduled trial appears exactly once in reports as completed, drawn, invalid-unsupported, competitor failure, or infrastructure failure.

## v2 Requirements

### Breadth and Scale

- **FULL-01**: Every officially published card is implemented and verified, beyond the owned and benchmark transitive card pool.
- **FULL-02**: Additional official and community formats can be selected through versioned format profiles.
- **SCALE-01**: Distributed simulation can execute reproducible job manifests across multiple machines.
- **SCALE-02**: A persistent database can index large historical run catalogs when JSON/JSONL storage becomes measurably insufficient.

### Multiplayer and Clients

- **MULTI-01**: Two remote humans can play through server-authoritative multiplayer with reconnection and private-information guarantees.
- **MULTI-02**: Users can create accounts, maintain cloud collections/decks, and share replays.
- **CLIENT-01**: Native mobile clients can consume the shared engine protocol.

## Out of Scope

| Feature | Reason |
|---------|--------|
| Interactive human CLI | Humans will play in the browser; CLI commands are automation-only. |
| Authoritative LLM referee or ranked-state mutation | The cited rules adviser may explain local authority for unranked use, but model output never mutates or overrules the engine. |
| Rule changes made to improve measured balance | Official rules are evidence inputs, never tuning parameters. |
| Silent card approximations or no-ops | They can reverse deck rankings without visible evidence. |
| Perfect-play solver | Computationally premature and unnecessary for comparing practical competitors. |
| General English-to-card DSL | Broad shallow coverage is less trustworthy than typed verified handlers. |
| Trading, marketplace, and pricing | Not part of rules simulation or owned-card deck evaluation. |
| Self-training model pipeline | Model evaluation is required; training infrastructure is not. |
| Docker-specific workflow | Use native tooling or Podman-compatible containers if containers are needed. |

## Traceability

| Requirement | Phase | Status |
|-------------|-------|--------|
| DATA-01 | Phase 1 | In progress |
| DATA-02 | Phase 1 | In progress |
| DATA-03 | Phase 1 | In progress |
| DATA-04 | Phase 6 | Pending |
| DATA-05 | Phase 6 | Pending |
| ENG-01 | Phase 2 | Pending |
| ENG-02 | Phase 2 | Pending |
| ENG-03 | Phase 2 | Pending |
| ENG-04 | Phase 2 | Pending |
| ENG-05 | Phase 2 | Pending |
| ENG-06 | Phase 2 | Pending |
| RULE-01 | Phase 3 | Pending |
| RULE-02 | Phase 3 | Pending |
| RULE-03 | Phase 3 | Pending |
| RULE-04 | Phase 3 | Pending |
| RULE-05 | Phase 4 | Pending |
| RULE-06 | Phase 3 | Pending |
| CARD-01 | Phase 4 | Pending |
| CARD-02 | Phase 4 | Pending |
| CARD-03 | Phase 4 | Pending |
| CARD-04 | Phase 6 | Pending |
| CARD-05 | Phase 4 | Pending |
| SIM-01 | Phase 5 | Pending |
| SIM-02 | Phase 5 | Pending |
| SIM-03 | Phase 5 | Pending |
| SIM-04 | Phase 5 | Pending |
| SIM-05 | Phase 5 | Pending |
| SIM-06 | Phase 5 | Pending |
| SIM-07 | Phase 5 | Pending |
| DECK-01 | Phase 6 | Pending |
| DECK-02 | Phase 6 | Pending |
| DECK-03 | Phase 6 | Pending |
| DECK-04 | Phase 6 | Pending |
| MODEL-01 | Phase 7 | Pending |
| MODEL-02 | Phase 7 | Pending |
| MODEL-03 | Phase 7 | Pending |
| MODEL-04 | Phase 7 | Pending |
| MODEL-05 | Phase 7 | Pending |
| MODEL-06 | Phase 7 | Pending |
| OPT-01 | Phase 8 | Pending |
| OPT-02 | Phase 8 | Pending |
| OPT-03 | Phase 8 | Pending |
| OPT-04 | Phase 8 | Pending |
| WEB-01 | Phase 9 | Pending |
| WEB-02 | Phase 9 | Pending |
| WEB-03 | Phase 9 | Pending |
| WEB-04 | Phase 9 | Pending |
| WEB-05 | Phase 9 | Pending |
| WEB-06 | Phase 9 | Pending |
| TEST-01 | Phase 9 | Pending |
| TEST-02 | Phase 2 | Pending |
| TEST-03 | Phase 2 | Pending |
| TEST-04 | Phase 6 | Pending |
| TEST-05 | Phase 5 | Pending |

**Coverage:**
- v1 requirements: 54 total
- Mapped to phases: 54
- Unmapped: 0

---
*Requirements defined: 2026-08-20*
*Last updated: 2026-08-20 after roadmap creation*
