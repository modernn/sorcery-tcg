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
- A deck-scoped, manifest-bound mechanical catalog that admits only Avatars, sites, and fully specified vanilla minions for this slice and rejects unsupported spell types before play.
- Mana payment, controlled-site affinity thresholds, Avatar spellcasting, public minion placement, unlimited shared-site occupancy, and end-of-turn summoning-sickness cleanup.
- Vanilla surface Move and Attack for Avatars and ready minions, including zero-step activation, one orthogonal site step, post-movement attack choice, and tap costs.
- Separate turn-owner and decision-seat tracking for non-active-player Defend and Intercept windows.
- Enemy-unit and enemy-site attack targets, any-number sequential defenders/interceptors, original-target retention/removal, and deterministic split-strike allocation.
- Simultaneous minion damage, persistent turn damage, End Phase damage cleanup, immediate lethal checks, and owner cemeteries.
- Avatar combat facts and life tracking, undefended-site life loss, Death's Door state, direct-damage immunity, death blows, Avatar defeat, and simultaneous-defeat draw state.
- A catalog-gated Avatar spell-draw ability with tap cost and opponent-safe hidden-card events.
- Charge bypasses summoning sickness for Move and Attack while ordinary summoned minions remain unavailable.
- A private-local actual-card adapter that verifies the normalized artifact and current Constructed format, builds two different legal 30/60 beginner decks under official rarity copy limits, and executes a deterministic real-card Charge and combat scenario without committing source data.
- Observer-safe actions, rejections, causal events, hashes, exact replay, and a deterministic unranked match runner.
- A no-dependency browser client that renders the authoritative 5x4 realm, scopes hidden information by seat, exposes only engine-issued actions, and verifies replay.

## Evidence

- `src/engine/game.ts`
- `src/commands/run-game-demo.ts`
- `src/prototype/game-server.ts`
- `tests/engine/game-setup.test.ts`
- `tests/engine/game-demo.test.ts`
- `tests/engine/game-server.test.ts`
- `pnpm game:demo -- 23`: completes in 138 accepted actions over 56 turns with byte-exact replay.
- `pnpm play`: serves the playable core at `http://127.0.0.1:4174/`.
- Browser checkpoint: a 22-action match moved a minion, handed Defend to the non-active seat, resolved a simultaneous trade into both cemeteries, and replayed byte-identically.
- `pnpm game:check-private`: a 27-action actual-card match immediately activates a real Charge minion, exercises the selected Avatar's spell draw, and resolves a real minion trade with byte-exact replay.
- `pnpm game:verify-private`: one passing ignored-authority integration scenario.
- Death's Door scenarios prove same-turn direct-damage immunity, later death blows, simultaneous-defeat draws, nonlethal site strikes, and exact replay.
- `pnpm verify`: 207 passing public tests at this checkpoint.

## Still required for Phase 3

- Rubble replacement, land/water regions, connection rules, control changes, and card-specific placement overrides.
- Full start/main/end phase triggers and duration cleanup beyond vanilla minion damage and summoning sickness.
- Additional costs, non-minion spells, card-provided movement, activated abilities, and card-specific targets.
- Combat tiers, projectiles, healing, prevention/modification, card-triggered damage/death behavior, and tournament ending overlays.
- Source-linked scenario and invariant coverage for every supported core mechanic.

The runner is intentionally classified `unranked_partial_rules`; it proves the real engine loop and replay contract without claiming complete Sorcery behavior.
