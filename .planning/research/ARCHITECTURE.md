# Architecture Patterns

**Project:** Sorcery Simulator
**Domain:** Deterministic Sorcery: Contested Realm rules simulator, AI/model testbed, deck optimizer, and later browser GUI
**Researched:** 2026-08-20
**Confidence:** HIGH for the core engine, replay, isolation, and worker boundaries; MEDIUM for the eventual card-effect breadth because the complete current card/ruling corpus still needs a rules-specific implementation audit

## Executive Recommendation

Build one environment-neutral TypeScript rules package around a deterministic state transition:

```text
legalActions(state, player) -> LegalAction[]
step(state, { stateVersion, actionId }) -> StepResult
```

The engine owns state, rule validation, effect resolution, deterministic randomness, entity IDs, and semantic events. A human UI, a scripted competitor, and a model adapter may only inspect a player-safe observation and choose an ID from the engine's current legal-action list. They never submit state patches or arbitrary mutations.

Use plain serializable state plus an append-only action/event transcript. Do **not** build full event sourcing: replay should recreate the initial state from a pinned manifest and reapply the recorded action selections, then compare canonical event/state hashes. This supplies trustworthy replay without making every state query reconstruct a game from its entire history.

Start as a local monorepo with filesystem artifacts. The engine needs no database, server, message bus, container, or React dependency. Add the browser client only after a supported rules slice, deterministic replay, and automated gauntlets are proven.

## Recommended Architecture

```text
Pinned rules edition + pinned card snapshot + behavior registry
                              |
                              v
                 +---------------------------+
                 | @sorcery/engine           |
                 |                           |
                 | createGame()              |
                 | observeFor(player)        |
                 | legalActions()            |
                 | step(actionId)             |
                 | coverageReport()          |
                 +-------------+-------------+
                               |
                    state + semantic events
                               |
             +-----------------+------------------+
             |                 |                  |
             v                 v                  v
      scripted agents    model adapters      browser session
             |           (observation only)  (presentation only)
             +-----------------+------------------+
                               |
                               v
                       match/run orchestrator
                  manifests, transcripts, summaries
                               |
               +---------------+----------------+
               |                                |
               v                                v
      deterministic worker pool          optimizer/evaluator
      (independent game jobs)        (train/held-out gauntlets)
```

### Package Boundaries

Keep the initial package count small. These are real dependency boundaries, not speculative services.

| Package | Responsibility | May Depend On | Must Not Depend On |
|---|---|---|---|
| `packages/engine` | State schema, board geometry, turn/storyline state machine, legal actions, validated transitions, rule queries, typed effect primitives, card behavior registry, observations, deterministic RNG | Plain card definitions bundled as data | Node I/O, HTTP, model SDKs, React, database, wall clock, global randomness |
| `packages/data` | Pinned official card snapshot, ruleset metadata, coverage manifest, normalized owned collection and deck artifacts | Shared JSON types | Live fetches during a game or evaluation |
| `packages/agents` | Deterministic policies, search/evaluation, model adapter contract, observation/prompt formatting, decision telemetry | `engine` public observation/action types | Internal mutable engine state |
| `packages/simulator` | Match loop, seat alternation, run manifests, replay verification, worker pool, JSONL output, aggregate statistics | `engine`, `data`, `agents`, Node standard library | UI code, database initially |
| `packages/optimizer` | Collection-constrained candidate generation and train/held-out evaluation | `simulator`, `data` | Rules reimplementations |
| `apps/web` | Human board, legal-action affordances, animation from events, replay viewer, UI-only state | Browser-safe `engine`, `data` | Direct game-state mutation, duplicated legality logic |
| `apps/tools` | Non-interactive developer commands for import, validation, simulation, coverage, and replay | All non-UI packages | Human gameplay flows |

`packages/cards` is not needed as a separate package initially. Keep executable card behaviors under `packages/engine/src/cards/` and static card records under `packages/data/`. Split only if engine build or ownership actually demands it.

### Dependency Direction

```text
data types/data -----> engine <----- agents
                         ^             ^
                         |             |
                         +-- simulator-+
                                ^
                                |
                            optimizer

web --------------------> engine
```

The engine imports no consumer. This is what lets Node workers and the browser execute the exact same rule code.

## Engine Contract

Use a small functional surface. The implementation may use internal helpers, but consumers get no setters.

```typescript
export type ActionSelection = Readonly<{
  stateVersion: number;
  actionId: string;
}>;

export type StepResult =
  | Readonly<{ status: "ok"; state: GameState; events: readonly GameEvent[] }>
  | Readonly<{ status: "rejected"; reason: RejectionReason }>
  | Readonly<{ status: "unsupported"; gap: CoverageGap }>;

export function createGame(spec: MatchSpec): GameState;
export function observeFor(state: GameState, seat: Seat): PlayerObservation;
export function legalActions(state: GameState, seat: Seat): readonly LegalAction[];
export function step(state: GameState, selection: ActionSelection): StepResult;
export function coverageReport(spec: MatchSpec): CoverageReport;
```

`step` regenerates the legal actions for the current state and resolves `actionId` from that set. It rejects a stale `stateVersion` or unknown ID. This prevents a client from taking a once-legal action after state changes and prevents a model from inventing an action payload.

Legal action IDs must be deterministic functions of the action kind and canonical parameters, for example `cast:e17:at:r2c3`. Do not use UUIDs. Always return legal actions in a documented stable order; agents may rely on order for deterministic tie-breaking.

### State Shape

Use only JSON-compatible plain objects, arrays, strings, integers, booleans, and null. Avoid `Map`, `Set`, `Date`, class instances, functions, `undefined`, and cached object references.

```text
GameState
├── schemaVersion, stateVersion
├── rulesetId, cardSnapshotHash, behaviorRegistryHash
├── phase, turn, activeSeat, prioritySeat
├── storyline[] / pendingChoice
├── players[seat] (life, mana, thresholds, once-per-turn flags)
├── realm.cells[] (fixed coordinates and region layers)
├── entities[id] (base card ID, owner, controller, counters, damage, flags)
├── zones[seat] (ordered arrays of entity IDs)
├── continuousModifiers[] / delayedTriggers[]
├── rng (algorithm ID, four-word state, draw count)
├── sequences (next entity/event/action number)
└── terminal (winner, draw, or invalidation)
```

Normalize entities by deterministic entity ID; zones and realm cells contain ordered IDs. This avoids copying full card records around the state and gives effects stable targets. Keep printed card characteristics in the pinned data snapshot, not repeated on every entity.

Do not cache derived attack, life, threshold, region, or legal-target values in state. Compute them through rule queries so continuous modifiers and rule changes have one source of truth.

### Turn and Resolution State Machine

Model the rulebook flow explicitly rather than scattering phase checks through card handlers:

```text
setup -> turnStart -> main/priority -> resolution/storyline -> turnEnd
                       ^                    |
                       +---- choice --------+
```

`pendingChoice` identifies the seat and constrained choices required to continue. While it exists, `legalActions` exposes only that choice set (plus a concession action if supported). Trigger discovery creates ordered storyline entries; resolving an entry may emit primitive effects, create another choice, or enqueue new triggers. This supports human and model players without a special interactive path.

The engine should not use async code. A step either completes synchronously or returns a deterministic pending choice. Model/network waiting happens outside the engine.

## Data Flow

### One Decision

```text
1. Orchestrator asks engine for observeFor(state, activeSeat).
2. Engine returns hidden-information-safe observation plus legal actions.
3. Agent/UI returns {stateVersion, actionId}.
4. Engine regenerates the legal set and validates that exact ID.
5. Engine resolves the action through rule and effect primitives.
6. Engine returns new immutable state plus ordered semantic events.
7. Orchestrator appends the decision, events, and hashes to the transcript.
8. UI renders/animates events; simulator advances immediately.
```

### Event Design

Events describe rules outcomes, not UI gestures:

```typescript
type GameEvent =
  | { seq: number; type: "card-moved"; entityId: EntityId; from: Zone; to: Zone }
  | { seq: number; type: "damage-dealt"; sourceId?: EntityId; targetId: EntityId; amount: number }
  | { seq: number; type: "random-result"; purpose: string; value: number }
  | { seq: number; type: "choice-requested"; choiceId: string; seat: Seat }
  | { seq: number; type: "game-ended"; winner: Seat; reason: EndReason };
```

Generate human prose in the GUI/report layer. Semantic events remain stable, machine-testable, and localizable. Event sequence numbers come from state counters, never time or randomness.

The action transcript is replay input. Events and post-state hashes are replay assertions. This is deliberately lighter than full event sourcing.

## Rules and Card Effects

### Recommended Representation

Use data for printed facts and typed TypeScript for behavior:

- **Static data:** name, type, cost, threshold, base power, elements, rules text, set/version, image reference.
- **Core rules:** turns, zones, site placement, movement, region/adjacency, summoning, combat, damage, death's door/death blow, Storyline ordering, targeting, costs, and keyword semantics.
- **Card behavior:** a registry keyed by the pinned card ID. Handlers return typed primitive effects or pending-choice descriptions; they do not mutate `GameState` directly.
- **Effects:** the engine alone applies primitives such as `moveEntity`, `dealDamage`, `tap`, `spendMana`, `addCounter`, `summonToken`, and `enqueueTrigger`, checking global invariants at the mutation point.

Do not parse English rules text into executable behavior and do not invent a general-purpose card DSL in the first milestone. Both hide unsupported semantics. Typed handlers plus shared primitives are less code and easier to audit against individual rulings. Extract a declarative DSL only after repeated card implementations reveal a stable vocabulary.

### Effects and Modifiers

Use three mechanisms, not one catch-all callback:

| Mechanism | Representation | Example use |
|---|---|---|
| Immediate effect | Ordered typed primitives applied during resolution | Draw, damage, move, summon, tap |
| Triggered/delayed effect | Storyline entry with source and timing metadata | Genesis, death triggers, end-of-turn work |
| Continuous/replacement rule | Registered query/replacement contribution while source is active | Ward, Stealth targeting restriction, stat/region modifiers |

Replacement and continuous effects must feed the same legality and resolution queries. An agent-side approximation of a keyword is not rules support.

### Coverage Gate: Fail Closed

Unsupported is the default. Maintain a machine-readable coverage manifest mapping:

```text
ruleset ID
  -> rule/keyword/mechanic ID
     -> implementation ID
     -> supporting tests and rule/ruling citations
card snapshot ID
  -> card ID
     -> behavior implementation ID
     -> referenced tokens/cards/mechanics
```

Before a ranked run, `coverageReport(matchSpec)` traverses both decks, avatars, tokens, referenced cards, and declared mechanic dependencies. Any missing or mismatched implementation prevents the job from starting. At runtime, a dynamically reached unsupported effect returns `status: "unsupported"`, leaves the prior state unchanged, emits a coverage diagnostic outside the canonical game events, and marks the game ineligible for statistics.

Never turn unknown text into a no-op, generic damage, guessed token, or model ruling. A GUI may label an unsupported deck/card before play, but it must not silently continue an authoritative match.

## Determinism and Reproducibility

### RNG

Use an engine-owned `xoshiro128** 1.1` implementation with four unsigned 32-bit state words. The authors publish the algorithm and jump functions; pin the algorithm/version and test it against fixed vectors. It is fast, compact, browser-compatible with explicit 32-bit operations, and intended for non-cryptographic simulation. Never call `Math.random()` inside engine, agent, or deterministic simulator paths.

Each game gets an independent 128-bit seed derived with SHA-256 from canonical run inputs:

```text
SHA-256(rootSeed || gameIndex || seatAssignment || purpose)
```

The game seed depends on `gameIndex`, not the worker that happens to run it. Worker count and scheduling therefore cannot change results. Store RNG state and draw count in `GameState`; emit the purpose and result of every random rule outcome.

Use deterministic monotonic counters for entity, choice, action, and event IDs. Wall-clock time, process IDs, UUIDs, filesystem ordering, locale-sensitive comparisons, and iteration over unsorted external keys do not belong in canonical game output.

### Canonical Serialization and Hashing

Serialize manifests, decisions, events, and state checkpoints with RFC 8785 JSON Canonicalization Scheme rules, then hash with SHA-256. A transcript row should include `preStateHash`, the selected action, emitted events, and `postStateHash`. Two deterministic runs are equal only if their canonical transcript bytes match—not merely their winner and turn count.

Telemetry that naturally varies, such as wall-clock duration, CPU time, model latency, request IDs, and file paths, belongs in a non-canonical sidecar. It must not change event bytes or game hashes.

### Run Manifest

At minimum, pin:

| Field | Reason |
|---|---|
| Manifest schema version and run ID derived from its canonical hash | Makes artifacts addressable without random IDs |
| Engine build/commit and package lock hash | Prevents code/dependency drift |
| Ruleset ID and source document hashes | Prevents rule-edition ambiguity |
| Card snapshot and behavior registry hashes | Prevents card text/implementation drift |
| Both normalized deck artifact hashes and seat mapping | Recreates exact inputs |
| Agent/model adapter ID, configuration, prompt version, and policy hash | Recreates decision behavior where possible |
| Root seed, game count, job indices, seat-alternation plan | Recreates randomness and scheduling-independent jobs |
| Simulation options and termination limits | Recreates adjudication |
| For model runs: provider/model identifiers and recorded decision transcript hash | External inference is not inherently repeatable |

For a scripted agent, manifest plus code and seed must reproduce byte-identical events. For a live external model, do not claim the same: record observations, exact prompts, raw responses, parsed action IDs, usage, and errors. Replay uses the recorded action IDs without calling the model again.

## Agent and Model Isolation

```typescript
interface Agent {
  readonly descriptor: AgentDescriptor;
  chooseAction(input: Readonly<{
    observation: PlayerObservation;
    legalActions: readonly LegalAction[];
    decisionIndex: number;
  }>): Promise<AgentDecision>;
}

type AgentDecision = Readonly<{
  actionId: string;
  telemetry?: DecisionTelemetry;
}>;
```

`PlayerObservation` omits opponent hidden zones and internal effect implementation details. Freeze it in development/tests. The adapter receives no `GameState`, setter, effect context, RNG, or engine internals.

Model adapters own provider SDK calls, prompt formatting, schema parsing, timeout/cost/latency capture, and redaction. The common harness verifies the returned ID against the provided list. Use these result classes:

- **Valid model choice:** advance normally.
- **Unparseable or non-listed action:** count an illegal decision and apply one documented deterministic fallback from the current legal list so the game can finish; reports must separate fallback-assisted results.
- **Transport/rate-limit/provider failure:** mark the game infrastructure-invalid and retry the whole job under a recorded retry policy; do not score it as strategy.
- **Unsupported engine behavior:** invalidate the game for everyone; it is not an agent fault.

Do not put provider-specific response formats into engine or simulator contracts. A fake adapter returning scripted responses provides the model harness test seam without paid calls.

## Simulation Workers

Use Node's stable `worker_threads` only for CPU-bound deterministic games, through a fixed worker pool. Node's documentation explicitly recommends a pool rather than spawning a worker per task. Default the pool from `os.availableParallelism()` with one coordinator thread retained, and allow an explicit override.

```text
coordinator
  -> creates ordered GameJob records with fixed job index and seed
  -> dispatches jobs to long-lived workers
  -> receives GameResult/transcript chunks
  -> sorts by job index
  -> writes artifacts and aggregates statistics
```

Workers load the immutable card snapshot and behavior registry once. Each job owns a fresh game state and agent instances; workers share no game mutation state. The coordinator is the only aggregate writer, avoiding locks and nondeterministic interleaving in report files.

External model calls are I/O-bound and rate-limited, so run them through a separate bounded async scheduler using the same match loop and `Agent` contract. Do not occupy the deterministic CPU pool while waiting on model APIs. Recorded model decisions can later be replayed cheaply in workers.

Start with local workers and JSON/JSONL files. A database, distributed queue, Redis, and Podman deployment are unnecessary until a measured workload cannot fit on one machine.

## Deck Provenance and Optimization Boundary

Every collection or deck import produces two immutable artifacts:

1. **Raw source snapshot:** original bytes plus source URL or supplied-file identity, author/event/placement when available, retrieval timestamp, and content hash.
2. **Normalized deck:** avatar, ordered/quantified Atlas and Spellbook entries keyed by pinned card IDs, format/ruleset, validation result, provenance link, and canonical hash.

The runner consumes only normalized, hashed artifacts and never fetches a live deck or card API during a run. If an online source changes, import it as a new artifact instead of mutating history.

The optimizer operates above the simulator. It proposes a normalized deck candidate under collection and format constraints, asks the simulator to evaluate it, and receives results. It cannot call engine mutation primitives or substitute heuristic card outcomes. Record the collection hash, base-deck hash, candidate diff, search parameters, training gauntlet, held-out gauntlet, and all seeds. Final recommendations require held-out matchup evidence; do not optimize and report on the same games.

## Browser GUI Integration

The first GUI should be a local human-vs-scripted-agent session. `apps/web` imports the same browser-safe engine package used by Node simulation:

```text
engine observation -> render board/hand and legal affordances
user gesture       -> select an existing actionId
engine step         -> new state + semantic events
events              -> animation/log/accessibility announcements
```

Drag-and-drop is only a gesture recognizer. It maps a source/target gesture to one current legal action; it never moves a card array itself. Keep hover, selection, camera, animation progress, and modal state in the UI layer, separate from `GameState`.

Do not add HTTP or WebSocket transport for the local product. If public multiplayer is later authorized, run `@sorcery/engine` on the server and send the same `{stateVersion, actionId}` protocol; the browser renders seat-safe observations and events. No rule rewrite is required.

Run the engine on the main browser thread initially. Move it behind a Web Worker only if profiling shows perceptible UI stalls; the serializable contract already permits that change.

## Testing Seams and Required Gates

| Seam | Smallest authoritative check | Gate it protects |
|---|---|---|
| PRNG | Published/frozen output vectors plus serialize/resume state test | Same seed means same random stream |
| State reducer | Frozen input state; `step` cannot mutate it | Branch/search safety and replay |
| Legal action contract | Every enumerated action applies; unknown/stale IDs reject without state change | Shared UI/agent legality |
| Hidden information | Snapshot of `observeFor` per seat contains no opponent hand/deck identities | Fair model/agent input |
| Board geometry | Table tests for adjacency, regions/layers, multi-square entities, and edges | Spatial rules correctness |
| Turn/Storyline | Scenario tests for priority, simultaneous triggers, nested choices, and cleanup | Resolution correctness |
| Rule/card behavior | One cited scenario test per supported mechanic/card interaction; reuse shared primitive tests | Coverage claim honesty |
| Coverage gate | An unknown card, keyword, referenced token, and dynamic unsupported path each invalidate | No silent approximation |
| Replay | Reapply transcript from manifest and compare every event and state hash | Reproducibility |
| Worker invariance | Same manifest at worker counts 1 and N produces byte-identical ordered transcripts | Scheduling independence |
| Agent conformance | Scripted/fake-model agents can only choose provided IDs; invalid output is classified | Model isolation and metrics |
| Seat fairness | Paired games swap decks/agents and derive independent fixed seeds | Trustworthy matchup results |
| Optimizer | Collection/format constraints plus held-out evaluation fixture | Actionable, non-overfit recommendations |

Maintain scenario fixtures as compact JSON inputs plus expected legal actions/events. Each rule scenario cites the exact rulebook/Codex/card ruling version it encodes. Golden tests should target stable semantic contracts, not full internal state snapshots for every action.

## Patterns to Follow

### Pure Transition with Explicit Random State

**What:** Calculate new state and events only from the old state, a legal action selection, pinned data, and RNG state stored in the old state.

**When:** Every authoritative game mutation.

**Why:** Pure reducer discipline makes behavior predictable and testable; Redux's official fundamentals explicitly classify `Math.random()` and `Date.now()` as side effects that do not belong inside pure reducer logic.

### Functional Core, Imperative Shell

**What:** Keep rules synchronous and deterministic; keep filesystem, HTTP, model calls, timing, workers, and rendering in orchestration adapters.

**When:** All simulator and GUI integrations.

**Why:** The same core can run in browser, coordinator, worker, test, and replay without environment branches.

### Enumerate, Then Select

**What:** The engine enumerates every legal choice; clients select a state-bound ID.

**When:** Human, scripted, search, and model decisions—including nested effect choices.

**Why:** It eliminates duplicated agent/UI rule logic and gives illegal-action rate a precise meaning.

### Pinned Inputs, Content-Addressed Outputs

**What:** Hash rules, cards, behaviors, decks, agents, manifests, and transcripts after canonical serialization.

**When:** Every ranked or comparative run.

**Why:** A result without exact inputs cannot support a deck recommendation or model comparison.

## Anti-Patterns to Avoid

### Client-Supplied State Patches

**What:** UI or bot sends changed zones, permanents, life, mana, or phase and a server merges them.

**Why bad:** Validation becomes a growing blacklist, triggers can observe partial states, and clients effectively become alternate rules engines.

**Instead:** Submit only a current legal-action ID to the engine.

### Agent Candidate Generation as Rules

**What:** A bot separately decides which casts, moves, and attacks are legal, often capping candidates.

**Why bad:** The bot and engine play different games; a capped list silently removes legal tactics.

**Instead:** The engine enumerates the complete legal set; agents may rank/prune only after receiving it.

### UI Store as Game Authority

**What:** Drag handlers and React/Zustand state directly move or modify cards.

**Why bad:** Headless simulation cannot reuse it, hidden state leaks, and replay becomes gesture-dependent.

**Instead:** UI state is presentation only; game gestures resolve to engine actions.

### Silent Effect Failure

**What:** An unknown handler catches errors and returns null/no-op.

**Why bad:** The game continues with false rules coverage and produces plausible but invalid rankings.

**Instead:** Typed error classification and runtime invalidation.

### Live Mutable Inputs

**What:** Fetch card data or online decklists while running a gauntlet.

**Why bad:** Identical commands can receive different inputs, and source changes cannot be audited.

**Instead:** Import, normalize, validate, and hash first.

## Reference Implementations: What to Reuse

### Contested Realms

**Recommendation:** Treat as behavioral and test-case reference unless the project deliberately accepts GPL-3.0 derivative obligations. It is not the foundation for this simulator.

Useful material to audit:

- Spatial/position transition validation and existing card interaction cases.
- Mana, threshold, placement, movement, and combat behavior as hypotheses to verify against the pinned official rules.
- Bot feature extraction, telemetry fields, and regression ideas.
- Deck import/validation and board interaction workflows.

Do not inherit:

- Its `off` / `bot_only` / `all` enforcement modes; authoritative simulation has one strict mode.
- Client-originated state patches merged into server game state.
- The separate bot `generateCandidates` legality implementation and candidate cap.
- `Math.random()`, `Date.now()`, and random IDs throughout rule/card paths.
- Silent `null` returns for unknown/failed effects.

At inspected revision `bfde327a90e96f6281ef8f6876712665954d8317`, the server applies client patches before movement/trigger enrichments; the bot README acknowledges missing regions, instants, triggered/activated abilities, stack mechanics, graveyard interactions, and deck construction. These are strong warnings against treating its results as authoritative, even though its broad interaction inventory is valuable research input.

### Spells.bar (`sorcery-tcg-playtest`)

**Recommendation:** Reuse UX ideas and deck-format knowledge only. No license file was present at inspected revision `a570ff48e435abdc0eaa777a8bd1d2cc6a69a7b5`, so do not copy code unless the owner supplies compatible terms.

The project is an effective manual tabletop: drag actions directly rearrange `GameState` arrays and its WebSocket provider synchronizes player state. That is appropriate for free-form human play but supplies no authoritative legality, deterministic reducer, AI observation boundary, or trustworthy simulation baseline.

Useful material to audit:

- Grid/card interaction affordances and mobile gesture handling.
- Deck import mappings, preconstructed decks, and human board layout.
- Manual commands as a catalog of interactions the authoritative GUI will eventually need to express legally.

## Scalability Considerations

| Concern | Initial/local | Larger local studies | Distributed scale (only if measured) |
|---|---|---|---|
| Engine | One pure in-process state machine | Same engine in worker pool | Same immutable `GameJob` contract on remote workers |
| Artifacts | Canonical JSON/JSONL files | Partition by run/game and stream aggregation | Object storage plus manifest index |
| Parallelism | Single thread for correctness | Fixed `worker_threads` pool | External queue; never share RNG streams |
| Model calls | Small bounded async concurrency | Provider-aware rate limiter and recorded cache | Dedicated inference scheduler |
| Analytics | Stream aggregate counters and confidence intervals | Columnar export if profiling demands it | Analytical store after file workflow becomes painful |
| GUI | Main-thread local engine | Browser Worker if profiling demands it | Authoritative server only for future multiplayer |

Do not build the rightmost column during the simulator MVP.

## Suggested Build Order

1. **Reproducibility foundation**
   - Pin the December 2025 official rules artifacts and a card snapshot.
   - Define plain state/action/event/observation schemas, canonical serialization, xoshiro RNG, and deterministic IDs.
   - Prove byte-identical seed replay before implementing many rules.

2. **Core rules vertical slice**
   - Implement setup, ordered zones, 5x4 realm geometry/layers, turns, site play, mana/threshold costs, units, movement, combat, damage, and end conditions.
   - Use a tiny explicit supported card pool and scenario tests. Do not approximate other cards.

3. **Storyline, choices, effects, and coverage**
   - Add trigger ordering, pending choices, continuous/replacement queries, typed primitives, behavior registry, coverage manifest, and fail-closed preflight/runtime gates.
   - Expand cards by complete mechanic families, with cited interaction tests.

4. **Deterministic competitors and replayable match runner**
   - Add the shared `Agent` contract, a simple legal scripted agent, action/event JSONL, run manifests, replay verification, seat-paired jobs, and worker-count invariance.

5. **Deck/collection provenance and competitive gauntlet**
   - Import the supplied 223-card collection and decklists, then attributable common online decks as pinned normalized artifacts.
   - Run only decks whose transitive coverage is complete.

6. **Model adapters and evaluation telemetry**
   - Add provider-isolated adapters, observation/prompt versioning, fake-adapter tests, legal/illegal decision accounting, cost/latency sidecars, and recorded-decision replay.

7. **Collection-constrained optimizer**
   - Generate legal candidates, compare over seat-alternated multi-deck gauntlets, and validate on held-out matchups.

8. **Browser GUI**
   - Build human play and replay UI over `observeFor`, `legalActions`, `step`, and semantic events. No UI-specific rules.

**Ordering rationale:** Determinism and the legal-action boundary are architectural invariants that become expensive to retrofit. Core rules and fail-closed coverage must exist before match results mean anything. Scripted competitors validate the simulator cheaply before model variability/cost is introduced. Optimization depends on a representative gauntlet, and the GUI depends on a stable engine contract rather than driving its design.

## Sources

### High Confidence: Official and Primary Technical Sources

- [Sorcery: Contested Realm — December 2025 Rulebook Update](https://sorcerytcg.com/news/sorcery-contested-realm-december-2025-rulebook-update) — official current rules release page; confirms the edition, 60-card Spellbook, and newly codified/clarified mechanics.
- [Sorcery: Contested Realm — How to Play](https://sorcerytcg.com/how-to-play) — official rulebook/FAQ entry point and card-interaction guidance.
- [Redux official fundamentals: State, Actions, and Reducers](https://redux.js.org/tutorials/fundamentals/part-3-state-actions-reducers) — primary documentation for plain serializable state, action-driven updates, pure reducers, and exclusion of time/random side effects.
- [RFC 8785: JSON Canonicalization Scheme](https://www.rfc-editor.org/rfc/rfc8785) — canonical JSON rules for reproducible bytes and hashes.
- [xoshiro128** 1.1 reference implementation](https://prng.di.unimi.it/xoshiro128starstar.c) — algorithm authors' versioned reference and jump functions; public-domain dedication.
- [Node.js `worker_threads`](https://nodejs.org/api/worker_threads.html) — official documentation: workers suit CPU work and should be pooled rather than created per task.
- [Node.js `os.availableParallelism()`](https://nodejs.org/api/os.html#osavailableparallelism) — official API for estimating suitable local parallelism.
- [Node.js hashing API](https://nodejs.org/api/crypto.html#cryptocreatehashalgorithm-options) — official hashing primitive for run/input/transcript identity.

### High Confidence: Inspected Primary Source Repositories

- [Contested Realms repository](https://github.com/realms-cards/contested-realms) and [GPL-3.0 license](https://github.com/realms-cards/contested-realms/blob/main/LICENSE).
- [Contested Realms match leader](https://github.com/realms-cards/contested-realms/blob/bfde327a90e96f6281ef8f6876712665954d8317/server/modules/match-leader.ts) — inspected patch merge, rule enrichment, event, and broadcast flow.
- [Contested Realms bot engine documentation](https://github.com/realms-cards/contested-realms/blob/bfde327a90e96f6281ef8f6876712665954d8317/bots/engine/README.md) and [implementation](https://github.com/realms-cards/contested-realms/blob/bfde327a90e96f6281ef8f6876712665954d8317/bots/engine/index.js) — inspected candidate generation, search/evaluation, telemetry, and declared limitations.
- [Contested Realms rule effect registry](https://github.com/realms-cards/contested-realms/blob/bfde327a90e96f6281ef8f6876712665954d8317/server/rules/effects.js) and [position transition validator](https://github.com/realms-cards/contested-realms/blob/bfde327a90e96f6281ef8f6876712665954d8317/src/lib/game/stateTransitionValidator.ts) — examples of reusable behavior research and silent-failure risk.
- [Spells.bar repository](https://github.com/JollyGrin/sorcery-tcg-playtest), [grid actions](https://github.com/JollyGrin/sorcery-tcg-playtest/blob/a570ff48e435abdc0eaa777a8bd1d2cc6a69a7b5/src/utils/actions/grid.ts), [drag handler](https://github.com/JollyGrin/sorcery-tcg-playtest/blob/a570ff48e435abdc0eaa777a8bd1d2cc6a69a7b5/src/components/organisms/GameBoard/useHandleDrag.ts), and [WebSocket provider](https://github.com/JollyGrin/sorcery-tcg-playtest/blob/a570ff48e435abdc0eaa777a8bd1d2cc6a69a7b5/src/lib/contexts/WebGameProvider.tsx) — confirms manual tabletop/UI-state architecture and useful interaction patterns.

## Open Questions for Phase-Specific Research

- The pinned official rulebook PDF and Codex/card FAQ snapshot need content hashing and a citation scheme before rule implementation starts.
- The complete card snapshot must be mechanically classified to estimate behavior-family breadth and determine the first fully supported competitive gauntlet.
- Some official interactions may require explicit rulings beyond the base rulebook; card coverage must identify those dependencies rather than infer them.
- Direct code reuse from Contested Realms requires an explicit GPL-3.0 product licensing decision. Spells.bar needs affirmative license clarification before any copying.

## Confidence Assessment

| Area | Confidence | Notes |
|---|---|---|
| Engine/action boundary | HIGH | Follows deterministic reducer principles and directly prevents failure modes observed in both references |
| Determinism/replay | HIGH | Versioned PRNG reference, canonical JSON RFC, standard hashing, and scheduling-independent seeds are concrete and testable |
| Worker architecture | HIGH | Official Node guidance supports a pool for CPU-bound simulation |
| Model isolation | HIGH | Requirement-driven boundary; recorded decisions honestly handle external model nondeterminism |
| Card-effect representation | MEDIUM | Typed handlers/primitives are the right initial shape, but complete mechanic families require card/ruling audit |
| GUI integration | HIGH | Browser-safe engine and action-ID boundary avoid duplicated rules; remote multiplayer remains intentionally out of scope |
| Reference reuse | HIGH | Based on source inspection at recorded commits and declared/missing licenses |
