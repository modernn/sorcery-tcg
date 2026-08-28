# Phase 3: Complete Core Game Rules - Progress

## Implemented slice: setup through repeatable turns

- Manifest-bound deterministic Atlas and Spellbook shuffles with privileged draw evidence.
- Three-card Atlas and three-card Spellbook opening hands with opponent identity redaction.
- One fully engine-issued mulligan of up to three cards, preserving per-deck bottom order and hand size.
- Avatar placement at the middle square of each player's bottom row on the 5x4 realm.
- Manifest-selected first player, first-player opening draw skip, staged Atlas/Spellbook draw choice, and deck-out loss.
- Mandatory first-domain site placement at the Avatar's square, Avatar tapping, turn end, untap, mana from controlled sites, and repeatable turns.
- Observer-safe actions, rejections, causal events, hashes, exact replay, and a deterministic unranked match runner.

## Evidence

- `src/engine/game.ts`
- `src/commands/run-game-demo.ts`
- `tests/engine/game-setup.test.ts`
- `tests/engine/game-demo.test.ts`
- `pnpm game:demo -- 23`: completes in 114 accepted actions over 56 turns with byte-exact replay.
- `pnpm verify`: 189 passing tests at this checkpoint.

## Still required for Phase 3

- General site placement, void/region/adjacency/connection rules, and occupancy.
- Full start/main/end phase triggers and duration cleanup.
- Thresholds, costs, targets, summoning, movement, and activated abilities.
- Attack, defense, intercept, strikes, projectiles, damage, healing, death, Death's Door, and Avatar defeat.
- Source-linked scenario and invariant coverage for every supported core mechanic.

The runner is intentionally classified `unranked_partial_rules`; it proves the real engine loop and replay contract without claiming complete Sorcery behavior.
