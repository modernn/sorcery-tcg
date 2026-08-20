# Domain Pitfalls

**Domain:** Deterministic Sorcery: Contested Realm simulator, AI competitor testbed, and owned-card deck optimizer  
**Researched:** 2026-08-20  
**Overall confidence:** HIGH for engine/reproducibility risks; MEDIUM for ecosystem licensing and online-deck representativeness

## Non-Negotiable Release Gate

A result is **ranked evidence** only when its manifest pins the rules, card data, engine, PRNG, decks, competitors, and experiment design; every exercised rule path is implemented; and replay reproduces the deterministic engine-event hash. Anything else is an exploratory result and must not drive deck recommendations.

## Critical Pitfalls

### 1. Rules-Version Drift

**Confidence:** HIGH  
**Roadmap owner:** Authority/provenance phase, before engine behavior

**What goes wrong:** A live rulebook, Codex entry, format rule, or implicit community ruling changes while old results keep the same label. Results from incompatible rules editions then get pooled. This is already a real concern: the December 2025 rulebook set constructed Spellbooks to 60 cards and clarified Ward, Collection, Stealth, Pick Up, and Transform.

**Warning signs:** Manifests say only `current`; CI downloads rules live; an old replay changes after a data refresh; deck legality changes without a ruleset migration.

**Prevention:**

- Define one immutable `rulesetId` covering the exact rulebook edition, Codex/FAQ snapshot, official card updates, and format rules.
- Store retrieval time, source URLs, SHA-256 hashes, and raw source artifacts where redistribution permits.
- Create a new ruleset for an update; never edit a historical snapshot in place and never aggregate across rulesets by default.
- Turn official examples and clarified interactions into version-scoped conformance tests.

### 2. Mutable Card Text, FAQs, and Errata

**Confidence:** HIGH  
**Roadmap owner:** Card-data ingestion phase

**What goes wrong:** Printed text, API text, updated text, and card-specific FAQs disagree. In November 2025, official updates changed the Druid and added “against units” restrictions to six cards, demonstrating that card identity alone does not identify behavior.

**Warning signs:** Cards are fetched at match time; card behavior keys off display name; a card update silently changes historical runs; duplicate printings get separate rule implementations.

**Prevention:**

- Snapshot canonical card IDs, printed text, effective text, FAQ/ruling references, set/printing identity, and source hashes.
- Separate immutable card identity from printing and rules-effective behavior.
- Validate the owner's collection against stable IDs and retain the original import plus normalized mapping.
- Make card-data upgrades explicit migrations with deck-legality and replay impact reports.

### 3. Incomplete Mechanics Masquerading as Results

**Confidence:** HIGH  
**Roadmap owner:** Rules-kernel and card-coverage phases; gate every later ranked phase

**What goes wrong:** Unsupported targeting, movement, regions, carriers, auras, interception/defense, simultaneous effects, generated cards, or card interactions become no-ops or approximations. The simulator still emits a win rate, which looks more authoritative than it is.

**Warning signs:** “Implemented card count” is the only coverage metric; unknown keywords are ignored; generic damage/substitution logic stands in for card text; a reference simulator is treated as an oracle; ranked runs contain warnings.

**Prevention:**

- Maintain a machine-readable coverage registry for rules, keywords, cards, choices, and known interactions—not just card names.
- Preflight each matchup's declared mechanic closure, then fail closed at runtime for dynamic/generated behavior the preflight could not see.
- Mark the entire game invalid for ranking on an unsupported path; never salvage its outcome.
- Back implementations with official rulebook/Codex/FAQ scenarios, regression tests, and invariants. Reference tools may suggest cases but are not authoritative.
- Publish coverage and invalidation counts next to every result.

### 4. Nondeterminism Hidden by a Seed Field

**Confidence:** HIGH  
**Roadmap owner:** Deterministic-kernel phase

**What goes wrong:** A manifest contains a seed, but shuffling occurred before seeding, `Math.random()` or time/UUID calls bypass the injected RNG, workers consume a shared stream in schedule order, or result aggregation follows completion order. ECMAScript explicitly leaves the `Math.random()` algorithm implementation-defined.

**Warning signs:** Same seed differs across runs, Node versions, or worker counts; failures cannot be replayed; adding logging changes outcomes; deterministic and parallel runs hash differently.

**Prevention:**

- Inject one named, versioned PRNG into every chance operation; ban `Math.random`, wall-clock time, random UUIDs, and unseeded library randomness from engine code.
- Derive each game's seed from `(rootSeed, stableGameId)`; never share a mutable RNG across games or workers.
- Use integers for game quantities and immutable per-game state. Keep latency/cost telemetry outside deterministic events.
- Aggregate outputs in stable game-ID order and canonicalize event JSON before hashing.
- CI must replay a golden corpus byte-for-byte and compare 1-worker versus N-worker event hashes.
- For remote AI models, replay recorded chosen actions; never claim that re-querying a model is deterministic.

### 5. Hidden-Information Leakage

**Confidence:** HIGH  
**Roadmap owner:** Observation/action-contract phase, before AI adapters or GUI

**What goes wrong:** A model or UI receives the authoritative state, opponent hand/deck order, hidden identities, or a legal-action list whose shape reveals hidden facts. It then appears strategically stronger than a legal player.

**Warning signs:** Agents accept `GameState`; prompt/debug logs contain both hands; spectator and player serializers are shared; changing an opponent's hidden hand changes the active player's observation without a public event.

**Prevention:**

- Keep authoritative state private and expose explicit `observationFor(player)` and player-scoped legal actions.
- Separate player, public/spectator, and privileged replay projections. Make model adapters accept only the player projection.
- Add an information-noninterference test: vary opponent-private state while public/own information is fixed and assert identical observations and available actions.
- Log the exact redacted observation sent to each competitor; restrict full-state logs to the replay artifact.

### 6. Replay Incompatibility and Unverifiable Logs

**Confidence:** HIGH  
**Roadmap owner:** Kernel/event-log phase

**What goes wrong:** Replays store array indexes or mutable card names, omit engine/data versions, contain wall-clock fields, or are interpreted using new rules. Old runs can no longer be proven or viewed accurately.

**Warning signs:** Replays require today's card API; action IDs change when enumeration order changes; event snapshots cannot be hashed consistently; schema changes overwrite old fixtures.

**Prevention:**

- Manifests must include schema versions, engine commit/build hash, ruleset/card hashes, PRNG algorithm/version, root and derived game seeds, canonical deck hashes, and competitor configuration.
- Use structured stable action IDs, not legal-action array positions.
- Define two promises: exact validation replay on the recorded engine/ruleset, and viewer compatibility through explicit event-schema migrations.
- Keep deterministic events append-only and free of telemetry. Use RFC 8785-style canonical JSON for hashes.
- Keep a golden replay corpus in CI; do not build checkpoint/snapshot machinery until log size proves it necessary.

### 7. GUI/Engine Rules Drift

**Confidence:** HIGH  
**Roadmap owner:** GUI phase, after engine verification

**What goes wrong:** The browser duplicates legality, card effects, derived totals, or turn transitions. Humans and agents then play subtly different games.

**Warning signs:** React components inspect card text to decide actions; UI reducers mutate game state; browser-only rules bugs; optimistic UI becomes authoritative.

**Prevention:**

- Ship the exact engine and observation/action contracts as shared TypeScript packages.
- The GUI renders an engine projection and submits an engine-issued action; it never computes authoritative legality or mutations.
- Card display metadata may be duplicated for presentation, but card behavior may not.
- Run transcript contract tests through both the headless adapter and browser adapter and compare engine event hashes.

## Evaluation and Optimization Pitfalls

### 8. Action-Space Explosion

**Confidence:** HIGH  
**Roadmap owner:** Observation/action-contract and agent phases

**What goes wrong:** Eagerly materializing complete combinations of card, mode, target, path, payment, and optional choices creates a combinatorial list, bloated model prompts, unstable action indexes, and slow search.

**Warning signs:** Thousands of legal actions in ordinary turns; prompt cost grows with board complexity; agents choose arbitrary JSON because enumeration is impractical; equivalent actions appear repeatedly.

**Prevention:** Use engine-owned staged decisions (`choose action` → `choose mode/target/path`) with stable typed choices. Validate each continuation, but mutate authoritative state only when the complete action commits. Cache only after profiling; correctness and stable ordering come first.

### 9. Model-vs-Deck Confounding

**Confidence:** HIGH  
**Roadmap owner:** Statistical harness and AI-adapter phases

**What goes wrong:** Model A plays deck X while model B plays deck Y, so a win difference cannot be attributed to model skill or deck strength. Seat, seed, opponent, prompt, retry policy, and model version are additional nuisance factors.

**Warning signs:** A single headline leaderboard mixes decks and models; models see different action interfaces; only aggregate win rate is reported; illegal-action retries are hidden.

**Prevention:**

- Cross every model with every benchmark deck and use the same seat-swapped seed blocks and opponents.
- Keep a frozen deterministic competitor as a calibration baseline.
- Report model main effects, deck main effects, model×deck interactions, illegal-action/retry rate, latency, and cost separately.
- Pin prompt/template, adapter code, provider model ID/version if exposed, and sampling configuration.

NIST's randomized-block guidance is directly applicable: hold controllable nuisance factors constant within blocks, then compare the factor of interest.

### 10. Biased “Common Online Deck” Sampling

**Confidence:** MEDIUM  
**Roadmap owner:** Benchmark-corpus phase

**What goes wrong:** Tournament winners, public Curiosa lists, creator deck techs, and easy-to-scrape lists are self-selected populations. Treating them as meta share makes optimizer conclusions overconfident and can omit ordinary or unfavorable archetypes.

**Warning signs:** “Common” has no timeframe or metric; only champions are imported; duplicates inflate an archetype; old-format lists survive a rules update; source URLs and authors are lost.

**Prevention:**

- Define separate strata: tournament top/field lists, source-measured popular lists, official precons, mechanics-coverage adversaries, and owner-legal decks.
- Preserve URL, author, publication/update date, event/placing, format, ruleset, import date, and canonical deck hash.
- Deduplicate exact lists and report archetype weighting explicitly; never infer population meta share without a documented sampling frame.
- Freeze a benchmark release and update it deliberately after rules/card/meta changes.

### 11. Adaptive Overfitting During Deck Search

**Confidence:** HIGH  
**Roadmap owner:** Optimizer phase

**What goes wrong:** Repeated swaps are chosen after inspecting the same gauntlet, so the final deck overfits those opponents and seeds. Looking at the “holdout,” changing the deck, and testing again turns it into training data.

**Warning signs:** Recommendations improve only against one deck; holdout results influence further candidates; only the best of hundreds of trials is reported; final evaluation reuses search seeds.

**Prevention:**

- Search on a development gauntlet and seed set; allow final candidates only one locked evaluation on unseen decks/seeds.
- If a held-out result drives another change, retire it to development and create a fresh holdout.
- Log every candidate tried, respect owned-card and deck-legality constraints throughout, and compare against the unchanged starting deck.
- Require improvement across archetypes or an explicitly declared matchup tradeoff, not just one aggregate number.

Adaptive-data-analysis research shows that repeatedly reusing a holdout can overfit the holdout itself.

### 12. Invalid or Misleading Statistics

**Confidence:** HIGH  
**Roadmap owner:** Statistical harness phase

**What goes wrong:** Point estimates from too few games are ranked as facts; paired seat-swapped games are treated as independent; invalid games disappear from the denominator; sequential runs stop when a preferred answer appears; many candidates are compared without accounting for selection.

**Warning signs:** Win rates have no uncertainty interval; no seat-specific table; sample size was chosen after looking; draws/timeouts/coverage failures are collapsed into losses or omitted; game count is mistaken for independent evidence.

**Prevention:**

- Predeclare matchup matrix, seed blocks, stopping rule, and minimum precision.
- Alternate seats within matched seed blocks and analyze the pairing; report per-seat, per-opponent, and aggregate results.
- Report wins/losses/draws/invalidations/timeouts separately, with Wilson intervals for simple independent proportions and block/bootstrap analysis when observations are paired or clustered.
- Publish effect sizes and intervals, not only ranks or p-values. Correct or confirm on fresh data after selecting among many candidates.

## Legal and Ecosystem Pitfalls

### 13. License and IP Contamination

**Confidence:** HIGH for Contested Realms; MEDIUM for spells.bar and game assets  
**Roadmap owner:** Reference audit before implementation; recheck before distribution

**What goes wrong:** Useful TypeScript is copied before deciding whether GPL-3.0 obligations are acceptable, unlicensed code is treated as open merely because its repository is public, or card art/text is redistributed without confirmed permission.

**Warning signs:** Copied snippets lack provenance; a dependency audit starts after implementation; “behavioral reference” code appears verbatim; the product implies official affiliation.

**Prevention:**

- Decide project licensing before copying: Contested Realms declares GPL-3.0, and combining/copying it can place the combined distribution under GPL obligations.
- Treat spells.bar as behavior-only unless its copyright holder supplies an explicit compatible license; its public root currently shows no license file even though the README calls it open source.
- Keep a provenance ledger for every imported algorithm, test vector, asset, and dataset. Prefer clean-room reimplementation from official rules and independently written tests.
- Confirm terms for official API data, card text, and images before bundling or redistributing them; use identifiers/metadata and runtime links where permission is unclear.
- Include attribution and a clear unofficial/non-affiliation notice. Escalate actual distribution/licensing decisions for legal review.

## Moderate Pitfalls

### Optimizing Engine Throughput Before Correctness

**What goes wrong:** Mutable object pools, shared memory, caches, or native add-ons arrive before replay and conformance tests, making subtle state contamination hard to diagnose.  
**Prevention:** Profile a verified single-process engine first. Parallelize independent games with worker-local state; optimize only measured hot paths while keeping replay hashes invariant.  
**Roadmap owner:** Performance phase after a trustworthy end-to-end gauntlet.

### Treating Reference Simulators as Rules Oracles

**What goes wrong:** Contested Realms advertises configurable enforcement whose default is helper/free-form mode, while spells.bar is a virtual tabletop. Their workflows and test ideas are valuable, but behavior can encode omissions or house interpretations.  
**Prevention:** Triangulate each borrowed behavior against the pinned official rulebook/Codex/FAQ. Record disagreements as conformance cases.  
**Roadmap owner:** Reference audit and each rules slice.

### Mutable Collection and Deck Imports

**What goes wrong:** Fuzzy name matching, printing aliases, side metadata, or silently repaired counts make recommendations impossible to reproduce or exceed the owner's 223 cards.  
**Prevention:** Preserve raw imports, normalize to stable IDs with an explicit mapping report, reject ambiguous names and illegal counts, and hash the normalized collection/decks into every optimizer run.  
**Roadmap owner:** Collection/deck ingestion phase.

## Phase-Specific Warnings

| Phase topic | Exit gate | Deeper research flag |
|---|---|---|
| Rules and card authority | Immutable ruleset/card snapshots with hashes and update procedure | **Required:** Codex/API terms, format rules, and official ruling precedence |
| Reference-tool audit | License/provenance matrix plus rule-behavior comparison; no copied code without approval | **Required:** decide GPL acceptance; seek explicit spells.bar license if reuse is desired |
| Deterministic kernel | Golden replay is byte-identical across reruns and worker counts | Standard engineering once PRNG/event contracts are fixed |
| Core rules and cards | Official scenarios pass; unsupported paths invalidate ranked games | **Required per mechanic cluster:** movement/regions, combat/defense, targets, continuous effects, timing |
| Observation/action API | Information-noninterference and staged-choice tests pass | **Required:** hidden-zone and target-availability audit |
| Deck benchmark corpus | Provenance-complete, ruleset-legal, stratified frozen release | **Required:** define “common” and weighting without claiming unsupported meta share |
| Statistical harness | Predeclared blocked design, uncertainty output, invalid-game accounting | Review pairing/clustering and multi-candidate selection before publishing rankings |
| AI competitors | Same observation/action API as deterministic agents; decisions recorded | Provider-version reproducibility and prompt/token-budget fairness need adapter-specific research |
| Owned-card optimizer | Constraints enforced during search; one untouched final holdout | **Required:** candidate budget, tradeoff policy, and holdout refresh procedure |
| Browser GUI | No rules code in UI; headless/browser transcript hashes agree | Standard frontend work after engine contract stabilizes |

## Sources

- **HIGH:** [Official How to Play page](https://sorcerytcg.com/how-to-play) — current rulebook and card-FAQ authority links.
- **HIGH:** [December 2025 Rulebook Update](https://sorcerytcg.com/news/sorcery-contested-realm-december-2025-rulebook-update) — ruleset changes and current 60-card Spellbook rule.
- **HIGH:** [Official Card Updates 2025](https://sorcerytcg.com/news/sorcery-contested-realm-card-updates-2025) — effective-text changes and changelog practice.
- **HIGH:** [Curiosa Codex](https://curiosa.io/codex) — official deeper rules/card-ruling reference structure.
- **HIGH:** [ECMAScript `Math.random`](https://tc39.es/ecma262/multipage/numbers-and-dates.html#sec-math.random) — implementation-defined PRNG behavior.
- **HIGH:** [RFC 8785 JSON Canonicalization Scheme](https://www.rfc-editor.org/rfc/rfc8785.html) — invariant, deterministically sorted JSON representation.
- **HIGH:** [Node.js worker threads](https://nodejs.org/api/worker_threads.html) — worker-local cloned data and shared-memory behavior.
- **HIGH:** [OpenSpiel observation API](https://github.com/google-deepmind/open_spiel/blob/master/open_spiel/python/observation.py) and [API reference](https://github.com/google-deepmind/open_spiel/blob/master/docs/api_reference.md) — player-scoped observations, information states, legal actions, and serialization patterns.
- **HIGH:** [NIST randomized block designs](https://www.itl.nist.gov/div898/handbook/pri/section3/pri332.htm) — controlling nuisance factors.
- **HIGH:** [NIST proportion confidence intervals](https://www.itl.nist.gov/div898/handbook/prc/section2/prc241.htm) — interval estimation for win proportions.
- **HIGH:** [NIST populations and sampling](https://www.itl.nist.gov/div898/handbook/ppc/section1/ppc134.htm) — representativeness, size, variability, and precision.
- **HIGH:** [Dwork et al., Generalization in Adaptive Data Analysis and Holdout Reuse](https://papers.nips.cc/paper_files/paper/2015/hash/bad5f33780c42f2588878a9d07405083-Abstract.html) — adaptive holdout overfitting.
- **HIGH:** [Contested Realms repository](https://github.com/realms-cards/contested-realms) and [GPL-3.0 license](https://github.com/realms-cards/contested-realms/blob/main/LICENSE) — reference behavior and declared license.
- **HIGH:** [GNU GPL FAQ](https://www.gnu.org/licenses/gpl-faq.en.html) — combination, modification, and translation obligations.
- **MEDIUM:** [spells.bar repository](https://github.com/JollyGrin/sorcery-tcg-playtest) — useful UI behavior reference; no explicit root license was visible during this review.

## What Might Still Be Missing

- The official API/card-image terms and the precise precedence among printed text, updated text, FAQ, Codex, and judge rulings need a dedicated authority audit.
- Sorcery-specific timing and interaction clusters should each receive phase research before implementation; a general architecture document cannot safely enumerate every edge case.
- “Common deck” population data may not exist publicly. If no source exposes a documented sampling frame and popularity window, report a curated benchmark—not a measured metagame.
