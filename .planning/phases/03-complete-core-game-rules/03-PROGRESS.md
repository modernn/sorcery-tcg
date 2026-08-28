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
- Minion-provided elemental affinity participates in threshold checks and disappears when the provider leaves the realm.
- Positive Lethal damage destroys a minion regardless of defense; zero damage does not.
- A narrow Genesis effect draws a hidden site after summoning and loses on an empty Atlas.
- A narrow minion mana ability that requires readiness, taps for temporary mana, and expires at End Phase.
- Deathrite site draws resolve before simultaneous dead minions enter their cemeteries, including hidden draws and deck-empty loss.
- A narrow site Genesis effect grants temporary mana on entry, emits causal gain evidence, and expires through the existing End Phase reset.
- Numeric Deathrite healing resolves before cemetery entry, caps at printed Avatar life, and cannot change life at Death's Door.
- A minion that cannot move to Defend is excluded only when movement is required; stationary Defend and Intercept remain legal.
- A minion prohibited from Defend and Intercept is excluded from both response abilities while a directly attacked unit may still participate without using Defend.
- Movement +1 uses engine-issued explicit surface paths for both Move and Attack and Defend, including exact two-step and legal returning routes.
- A private-local actual-card adapter that verifies the normalized artifact and current Constructed format, builds legal 30/60 beginner decks under official rarity copy limits, and executes a deterministic real-card Charge and combat scenario without committing source data.
- A second real-card Earth ramp deck that proves Ghost Town's site-entry temporary mana, provided Earth affinity, Field Laborers' readiness-gated temporary mana, Zombie Horde as a five-mana payoff with restricted Defend, Land Surveyor's Genesis draw, and Kettletop Leprechaun's Deathrite draw.
- A third legal 30/60 Air teaching deck that proves Snallygaster's exact two-step Movement +1 paths and Roaming Monster's permission to summon onto an enemy site while ordinary minions remain restricted.
- A fourth legal 30/60 Water teaching deck that proves Muddy Pigs heals exactly 3 before entering its cemetery after a real simultaneous combat death.
- A fifth legal 30/60 Fire teaching deck that proves Monstrous Lion can Charge into an opposing unit but cannot target its site, then proves Lumbering Giant cannot use Defend or Intercept while ready and in range.
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
- `pnpm game:check-private`: 34-action combat, 38-action Earth ramp/Deathrite, 26-action Air movement, 25-action Air unrestricted-summon, 43-action Fire targeting/response, and 35-action Water healing matches exercise five concrete real-card teaching decks with byte-exact replay.
- `pnpm game:verify-private`: one passing ignored-authority integration scenario.
- Death's Door scenarios prove same-turn direct-damage immunity, later death blows, simultaneous-defeat draws, nonlethal site strikes, and exact replay.
- `pnpm verify`: 220 passing public tests at this checkpoint.

## Still required for Phase 3

- Rubble replacement, land/water regions, connection rules, control changes, and card-specific placement overrides.
- Full start/main/end phase triggers and duration cleanup beyond the supported narrow Genesis/Deathrite effects, minion damage, and summoning sickness.
- Additional costs, non-minion spells, movement beyond Movement +1, further activated abilities, and card-specific targets.
- Combat tiers, projectiles, healing, prevention/modification, additional card-triggered damage/death behavior, and tournament ending overlays.
- Source-linked scenario and invariant coverage for every supported core mechanic.

The runner is intentionally classified `unranked_partial_rules`; it proves the real engine loop and replay contract without claiming complete Sorcery behavior.
