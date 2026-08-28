# Phase 3: Complete Core Game Rules - Progress

## Implemented slice: setup through repeatable turns

- Manifest-bound deterministic Atlas and Spellbook shuffles with privileged draw evidence.
- Three-card Atlas and three-card Spellbook opening hands with opponent identity redaction.
- One fully engine-issued mulligan of up to three cards, preserving per-deck bottom order and hand size.
- Avatar placement at the middle square of each player's bottom row on the 5x4 realm.
- Manifest-selected first player, first-player opening draw skip, staged Atlas/Spellbook draw choice, and deck-out loss.
- Mandatory first-domain site placement at the Avatar's square, Avatar tapping, turn end, untap, mana from controlled sites, and repeatable turns.
- Ordinary site placement on unoccupied orthogonal cells bordering a controlled site, with controller-aware domains and immediate mana on entry.
- The Avatar's once-per-turn choice to play a site or privately draw one from the Atlas, including empty-Atlas defeat.
- Observer-safe actions, rejections, causal events, hashes, exact replay, and a deterministic unranked match runner.
- A no-dependency browser client that renders the authoritative 5x4 realm, scopes hidden information by seat, exposes only engine-issued actions, and verifies replay.

## Evidence

- `src/engine/game.ts`
- `src/commands/run-game-demo.ts`
- `src/prototype/game-server.ts`
- `tests/engine/game-setup.test.ts`
- `tests/engine/game-demo.test.ts`
- `tests/engine/game-server.test.ts`
- `pnpm game:demo -- 23`: completes in 132 accepted actions over 56 turns with byte-exact replay.
- `pnpm play`: serves the playable core at `http://127.0.0.1:4174/`.
- `pnpm verify`: 196 passing tests at this checkpoint.

## Still required for Phase 3

- Rubble replacement, land/water regions, connection rules, control changes, and card-specific placement overrides.
- Full start/main/end phase triggers and duration cleanup.
- Thresholds, costs, targets, summoning, movement, and activated abilities.
- Attack, defense, intercept, strikes, projectiles, damage, healing, death, Death's Door, and Avatar defeat.
- Source-linked scenario and invariant coverage for every supported core mechanic.

The runner is intentionally classified `unranked_partial_rules`; it proves the real engine loop and replay contract without claiming complete Sorcery behavior.
