# Roadmap: Sorcery Simulator

## Overview

Sorcery Simulator advances through one increasingly trustworthy evidence path: pin the official authority, establish a deterministic engine contract, implement complete core rules and supported card interactions, prove those rules through replayable simulation, then evaluate owned and attributable competitive decks with deterministic and AI competitors. Collection-constrained optimization follows only after the benchmark is reliable, and browser human play is built last over the same verified engine rather than introducing a second rules implementation.

## Phases

**Phase Numbering:**

- Integer phases (1, 2, 3): Planned milestone work
- Decimal phases (2.1, 2.2): Urgent insertions (marked with INSERTED)

- [x] **Phase 1: Rules and Data Authority** - Pin the official sources, normalized card data, identities, provenance, and reuse boundary that every later result cites. (completed 2026-08-28)
- [ ] **Phase 2: Deterministic Engine Contract** - Establish the authoritative state, seeded randomness, private observations, legal actions, events, and replay-safe public API.
- [ ] **Phase 3: Complete Core Game Rules** - Make complete games obey official setup, spatial, resource, movement, combat, damage, and ending rules.
- [ ] **Phase 4: Storyline, Card Effects, and Coverage** - Execute supported card interactions through typed effects while proving eligibility and failing closed elsewhere.
- [ ] **Phase 5: Replayable Simulator and Baseline Gauntlets** - Run reproducible matches and seat-balanced experiments with deterministic competitors and auditable statistics.
- [ ] **Phase 6: Owned Collection and Competitive Deck Field** - Validate the owner's cards and supplied decks, freeze attributable common online decks, and verify the first ranked field.
- [ ] **Phase 7: AI Model Competitors** - Let provider-neutral models play through the same safe action contract with reproducible decisions and honest reliability/cost reporting.
- [ ] **Phase 8: Collection-Constrained Deck Optimization** - Search for evidence-backed improvements that are legal, owned, supported, and confirmed on untouched holdout games.
- [ ] **Phase 9: Browser Human Play and Replay** - Let a human play and inspect games in an accessible browser interface over the verified engine.

## Phase Details

### Phase 1: Rules and Data Authority

**Goal**: Developers can identify and reproduce the exact official rules, format, card data, and provenance governing every later game.
**Depends on**: Nothing (first phase)
**Requirements**: DATA-01, DATA-02, DATA-03
**Success Criteria** (what must be TRUE):

  1. A developer can validate an immutable authority bundle offline and see its pinned rulebook, format rules, Codex/FAQ/card updates, precedence, source URLs, retrieval dates, and SHA-256 hashes.
  2. A developer can build and validate the same versioned normalized card snapshot without live network access during a game or experiment.
  3. Every canonical rules, card, format, deck, collection, behavior, and experiment artifact resolves to a stable ID, schema version, provenance record, and content hash.

**Plans**: 15 plans

Plans:
**Wave 0**

- [x] 01-01-PLAN.md — Pin the one-package TypeScript toolchain and Phase 1 test contracts.

**Wave 1** *(blocked on Wave 0 completion)*

- [x] 01-02-PLAN.md — Implement canonical JSON, SHA-256 identity, and strict shared provenance schemas.

**Wave 2** *(blocked on Wave 1 completion)*

- [x] 01-03-PLAN.md — Build deterministic offline card normalization over pinned local bytes.
- [x] 01-04-PLAN.md — Validate authority paths, references, hashes, precedence, and storage policy offline.

**Wave 3** *(blocked on Wave 2 completion)*

- [x] 01-05-PLAN.md — Add atomic write-once local build and deterministic validation commands.

**Wave 4** *(blocked on Wave 3 completion)*

- [x] 01-06-PLAN.md — Document official precedence and enforce the private-local clean-room/no-leakage policy.

**Wave 5** *(blocked on Wave 4 completion)*

- [x] 01-07-PLAN.md — Verify the complete manually saved official source set, independent private backup, provenance, and scoped attestation.

**Wave 6** *(blocked on Wave 5 completion)*

- [x] 01-08-PLAN.md — Build and prove the immutable private official revision, deterministic rebuilds, offline operation, and zero Git/package leakage.

**Wave 7** *(blocked on Wave 6 completion)*

- [x] 01-09-PLAN.md — Preserve game-critical card fields and revision-independent logical card identities.
- [x] 01-11-PLAN.md — Make private collection complete, visible-date-bound, sanitized, and timeout-safe.
- [x] 01-12-PLAN.md — Strengthen private leakage detection and align the derivative-data policy.

**Wave 8** *(blocked on Wave 7 completion)*

- [x] 01-10-PLAN.md — Fail closed on precedence, calendar, locator, evidence, and derivation-graph defects.
- [x] 01-14-PLAN.md — Close out the committed reusable offline verifier without claiming the nonconforming historical roots passed.

**Wave 9** *(blocked on Wave 8 completion)*

- [x] 01-15-PLAN.md — Import the user-provided seven-file v3 manual inbox offline and prove both fresh roots complete, immutable, and non-disclosing.

**Wave 10** *(blocked on Wave 9 completion)*

- [x] 01-13-PLAN.md — Build and select the final immutable v3 authority revision from only the verified fresh roots, with complete release gates.

**Research**: Required — audit current rulebook/Codex/FAQ/card-update precedence, official data and image terms, format policies, and the GPL/no-license boundary for Contested Realms and spells.bar before any reuse.

### Phase 2: Deterministic Engine Contract

**Goal**: Every client and competitor can advance the same authoritative, deterministic game state only through current engine-issued legal actions.
**Depends on**: Phase 1
**Requirements**: ENG-01, ENG-02, ENG-03, ENG-04, ENG-05, ENG-06, TEST-02, TEST-03
**Success Criteria** (what must be TRUE):

  1. Creating the same game from the same pinned manifest in fresh processes produces JSON-compatible state, identical seeded randomness, and byte-identical canonical transcripts for deterministic agents.
  2. Each seat sees all public information but no opponent-private information, and changing unrevealed opponent state cannot alter that seat's observation or legal choices.
  3. Legal actions are stably ordered and state-version-bound; stale, forged, or illegal action IDs are rejected without changing the authoritative state hash.
  4. Every accepted action produces ordered semantic events, canonical hashes, and the information required to replay the action exactly.
  5. Deterministic competitors, model competitors, simulations, replays, and future human clients all consume the same observation, legal-action, and step contract.

**Plans**: TBD
**Research**: Not required — pure transitions, canonical JSON/SHA-256, injected PRNG state, and player-scoped observations have established implementation patterns.

### Phase 3: Complete Core Game Rules

**Goal**: Users and competitors can complete rules-correct Sorcery games using the universal game mechanics, with unsupported routes made explicit.
**Depends on**: Phase 2
**Requirements**: RULE-01, RULE-02, RULE-03, RULE-04, RULE-06
**Success Criteria** (what must be TRUE):

  1. Games enforce official setup, zones, opening hands, mulligan, first-player draw, turns and draw choice, decking, Death's Door, and game-end conditions.
  2. Players can legally place sites and Avatars and act across the 5×4 realm with correct voids, occupancy, adjacency, connections, surfaces, and regions.
  3. Mana, thresholds, targets, additional costs, and payment ordering allow valid plays and reject invalid ones without partial payment or mutation.
  4. Units summon, tap, move, attack, defend, intercept, strike, fight, fire projectiles, take or heal damage, and die according to official movement, regional, and airborne rules.
  5. Every supported core mechanic passes source-linked scenarios and invariants, while any route not yet implemented returns an explicit unsupported outcome.

**Plans**: TBD
**Research**: Required — audit official movement/regions/layers, combat/defense/intercept/projectiles, costs/targets, Death's Door, and timing per mechanic cluster.

### Phase 4: Storyline, Card Effects, and Coverage

**Goal**: Supported cards resolve complete official interactions through engine-owned effects, and every matchup has an honest, evidence-backed eligibility result.
**Depends on**: Phase 3
**Requirements**: RULE-05, CARD-01, CARD-02, CARD-03, CARD-05
**Success Criteria** (what must be TRUE):

  1. Players can resolve active/non-active Storyline ordering, pending choices, and immediate, triggered, delayed, replacement, and continuous effects required by supported cards.
  2. Supported card behavior runs through typed handlers and validated engine-owned effect primitives, never direct state mutation or runtime English interpretation.
  3. A coverage report shows a versioned, evidence-linked status for every relevant rule, keyword, card, token, ruling, and known interaction.
  4. Static preflight follows transitive dependencies, while a dynamically reached unsupported behavior preserves prior state and invalidates the game instead of partially resolving it.
  5. A capability report identifies the smallest verified rule/card increments that unlock the most currently ineligible owned and benchmark decks.

**Plans**: TBD
**Research**: Required — audit Storyline, activated/triggered/delayed/replacement/continuous effects, Golden Rule cases, tokens, and multi-card rulings family by family before claiming coverage.

### Phase 5: Replayable Simulator and Baseline Gauntlets

**Goal**: Developers can run auditable, reproducible games and fair gauntlets with deterministic baseline competitors before paying for model experiments.
**Depends on**: Phase 4
**Requirements**: SIM-01, SIM-02, SIM-03, SIM-04, SIM-05, SIM-06, SIM-07, TEST-05
**Success Criteria** (what must be TRUE):

  1. A non-interactive command completes a match between any two shared-contract competitors, including deterministic random and scripted baselines.
  2. Every game writes an immutable manifest, action transcript, semantic-event JSONL, terminal hash, coverage evidence, and classified outcome that replay verifies action by action.
  3. A gauntlet runs predeclared seat-swapped seed blocks, weights, caps, stopping, termination, and failure policies and reports W/D/L, uncertainty, opponents, seats, seat effect, game length, reliability, and eligibility.
  4. Every scheduled trial appears exactly once as completed, drawn, invalid-unsupported, competitor failure, or infrastructure failure; no failed or invalid trial silently disappears.
  5. If profiling justifies parallel workers, one-worker and multi-worker runs produce identical per-job transcripts and reports.

**Plans**: TBD
**Research**: Not required — local orchestration, JSONL, paired scheduling, replay, and optional worker pools are standard once the engine contracts are fixed.

### Phase 6: Owned Collection and Competitive Deck Field

**Goal**: The simulator can rank fully supported owned decks against a frozen, attributable field of representative common and competitive online decks.
**Depends on**: Phase 5
**Requirements**: DATA-04, DATA-05, CARD-04, DECK-01, DECK-02, DECK-03, DECK-04, TEST-04
**Success Criteria** (what must be TRUE):

  1. Importing the supplied collection yields exactly 223 copies across 149 canonical card names and reports every unknown, ambiguous, or mismatched entry.
  2. Supplied decklists remain byte-accountable through import and report format, copy-limit, inventory, card-data, and coverage failures, including an explicit correction or rejection for the illegal 62-card Bone Engine list.
  3. Common and competitive online deck revisions preserve attributable source, author/pilot, event result when published, date, ruleset, raw input, and hashes.
  4. Users can select a frozen, deduplicated, archetype-labeled benchmark with documented strata and weights, and reports distinguish equal-opponent evidence from any evidence-weighted field result.
  5. A matchup is ranked only when both decks are legal and every transitively used card and mechanic is verified; all other results state which eligibility gate failed.

**Plans**: TBD
**Research**: Required — validate Curiosa or other source endpoints and terms, define and justify “common” deck strata/weights, and use “curated benchmark” unless population evidence supports a metagame claim.

### Phase 7: AI Model Competitors

**Goal**: Users can compare AI models through a versioned legal-action player skill and use a cited rules-adviser skill without granting either rules authority or hiding reliability, latency, and cost tradeoffs.
**Depends on**: Phase 6
**Requirements**: MODEL-01, MODEL-02, MODEL-03, MODEL-04, MODEL-05, MODEL-06
**Success Criteria** (what must be TRUE):

  1. A versioned player skill gives every model only its redacted observation and the same ordered legal-action list as other competitors and accepts only one action ID.
  2. Every model decision records model/configuration, prompt version, response or hash, selected or fallback action, classification, latency, tokens, cost, and provider failure details.
  3. Malformed, illegal, timed-out, and failed decisions follow the predeclared deterministic fallback or forfeit policy and remain visible in reliability reports.
  4. Fixed model experiments use identical decks, opponents, seats, seeds, observations, action ordering, prompts, sampling, and budgets, and reports separate deck, model, and model-by-deck effects.
  5. Recorded model action IDs replay to the same engine outcome without contacting the provider.
  6. A versioned rules-adviser skill retrieves and cites only the selected local authority, cannot mutate state or invent actions, and every game that consults it remains explicitly unranked or invalid-unsupported.

**Plans**: TBD
**Research**: Required — verify provider model identifiers and schemas, nondeterminism and retry semantics, prompt/tool formats, fair token/time budgets, and cost-aware sample sizes during planning.

### Phase 8: Collection-Constrained Deck Optimization

**Goal**: Users can test and receive trustworthy improvements to a starting deck using only cards they own and evidence the simulator can rank.
**Depends on**: Phase 7
**Requirements**: OPT-01, OPT-02, OPT-03, OPT-04
**Success Criteria** (what must be TRUE):

  1. Every generated candidate is a recorded local diff that remains format-legal, inventory-valid, and fully implementation-verified.
  2. Candidate generation uses only the frozen development field and seeds, while final comparison uses one untouched, versioned holdout field and seed set.
  3. A recommendation compares the unchanged starting deck with the finalist under the same deterministic pilot and shows exact swaps, inventory proof, matchup deltas, uncertainty, worst matchup, and representative replays.
  4. The optimizer can honestly report no statistically distinguishable owned-card improvement and never tunes further search against a consumed holdout.

**Plans**: TBD
**Research**: Required planning review — predeclare candidate budget, blocked statistical design, multiple-candidate selection, objective tradeoffs, and holdout retirement/refresh rules.

### Phase 9: Browser Human Play and Replay

**Goal**: A human can play and inspect a complete local Sorcery game in an accessible browser while the verified engine remains the sole rules authority.
**Depends on**: Phase 8
**Requirements**: WEB-01, WEB-02, WEB-03, WEB-04, WEB-05, WEB-06, TEST-01
**Success Criteria** (what must be TRUE):

  1. A human can start and complete a local browser game against an automated or model competitor without interactive CLI play.
  2. The browser presents the realm, regions, zones, cards, public state, current Storyline, and forced choices from engine observations and events using only original/project-owned presentation art or user-supplied private local images.
  3. Every target, path, defense, reaction, and choice offered to the human is engine-issued, and browser code never computes rules or card effects independently.
  4. A human can load a recorded replay and inspect its action, event, state, and rank-eligibility evidence step by step.
  5. Play has keyboard equivalents, visible focus, non-color cues, and accessible action/rejection/event text; the single project verification command proves headless and browser adapters produce identical hashes for the same transcript.

**Plans**: TBD
**Research**: Not required as a separate phase — use established React/Vite and accessible interaction patterns, re-pin versions during planning, and validate spatial and forced-choice usability with humans.
**UI hint**: yes

## Progress

**Execution Order:**
Phases execute in numeric order: 1 → 2 → 3 → 4 → 5 → 6 → 7 → 8 → 9

| Phase | Plans Complete | Status | Completed |
|-------|----------------|--------|-----------|
| 1. Rules and Data Authority | 15/15 | Complete   | 2026-08-28 |
| 2. Deterministic Engine Contract | 0/TBD | Not started | - |
| 3. Complete Core Game Rules | 0/TBD | Not started | - |
| 4. Storyline, Card Effects, and Coverage | 0/TBD | Not started | - |
| 5. Replayable Simulator and Baseline Gauntlets | 0/TBD | Not started | - |
| 6. Owned Collection and Competitive Deck Field | 0/TBD | Not started | - |
| 7. AI Model Competitors | 0/TBD | Not started | - |
| 8. Collection-Constrained Deck Optimization | 0/TBD | Not started | - |
| 9. Browser Human Play and Replay | 0/TBD | Not started | - |
