# Shared rule behaviors

This is the active rule and card batching workflow. The [engine design](engine-redesign.md)
defines the shared Rust architecture; the private source review and dependency catalog
supply the current work queue. Implementation snapshots below explain particular
slices, not complete card coverage. Read current code and evidence before reusing one.

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

The [official Codex](https://sorcerytcg.com/codex) defines the global rule model.
Card review discovers declarative inputs to that model; repeated wording is not a
substitute for the Codex's timing and ordering rules. The ignored authority snapshot
records the exact Codex revision used for an implementation cycle. Join exact entry
and subsection IDs/URLs, preserving parents and the scope of each reference. Generic
family context is distinct from a clause-specific authority match; follow
[authority precedence](authority-precedence.md) when sources disagree.

Derive the shared contracts from complete Codex clauses first, then reconcile existing
Rust helpers, scenario expectations, and card dependencies against them. Keep the
private normative obligation ledger distinct from the card wording inventory and
implementation hypotheses. Audit existing green tests rather than treating their
expected outcomes as authority. Reuse conforming helpers; replace incorrect behavior
in bounded slices with discriminating proofs. Start with death marking, Deathrite
ordering, zone departure, replacement, and relationship cleanup before expanding
batches that depend on those contracts. This is incremental reconciliation of the
single Rust engine, not generated legality or a parallel rules interpreter.

Private behavior inventories and identity-comparison receipts remain under
`.local/authority/`. Reuse the accepted complete semantic and characteristic reviews.
The older `mechanic-workload` classifier is only a lexical inventory, not the semantic
dependency queue. Negation, owner type, regions, target versus ordinary choice,
source lifetime, rarity, thresholds, and ability suppression can make similar wording
require different behavior. No clause or characteristic may be stripped to fit a fact.

## Machine records and human views

Agents query `.local/authority/catalog/card-tracker.sqlite3` directly, using a
read-only connection and stable card IDs to retrieve current status, contextual
requirements, blockers and validation jobs. Read only the rows and fields needed
for the current packet; do not load or render the workbook to make decisions.

Official source snapshots define authority. Reviewed binding JSON and immutable
source, contract and proof receipts supply the guarded evidence. SQLite indexes
that evidence for operational queries; check the source, dependency and engine
hashes before dispatch or admission. A copied status or readiness annotation is
insufficient, and a stale database must be synchronized from its machine inputs.

Data flows from authority and reviewed machine records into SQLite, then into the
workbook and its JSON/CSV exports. These exports are human views, never inputs to
rule design, worker dispatch, card eligibility or registry promotion. Preserve
historical export-based receipts as provenance; recheck current decisions against
the database and exact receipts. Refresh the workbook after accepted batches for
human review; agents do not need it to continue work.

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

## Codex-derived engine kernel

The shared kernel follows the official [Storyline](https://sorcerytcg.com/codex/081fc13c-49fb-4893-90c1-1ba5324e6241),
[Target](https://sorcerytcg.com/codex/98943611-2a6f-4756-ad9b-a7d6598e9729),
[Replacement Effect](https://sorcerytcg.com/codex/68c087c1-84f7-41f3-b44b-fb1753fce12e),
[Prevention Effect](https://sorcerytcg.com/codex/05a7fdfc-6427-42d7-9ec6-9cefa7036fc9), and
[Ongoing Effect](https://sorcerytcg.com/codex/e598f1a7-a876-431c-a414-33e1893cd4d8)
entries:

1. Propose an action from engine-issued legal actions, declare its complete costs
   and targets, and reject it before commitment when those declarations are illegal.
2. Add a spell, activated ability, or triggered ability to an interruptible
   storyline. Preserve the source realm incarnation and declared targets.
3. At resolution start, revalidate targets. Ignore an unresolved event whose realm
   source has left, except when that event already started and split into child
   events.
4. Split a resolving event at realm entry or exit, movement, and explicit sequential
   wording. Newly triggered events interrupt the remaining children. Resolve
   simultaneous-trigger ordering through explicit player choices.
5. Apply replacement effects immediately before the event they replace, then apply
   damage prevention. Neither uses the storyline.
6. Between storyline events, reevaluate all ongoing effects using the official
   layer, timestamp, and dependency order before checking whether minions die.
7. When the storyline empties, enumerate any currently legal
   [Mandatory Actions](https://sorcerytcg.com/codex/af65379e-7a9a-4d6f-9a9e-99f1434575e8)
   before ordinary main-phase actions.

Typed card facts describe selectors, predicates, parameters, durations, and usage
limits consumed by this kernel. They must not create a parallel timing path. If the
kernel cannot express reviewed wording, record the missing global primitive before
adding another card fact.

Prioritize selectors and counts, reusable event triggers, zone transfers/casting,
temporary grants, and carrying. The private inventory finds these across multiple
card types. Its overlapping hit counts are workload estimates, not promised card
unlocks. Prove the selector, cost, timing, and complete effect of each binding.

## Incremental implementation

Start from an existing effect and replace duplicated decision logic with a small
typed helper. The initial migration shared site-count selection across conditional mana,
mana per occupied site, and drawing for adjacent copies. Keep existing fact inputs
compatible while routing them through the shared evaluator. Add a direct scenario
that distinguishes adjacent/nearby/realm scope, controller, same-card identity,
surface occupancy, multiple occupants, and avatar occupancy.

Avoid an all-purpose scripting language or a second legality engine. Add a new
primitive only when reviewed card text requires a behavior the existing helpers
cannot express. A truly unusual card can use a specialized primitive, but it still
uses the shared legality, targeting, settlement, and replay mechanisms.

## Binding and validation batch loop

1. **Review source and Codex.** Preserve exact authority/card hashes and review every
   clause, printed characteristic, card-specific ruling, and transitive token or other
   referenced definition. Record unresolved requirements explicitly. Prioritize complete
   mappings to existing operations, including truly simple cards; blank text alone does
   not remove characteristic or inherent-type requirements.
2. **Approve architecture fit.** Design a bounded packet before coding. Name the existing
   representations, reused helpers and affected callers, one owner for mutation,
   settlement, trigger arbitration and parent continuation, identity/lifetime policy,
   discriminating proofs, permitted files and compatibility consequences. Use the
   implementation contract below. A new representation, operator, timing boundary or
   material hot-path ownership change returns to design review; approved reuse does
   not require another whole-engine review. Assign one writer per conflicting slice.
3. **Prove shared rules.** Group cards by concrete operations and contextual requirements,
   then pair companion rules needed together: selection with targeting, entry with
   regional settlement, a grant with suppression and expiry, or replacement with damage
   ordering. Rank by conditional card reach and selected-deck demand, with explicit
   blockers. Implement the smallest coherent Rust slice and a discriminating native
   scenario, including negative cases and relevant interactions. Reuse shared proof
   families without creating a card-name dispatcher or duplicated legality in a driver.
4. **Review semantics and conformance.** An independent reviewer checks complete scoped
   source obligations and architectural fit, including equivalent callers, identity,
   settlement order, interruption/resume and execution-mode parity where affected.
   A green test is evidence, not authority. Deviations return to the design packet;
   source contradictions return to the source review. Root integrates the accepted
   slice, runs required gates, commits and pushes before downstream validation uses it.
5. **Recompute eligibility.** Refresh exact per-card dependencies in the ignored SQLite
   catalog after rule verification. Preserve shared rule identity separately from each
   card's contextual mapping, source guard, Codex links, and proofs. A repeated semantic
   requirement ID does not make those card contexts interchangeable. Group counts are
   conditional planning estimates, never blanket unlocks.
6. **Validate eligible cards.** Queue all newly eligible cards with complete guarded
   candidate facts and no unresolved contextual requirement. Readiness means a card
   needs validation; it must not require prior admission or an already-passing card
   proof. Dispatch bounded independent validation agents. Pin source, candidate facts,
   complete dependency set, and engine identity; stale evidence cannot authorize a job.
   Launch the preserved executable whose exact path and hash the receipt records;
   hashing one binary and starting a Cargo rebuild does not freeze the runtime.
   Use engine-issued actions and checkpoint/replay to exercise complete characteristics,
   abilities, and discriminating interactions. No arbitrary state edits or arbitrary
   one-to-three-card admission limit. Resource limits bound concurrency, not review scope.
7. **Promote passing receipts.** Each card needs its own source/admission evidence and
   direct native scenario receipt before adding its complete facts to the private
   `bindings/reviewed.json`. Refer to shared rule proofs for common mechanics, retaining
   per-card evidence rather than duplicating an engine or test suite. Keep partial,
   failed, stale, and source-ambiguous cases unbound. Loader acceptance and a completed
   game alone are insufficient.
8. **Refresh and verify.** Refresh the catalog, queue, workbook, and deck coverage from
   accepted receipts. Run the required Rust gates for Rust changes and `pnpm verify`;
   make a coherent verified commit and push. Keep all official data, private proof
   drivers/receipts, registry inputs, and workbook bytes ignored under `.local/authority/`.

Completion includes delivery: root records the verified commit and confirms it is on
the authorized remote before marking a batch complete. Do not accumulate accepted
changes only in local worktrees. Failed checks or a failed push leave the batch open
with an explicit blocker. Private authority artifacts remain local and ignored.

Source review, existing-operation mapping, a verified shared rule, per-card validation,
and binding admission are separate states. None grants ranked eligibility or proves
Constructed legality, strategic strength, price, or all possible interactions. An
unsupported exercised mechanic invalidates the result and remains an explicit blocker.
The catalog and workbook index evidence; neither executes rules or replaces Rust facts.

Once cards are admitted, deck experiments may vary a small controlled set to isolate a
hypothesis. Run both seats over fixed seeds and use checkpoint scenarios for mechanics
the baseline policy did not choose. This experimental design does not cap rule batches.
Measure native worker throughput and memory before increasing concurrency; preserve
input order and manifest/seed identity. Swap capacity is not a rollout budget.

### Worker implementation contract

Keep immutable `Arc<RulesContext>` shared and each branch's `Position` owned. Reuse
typed `CardFacts`, bounded `AbilityProgram`/`EffectFrame`, `SelectionSpec`/`UnitQuery`
and shared footprint queries. Use `RealmReference` for deferred incarnation identity,
`TriggerBatch` for trigger ordering, `ResolutionContinuation` for suspended work,
and existing damage/modifier helpers. These are reuse patterns, not certification
that every existing helper already satisfies every Codex interaction.

Do not add card-name branches, copied selectors or prevention logic, alternate
producer queues, a shadow realm, worker-local frameworks or a second legality engine.
Eligibility and payment must agree through shared checks; no universal cost hierarchy
is assumed. Distinguish source, actor, owner, current controller and captured trigger
controller, and snapshot only the values required by the reviewed source contract.

`OutcomeLog` observes native state transitions and does not define timing. Changed
resolution boundaries need authoritative state, legal-action and terminal parity
between owned/Ignore and recorded execution, source-derived negative/composition
cases, and serialized checkpoint/resume/replay at actual interruptions. Parent work
resumes once; terminal state stops later work. Keep wire/schema compatibility separate
from behavioral and executable identity, and preserve historical receipts unchanged.

Measure material hot-path ownership/allocation changes on paired release workloads;
trivial deletions need no invented benchmark gate. Root accepts design deviations,
integrates verified slices and promotes current complete card evidence. After each
refresh, dispatch the next dependency-ready packet through these same gates.

When diagnosis benefits from a second view, use [Jev's existing synthetic adapters](agent-evaluation.md).
Its suggested counterexample, probe, or action ordering must pass the same local native
proof. Keep official source context and private derivatives off the external service.

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
realm minions or artifacts during their respective token entry. These are admission/runtime support bounds, not
Sorcery rules. A unit spanning multiple locations opens an ordinary location choice
restricted to that live realm incarnation's footprint. Its departure empties the
choice; token entry is skipped and independent following operations continue.
Ordinary site choice and site-cohort summoning remain unfinished selector work.

`ChooseLocation` supplies a separate ordinary binding for `chosen-location` token
entry. Anywhere includes every existing location across all regions; local relations
use the source's geometry and region. These choices do not invoke declared-target
protections or hide destinations where tokens cannot survive. The existing regional
settlement handles those entries before the parent continuation. One location choice
places an entire simultaneous token group; pending choices share the same engine-issued
action, checkpoint, and replay lifecycle as ordinary unit choices.

`ConjureToken` uses the same destination bindings, ordinary location choices,
identity generation, and dependency validation to create artifact tokens. Placement
defaults to loose; `placement: "carried"` creates them already held by the source,
chosen, or targeted unit without taking a pickup action. An oversized bearer uses
the ordinary location choice to select one occupied cell. The creator owns the token;
its controller follows the bearer. Stale bearer references cannot attach new tokens.
The operation requires an artifact token; `SummonToken` and legacy summoning facts
still require minion tokens. Artifact tokens use ordinary artifact positions,
pickup/drop, bearer relationships, and zone exits. Lower-region or void occupancy
follows artifact rules rather than minion survival rules. Destruction, sacrifice,
and return-to-hand banish tokens instead of inserting them into another zone.
`bearerUnitStrike` composes optional additive damage, first-strike timing, and
source destruction after the strike. These facts are independent of an artifact's
other admitted ability. They are evaluated from actual carried artifacts for avatars
and minions; loose artifacts do not contribute. Strike damage stays separate from
current power. Each simultaneous strike group retains its contributing source
incarnations, then consumes marked artifacts after damage even when damage was
prevented. Normal artifacts enter their owner's cemetery; tokens are banished.
Source removal and resulting power-loss deaths settle together with combat deaths.
Undefended site strikes neither gain the unit-only bonus nor consume its sources.
Fixed-damage projectile abilities deal their specified damage without strike-only
modifiers. Ranged projectiles that cause a strike still use the shared strike rules.
Independent next-strike doubling grants multiply separately: two grants produce
four times the strike damage, including repeated grants from the same source.
They leave current power unchanged and are all consumed by the qualifying strike.

Ranged strikes, simultaneous fights, and effect-driven strikes combine carried
additive bonuses and nearby doubling through one replacement queue. The active player applies their effects first, then the other
player. A carried artifact follows bearer control; an uncontrolled artifact follows
its owner for ordering. Mixed arithmetic offers `choose-damage-modifier` actions;
commuting effects resolve automatically. Pending choices retain source incarnations,
are visible to clients, and resume through checkpoints before prevention, source
consumption, deaths, and the optional post-strike step.

Simultaneous fights hold all damage until replacement choices finish, then apply
prevention and settle source consumption and deaths together. First-strike survivors
resume the later fight window. Leap and Genesis effects share one full-power strike
group resolver: each target receives a full strike, without retaliation, and a
consumable source is removed once for the group. Later Genesis effects and Magic
completion wait through damage choices and any resulting Deathrites.

Mixed additive/doubling damage split across several defenders remains explicitly
unsupported pending authoritative allocation timing. Combinations involving temporary
doubling also remain guarded until lasting-effect controller provenance is retained.
These gaps remain fail-closed for newly bound cards as well as existing ones.
Provisional admission requires complete source-reviewed facts and direct private
card scenarios; it does not certify every interaction or ranked eligibility.
Synthetic scenario coverage alone does not admit an official card. The old Lance
counter remains a compatibility path, not a fact used by new bindings.

Minion `entersCarrying` lists 1–32 carriable artifact-token references, with repeated
references creating separate artifacts. It lowers to restricted carried-token effects,
independent of suppressible Genesis. Paid summons, reanimation, and token batches
create this equipment before Genesis; token batches place every minion and its equipment
before regional settlement. Each artifact on an oversized bearer uses the existing
location choice independently. Pending entry work retains bearer incarnations and
resumes through the ordinary checkpointable continuation. Token identity separates
entry equipment from Genesis creation by the same source in the same action.

Token definitions preserve an absent printed mana cost as explicit `null`, distinct
from a printed zero. Token minions and artifacts may have that absence; tokens cannot
enter the spellbook or be cast as ordinary spells. Effect entry pays zero without changing
the printed characteristic. Optional complete minion and artifact `elements` and `subtypes` lists
preserve elemental identity independently of casting thresholds. The existing Demon,
Mortal, and Undead predicates derive from explicit subtypes at admission; contradictory
legacy flags are rejected. Artifacts also retain optional `rarity` using the four
existing rarity values. Omitted characteristics remain unspecified for older bindings;
explicit null rarity is rejected.

When retained normalized data omits a characteristic, reviewed private bindings may
add an element or subtype with a source hash and locator. For artifacts only, a
supplement may also fill absent rarity. These supplements cannot remove retained
traits or change printed costs, stats, thresholds, or existing rarity. The
references record the review's provenance; they do not automatically verify the
referenced text or grant ranked eligibility. Authority snapshots remain unchanged.

An intrinsic Minion Spellcaster may supply `spellcasterElements`, a nonempty set
in canonical element order. This requires `spellcaster: true` and permits spells
with a positive threshold in any listed element. It is independent of the caster's
printed elemental identity and of the player's available thresholds. Magic,
Minion, Aura, and Artifact action enumeration uses the same predicate; staged
chain casting preserves that restriction. Ordinary Spellcasters and Avatars
remain unrestricted. An existing unrestricted tower grant provides unrestricted
casting while active; leaving its range restores the intrinsic restriction.
This fact does not yet admit site or artifact Spellcasters, negative elemental
qualifiers, or new bearer grants.

Damage prevention can select `preventsDamageFrom: "ranged-strikes"`, `"magic"`,
or an elemental Magic source such as `"fire-magic"`. The compact damage context
captures that source class before resolution; spell elements come from the spell's
thresholds, independently of its caster. The reviewed released Magic corpus has
matching element and positive-threshold sets; a future frame-only element would
require explicit metadata before admission. Ranged origin is assigned only to the
strike generated by the Ranged ability. Ordinary strikes, effect-driven strikes,
and direct projectile damage do not acquire it merely from the attacker's abilities.
A permanent's later damage is not resolving Magic damage. Composed Magic retains
its origin through pending choices and checkpoints.

The shared transaction applies source-sensitive prevention per contribution after
damage modification and before Ward, damage accumulation, and death settlement.
Disabled and Silence both suppress printed prevention, including existing numeric
reduction and power-based immunity. Competing printed prevention
facts remain rejected, and an exercised Ward-ordering choice that could change
Ward consumption remains explicitly unsupported.

Printed `takesLessDamage` is a bounded positive amount for both Avatars and
Minions. Both use the same per-source prevention helper: simultaneous sources
are each reduced, rather than reducing their combined total once. Reduction
saturates at zero. Avatar damage is reduced before life loss or Death's Door
settlement; direct life loss (including a strike on a controlled site) bypasses
this damage rule. A fully prevented hit cannot deliver a death blow.
Temporary proximity-dependent reduction still requires its own duration and
count semantics.

Minions can provide `nearbyDamagePrevention` with one existing prevention
selector and optional `alliedOnly: true`. Nearby includes the provider itself,
uses full occupied footprints, and stays within a region. An allied-only grant
checks controller; otherwise both players' units can receive it. Avatar and
Minion recipients share the same compact protection snapshot. Numeric grants
add, source immunities combine, and power-qualified immunities use the lowest
applicable threshold. A recipient losing its own abilities does not remove
protection supplied by another active provider. Silence, Disabled, or departure
of the provider ends its grant for later damage.

Every simultaneous damage path captures recipient protection before applying
any damage, including area effects, fights, effect-driven strikes, and staged
Magic. A provider awakening or dying in the group cannot change protection
halfway through that group; subsequent damage uses fresh protection. The same
ability-loss predicate now suppresses existing power grants to Avatars and
prospective minion entry power.
The immutable rules context records whether any provider exists in the manifest,
so decks without this mechanic skip the provider scan. Protection snapshots
combine numeric values and elemental bits without allocating a provider list.
This does not yet admit equipment providers,
or consumable prevention ordering; the existing Ward-order guard remains.

Nearby protection accepts an optional `recipientSubtype` filter. It queries the
recipient minion's subtype metadata, independently of its abilities, controller,
and the damage source's power. Avatars do not match a minion subtype filter.
Missing legacy subtype metadata raises an unsupported-mechanic error when an
active nearby grant could apply; it is not treated as an empty subtype list.
Currently admitted minions use their complete bound subtype lists. Future
subtype-changing effects must extend this shared membership query before being
admitted; this does not yet support subtype grants from equipment or global
subtype transformations.

End-of-controller-turn untapping and Stealth gains check the same ability-loss
predicate as continuous nearby enemy Stealth removal. Both Disabled and Silence
suppress these special abilities. End-turn eligibility is captured before
expiring temporary Silence, so expiry does not retroactively trigger a removed
ability during that end phase.

Sites may declare `siteAffinity` as four integer production counts (Earth, Fire,
Water, Air; each 0–100), with positive components matching their `elements`.
Element membership still controls classification; counts control casting
threshold availability. Omission preserves one affinity per listed element.
The Rust parser compiles the counts once into a four-byte array, and production
sums those arrays without per-query allocation.

Flooded supplies a minimum of one Water affinity and retains existing counts.
Drought removes only Water production. A water-only flooding effect removes
non-Water counts while retaining printed Water production, with a minimum of
one. Site suppression zeros the full vector; Silence and Disabled suppress
minion abilities that remove site production or supply additional affinity.
Private ingestion validates explicit production against the source site's
printed counts, separately from non-site casting requirements. Repeated source
production requires explicit counts; omitting them cannot silently fall back
to single-affinity production.

Sites that cannot be modified retain their printed classification, affinity, and
abilities under terrain overlays, including water-only flooding and ability
loss. The same protection prevents site-production suppression.

## Minion site entry

`siteEntryEffect: "grantStealthToEnteringMinion"` is a deliberately narrow
site-transition fact. It means an unconditional effect on every minion that enters
that site. With `siteEntryUsage: "firstEntry"`, it resolves only for the first
qualifying minion to enter that site incarnation; the marker does not reset at a turn
boundary. This is the complete Dark Alley clause. The fact still does not encode
controller or Airborne filtering, Avatars, exits, damage, killing, Submerge, discard,
strikes, or fights. Cards containing any of those clauses remain unsupported.

Entry compares the site's realm incarnation across the minion's old and new occupied
footprints. Summoning and simultaneous token placement start with an empty old set;
ordinary, incremental, forced, teleported, dragged, defending, and effect-driven
movement use the shared relocation boundary. Overlapping cells of an oversized minion
do not re-enter the same site. Surface/subsurface changes at one site, playing a site
under a unit, and occupants carried by a flying site do not enter a new site. The
effect is non-interrupting and uses the existing permanent Stealth helper; this narrow
slice therefore does not open a trigger-order choice or movement continuation.
First-entry markers are keyed by the site's realm incarnation, survive turns, are
cloned through checkpoints, and disappear semantically when that incarnation leaves
the realm.
