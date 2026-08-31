---
name: sorcery-master
description: "Play, evaluate, and improve an assigned Sorcery: Contested Realm deck through the repository's authoritative Rust engine. Use for deck piloting, matchup simulation, legal line search, cost-versus-power analysis, or replay-verified policy improvement."
---

# Sorcery Master

Win only through engine-issued legal actions. Treat the Rust engine as authority and never infer legality from card names, deck-site prose, or this skill.

## Authority order

1. Use the ignored local authority revision when available.
2. Use official rules and card rulings bound into that revision.
3. Use `data/rules/catalog.json` as the human-reviewable map from plain-language rule slices to direct scenario proofs.
4. Treat websites, deck lists, prices, and external simulators as non-authoritative evidence.

Stop and mark a ranked result invalid when an exercised mechanic is unsupported. Never substitute a no-op or a guessed ruling.

## Assigned-deck workflow

1. Import without silently changing card names or counts.
2. Resolve every row to one stable card identity; report unresolved or ambiguous rows.
3. Validate format, copy limits, rule coverage, and the exact authority revision; canonicalize the list and hash its exact contents.
4. Bind the policy snapshot to the authority hash, deck-content hash, deck ID, and Rust engine version.
5. Simulate both seats over fixed seeds against the requested opponent field.
6. Search only from compact Rust checkpoints using engine-issued actions.
7. Replay selected lines through the authoritative session and verify receipts, events, hashes, and terminal state.
8. Report wins, draws, invalid games, matchup evidence, and exact reproducible inputs separately.

Once bound, pilot the exact assigned list. Treat substitutions as separate deck-building candidates and never silently feed them back into the assigned deck or its policy.

For budget analysis, keep price observations timestamped and sourced, including market, currency, printing/condition, and excluded costs such as shipping. Never treat a missing or ambiguous price as zero. A deck is Pareto-dominated only when another measured deck is no worse in both power and acquisition cost and strictly better in at least one; do not collapse the two dimensions without user-supplied weights.

## Metagame and price intake

When the user requests deck-site or marketplace intake:

- Collect only public facts the site permits collecting; honor access controls, terms, rate limits, and robots directives. Never bypass a login, paywall, or anti-bot control.
- Record source URL, retrieval time, event/result context, parsed deck-content hash, and price-observation fields. Label popularity, placement, and claimed win rate as external evidence, never rules authority.
- Resolve cards through the selected authority revision. Exclude unresolved lists and incomplete prices from ranked comparisons instead of guessing.
- Keep page bodies, caches, assigned lists, and detailed price observations ignored under `.local/authority/`; never acquire artwork or commit redistributed source content. Publish only provenance-safe aggregates when allowed.

## Self-improvement contract

Improve the deck-bound deterministic policy, never the rules:

- Generate the smallest neighboring policies: one adjacent feature-priority swap or `atlasReserve` plus/minus one.
- Select a nominee on fixed training seeds using integer half-points and canonical policy ID as the final tie-break.
- Evaluate champion and nominee on disjoint held-out seeds, with every matchup seat-swapped.
- Promote only a strict total improvement with no opponent-subgroup regression.
- Require every counted game to terminate within its bound, remain ranked-eligible, and pass byte-identical authoritative replay.
- Persist a canonical evaluation manifest containing the deck, policy, authority, engine, opponent-field, training-seed, and held-out-seed hashes so every promotion decision is reproducible.
- Store the promoted snapshot immutably with its parent policy ID and generation. Never mutate a policy during a game.

Keep assigned decks, private card facts, learned policies, and detailed rollouts under the ignored `.local/authority/` boundary. Commit only synthetic/public proofs and aggregate measurements that reveal no private source data.

## Learning discipline

Turn a discovered tactic into a generic policy feature only after a direct scenario proves it and held-out self-play promotes it. Prefer deleting a losing heuristic over adding overlapping exceptions. Default to compact in-memory indexed data; do not add a database, embeddings, a neural model, or card-name branches unless measurement demonstrates that representation is the bottleneck or materially limits policy quality.
