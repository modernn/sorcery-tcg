# Site-transition authority audit

The shared engine uses the official Codex as its lifecycle authority. The audit was performed
against the Codex entries for [Enter](https://sorcerytcg.com/codex/5b927a70-2900-4f83-bcf3-595f5a5f4212),
[Storyline](https://sorcerytcg.com/codex/081fc13c-49fb-4893-90c1-1ba5324e6241),
[Triggered Ability](https://sorcerytcg.com/codex/511a6488-28f7-420b-80c4-fcf94993d050),
[Target](https://sorcerytcg.com/codex/98943611-2a6f-4756-ad9b-a7d6598e9729),
[Mandatory Actions](https://sorcerytcg.com/codex/af65379e-7a9a-4d6f-9a9e-99f1434575e8),
[Replacement Effect](https://sorcerytcg.com/codex/68c087c1-84f7-41f3-b44b-fb1753fce12e), and
[Prevention Effect](https://sorcerytcg.com/codex/05a7fdfc-6427-42d7-9ec6-9cefa7036fc9).

## Current status

The engine now extracts site-incarnation boundaries into one shared detector used by the
existing minion-entry hook. It compares complete before/after footprints, de-duplicates boundaries,
preserves footprint order, and distinguishes a physical card's new realm incarnation from its
previous one. Existing movement and summon entry scenarios retain their event and replay behavior.

This extraction does not add new card support or dispatch exit, Avatar, or Aura-owned triggers.
The experimental site-transition branch was removed after review because it added a second
transition-specific runtime path before proving that it could replace the existing generic
trigger and continuation protocol. That continuation integration remains the next prerequisite.

## Decisions

- Site entry and exit compare site identity and the unit's full occupancy footprint. Summoning,
  basic movement, forced movement, teleportation, carrying, and other movement methods share the
  same boundary detector. Layer changes inside one site, site placement under an existing unit,
  and flight over a site do not create a site-entry event.
- The next implementation must compile a qualifying transition into the existing storyline trigger
  batch, rather than introduce a transition-only pending order or resolution path. It therefore
  needs to obey source-incarnation checks, target revalidation, active-player then non-active-player
  ordering, interruption, and checkpoint/resume serialization. A transition that loses its source
  before it resolves is ignored; a movement step resumes from the stored continuation.
- Storyline is split at movement steps, site enter/leave boundaries, and explicit “one at a time”
  instructions. Effects that say “each,” “all,” or “everything” remain one event unless the text
  explicitly asks for one-at-a-time processing; movement paths are the important per-step case.
- Simultaneous triggered abilities are placed active-player first, then non-active-player, and
  resolve in reverse placement order. Checkpoint processing and trigger collection must use that
  shared ordering for site transitions as well as existing triggered abilities.
- Targets are declared before the Storyline begins and checked again at resolution. Object identity
  survives a control change, but an object that has left the realm is no longer a valid target.
- If a triggered ability's source leaves the realm before the ability resolves, the event is ignored.
  A Storyline split that has already begun still finishes its current split before later events are
  considered.
- Replacement, prevention, and ongoing effects are not transition triggers. They remain in their
  pre-event/post-replacement/continuous-effect paths and must not be added to this schema. In
  particular, a Troll Bridge-style discard-or-strike choice is a replacement effect, not a
  Storyline trigger, so it is deliberately rejected by the transition schema.
- Mandatory Actions are a single game-rule action boundary; an effect may separately mandate an
  action. The transition kernel must not encode a replacement choice or silently turn a triggered
  effect into a game-rule mandatory action.
- A transition with a player choice is a pending legal-action boundary. The eventual shared
  continuation must retain the original phase, decision seat, and remaining summon/token/movement
  work; it must never silently resume at Main.

## Deliberate limits

The first implementation slice should be limited to one generic typed trigger/event/source/target
record and one continuation seam shared by an existing trigger family plus entry/exit detection.
It should land only after the old duplicated path is deleted and direct scenarios prove entry,
exit, movement splitting, source loss, target revalidation, and simultaneous ordering. Composite
card text such as submerge-and-return is not smuggled into the global model. A new card binding
must first map to these Codex concepts and add a direct scenario proof; otherwise it must fail
closed as unsupported rather than introduce a card-name branch.
