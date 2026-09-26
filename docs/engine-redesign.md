# Engine redesign from the released card corpus

## Decision and evidence

Build one Rust rules kernel that executes composed ability data. Replace the existing
card-shaped runtime facts and continuations as each shared mechanism is migrated.
Retain the deterministic session boundary, seeded randomness, replay checks, and
native batch scheduling. Do not grow another collection of named card handlers.

The private review now contains an individually authored semantic description for
every card in the pinned 1,100-card corpus: 489 minions, 202 magics, 129 artifacts,
38 auras, 208 sites, and 34 avatars. A coverage check verifies the exact source-index
union. Independent spot checks compared descriptions against complete text and the
local official Codex. Preliminary word-based classifications failed those checks and
were rejected; they are not evidence for this design.

The reviewed data, source-specific questions, and reproduction script remain under
ignored `.local/authority/global-rule-analysis/`. The annotations describe requirements;
they neither admit cards nor certify implemented interactions. Unresolved wording and
missing source data remain explicit. In particular, 15 cards explicitly depend on
damage diagrams absent from the available structured text. Do not invent their grids.

A separate characteristic audit compared all 1,100 definitions against retained source
metadata and official rules. Numeric normalization preserves the source values, but
the source does not encode every gameplay characteristic. The audit found 26 split-power
minions, ten artifact/minion definitions, three unresolved X costs, and missing token
traits. Empty fields cannot universally mean zero or an empty property set.

The current engine has 83 Magic effect variants, 12 Artifact variants, seven Aura
variants, 88 minion fact fields, and 25 site fact fields. Several variants bake together
an effect, selector, magnitude, duration, and follow-up draw. Separately maintained
Genesis, Deathrite, turn, movement, and spell continuations duplicate event handling.
Those counts describe the existing code, not the number of global rules required.

The existing 190 admitted bindings are provisional. For example, the current Lance
counter models strike bonuses and breaking, but not normal token pickup and transfer.
An accepted manifest and a deterministic replay do not prove complete rules support.
Current result eligibility therefore reports both `partial-rules` and
`unverified-authority`; completing a match cannot erase those limitations.

## Global model

### 1. Objects, forms, zones, and relationships

Separate stable game-object identity from the underlying card definition, current
copied or transformed form, owner, and controller. A transform must preserve references,
damage, tap state, attachments, and applicable marks without fabricating death or entry.
Creating a copy and entering as a copy are different operations. Named companion
references belong in private data as definition IDs, not Rust card-name branches.

Runtime types must permit multiple simultaneous roles. An animated aura remains an
aura while also being a minion; some artifact subtypes already imply minion behavior.
Printed characteristics, effective characteristics, and observer-relative predicates
must be distinct. Treating something as Evil for one player's effects is different
from globally changing its subtype.

Zones include ordered decks, hands, cemeteries, the realm, banishment, collections,
the storyline, and object-bound holdings. Zone moves share storage machinery but keep
their semantic cause: draw, put into hand, discard, summon, cast, search, and banish
must generate different events. A token follows the common object and zone lifecycle.
Lances must be actual artifact tokens participating in the common carrying rules.

Represent carrying, attachment, captivity, source-linked control, and composite
structures as explicit relationships. Do not add an unrelated vector or boolean to
every unit for each new named card. Rare relationships can have specialized records
while using the same object IDs, zone rules, and checkpoint protocol.

Base characteristics need typed absence, fixed values, chosen expressions, derived
expressions, and unresolved inputs. Separate elemental identity from threshold requirements
and site affinity; separate gameplay rarity from printing metadata. Apply inherent
type/subtype rules centrally, including token behavior and artifact carriability.
Preserve attack and defense separately; generic power uses their floor average.
Composite bodies can require per-component damage and destruction state.

Game effects that archive and restore the realm need their own explicit state. They
are not engine checkpoint rollback: a shared latest archive can exclude life and the
storyline, restore eligible objects simultaneously without movement, and continue the
current storyline. Bind restoration scope, exclusions, token recovery, and obstruction
handling through typed rules.

### 2. Locations, footprints, topology, and queries

Keep square, site object, location/region, terrain, border, and footprint separate.
An object may occupy multiple locations; a border or square anchor is not a unit's
location. A fixed 2×2 unit footprint is insufficient for persistent expanding bodies.
Site presence and void-like properties are not universally exclusive.

Use shared typed queries for object kind, owner/controller, subtype, derived properties,
source-relative region, adjacency, range, row/column, connected terrain, and occupancy.
Selectors must distinguish targeting from ordinary selection. A query returning sites
is not equivalent to one returning units at those sites. Count distinct objects unless
the rule explicitly counts their occupied locations or accumulated grid damage.

Movement kind is an operand: voluntary step, forced displacement, lure, teleport,
flying, site relocation, and virtual connection have different permissions and events.
Moving a site with its contents must not synthesize ordinary movement or entry events
for its passengers. Virtual movement adjacency must not silently change targeting range.
Keep these distinctions in one spatial implementation.

Damage grids are private verified patterns with offsets, per-cell quantities, anchors,
regional scope, and allowed rotations/reflections. They are separate from an object's
footprint. Neither a missing pattern nor a null numeric field receives a guessed default.

### 3. Setup, permissions, requirements, and costs

Apply format and avatar deck/setup modifications before the first turn. Deck topology,
copy constraints, initial draws, starting positions, and selected collection contents
cannot be late exceptions inside avatar activation code.

Casting and summoning are distinct. Casting permission specifies source zone, caster,
destination, alternative form, and applicable requirements. Summon or conjure effects
use the corresponding entry operation without automatically pretending a cast occurred.
Restrictions can apply to either operation, so the entry path still checks them.
Site play enters the realm immediately without putting the site itself on the
storyline; its resulting triggers still follow their prescribed timing.

Build a cost plan from base payment, additional costs, increases, reductions, and any
free-cast override. Threshold is a requirement, not a consumable mana payment. Model
mana, life, tapping, discarding, sacrifice, and banishment as different cost operations.
Alternative payment is a choice among plans; optional payment during resolution belongs
to that continuation. Per-turn permissions are not turn-boundary triggers.

Turn scheduling must distinguish the active player, the player making decisions for
that turn, and the controller of each source. Skipped turns, phase jumps, temporary
turn control, and Avatar-role reassignment must use those identities consistently for
visibility, legal choices, expiration, and terminal checks. Mana production happens at
defined events such as entry and turn start; it is not a continuously refilled balance.

### 4. Abilities and composed effect programs

Card data should contain base characteristics plus typed definitions for passive
effects, activated abilities, triggers, replacements/prevention, setup modifications,
and play/cast permissions. Each refers to shared selectors, conditions, quantities,
costs, limits, effects, and lifetimes. The source's card type does not select a second
implementation of damage, draw, or a status grant.

An ability payload can be invoked again or receive another trigger binding without
fabricating an entry event. Ability references and self-references must rebind to the
correct receiving object. Zone access and source-relative zone references also need
shared hooks for access taxes and effects that reinterpret which cemetery is referenced.

Use a closed Rust representation of operations and control flow: sequences, conditions,
bounded repetition, simultaneous groups, and engine-issued choices. Carry dependent
results explicitly: objects actually moved, damage actually dealt, a projectile's hit,
cards actually sacrificed, or a target that survived a protection check. A failed
dependent effect must not cancel an independent later draw.

For example, a synthetic ability can select an allied unit, grant a status with an
explicit lifetime, and draw a card. Another can invoke the same grant without drawing,
or invoke it on entry rather than through a spell. None requires a new combined
`GrantStatusThenDraw` effect variant. Magnitudes and count expressions are operands.

This is an executable rule representation, not an unrestricted scripting language or
a runtime natural-language interpreter. Compile and validate private definitions once
at admission. Unsupported operations, unresolved references, and incomplete clauses
must prevent admission. Models submit deck compositions and engine-issued decisions,
never executable ability programs or arbitrary state changes.

### 5. Storyline, decisions, and event provenance

Use one serializable continuation model. The storyline defines when an operation
splits and when newly triggered events interrupt remaining work. Do not yield between
every instruction merely because an ability contains several effects. Search procedures
and simultaneous operations have particular atomicity requirements.

Explicit targets are announced before the event enters the storyline and checked again
at resolution, subject to explicit rules for delegated targeting during resolution.
Other choices occur at their specified resolution point. Opponent
payments, tie choices, mode choices, and ordered trigger groups become engine-issued
decisions for the correct player. They must survive checkpoint resume unchanged.

Events retain cause, source ability, acting player, affected objects, relevant prior
state, and actual outcome. Distinguish cast, summon, entry, leave, move, stop, attack,
strike, damage, kill credit, and death. A kill caused by a particular attack is not an
arbitrary death during the same turn. Simultaneous damage and simultaneous triggers
have different ordering rules. Source validity must be checked under the appropriate
ability lifetime rather than a universal source-still-alive condition.

Multi-card draws declare the deck split before drawing, then perform individual draws
with their required trigger windows. This differs from both an atomic hand transfer
and a sequence that lets the player reconsider the deck after seeing each card.

### 6. Continuous effects, marks, history, and lifetimes

Compute effective characteristics using official layer order, timestamps, and dependency
rules. Copy, type changes, ability removal, control, ability/affinity additions, and
power modification cannot simply run in card iteration order. Apply the complete
derived-effect pass before checking deaths from changed power. Do not use a naive
repeat-until-stable loop: some effects intentionally apply once in a dependency order.

Use shared modifier records with source and subject references, activation timestamps,
conditions, and typed expiration. Turn end, a particular player's next turn, source
departure, next qualifying cast, next strike, being damaged, and reusing an ability are
different lifetimes. An aura lasting a number of its controller's turns may require
an actual trigger/counter, including suppression behavior; a numeric TTL is insufficient.

Ward and Stealth are marks with creation, transfer, removal, and protection rules.
They are not merely printed booleans. Track per-ability, per-object, and per-player
history separately: first ever, first this turn, once per game, visited locations,
paid quantities, cast thresholds, and causal events. Include all of it in checkpoints.

Split power needs attack, defense, and the official general-power reading. Setting,
addition/subtraction, and multiplication must modify the appropriate characteristics
through the shared layer evaluator. Do not assume a single printed number everywhere.

### 7. Replacement, prevention, combat, and terminal state

Route proposed damage, destruction, movement, zone transfer, and terminal outcomes
through their applicable replacement and prevention stages. Those stages are immediate
rules, not ordinary queued triggered abilities. Preserve official choice order and
prevent a replacement from repeatedly applying to the same event when disallowed.

Combat reuses target queries, movement, strike creation, damage modification/prevention,
death processing, and causal events. Damage, life loss, healing, and maximum-life
changes remain distinct. A strike can occur without positive damage. Death's Door,
its protection interval, death blows, death replacement, and alternate victory
conditions belong to one terminal-state evaluator.

Projectiles are stateful paths with hit selection and ordered payloads. Use common
path execution with explicit range, attenuation, piercing, redirection, passengers,
and movement-on-hit behavior. A line-shaped damage area cannot substitute for this.

## What to retain and what to replace

Retain the small board-coordinate types where they remain valid, deterministic PRNG,
canonical identities, action-ID/state-version boundary, transcript verification, owned
rollout positions, and native ordered batches. Keep existing rule tests as evidence
for the slices they actually exercise; review them against authority before treating
their old traces as a golden standard.

Replace single-effect card-type enums, compound fact flags, separate temporary-keyword
vectors, per-card activation fields, and special-purpose pending continuations with
the shared representations above. Existing fact inputs can be translated at admission
during migration, but old and new definitions must execute through the same migrated
runtime. Delete superseded dispatch after each cutover. Do not retain two legality
engines, including a permanent TypeScript fallback.

Version semantic changes. The constant `sorcery-core-v1` alone is insufficient to
identify a changing executable. Run evidence must pin the engine implementation/build,
authority, compiled definitions, decks, policies, and seeds. Correcting an old rule
can legitimately change its events; do not alter a rule to preserve an incorrect trace.
Version checkpoint and compiled-definition schemas and reject incompatible artifacts.
Translating legacy facts does not automatically carry forward their provisional admission.

## First migration and deletion plan

The first implementation checkpoint uses a shared damage transaction: one source is
a one-element damage group, and simultaneous sources use the same prevention, Ward,
damage, awakening, and lethal outcome rules. The duplicate single-hit, combat, and
Deathrite enforcement is removed. Source-specific prevention remains distinct within
a simultaneous group, and fully prevented Lethal damage cannot kill a wounded unit.
Damage sources use averaged split power while strike amounts retain attack power.

An exercised prevention-order choice with different outcomes currently aborts the
session explicitly. Borrowed game actions restore their pre-action position on error;
aborted sessions retain readable state but cannot continue or certify/export a result.
Previously saved checkpoints remain available as independent branches. Disposable
rollouts consume their game state through the same dispatcher, returning it only on
success, so they do not copy a rollback position on every action.

Next migrate a complete composed slice across spells, Genesis, minion activations,
and artifact activations: common selectors and atomic costs, then damage, untap, and
draw operations in a resumable effect frame. Source context retains instance identity,
owner, effect controller, actor/caster, declared choices, paid costs, and relevant
last-known state. Target protection and actual damage are distinct result values.

The frame preserves the next operation and its dependencies across interrupting
events. Replace the migrated action enumeration and execution branches with common
selection and invocation; delete their old cohort/damage loops and special tail
continuations. Keep thin action-format adapters only where the public protocol needs
them. Test the same operation through different ability origins, including a protected
target with an independent draw and a death-trigger interruption followed by resume.

Migrate persistent effects through the same invocation boundary next, using typed
lifetimes and official characteristic layers. Expand identity/forms/zones, event
discovery, and spatial operations as their complete shared slices land. Do not wait
for every rare subsystem before replacing common execution, and do not call a wrapper
around old dispatch a completed migration.

## Performance and learning

Separate immutable compiled rules from per-match seed/deck configuration. Current
rules contexts are shared among cloned positions, but include manifest/seed identity
and are reconstructed for each batch job. A common compiled catalog should be reused
across independent seeded jobs without copying all 1,100 definitions into every request.

Keep each worker's position, RNG, event continuation, and temporary storage private.
Index triggers by event kind and use compact query results. Start with straightforward
bounded derived-state evaluation; add invalidation caches only after profiling proves
they are useful and their rule dependencies are understood. Avoid allocating JSON or
looking up card text in inner simulation loops.

Preserve ordered batch results and byte-identical worker-count equivalence. The current
local benchmark measured roughly 3.53 games/second with one worker and 38.59 with 16;
use it as a regression baseline, not a universal performance promise. Measure the new
kernel on both simple and interaction-heavy positions.

Expose public semantic action information to playing policies from the same compiled
definitions. This lets a policy evaluate effect purpose and tactical outcomes without
reimplementing legality or reading opponent hidden state. Use held-out paired batches
to measure improvement and update the playtesting skill only with verified lessons.
Price comparisons follow complete supported decks and dated price data; an unsupported
deck must never acquire a simulated win rate.

## Cutover acceptance

1. The semantic review and characteristic audit cover the entire released corpus;
   unresolved rules, missing diagrams, and external procedures remain listed explicitly.
2. Each migrated shared rule has a direct synthetic scenario with meaningful choices,
   timing, and negative cases. Add interaction proofs where the official interaction
   has distinct semantics, rather than duplicating tests by card name.
3. Rebind complete abilities through the common representation; reject partial programs.
   Keep admitted-binding counts separate from fully exercised global-rule coverage.
4. Validate checkpoint branching/resume and replay, both seats, fixed seeds, and worker
   equivalence. Run all locked Rust gates and `pnpm verify` before each source checkpoint.
5. Remove the corresponding obsolete runtime fields/branches, commit, and push the
   verified change. Continue until all released cards and their required shared rules
   are implemented, including resolved source gaps. This document is the design, not
   a declaration that the goal is complete.
