# Project Research Summary

**Project:** Sorcery Simulator  
**Domain:** Deterministic trading-card-game rules simulator, AI/model testbed, collection-constrained deck optimizer, and later browser client  
**Researched:** 2026-08-20  
**Confidence:** HIGH for the core stack, trust boundaries, build order, and failure gates; MEDIUM for complete card/ruling breadth, benchmark representativeness, model budgets, and future GUI versions

## Executive Summary

Sorcery Simulator should be built as a new authoritative TypeScript game engine, not as a repair of the prototype or a fork of either reference tool. Experts build this class of system around a pure, synchronous transition boundary: pinned inputs create state; the engine exposes a player-safe observation and stable legal-action IDs; a human, deterministic agent, or model selects one ID; and the engine alone returns the next immutable state plus ordered semantic events. A result is useful for ranking only when the exact rules, cards, decks, competitors, experiment design, and randomness are pinned, every exercised capability is verified, and replay reproduces the deterministic transcript hash. ([PROJECT.md](../PROJECT.md), [ARCHITECTURE.md](./ARCHITECTURE.md), [PITFALLS.md](./PITFALLS.md))

Start with one small `pnpm` package on Node 24 and TypeScript 6, using `node:test`, Node standard-library I/O/crypto, `pure-rand`, Zod, and JSON/JSONL artifacts. Keep environment-neutral module boundaries so the engine can later run unchanged in a React/Vite browser app. Do not add a monorepo, database, server, provider SDK, worker pool, or GUI until an observed need justifies it. Implement the complete core rules first, then admit small, complete vertical slices of cards from the owner's decks and a frozen current benchmark field. Unsupported behavior must fail closed; broad but approximate card support would make the deck optimizer actively misleading. ([STACK.md](./STACK.md), [FEATURES.md](./FEATURES.md), [ARCHITECTURE.md](./ARCHITECTURE.md))

The largest risks are rules/card drift, hidden nondeterminism, incomplete mechanics presented as evidence, hidden-information leakage, biased deck/model experiments, adaptive optimizer overfitting, and license/IP contamination. Mitigate them with immutable source snapshots and hashes, one injected/versioned PRNG, canonical transcripts and golden replay tests, player-scoped observations, coverage preflight plus runtime invalidation, seat-swapped blocked experiments with uncertainty reporting, a locked holdout, and clean-room implementation. Contested Realms is GPL-3.0 and spells.bar had no explicit license at the audited revision: borrow concepts and test ideas only unless the project consciously accepts compatible license terms or receives permission. ([PITFALLS.md](./PITFALLS.md), [STACK.md](./STACK.md))

## Key Findings

### Recommended Stack

Use the smallest stack that supports deterministic simulation. Pin exact versions in the runtime file, `packageManager`, and lockfile; reverify versions only when a deferred phase starts. The detailed rationale and audited alternatives are in [STACK.md](./STACK.md).

**Core technologies:**

- **Node.js 24.19.0 LTS:** runtime, native erasable-TypeScript execution, tests, hashing, files, and later CPU workers.
- **TypeScript 6.0.3:** strict type checking with the complete programmatic API; run `tsc --noEmit` because Node strips types but does not type-check.
- **pnpm 11.22.0:** package manager and committed lockfile; begin with one package, not speculative workspace orchestration.
- **`node:test` and `node:assert/strict`:** all headless unit, scenario, replay, invariant, and integration tests; no Jest/Vitest dependency initially.
- **`pure-rand` 8.4.2:** the single injected PRNG, with `xoroshiro128plus` and its version/algorithm recorded in manifests.
- **Zod 4.4.3:** validate only untrusted persisted/imported/model boundaries, not typed hot-loop state.
- **Node `crypto`, `fs`, and `readline`:** SHA-256 identities and versioned JSON/append-only JSONL artifacts.
- **Node `worker_threads`:** add a reusable pool only after single-thread replay is proven and profiling shows CPU pressure.
- **React 19 + Vite 8 directionally:** add and re-pin during the final browser phase; the GUI must consume the same browser-safe engine contract.

**Conflict resolution:** [ARCHITECTURE.md](./ARCHITECTURE.md) also proposes a custom `xoshiro128**` implementation and an initial package graph. Prefer the already-reviewed `pure-rand` dependency from [STACK.md](./STACK.md) to avoid maintaining PRNG/distribution code, and represent its serializable generator state in `GameState`. Begin with `src/engine`, `src/data`, `src/agents`, `src/simulator`, `src/optimizer`, and `src/commands` boundaries inside one package; split packages only when the browser app or build ownership creates a real enforced boundary.

**Explicit non-choices:** no Python, human-play CLI, Next.js, Socket.IO, database/ORM/cache server, LLM referee, general card DSL, ML/GPU framework, or container requirement. If containers later become necessary, use Podman-compatible OCI files and commands. ([PROJECT.md](../PROJECT.md), [STACK.md](./STACK.md))

### Expected Features

The minimum credible product is an auditable game, not merely a loop that declares a winner. Detailed acceptance behavior is in [FEATURES.md](./FEATURES.md).

**Must have before a result can be ranked:**

- Immutable rules, format, FAQ/Codex, card, deck, collection, competitor, PRNG, and experiment artifacts with provenance and hashes.
- A headless authoritative engine with complete core rules and no mutation path except a current engine-enumerated action ID.
- Player-scoped observations, staged legal choices, stable action IDs/order, typed semantic events, canonical state hashes, and replay.
- A coverage registry for rules, keywords, cards, rulings, referenced tokens, and known interactions, with `verified`, `implemented-unverified`, `unsupported`, and `not-applicable` states.
- Static deck/mechanic preflight plus runtime fail-closed invalidation; no silent no-op, approximation, or model ruling.
- Exact import/validation of the owner's supplied collection (expected: 223 cards, 149 canonical names) and decklists, preserving raw imports and mapping diagnostics.
- Frozen, attributable online-deck revisions and a documented benchmark release rather than live URLs or an unsupported claim about metagame share.
- Deterministic scripted agents, seat-balanced batch scheduling, predeclared seeds/stopping rules, machine-readable reports, uncertainty, seat effects, and visible invalid/failure counts.
- Model adapters that receive only the same observation/legal-action contract, return an action ID, and record prompt/model/configuration, raw response or hash, reliability, latency, tokens, cost, and outcome.
- Collection-constrained deck search that enforces legality/support/inventory during candidate generation and validates finalists once on a locked holdout.
- A browser human-play and replay client after the engine contracts are verified; no interactive human CLI and no duplicated UI rules.

**Should have because they distinguish this product:**

- A rank-eligibility proof showing why every matchup is or is not trustworthy.
- A capability-unlock report prioritizing the smallest mechanic/card work that unlocks the most valuable owned and benchmark decks.
- Frozen competitive-field releases with provenance, deduplication, archetype labels, and explicit weights.
- Deck × pilot × model analysis that separates deck quality from competitor quality.
- Auditable model decisions and representative replay links for surprising wins, losses, and fallbacks.
- Explainable owned-card recommendations: exact swaps, inventory proof, matchup deltas, intervals, worst matchup, and an allowed “no distinguishable improvement” conclusion.
- A source-linked conformance corpus accumulated from official rules, Codex, FAQ, card updates, and regression interactions.

**Defer until the core evidence path works:**

- Browser GUI implementation, though its observation/action/event boundary must be preserved from the start.
- Parallel workers until replay correctness is proven and profiling supports them.
- Provider SDKs until global `fetch` plus `AbortController` lacks a required feature.
- Database, distributed execution, accounts, public multiplayer/matchmaking, native apps, marketplace/prices, Elo-only ladders, self-training, perfect-play solving, and exhaustive deck search.

### Architecture Approach

Use a functional core and imperative shell. `createGame`, `observeFor`, `legalActions`, `step`, and `coverageReport` are the public authority; state is JSON-compatible and serializable, action IDs are deterministic and state-version-bound, and `step` regenerates the current legal set before applying a selection. The engine owns RNG state, entity/event counters, turn/Storyline/pending-choice state, all rule queries, card behavior handlers, and primitive effects. Filesystem, HTTP, model calls, scheduling, analytics, and rendering stay outside it. Replay reconstructs the initial state from a pinned manifest and reapplies recorded actions while checking event/state hashes; this is not full event sourcing. ([ARCHITECTURE.md](./ARCHITECTURE.md))

**Initial module boundaries:**

1. **`engine`** — plain state, grid/region geometry, turns and Storyline, observations, legal actions, validated transitions, RNG, effects, card behavior registry, and coverage checks.
2. **`data`** — immutable rules/card metadata, normalized collection/decks, provenance, schema versions, and content hashes; never live-fetched during a run.
3. **`agents`** — deterministic policies/search plus provider-isolated model adapters; consumes only player observations and legal actions.
4. **`simulator`** — match loop, manifests, action/event transcripts, replay verification, paired schedules, statistics, and later worker orchestration.
5. **`optimizer`** — collection/format/coverage-constrained local mutations and train/holdout evaluation; never reimplements rules.
6. **`commands`** — non-interactive import, validation, coverage, replay, and batch entry points.
7. **`web` (later)** — board, legal affordances, accessibility, human play, and replay presentation over the same engine; it never mutates game state directly.

**Required engine contract:**

```text
observeFor(state, seat) -> hidden-information-safe observation
legalActions(state, seat) -> stable ordered legal choices
step(state, { stateVersion, actionId }) -> ok | rejected | unsupported
coverageReport(matchSpec) -> eligibility and dependency evidence
```

### Ranked Eligibility Gate

This gate is the product's trust boundary, synthesized from [FEATURES.md](./FEATURES.md), [ARCHITECTURE.md](./ARCHITECTURE.md), and [PITFALLS.md](./PITFALLS.md).

1. **Pinned inputs:** manifest includes schema/engine/lockfile, rules/format/card/behavior, decks/collection, agents/prompts, PRNG, root seed/job schedule, termination policy, and experiment hashes.
2. **Legal and verified matchup:** both decks satisfy the pinned format; all transitive cards, tokens, rules, keywords, and explicit rulings are `verified` before scheduling.
3. **Safe execution:** clients see only their observation and legal set; rejected/stale/forged actions leave state unchanged; all randomness uses the game PRNG.
4. **Runtime coverage:** every exercised capability is registered. A dynamic unsupported path leaves the prior authoritative state intact and marks the whole game `invalid_unsupported`.
5. **Replay proof:** deterministic competitors reproduce byte-identical canonical event JSONL and the final-state hash; worker count must not affect output once workers exist.
6. **Honest experiment:** seat alternation, seed blocks, field weights, failure policy, game cap, precision target, and stopping rule are declared before execution.
7. **Honest reporting:** invalid/unsupported/infrastructure/model failures remain visible. Unsupported games are excluded from performance denominators, not deleted or converted into losses.

Anything failing a gate is exploratory output and cannot drive a deck recommendation.

### Deck and Model Evaluation Design

**Benchmark field:** freeze attributable revisions in separate strata: current tournament top/field lists, source-measured popular lists, official precons, mechanics-coverage adversaries, and owned legal decks. Preserve source, author/pilot, event/placing/record when published, date, ruleset, import time, raw hash, normalized hash, and human-reviewed archetype. Deduplicate exact/near duplicates. Report an equal-opponent field first and any evidence-weighted field separately; call the result a curated benchmark unless a documented sampling frame proves metagame share. ([FEATURES.md](./FEATURES.md), [PITFALLS.md](./PITFALLS.md))

**Deck comparisons:** candidate and baseline use the same deterministic pilot class/version, opponent field, seat-swapped seed blocks, limits, and failure policy. Report raw W/D/L/draw/invalid counts, per-opponent and per-seat cells, equal-opponent macro score, worst matchup, effect-size intervals, and `conclusive`/`inconclusive` against a predeclared precision target and game cap.

**Model comparisons:** cross every model with the same benchmark decks, deterministic calibration opponents, seat/seed blocks, observation schema, legal-action ordering, prompt/template, sampling configuration, and budget. Report model, deck, and model×deck effects separately, alongside malformed/illegal choice, timeout, provider error, fallback, latency, token, and cost metrics. Do not advertise live model inference as deterministic; preserve inputs/responses and replay the recorded action IDs. A malformed response, illegal ID, or model timeout uses one predeclared deterministic fallback/forfeit and counts against reliability. Only a harness/infrastructure failure may invalidate and retry the identical whole job under a fixed recorded retry policy.

**Optimization:** search only legal local mutations within the intersection of owned, format-legal, and verified cards. Use a frozen development field/seeds for generation, log every tried candidate, and allow only a final short list one evaluation on untouched holdout decks/seeds. If a holdout result changes the search, retire that holdout and create a new one. Default objective: equal-opponent macro score, then worst-matchup score; recommend a change only when holdout evidence supports it.

### Critical Pitfalls

The full warning catalog is in [PITFALLS.md](./PITFALLS.md).

1. **Rules/card drift** — create immutable, content-addressed rulesets and card snapshots; upgrades create new revisions and migration/impact reports rather than mutating history.
2. **Incomplete mechanics masquerading as results** — require verified transitive coverage, fail closed at runtime, and show capability/invalid counts beside every score.
3. **Nondeterminism hidden by a seed field** — inject one versioned PRNG, derive per-game seeds from stable job identity, ban time/UUID/`Math.random()` from canonical paths, sort output by job ID, and verify golden replays across worker counts.
4. **Hidden-information and action-boundary leaks** — expose player-specific observations and staged legal choices only; prove information noninterference by varying opponent-private state without changing the active player's projection.
5. **Misleading deck/model statistics** — block by seat/seed/opponent, separate deck and model effects, retain every draw/invalid/failure, predeclare stopping, and report uncertainty rather than a single leaderboard number.
6. **Adaptive optimizer overfitting** — isolate search from one-time holdout evaluation and log every candidate, seed, field revision, and decision.
7. **License/IP contamination** — clean-room implement from official rules and independently authored tests; keep a provenance ledger; do not copy GPL Contested Realms code without accepting GPL implications, or unlicensed spells.bar code without permission; confirm official API/text/image redistribution terms before bundling.

## Implications for Roadmap

The roadmap should deliver one increasingly trustworthy vertical evidence path. Every phase includes the smallest executable tests needed to prove its own contract; no later phase may work around a failed earlier gate.

### Phase 1: Rules, Data Authority, and Reuse Boundary

**Rationale:** Every implementation and result depends on knowing which official rule/card behavior and which external material are authoritative. This must precede copied code, card handlers, or benchmark claims.  
**Delivers:** immutable `rulesetId` and format profile; raw/source/hash policy for rulebook, Codex/FAQ, card updates/API, and assets; official precedence/ruling policy; normalized ID scheme; provenance ledger; explicit GPL/no-license reuse decision.  
**Addresses:** pinned sources, canonical identities, versioned format/card behavior, license safety.  
**Avoids:** rules drift, errata drift, reference tools as rules oracles, IP contamination.  
**Exit gate:** one versioned authority bundle can be hashed and validated offline, and no external code is copied without an approved compatible boundary.

### Phase 2: Deterministic Kernel and Public Contracts

**Rationale:** Determinism, state ownership, hidden-information boundaries, and action identity are expensive to retrofit and are prerequisites for every client.  
**Delivers:** minimal Node/TS package; strict schemas; pure `createGame`/`observeFor`/`legalActions`/`step`; serializable state; deterministic entity/action/event IDs; pinned `pure-rand` state; canonical JSON/SHA-256; semantic events; action transcript and golden replay.  
**Addresses:** authoritative engine, legal-action interface, deterministic randomness, event logs/replay.  
**Avoids:** state patches, stale/forged actions, hidden leakage, seed theater, unverifiable logs.  
**Exit gate:** stale/unknown actions preserve the state hash; information-noninterference passes; the same manifest in fresh processes produces byte-identical transcripts.

### Phase 3: Complete Core Rules Vertical Slice

**Rationale:** Deck/card work is not trustworthy until the universal game skeleton is correct. Implement rules in complete scenario slices rather than shallow card breadth.  
**Delivers:** setup/mulligan; ordered zones; 5×4 realm, layers, regions, and adjacency; turn/priority flow; site play; mana/threshold/payment; units; movement; attack/defend/intercept; damage/death/Death's Door; projectiles; end conditions; small synthetic fixtures.  
**Addresses:** complete core rules and spatial legality.  
**Avoids:** tuning official rules to prototype outcomes, agent-side legality, hidden approximations.  
**Exit gate:** source-cited scenario and invariant tests pass for each supported core mechanic; any unimplemented route is explicitly unsupported.

### Phase 4: Storyline, Card Effects, and Coverage Eligibility

**Rationale:** Real decks require triggers, choices, modifiers, and interaction closures; a card-name count cannot prove support.  
**Delivers:** explicit Storyline/pending-choice state machine; immediate, triggered/delayed, continuous, and replacement effect mechanisms; shared validated primitives; typed card behavior registry; four-state coverage manifest; static closure preflight; runtime invalidation; capability-unlock report.  
**Addresses:** versioned card/ruling implementations and rank-eligibility proof.  
**Avoids:** silent failures, English-text interpretation, runtime model rulings, action-space explosion.  
**Exit gate:** unknown cards/keywords/tokens and dynamic unsupported interactions each fail closed; each `verified` capability links to passing official-source conformance tests.

### Phase 5: Replayable Simulator, Baseline Agents, and Statistics

**Rationale:** Cheap deterministic competitors validate the complete match/evidence path before external-model variability and cost are introduced.  
**Delivers:** common agent contract; random and simple deterministic policies; match loop; manifests and JSONL transcripts; replay verifier; paired seat/seed scheduler; termination/failure policy; W/D/L, intervals, seat effect, game-length, invalidation, and coverage reports. Add a worker pool only after profiling, with 1-vs-N transcript invariance.  
**Addresses:** batch runner, pilot artifacts, fair gauntlets, statistical/diagnostic output.  
**Avoids:** premature performance work, discarded trials, aggregate-only scores, schedule-dependent RNG.  
**Exit gate:** a small fully supported fixture field runs end to end, replay verifies every state hash, and paired reports account for every scheduled game.

### Phase 6: Owned Collection and Frozen Competitive Field

**Rationale:** The simulator must evaluate buildable owned decks against relevant opposition, but only after legality and coverage can be proved.  
**Delivers:** raw-plus-normalized import pipeline; exact 223-card/149-name collection check; supplied-deck shortage/legality/support diagnostics; source adapters for attributable online decks; immutable deck revisions; deduplicated, stratified benchmark release; coverage-driven card-family expansion until the first owned and common decks are fully eligible.  
**Addresses:** collection/deck validation and common online competitor decks.  
**Avoids:** fuzzy/silently repaired imports, live mutable decks, top-8-only bias, unsupported “meta share” claims.  
**Exit gate:** every benchmark revision is provenance-complete and ruleset-legal; every scheduled ranked matchup passes transitive coverage preflight.

### Phase 7: AI Model Competitors

**Rationale:** Models should enter only after the same legal interface and fair deterministic calibration field work without them.  
**Delivers:** provider-neutral model contract; first `fetch` adapter and fake-adapter tests; redacted observation/prompt versioning; action-ID parsing; predeclared failure/fallback policy; raw-response audit data; latency/token/cost/reliability sidecars; fixed model × deck × opponent × seat experiment matrix; recorded-decision replay.  
**Addresses:** model play, auditable decisions, and honest cross-model comparison.  
**Avoids:** LLM authority, hidden retries, model/deck confounding, false determinism claims, opaque AI scores.  
**Exit gate:** models and deterministic agents receive identical player-safe contracts; every decision/failure is classified; reports separate strategic outcomes from reliability, latency, and cost.

### Phase 8: Collection-Constrained Deck Optimizer

**Rationale:** Optimization is only meaningful once the field, pilot, simulator, statistics, and collection constraints are trustworthy.  
**Delivers:** legal local mutations; exact inventory consumption; candidate hashes/diffs; frozen training evaluation; one-time locked holdout; comparison against the unchanged starting deck; evidence-rich recommendations and representative replay links.  
**Addresses:** building the strongest supported deck from the owner's cards.  
**Avoids:** exhaustive premature search, single-matchup optimization, adaptive holdout reuse, claims beyond statistical precision.  
**Exit gate:** every candidate is legal, owned, and verified throughout search; final output includes train/holdout matrices and can conclude “no proven improvement.”

### Phase 9: Browser Human Play and Replay

**Rationale:** The GUI should reveal and exercise the verified engine, not drive a second rules implementation.  
**Delivers:** re-pinned React/Vite app; local human-vs-agent setup; 5×4 spatial board and zones; engine-issued legal target/path/reaction affordances; Storyline/forced-choice UX; private-information-safe views; replay inspector; keyboard equivalents, focus, non-color cues, and accessible event/rejection text.  
**Addresses:** browser human play over the same engine.  
**Avoids:** GUI/engine drift, authoritative drag mutation, inaccessible gesture-only actions, premature multiplayer infrastructure.  
**Exit gate:** the same scripted action transcript through headless and browser adapters yields identical engine event hashes; no browser module computes legality or card effects.

### Phase Ordering Rationale

- Authority and licensing precede implementation because incorrect source precedence or contaminated reuse can invalidate all later work.
- Deterministic state/action/event contracts precede rule breadth because they are the shared dependency of simulator, agents, optimizer, replay, and GUI.
- Core rules precede card breadth; Storyline/effect/coverage mechanisms then unlock complete real-deck slices without approximation.
- The deterministic simulator and statistics precede online/model evaluation so correctness and experimental fairness can be tested cheaply.
- Collection and benchmark imports can be researched earlier, but ranked use waits for legality and coverage gates.
- Model evaluation precedes optimization only after a deterministic calibration baseline exists; optimizer evaluation uses stable pilots and an untouched holdout.
- The GUI is last because it consumes stable contracts and must not become an alternate rules engine.

### Research Flags

Phases needing deeper research during planning:

- **Phase 1:** required authority audit for current rulebook/Codex/FAQ/card-update precedence, official API/card-text/image terms, format/event policies, and the GPL/no-license reuse decision.
- **Phase 3:** required Sorcery-specific research per mechanic cluster—movement/regions/layers, combat/defense/intercept/projectiles, costs/targets, Death's Door, and timing.
- **Phase 4:** required per-family audit of Storyline, triggered/activated/replacement/continuous effects, Golden Rule cases, tokens, and multi-card rulings before claiming coverage.
- **Phase 6:** required endpoint/terms research for Curiosa or other deck sources and a defensible definition/weighting of “common”; use “curated benchmark” if population data is unavailable.
- **Phase 7:** adapter-specific research for provider model identifiers, prompt/tool schema, nondeterminism, retry semantics, fair token/time budgets, and cost-aware sample size.
- **Phase 8:** planning review for candidate budget, blocked statistical design, multiple-candidate selection, objective tradeoffs, and holdout retirement/refresh rules.

Phases with established patterns that can skip a separate research phase:

- **Phase 2:** pure reducers, canonical JSON/SHA-256, injected PRNG state, and player-scoped observations are well specified; resolve details in the plan and tests.
- **Phase 5:** local match orchestration, JSONL, paired scheduling, replay, and worker pools are standard once contracts and statistical gates are fixed.
- **Phase 9:** React/Vite and accessible interaction patterns are standard after the engine API stabilizes; reverify package versions during planning.

## Confidence Assessment

| Area | Confidence | Notes |
|------|------------|-------|
| Stack | HIGH | Runtime/compiler/package/test/worker choices were checked against primary official sources; GUI versions are intentionally provisional. |
| Features | HIGH for trust gates; MEDIUM for prioritization details | Official rules/format sources support versioning and legality needs; benchmark/model/optimizer product recommendations require empirical validation. |
| Architecture | HIGH for engine/action/replay/isolation boundaries; MEDIUM for full card breadth | The boundaries directly prevent observed prototype/reference failures; complete card/ruling families still need phase-specific audit. |
| Pitfalls | HIGH for rules, determinism, information, replay, and experiment risks; MEDIUM for ecosystem representativeness/IP terms | Core risks have primary evidence; online-deck population coverage and some distribution rights remain unresolved. |

**Overall confidence:** HIGH for the recommended build order and ranked-evidence contract; MEDIUM for how much card coverage, model sampling, and GUI scope each later phase will require.

### Gaps to Address

- **Rules authority:** hash the actual current official artifacts and define precedence among printed text, effective/API text, rulebook, FAQ, Codex, card updates, and judge/project rulings.
- **Official data/assets terms:** confirm permission and attribution requirements for storing or redistributing API data, card text, and images before distribution.
- **Card-mechanic inventory:** mechanically classify the pinned card snapshot to estimate behavior families and choose the smallest fully supported owned/benchmark slice.
- **Reference reuse:** make an explicit project license decision before any Contested Realms copying; seek a compatible spells.bar license if code reuse is desired.
- **Online deck source contract:** validate actual endpoints, change behavior, rate/usage terms, and raw snapshot format; preserve adapters as replaceable ingestion boundaries.
- **“Common” population:** if no documented popularity sampling frame exists, publish a curated benchmark with strata and weights rather than a metagame-share claim.
- **Model budgets:** measure per-decision latency/cost and action-space size before fixing sample counts; exploratory/inconclusive results are acceptable.
- **Statistical thresholds:** predeclare score convention, interval method, precision target, game cap, cluster/pair handling, and multiple-candidate confirmation before publishing rankings.
- **GUI usability:** validate spatial, reaction, forced-choice, card-detail, replay, and accessibility flows with humans when that phase begins; do not change engine rules to accommodate UI shortcuts.

## Sources

### Detailed Project Research

- [STACK.md](./STACK.md) — exact stack, deterministic runtime, reference-source audit, reuse boundary, testing, workers, and storage.
- [FEATURES.md](./FEATURES.md) — table stakes, acceptance behavior, coverage product, benchmark/model/optimizer design, GUI needs, anti-features, and MVP dependencies.
- [ARCHITECTURE.md](./ARCHITECTURE.md) — authoritative engine contract, state/effect/event design, replay, observation isolation, workers, optimizer and GUI boundaries, and build order.
- [PITFALLS.md](./PITFALLS.md) — release gate, failure modes, prevention strategies, phase exit gates, research flags, and unresolved gaps.
- [PROJECT.md](../PROJECT.md) — active user requirements, constraints, product scope, and explicit non-goals.

### Primary (HIGH confidence)

- [Sorcery: Contested Realm — How to Play](https://sorcerytcg.com/how-to-play) — official rules and FAQ authority entry point.
- [December 2025 Rulebook Update](https://sorcerytcg.com/news/sorcery-contested-realm-december-2025-rulebook-update) — current researched rules edition and constructed Spellbook change.
- [Sorcery Constructed Format](https://sorcerytcg.com/constructed) — deck construction and copy limits.
- [Official Card API](https://api.sorcerytcg.com/api/cards) and [Card Updates 2025](https://sorcerytcg.com/news/sorcery-contested-realm-card-updates-2025) — card metadata and evidence of mutable effective text.
- [Curiosa Codex](https://curiosa.io/codex) and [Curiosa FAQ](https://curiosa.io/faqs) — deeper rule/card interaction sources to snapshot and cite.
- [RFC 8785](https://www.rfc-editor.org/rfc/rfc8785.html) — canonical JSON for deterministic bytes and hashes.
- [Node.js TypeScript](https://nodejs.org/api/typescript.html), [test runner](https://nodejs.org/download/release/latest-v24.x/docs/api/test.html), and [worker threads](https://nodejs.org/api/worker_threads.html) — runtime, testing, and concurrency behavior.
- [NIST randomized block designs](https://www.itl.nist.gov/div898/handbook/pri/section3/pri332.htm) and [proportion intervals](https://www.itl.nist.gov/div898/handbook/prc/section2/prc241.htm) — experimental blocking and uncertainty.
- [Dwork et al., Holdout Reuse](https://papers.nips.cc/paper_files/paper/2015/hash/bad5f33780c42f2588878a9d07405083-Abstract.html) — adaptive evaluation overfitting.
- [Contested Realms](https://github.com/realms-cards/contested-realms) and its [GPL-3.0 license](https://github.com/realms-cards/contested-realms/blob/main/LICENSE) — behavior reference and binding reuse constraint.

### Secondary (MEDIUM confidence)

- [spells.bar source repository](https://github.com/JollyGrin/sorcery-tcg-playtest) — board/interaction reference; no explicit repository license was found at the audited revision, so behavior-only study is the safe default.
- Official event reports and linked Curiosa deck revisions listed in [FEATURES.md](./FEATURES.md) — strong attributable benchmark seeds, but not by themselves a representative metagame sampling frame.

---
*Research completed: 2026-08-20*  
*Ready for roadmap: yes*
