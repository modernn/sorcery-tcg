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
- A sibling Genesis effect draws a hidden spell after summoning and loses on an empty Spellbook, preserving summon-before-draw event order.
- Leyline Henge draws one hidden spell for each other orthogonally adjacent copy on entry, emits one source-linked event per successful draw, and loses after any required draw reaches an empty Spellbook.
- A narrow minion mana ability that requires readiness, taps for temporary mana, and expires at End Phase.
- Deathrite site draws resolve before simultaneous dead minions enter their cemeteries, including hidden draws and deck-empty loss.
- A narrow site Genesis effect grants temporary mana on entry, emits causal gain evidence, and expires through the existing End Phase reset.
- Numeric Deathrite healing resolves before cemetery entry, caps at printed Avatar life, and cannot change life at Death's Door.
- A minion that cannot move to Defend is excluded only when movement is required; stationary Defend and Intercept remain legal.
- A minion prohibited from Defend and Intercept is excluded from both response abilities while a directly attacked unit may still participate without using Defend.
- Movement +1 and +2 use engine-issued explicit surface paths for both Move and Attack and Defend, including exact bounded paths and legal returning routes without repeating a directed step; sideways-only basic movement excludes forward, backward, and diagonal steps from both actions.
- Card-granted top/bottom realm connection adds exact wraparound steps for Move and Attack and Defend without changing ordinary unit or site adjacency.
- Submerge minions can be summoned underwater at Water sites, move between surface and underwater at a Water site, and swim between adjacent Water sites; attacks, Defend, Intercept, and projectiles respect exact regions while sites remain surface targets.
- Burrowing minions can be summoned underground at land sites, move between surface and underground at a land site, and travel between adjacent land sites; units with both Burrowing and Submerge cross directly between adjacent underground land and underwater Water locations.
- Voidwalk minions can be summoned to any empty void, move between adjacent voids and adjacent site surfaces, and cross directly between void and eligible subsurface regions; playing a site into an occupied void places its units on the new surface without moving them.
- A card-specific outer-column casting restriction filters every surface, subsurface, and Voidwalk location offered by the shared hand-cast action without restricting later movement.
- Ranged issues cardinal one-step projectile paths, stops at the first occupied location, lets the controller choose among multiple hit units, never hits sites, and resolves a tapped one-way strike through the normal damage and death pipeline.
- Minion Ward enters with one public mark, prevents one complete positive damage event through the shared combat/projectile path, then breaks before later damage resolves normally.
- Attacking-only first strike resolves its attacker's allocations, deaths, Deathrites, and terminal results in an early window; only surviving defenders then make their normal return strikes.
- Airborne moves diagonally while remaining on the surface, can attack other Airborne units, cannot be attacked by ground units, and can be intercepted only by Airborne or Ranged units.
- Minion Stealth enters with a public mark, blocks opponent attacks and projectile hits, cannot be intercepted, skips Defend when it attacks, and is lost after the minion interacts with the realm.
- End-turn Stealth starts unmarked, gains its public mark before the controller's turn-end event, and then uses the same targeting restrictions.
- A private-local actual-card adapter that verifies the normalized artifact and current Constructed format, builds legal 30/60 beginner decks under official rarity copy limits, and executes a deterministic real-card Charge and combat scenario without committing source data.
- A second real-card Earth ramp deck that proves Ghost Town's site-entry temporary mana, provided Earth affinity, Field Laborers' readiness-gated temporary mana, Zombie Horde as a five-mana payoff with restricted Defend, Land Surveyor's Genesis draw, and Kettletop Leprechaun's Deathrite draw; short replays of the same deck prove Belmotte Longbowmen's one-step Ranged strike, Holy Warrior's one-use Ward against two consecutive shots, and Albespine Pikemen killing Bosk Troll before its return strike.
- A third legal 30/60 Air teaching deck that proves Snallygaster's exact two-step Movement +1 paths and Roaming Monster's permission to summon onto an enemy site while ordinary minions remain restricted; variants prove Plumed Pegasus's diagonal flight and asymmetric attack/Intercept permissions, Band of Thieves's Stealth against Snow Leopard, and Cloud Spirit's legal return path, rejected repeated directed step, and exact three-step Airborne path ending with an available attack against Ghoul.
- A legal 30/60 Air teaching-deck variant that proves Spectral Stalker can be summoned to an arbitrary real void, move through void, exit onto an enemy site's surface, and attack that site while a real non-Voidwalk minion has no void summon choice; the same opening proves Forsaken has outer-column void choices but no inner-column surface or void cast.
- A legal 30/60 Air teaching-deck variant that proves Apprentice Wizard's Genesis draws a hidden spell after it is summoned while keeping the opponent view redacted.
- A legal 30/60 Air teaching-deck variant with four Leyline Henges that proves the first Henge draws nothing and an adjacent second copy draws exactly one opponent-hidden spell.
- A fourth legal 30/60 Water teaching deck that proves Muddy Pigs heals exactly 3 before entering its cemetery after a real simultaneous combat death; variants prove Sly Fox's end-turn Stealth timing and Sedge Crabs' C3-to-B3 sideways movement while real C2/C4 paths remain unavailable.
- A legal 30/60 Water teaching-deck variant that proves Polar Bears can move directly from the top to bottom realm edge and attack the opposing site while its co-located Avatar has no wraparound move.
- A legal 30/60 Water teaching-deck variant that proves Coral-Reef Kelpie has distinct surface and underwater summon choices at a real Water site while a real non-Submerge minion has only the surface choice.
- A legal 30/60 Earth teaching-deck variant that proves Cave Trolls has distinct surface and underground summon choices at a real land site, travels underground beneath an enemy land site without attacking it, then surfaces and can attack that site; a real non-Burrowing minion has only the surface summon choice.
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
- `pnpm game:check-private`: 34-action combat, 38-action Earth ramp/Deathrite, 28-action Earth Burrowing, 24-action Earth first strike, 22-action Earth Ranged, 27-action Earth Ward, 27-action Airborne, 26-action Air Stealth, 26-action Air movement, 24-action Air Movement +2, 25-action Air unrestricted-summon, 17-action Air Voidwalk/outer-column casting, 15-action Air Genesis spell-draw, 9-action Air Leyline Genesis, 43-action Fire targeting/response, 23-action Water end-turn Stealth, 22-action Water sideways movement, 16-action Water edge connection, 16-action Water Submerge, and 35-action Water healing matches exercise five concrete real-card teaching decks with byte-exact replay.
- `pnpm game:verify-private`: one passing ignored-authority integration scenario.
- Death's Door scenarios prove same-turn direct-damage immunity, later death blows, simultaneous-defeat draws, nonlethal site strikes, and exact replay.
- `pnpm verify`: 235 passing public tests at this checkpoint.

## Still required for Phase 3

- Rubble replacement, terrain mutation, connection rules beyond the supported region graph and top/bottom edge wrap, control changes, and other card-specific casting or placement overrides.
- Full start/main/end phase triggers and duration cleanup beyond the supported narrow Genesis/Deathrite effects, minion damage, and summoning sickness.
- Additional costs, non-minion spells, movement beyond bounded +1/+2 bonuses and sideways-only self-movement, further activated abilities, and card-specific targets.
- Combat tiers beyond attacking-only first strike, projectile ranges and effects beyond Ranged 1, additional healing sources, prevention/modification beyond minion Ward, additional card-triggered damage/death behavior, and tournament ending overlays.
- State-based banishment/death when forced movement, ability loss, or a deliberately suicidal step leaves a unit unable to survive in its region.
- Source-linked scenario and invariant coverage for every supported core mechanic.

The runner is intentionally classified `unranked_partial_rules`; it proves the real engine loop and replay contract without claiming complete Sorcery behavior.
