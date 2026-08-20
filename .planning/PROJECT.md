# Sorcery Simulator

## What This Is

Sorcery Simulator is a new TypeScript application for playing and simulating Sorcery: Contested Realm with an authoritative, deterministic rules engine. It lets automated and AI competitors play legal games, evaluates owned and commonly played online decks, supports deck optimization, and later provides a browser GUI for human play over the exact same engine.

## Core Value

Simulation results must be reproducible and rules-correct enough that deck and model comparisons are trustworthy.

## Requirements

### Validated

(None yet — ship to validate)

### Active

- [ ] Pin the official rules edition and card dataset used by every run.
- [ ] Implement a deterministic, headless TypeScript rules engine whose state can only change through validated legal actions.
- [ ] Track which rules, keywords, cards, and interactions are implemented and fail closed when a ranked simulation reaches unsupported behavior.
- [ ] Provide one legal-action interface shared by human UI clients, deterministic competitors, and AI model competitors.
- [ ] Import and validate the owner's 223-card collection and supplied decklists.
- [ ] Import representative competitive and commonly played decks from attributable online sources.
- [ ] Run reproducible, seat-alternated gauntlets that compare decks and AI competitors with useful statistical output.
- [ ] Support model adapters that record decisions, illegal-action rate, cost, latency, and outcomes without allowing models to mutate state directly.
- [ ] Recommend deck changes using collection constraints, matchup results, and held-out evaluation rather than a single matchup.
- [ ] Provide a browser GUI for human play after the rules engine and simulator are verified.
- [ ] Keep machine-readable event logs and replayable run manifests for every evaluated game.

### Out of Scope

- Human gameplay through a CLI — humans will use the browser GUI; CLI commands are for development and automation only.
- An LLM referee that directly decides authoritative game mutations — unreviewed model rulings make competitive results untrustworthy.
- Trading, marketplace, collection pricing, and card purchasing — unrelated to rules simulation and deck evaluation.
- Native mobile applications — the browser interface is the initial human client.
- Public multiplayer infrastructure and matchmaking — defer until local human play and simulation are correct.

## Context

- The workspace began empty; `simulator-handoff.zip` is a reviewed prototype and research artifact, not production source.
- The prototype demonstrates deck parsing, policy files, simple batch matches, model action selection, and reporting, but materially changes or omits rules and is not deterministic.
- The supplied collection contains 223 cards across 149 unique card names. All names resolved against the official card API during review.
- The user wants a whole new piece of software, with rules and simulation quality established before the GUI.
- Human play will not happen through a CLI.
- TypeScript is required; Python is not an acceptable implementation language.
- Existing TypeScript tools, especially Contested Realms and spells.bar if those are the previously researched tools, should be audited for reusable logic and behavior before implementation.
- Common online decks should be used as the competitive field for automated and AI-model testing.

## Constraints

- **Language**: TypeScript for the engine, simulator, agents, and GUI — user preference and potential reuse of existing TypeScript implementations.
- **Rules fidelity**: Official rules and card rulings are authoritative — simulator balance must never be tuned by changing a real rule.
- **Determinism**: A run manifest plus seed must reproduce byte-identical engine events for deterministic agents.
- **Action safety**: The engine enumerates legal actions and is the sole state owner — clients and models cannot submit arbitrary mutations.
- **Coverage honesty**: Ranked results require complete coverage of every exercised mechanic — unsupported behavior invalidates the game instead of becoming a silent no-op.
- **Licensing**: External source may be copied only when its license is compatible and attribution/derivative obligations are accepted; otherwise it is behavioral reference only.
- **Containers**: Use Podman rather than Docker when containers become necessary.
- **Delivery**: Commit small verified slices with tests throughout development.

## Key Decisions

| Decision | Rationale | Outcome |
|----------|-----------|---------|
| Build a new product instead of refactoring the prototype | Prototype baselines encode incorrect and incomplete game behavior | — Pending |
| Use TypeScript throughout the core product | User preference, shared contracts, and alignment with reference simulators | — Pending |
| Keep the engine headless and client-independent | Simulator, model agents, and GUI must use identical rules | — Pending |
| Build and verify rules before the GUI | Deck and model results are worthless without rules fidelity | — Pending |
| Use one enumerated legal-action API | Prevents agent-specific rules and illegal direct state changes | — Pending |
| Fail closed on unsupported mechanics | Silent approximations produce misleading deck rankings | — Pending |
| Use common online decks plus the owned collection | Evaluation needs both realistic opponents and actionable collection constraints | — Pending |
| No interactive human CLI | Human gameplay belongs in the browser GUI | — Pending |

## Evolution

This document evolves at phase transitions and milestone boundaries.

**After each phase transition**:
1. Requirements invalidated? Move to Out of Scope with reason.
2. Requirements validated? Move to Validated with phase reference.
3. New requirements emerged? Add to Active.
4. Decisions to log? Add to Key Decisions.
5. "What This Is" still accurate? Update if drifted.

**After each milestone**:
1. Review all sections.
2. Confirm the Core Value remains the right priority.
3. Audit Out of Scope reasons.
4. Update Context with current evidence.

---
*Last updated: 2026-08-20 after initialization*
