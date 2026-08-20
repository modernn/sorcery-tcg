# Feature Landscape

**Project:** Sorcery Simulator  
**Domain:** Deterministic trading-card-game rules simulator, AI/model testbed, collection-constrained deck optimizer, and later browser client  
**Researched:** 2026-08-20  
**Overall confidence:** HIGH for current official format/rules sources; MEDIUM for product and evaluation recommendations

## Research Conclusion

The product's minimum credible unit is not “a game that reaches a winner.” It is a game whose rules/card sources, decks, pilot configuration, randomness, actions, and outcome can all be audited. A result is rankable only when every mechanic exercised by that game is implemented and verified. Everything else may be useful as a development sandbox, but must be visibly excluded from deck and model rankings.

The simulator should grow in verified vertical slices. Implement the complete base rules first, then add cards required by a small current gauntlet and the owner's decks. Expand the card pool from real deck demand. This is safer than shallowly implementing every card and is compatible with the long-term goal of full coverage.

Current official Constructed guidance is **1 Avatar, at least 60 Spellbook cards, at least 30 Atlas cards**, with rarity copy limits of Ordinary 4, Exceptional 3, Elite 2, and Unique 1. The format is a configurable, versioned input because official rules and card text do change. The December 2025 update changed the Spellbook requirement and added Ward/Collection glossary entries; the November 2025 card update changed official card text. A live API fetch is therefore not enough for reproducible research.

## Table Stakes

Features users need before simulator output can be trusted. Missing one does not merely reduce polish; it invalidates conclusions drawn from the runs.

| Feature | Why Expected | Complexity | Notes |
|---------|--------------|------------|-------|
| Pinned rules, format, and card snapshot | Results must mean the same thing after official updates | Medium | Store source URL, publication/effective date, fetched timestamp, raw snapshot, and content hash; never resolve live data during a run |
| Canonical card and deck identities | Names, printings, updated text, and quantities must resolve consistently | Medium | Normalize to stable internal IDs while preserving source names and printings for audit |
| Authoritative headless rules engine | Simulator, models, and future GUI must play the same game | High | Only validated actions can change state; clients never submit mutations |
| Complete core rules | Sorcery's grid, regions, timing, reactions, and combat materially affect deck strength | High | Include setup/mulligan, phases, mana/threshold, sites, movement, attack, defend, intercept, projectiles, Storyline, zones, death, Death's Door, and win/loss rules |
| Versioned card/ruling implementations | Card text and official FAQ interactions determine legal behavior | High | Human-reviewed implementations and source-linked conformance cases; no runtime natural-language adjudication |
| Enumerated legal-action interface | Humans and agents need the same choices, targets, paths, and reactions | High | Actions need stable IDs and typed parameters; rejected actions leave state unchanged |
| Rules/card coverage gate | Partial support must never masquerade as ranked evidence | Medium | Static deck preflight plus runtime mechanic tracking; unsupported behavior invalidates the game |
| Deterministic randomness | Same experiment must be rerunnable | Medium | Explicit seeded RNG owned by the engine; no global/random platform source in ranked execution |
| Typed event log and replay | Every decision and mutation needs an audit trail | Medium | Manifest + initial state + actions/events reproduce byte-identical events for deterministic agents |
| Deck and collection import/validation | The optimizer must use the owner's actual 223 cards | Medium | Report exact parse errors, unknown names, shortages, legality violations, and normalized totals |
| Attributable online-deck ingestion | “Common deck” must be evidence, not a hand-picked label | Medium | Freeze the source artifact and provenance; separate event results from community popularity |
| Versioned pilot/strategy artifact | A decklist alone does not define competent play | Medium | Record agent, policy/prompt, model settings, and version beside the deck hash |
| Batch runner and fair gauntlet scheduler | Single games and unbalanced seats are misleading | Medium | Balanced seats, predeclared seeds, fixed field/weights, fixed draw/timeout policy, no discarded trials |
| Statistical and diagnostic report | A win percentage without sample size or uncertainty is not useful | Medium | Raw W/D/L, intervals, matchup matrix, seat effect, invalid games, game length, and coverage |
| Model adapter and telemetry | New models must be comparable without being trusted with state | Medium | Observation-only input; action-ID output; record invalids, failures, tokens, cost, and latency |
| Collection-constrained deck search | Recommendations must be buildable from cards actually owned | High | Enforce inventory and legality during generation, not after; score across a gauntlet and locked holdout |
| Browser human-play client | Human play eventually needs a usable spatial interface | High | Build only after engine verification; consume the same observation/legal-action/event contracts |

## Required Acceptance Behavior

These behaviors are the practical definition of each feature, and should become automated acceptance tests or run gates.

| Area | Measurable acceptance behavior |
|------|--------------------------------|
| Source pinning | Every run manifest contains engine version, rules/format IDs, rulebook and FAQ/Codex source URLs, card-snapshot hash, deck hashes, pilot hashes, seed schedule, and runner options. A ranked run refuses an unpinned or missing artifact. |
| Format legality | Under the current Constructed profile, validation applies 1 Avatar, Spellbook minimum 60, Atlas minimum 30, and 4/3/2/1 rarity copy limits. The profile itself is versioned rather than hard-coded as timeless truth. |
| State ownership | Submitting a stale, forged, malformed, or non-enumerated action produces a rejection event and the pre/post state hashes are identical. |
| Hidden information | A seat observation contains its own private zones and only public/count information for the opponent. Contract tests prove opponent hand contents and deck order never reach a human/model adapter. |
| Determinism | Two fresh Node processes using the same manifest, decks, deterministic agents, and seeds produce byte-identical normalized event JSONL and identical final-state hashes. Worker count and scheduling do not alter output. |
| Replay | Replaying a recorded action stream reaches every recorded state hash. The verifier stops at and identifies the first divergent event if code/data changes. |
| Coverage preflight | Before a ranked batch, each deck reports every required card, keyword, rule module, and explicit ruling as `verified`, `implemented-unverified`, or `unsupported`; only `verified` dependencies are eligible. |
| Coverage runtime | Every executed mechanic/ruling emits its capability ID. Any unregistered or unsupported branch immediately marks the game `invalid_unsupported`, records the state/action/capability, and excludes that game from rankings without deleting it. |
| Coverage reporting | Reports show the exact required and exercised capability sets and excluded-game count. They do not claim a misleading global “percent of all possible interactions.” |
| Rules verification | Each supported rule/card behavior has at least one deterministic test linked to a rulebook, Codex, FAQ, or documented project ruling. Multi-card fixes include a regression fixture for the interaction that exposed them. |
| Collection import | Importing the supplied file yields 223 total cards and 149 canonical names, or fails with line-specific discrepancies. Reimport is idempotent. No generated deck uses more copies than the normalized owned quantity. |
| Deck validation | Unknown/ambiguous card names, wrong zones, illegal counts, and unsupported cards are separate errors. A deck can be legal but simulation-ineligible, and the UI/report says which. |
| Online deck provenance | Each imported revision stores source URL/ID, title, author/pilot when published, event, placement/record when published, event date, fetch time, raw-source hash, normalized deck hash, and rules/card snapshot used to validate it. |
| Immutable online decks | A gauntlet reads the frozen imported revision, never the live URL. If the source changes, reimport creates a new revision and diff instead of mutating past evidence. |
| Gauntlet fairness | Every deck-agent-opponent cell has an even game count split exactly by seat. The field, opponent weights, seeds, maximum-turn/draw policy, and all failures are fixed before the batch and included in the manifest. |
| Deck comparison | Candidate and baseline use the same field, pilot class, seat schedule, and seed schedule. Reports include per-opponent and equal-opponent macro results so one numerous/easy matchup cannot dominate. |
| Model comparison | Models receive the same observation schema, legal-action encoding/order, deck/policy revisions, opponents, seats, seeds, sampling settings, and time limits. Provider nondeterminism is acknowledged; raw responses and returned model IDs make the run auditable, not falsely deterministic. |
| Model failures | Timeout, provider error, malformed response, or illegal choice becomes one configured engine-safe failure action and is counted in outcomes. Failed calls are never silently retried until a favorable action appears. |
| Statistical output | Show sample size and raw W/D/L for every cell; report score only with its stated draw value. Provide 95% uncertainty for per-matchup and aggregate score, seat-one effect, invalid/error rate, and a machine-readable result table. |
| Precision | Batch configuration declares a target interval half-width and a maximum game budget. If the budget is reached first, the report labels the comparison inconclusive instead of asserting a winner. |
| Optimizer legality | Every proposed intermediate and final candidate satisfies the chosen format, card support gate, and exact inventory quantities. An invalid starting list produces a shortage/legality report rather than being “fixed” silently. |
| Optimizer holdout | Search uses only training seeds/field partition. The selected candidate is evaluated once on locked holdout seeds (and, when the field is large enough, held-out deck revisions); holdout results do not feed the same search run. |
| Recommendation evidence | A recommendation contains the exact card diff, baseline and candidate hashes, training and holdout matchup matrices with intervals, worst matchup, collection proof, and replay links for representative wins/losses. It may conclude that no improvement is distinguishable. |
| Browser client | The client renders the 5x4 realm and relevant regions/zones, highlights only engine-returned actions/targets/paths/reactions, supports forced choices and Storyline timing, preserves private information, and never duplicates legality logic. |
| Human accessibility | All actionable cards/locations have keyboard-operable equivalents, visible focus, non-color-only state cues, readable card detail, and text descriptions of action/rejection events. Human play is not routed through an interactive CLI. |

## Rules and Card Coverage Product

Coverage is a first-class feature, not a README checklist.

### Coverage model

Use a registry with four honest states:

- `verified`: implementation exists and its cited conformance tests pass.
- `implemented-unverified`: code exists but it is not eligible for ranked use.
- `unsupported`: known missing behavior; preflight blocks it or runtime invalidates it.
- `not-applicable`: capability does not apply to this pinned rules/card revision.

Track at least:

- core rule sections and timing rules;
- keywords and regions;
- card implementations and updated/oracle text revisions;
- Avatar abilities;
- triggered, activated, replacement, and continuous effects;
- explicit FAQ/Codex rulings;
- known multi-card interaction fixtures.

Static analysis can prove that named cards and declared mechanics are present. It cannot prove every emergent interaction. Runtime capability events and interaction regression fixtures close that gap incrementally. When the Golden Rule or an unencoded official interaction is required, the deterministic engine should stop with an unsupported diagnostic. A human-reviewed ruling can then be added with a source and test; an LLM must not invent the result during the game.

### Coverage report views

The report should answer:

1. Can this deck start a ranked game under this snapshot?
2. Which dependencies prevent it?
3. Which capabilities did completed games actually exercise?
4. Which runtime state first reached an unsupported interaction?
5. Which current collection and competitive-field decks become eligible if one missing capability is implemented?

That fifth view is especially useful for roadmap ordering: implement the small capability set that unlocks the most valuable real decks.

## Common Online Deck Field and Provenance

### Recommended source policy

Use this order:

1. **Official tournament report linked to the exact public deck revision.** Highest-value competitive evidence.
2. **Organizer results plus player-authored public deck page.** Preserve both links and note identity matching confidence.
3. **Player-authored deck with a primer or recorded result.** Useful but do not promote an unverifiable result to fact.
4. **Popular public deck without results.** Label `community-popular`, not competitive-proven.
5. **Official preconstructed deck.** Use as rules/tutorial regression data, not as evidence of the constructed metagame.

Official 2026 Washington DC reporting is a strong seed source because it links all Top 8 Curiosa lists and also gives field representation (75 players, 24 Avatars, with Necromancer most played). The official Las Vegas report adds a different event and Top 8 mix. The 2025 Gen Con champion article is unusually valuable because it links the exact list and includes the pilot's strategy, essential cards, and weaknesses. Older lists remain useful only when run under their historical rules/card snapshot or revalidated as current.

### Field construction

Do not define “common” as the first N search results or Curiosa likes. Build and freeze a field release with:

- current Top 8 lists from at least two attributable major events when available;
- at least one representative of popular archetypes that did not convert to Top 8, when official field-share evidence exists;
- geographic/event diversity so one local metagame does not become the whole benchmark;
- archetype, Avatar, element, and game-plan labels that are human-reviewed;
- near-duplicate detection by normalized deck distance;
- a documented equal-weight field and, optionally, a separately reported evidence-weighted field.

Never overwrite a field release. A new rules/card update or event produces a new release. Results from different releases may be compared descriptively, but should not be merged into one leaderboard without stratification.

### Deck versus pilot confounding

Every online deck needs a pilot configuration. For deck strength testing, freeze the same agent class/version across candidates, while allowing a source-linked deck strategy artifact where needed. For model strength testing, freeze both deck and strategy/prompt across models. Report `deck × pilot × matchup`; do not collapse all three into a vague global score.

## Fair Gauntlet Design

Use two evaluation layers:

### Deck gauntlet

- Candidate versus every frozen field deck.
- Equal trials per opponent and exact seat alternation.
- Same deterministic pilot implementation and run configuration for candidate/baseline comparisons.
- Per-opponent results plus equal-opponent macro-average; evidence-weighted average is secondary and separately labeled.
- Raw draws and invalid games remain visible.

### Model gauntlet

- Each model pilots the same suite of common decks against fixed deterministic benchmark agents first; this isolates model capability more cleanly than only running a model-vs-model tournament.
- Then run symmetric model head-to-head cells as a secondary test, with each model piloting each assigned deck in both seats.
- Stratify by deck, opponent archetype, seat, and action-space size. A model may be good with aggro and poor with interaction-heavy control; the aggregate must not hide this.
- Keep prompt/policy content, legal-action ordering, timeouts, tool schema, and model sampling settings identical.

For deterministic deck experiments, run until a configured confidence target or cap. Model calls are costlier and provider behavior is nondeterministic, so model reports should use a predeclared budget and may legitimately remain exploratory.

## Model Comparison Metrics

### Outcome metrics

| Metric | Why it matters |
|--------|----------------|
| W/D/L and score by deck/matchup/seat | Primary evidence of play effectiveness without hiding draws |
| 95% interval and sample size | Prevents small-sample rankings from looking definitive |
| Seat-one effect | Detects rules, scheduling, and tempo bias |
| Game length distribution | Distinguishes fast wins, stalls, turn caps, and policy pathologies |
| Invalid/unsupported rate | Prevents a model from benefiting from games the engine could not judge |

### Decision-quality and operational metrics

| Metric | Why it matters |
|--------|----------------|
| Legal-choice rate | Direct measure of schema and rules-following reliability |
| Malformed-response, timeout, provider-error, and fallback rates | Separates strategic failure from adapter/API failure |
| Decisions per game and actions per turn | Context for both outcome and cost |
| Input/output tokens and estimated cost per completed game | Makes model tradeoffs concrete |
| Latency p50/p95 per decision and per game | Average latency hides poor interactive tails |
| Outcome by legal-action count/decision type | Reveals models that fail when targets, paths, or reactions become complex |
| Deterministic-policy agreement (diagnostic only) | Useful for regression, but must not be called optimality |

Do not create one opaque “AI score.” Present outcome, reliability, latency, and cost as a Pareto tradeoff. Elo/Glicko can be added after there is a sufficiently connected repeated field, but it is not a substitute for matchup cells or uncertainty.

## Statistical Reporting Contract

Every human-readable report must have a machine-readable sibling and include:

- manifest ID and all artifact hashes;
- total scheduled, completed, drawn, invalid, unsupported, and infrastructure-failed games;
- raw W/D/L by matchup, deck, pilot/model, and seat;
- score convention (for example, win 1 / draw 0.5 / loss 0) stated explicitly;
- stratified 95% bootstrap intervals for aggregate score and candidate-minus-baseline difference;
- per-matchup intervals and equal-opponent macro-average;
- seat effect with interval;
- mean, median, p90, and maximum game turns/actions;
- coverage dependencies and exercised capability counts;
- model token, cost, latency, invalid-action, timeout, and fallback metrics;
- the configured precision target, game cap, and `conclusive`/`inconclusive` label.

Do not discard draws, turn-cap games, provider failures, or unsupported games. Unsupported games are excluded from the performance denominator but remain a separately reported invalid outcome. Model failures governed by the predeclared failure action remain part of model performance.

## Collection-Constrained Deck Search

### Required behavior

The optimizer should search legal local mutations of a valid starting deck, not attempt an exhaustive solution to the entire card game.

- Candidate pool is the intersection of owned cards, format-legal cards, and `verified` simulator cards.
- Avatar, Atlas, and Spellbook are separate zones with their own requirements.
- Quantities are consumed exactly; the same physical copy cannot satisfy two slots in one deck.
- Each mutation produces a normalized deck hash and exact add/remove diff.
- Search objective is explicit: default to equal-opponent macro score, then worst-matchup score as a tie-breaker; show both instead of claiming universal “best deck.”
- Search and selection use a frozen training schedule. Only the final short list reaches locked holdout evaluation.
- A recommendation is issued only when the holdout evidence supports it; otherwise report the candidates as indistinguishable.
- Preserve replay examples for failure analysis, especially systematic losses.

### Useful diagnostics, not ranking shortcuts

Goldfish metrics such as early castability, threshold access, mulligan rate, dead turns, and curve distribution help prune obviously inconsistent candidates. They cannot replace interactive gauntlet games because Sorcery's spatial combat, sites, reactions, and card interactions determine outcomes.

## Human Browser GUI Needs (Later Phase)

The first human client should target local human-versus-agent play and replay inspection. Public matchmaking, accounts, and network synchronization are explicitly later concerns.

### Required play surface

- 5x4 realm with unambiguous ownership, coordinates, adjacency, and surface/subsurface/void state.
- Own hand, Atlas, Spellbook, cemetery, banished zone, carried cards, life, mana, thresholds, statuses, and opponent public zone counts.
- Engine-supplied highlights for legal cards, destinations, paths, targets, defenders/interceptors, optional triggers, and forced choices.
- Visible current phase, active seat, Storyline items, pending reaction/choice, and action history.
- Card detail that shows the pinned effective rules text and source-linked rulings.
- Clear rejection/error explanation without exposing hidden state.
- Match setup for deck revisions, opponent agent/model, rules snapshot, and seed; ranked eligibility shown before start.
- Replay viewer with step, jump, public/full-debug visibility modes, and state-hash verification.
- Keyboard-operable actions and non-color-only visual cues from the first GUI slice.

### Boundary

The browser maps user gestures to existing action IDs. It must not calculate mana, movement, targeting, combat, or card effects independently. If a UI feature requires a new legality rule, add it to the engine contract first.

## Differentiators

Features that make this more useful than a generic browser playmat or a prototype batch loop.

| Feature | Value Proposition | Complexity | Notes |
|---------|-------------------|------------|-------|
| Rank-eligibility proof | Users can tell exactly why a result is or is not trustworthy | Medium | Coverage, source, legality, and replay gates combined |
| Capability-unlock roadmap | Shows which missing rule/card unlocks the most owned/current decks | Medium | Turns coverage into actionable implementation order |
| Reproducible competitive-field releases | “Common decks” remain attributable and stable as the metagame moves | Medium | Freeze official event lists, field evidence, and normalized revisions |
| Deck × pilot × model analysis | Separates deck quality, strategy quality, and model quality | High | Essential for honest AI claims |
| Auditable model decisions | Replay any surprising action with exact observation, legal set, raw response, latency, tokens, and cost | Medium | Never exposes hidden authoritative state |
| Owned-card robust optimizer | Produces a deck the user can actually assemble and validates it on unseen trials | High | Exact inventory proof and held-out gauntlet |
| Explainable recommendations | Exact swaps, matchup deltas, uncertainty, and representative replays | Medium | “No confident improvement” is a valid result |
| Source-linked conformance corpus | Rules/card fixes remain anchored to official examples | High | Especially valuable for Storyline and multi-card interactions |
| Same-engine human replay/play | Human inspection catches bad simulations without creating a second rules path | High | GUI consumes the headless contracts |

## Anti-Features

Features to explicitly not build, because each weakens trust or distracts from the core value.

| Anti-Feature | Why Avoid | What to Do Instead |
|--------------|-----------|-------------------|
| LLM referee with authoritative mutations | Nondeterministic, unreviewed rulings contaminate results | Fail closed; add a sourced human-reviewed implementation and regression test |
| Silent no-op or approximate card effects | Produces plausible but false deck rankings | Mark unsupported and invalidate the game |
| Runtime use of live rules/cards/deck URLs | Past runs cannot be reproduced after edits or upstream changes | Snapshot, hash, and revision every artifact before execution |
| Client-side legality or direct state editing | Creates different games for UI, scripts, and models | Use one engine-owned legal-action/application path |
| Popularity-by-search/likes as “the meta” | Search ranking and likes do not establish tournament prevalence | Use attributable event lists and field-share evidence; label community decks honestly |
| Top-8-only gauntlet | Selection bias omits common decks that underperformed one event | Add field-share representatives and multiple events/regions |
| Single-matchup or aggregate-only deck score | Hides hard counters and rewards field-weight accidents | Report the full matrix, equal-opponent macro score, and worst matchup |
| Same-data optimization and evaluation | Overfits seeds and opponents, exaggerating improvements | Lock a holdout and allow an inconclusive result |
| Elo-only model leaderboard | A scalar hides deck, matchup, seat, failure, cost, and uncertainty | Use stratified cells and a Pareto view; add ratings only later |
| Retrying bad model calls until valid | Erases a real reliability failure and biases outcomes | Apply one predeclared failure rule and count it |
| Claiming model-run determinism | Hosted models can change or remain stochastic despite fixed inputs | Preserve full audit artifacts and repeat samples |
| Interactive human CLI | Spatial play is a poor CLI fit and the user rejected it | Keep CLI for automation; use the browser for humans |
| GUI before verified engine slices | Polished interaction would conceal incorrect rules | Build replay/debug views only after the corresponding contracts are verified |
| Runtime card-text interpreter/code generator | Text is ambiguous and prompt-like; generated behavior is not a ruling | Use code generation only as an offline draft requiring review/tests, if ever |
| Perfect-play solver or self-training platform in the first product | Speculative and far beyond what trustworthy deck testing requires | Start with deterministic baselines and pluggable model competitors |
| Marketplace, prices, purchasing, public matchmaking, or native apps | Unrelated to proving rules and deck/model quality | Keep them out of this roadmap |

## Feature Dependencies

```text
Official rules / format / FAQ / card sources
                    |
                    v
       Versioned snapshots + canonical IDs
              |                 |
              v                 v
      Rules/card registry   Deck/collection validation
              |                 |
              +--------+--------+
                       v
       legalActions(state) + applyAction(action)
                       |
             +---------+----------+
             v                    v
      Typed events/state hash   Coverage gate
             |                    |
             +---------+----------+
                       v
          Deterministic runner + replay
              |        |         |
              v        v         v
      Baseline agent  Model adapters  Browser client (later)
              \        /                 |
               v      v                  v
             Fair gauntlet          Human play/replay
                    |
          +---------+----------+
          v                    v
  Statistical reports   Collection-constrained search
                                 |
                                 v
                     Held-out recommendation report

Online source adapters -> frozen deck revisions -> curated field release -> fair gauntlet
```

Rules/card snapshots and deterministic state transitions are the dependency root. Deck ingestion can begin in parallel, but ranked gauntlets cannot precede the coverage gate. The optimizer depends on a trustworthy gauntlet, not merely deck parsing. The GUI depends on the observation/action/event contracts, not on the optimizer.

## MVP Recommendation

Prioritize the first **rankable simulator slice**, not the GUI and not superficial whole-card-pool coverage:

1. Pin the current official rules/format/card/FAQ sources; import the owner's collection and supplied decks with exact legality/inventory diagnostics.
2. Implement and verify the complete core rules kernel, legal-action interface, event log, state hashes, deterministic replay, and coverage registry using synthetic rule fixtures.
3. Implement a complete card/ruling slice for two legal owned decks and a small frozen current field spanning aggressive, midrange/spatial, and control strategies. A deck is admitted only when its entire dependency set is verified.
4. Add one deterministic baseline pilot, fair seat-balanced batch scheduling, and the full statistical reporting contract.
5. Add one model adapter and evaluate multiple models over the exact same common-deck schedule with reliability/cost/latency telemetry.
6. Add collection-constrained local deck mutations and locked holdout evaluation; allow “no proven improvement.”
7. Expand eligible current competitive decks and owned cards by coverage-unlock value, then build the browser play/replay UI over the stable engine contracts.

**Defer:** public multiplayer, accounts, marketplace/prices, mobile apps, Elo ladders, self-training, exhaustive search, and runtime AI rulings. Add them only after they serve a measured need and cannot compromise ranked evidence.

## Research Gaps and Phase Flags

- **Deck ingestion API:** Curiosa public pages and multiple community tools support URL imports, but this research did not find a documented stable official deck API contract. Treat URL ingestion as a replaceable source adapter, preserve raw responses, and research the actual endpoint/terms during that phase. **Confidence: MEDIUM.**
- **Tournament policy details:** The public Constructed page supplies base deck rules, but event-specific sideboards, bans, and policies may differ. Each field release needs its event policy artifact rather than assuming one universal profile. **Confidence: HIGH.**
- **Golden Rule coverage:** Official rules intentionally include judgment-heavy interactions. A deterministic ranked engine needs an explicit project ruling registry grounded in the Codex/FAQ; phase-specific rules research will be substantial. **Confidence: HIGH.**
- **Full card interaction space:** No finite percentage can prove completeness. Eligibility can be proven for declared dependencies and exercised paths, while interaction regressions accumulate from sourced cases. **Confidence: HIGH.**
- **Model sample size:** Costs may prevent tight intervals. The system must expose precision/budget tradeoffs and label exploratory results. **Confidence: HIGH.**
- **Reference code reuse:** Contested Realms exposes rules, bot, deck, collection, and test behavior but is GPL-3.0. spells.bar is valuable as a browser interaction reference, but no clear reuse license was visible on the repository page reviewed. Audit licensing before copying; behavioral study is safer meanwhile. **Confidence: HIGH for Contested Realms license, MEDIUM for spells.bar reuse status.**

## Sources

### Official / primary — HIGH confidence

- [Sorcery Constructed Format](https://sorcerytcg.com/constructed) — current public deck construction, copy limits, life, grid, and victory outline; accessed 2026-08-20.
- [December 2025 Rulebook Update](https://sorcerytcg.com/news/sorcery-contested-realm-december-2025-rulebook-update) — current linked rulebook, 60-card Spellbook update, Ward/Collection glossary additions; published 2025-12-19.
- [How to Play](https://sorcerytcg.com/how-to-play) — official rulebook/FAQ routing and role of contextual card FAQs; accessed 2026-08-20.
- [Official Card API](https://api.sorcerytcg.com/api/cards) — current card metadata, updated rules text, sets, and variants; accessed 2026-08-20.
- [Curiosa FAQ](https://curiosa.io/faqs) — official card-specific and interaction rulings, including Ward, regions, Intercept, and Storyline cases; accessed 2026-08-20.
- [Sorcery Card Updates 2025](https://sorcerytcg.com/news/sorcery-contested-realm-card-updates-2025) — evidence that official card behavior changes and Curiosa carries updated text; published 2025-11-25.
- [Washington DC Grand Contest recap](https://sorcerytcg.com/news/where-tables-connected-scg-con-washington-dc-recap) — official Top 8 links and field/avatar representation; published 2026-06-01.
- [Las Vegas Grand Contest recap](https://sorcerytcg.com/news/scg-con-las-vegas-the-second-grand-contest-of-2026) — official second-event Top 8 field seed; published 2026-06-29.
- [Gen Con 2025 champion deck breakdown](https://sorcerytcg.com/news/gen-con-2025-crossroads-champion-deck-breakdown) — official event result, exact Curiosa deck link, and pilot-authored strategy/weaknesses; published 2025-08-08.
- [Curiosa official preconstructed lists](https://curiosa.io/precons) — regression/tutorial deck sources; accessed 2026-08-20.
- [Heavier than a Duck — Curiosa deck](https://curiosa.io/decks/cmm45q5mu00dh04l7mzhl477o) and [Gen Con Hot Springs — Curiosa deck](https://curiosa.io/decks/cmbjh1lx5000hl70448ozy6ee) — examples of exact player-authored deck revisions linked from official reports; accessed 2026-08-20.

### Reference implementations — source facts HIGH, suitability requires audit

- [realms-cards/contested-realms](https://github.com/realms-cards/contested-realms) — TypeScript rules, bots, tests, deck import/export, and collection behavior; repository declares GPL-3.0.
- [JollyGrin/sorcery-tcg-playtest](https://github.com/JollyGrin/sorcery-tcg-playtest) — spells.bar browser grid/playtest interaction reference; no clear license was visible in the reviewed repository page.

### Recommendation confidence

| Area | Confidence | Basis |
|------|------------|-------|
| Rules/format/card versioning | HIGH | Current official rules, format, API, FAQ, and update announcements |
| Common competitive deck sourcing | HIGH | Current official event reports link exact deck pages and expose some field context |
| Coverage and deterministic replay features | HIGH | Directly required by project trust constraints and handoff failures |
| Fair gauntlet/statistical contract | MEDIUM | Established experimental design applied to this product; thresholds remain configurable |
| Model evaluation metrics | MEDIUM | Observable adapter/outcome metrics; provider cost and determinism vary |
| Collection-constrained optimizer behavior | MEDIUM | Sound evaluation design; search performance must be measured after engine throughput exists |
| Browser GUI needs | MEDIUM | Sorcery rules and existing clients establish interaction needs; usability still requires testing |
