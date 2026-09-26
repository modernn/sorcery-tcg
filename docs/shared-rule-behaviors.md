# Shared rule behaviors

The corpus-first [engine redesign](engine-redesign.md) now defines the architectural
direction. It follows a full-text review of all 1,100 released card identities and
supersedes expanding the current runtime merely by adding more compound fact flags.
The small site-count slice below is historical groundwork, not the final model.

The expansion unit is a reusable rule behavior, not a card implementation. Card IDs
belong in private binding data. The Rust engine parses those bindings into typed
facts at manifest admission and owns every legal action and resulting state change.
No card text or model-generated code is evaluated during a rollout.

The released-card inventory was checked against the official card API on
2026-09-26: 1,100 unique names matched the private snapshot exactly, with matching
set counts and no missing or extra names. This verifies identity coverage, not
current wording, rulings, or engine support. Keep the authority revision pinned;
do not silently replace it with newer API metadata. Unreleased previews are outside
the current work scope.

Private behavior inventories and the exact identity-comparison receipt are under
`.local/authority/binding-cycles/`. The existing `mechanic-workload` classifier is
only a review queue. Lexical overlap must never admit a card or imply that all of
its behavior is implemented.

## What to share

- Triggers: entry, departure, strike, damage, casting, and turn boundaries. Reuse
  event ordering and pending-choice continuations, including nested Deathrites.
- Selectors: object type, controller, region, distance, occupancy, identity, and
  subtype. Selection and counting must agree with the same spatial rules used by
  legal actions. Count sites independently of the number of occupants.
- Costs and permissions: mana, tap, discard, sacrifice, frequency limits, and
  source-zone casting. Cost payment and target legality are separate from effects.
- Effects: damage, healing, mana, draw, zone transfer, movement, destruction,
  summoning, control changes, and ability grants. Route through existing settlement
  helpers so wards, death, terrain, and carried objects are handled consistently.
- Duration and replacement: explicit expiry, source lifetime, prevention, and
  replacement ordering. Never implement these by editing printed base stats.
- Composition: ordered effects, optional choices, repetition, and dependencies on
  an earlier result. Preserve the continuation across checkpoints and replay.

Prioritize selectors and counts, reusable event triggers, zone transfers/casting,
temporary grants, and carrying. The private inventory finds these across multiple
card types. Its overlapping hit counts are workload estimates, not promised card
unlocks. Prove the selector, cost, timing, and complete effect of each binding.

## Incremental implementation

Start from an existing effect and replace duplicated decision logic with a small
typed helper. The first slice shares site-count selection across conditional mana,
mana per occupied site, and drawing for adjacent copies. Keep existing fact inputs
compatible while routing them through the shared evaluator. Add a direct scenario
that distinguishes adjacent/nearby/realm scope, controller, same-card identity,
surface occupancy, multiple occupants, and avatar occupancy.

Avoid an all-purpose scripting language or a second legality engine. Add a new
primitive only when reviewed card text requires a behavior the existing helpers
cannot express. A truly unusual card can use a specialized primitive, but it still
uses the shared legality, targeting, settlement, and replay mechanisms.

## Binding and experiment cycle

1. Rank missing behavior families by demand in the fixed benchmark decks.
2. Review complete official card text and rulings; record unresolved clauses.
3. Implement shared Rust behavior and a direct scenario proof. Keep unsupported
   compositions blocked, even if some of their keywords already work.
4. Store reviewed facts, exact source hashes, and proof references in the ignored
   `bindings/reviewed.json`. The loader checks source identity, fact shape, base
   stats, and conflicts; review references do not confer ranked eligibility.
5. Introduce one to three newly bound cards in a playable deck variation. Run both
   seats over fixed seeds, verify replay, and use checkpoint scenarios to exercise
   mechanics the baseline policy did not choose.
6. Keep simulation outcomes separate from coverage evidence. A completed game is
   not proof of an unexercised ability, nor evidence of competitive deck strength.

Full Rust gates and `pnpm verify` must pass before accepting a cycle. Parallelize
independent games with native workers; preserve input-order output and manifest/seed
identity. Measure release throughput across worker counts before choosing a run's
worker budget. Swap capacity does not establish an efficient rollout budget.


## Composed token entry

`SummonToken` is an ordered effect shared by Magic and authored minion Genesis.
Its data identifies a token definition, a simultaneous count, and a source,
ordinarily chosen unit, declared unit target, or declared location destination.
The existing summon-and-draw input lowers to ordinary choice, token entry, then
Draw; it has no separate runtime continuation.

Entry uses the common terrain restriction, regional survival, Genesis, Deathrite,
and continuation helpers. Every member of a simultaneous group enters before its
Genesis resolves. Regional death or banishment settles before those triggers, and
interruptions finish before the next parent operation. A failed destination does
not cancel an independent later draw. Token dependencies are collected transitively
when preparing and admitting manifests, including several token types in one program.

Explicit implementation limits remain: at most 32 tokens per operation, no cyclic
token dependencies, at most 64 definitions on a dependency path, and at most 4,096
realm minions during token entry. These are admission/runtime support bounds, not
Sorcery rules. A unit spanning multiple locations still needs an ordinary location
choice; token placement there reports unsupported rather than guessing a location.
Ordinary site/location choice and site-cohort summoning remain separate unfinished
selector work. This engine slice alone does not certify additional real cards.
