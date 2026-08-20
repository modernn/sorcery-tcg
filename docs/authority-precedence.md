# Authority precedence

This document is the Sorcery Simulator's D-01/D-02 fail-closed policy. It is not a claim that Erik's Curiosa has published a complete hierarchy for conflicting official sources. The repository records policy and source references only; it does not reproduce rule or card text.

## Normative authority

Only records whose `authorityClass` is `official` can determine an outcome. The five normative source classes are:

1. official rulebooks;
2. official format rules, including explicitly selected event overlays;
3. the official Codex and official FAQ;
4. official card updates, errata, clarifications, and reversals; and
5. official card data for current card characteristics.

Community tools, deck sites, gameplay reports, and external implementations are non-authoritative. They may be retained as provenance or examples, but they never supply a rule or win a conflict.

## Resolution order

`resolveAuthorityPrecedence` in `src/authority/validate-bundle.ts` resolves one topic for an explicitly selected scope and effective date:

1. Discard non-official records from normative consideration, retaining their source references as provenance.
2. Require one topic, a valid resolution date, and an effective date for every applicable official record. Only records effective on or before the selected date participate.
3. Apply an explicit official supersession or reversal first. Every `supersedes` reference must name an applicable official record; the newer record wins and displaced records remain recorded as superseded.
4. Apply current card-specific official card updates or official card data only to the matching `card:*` scope.
5. Apply an explicitly selected scoped overlay only inside its exact scope. A selected format overlay outranks compatible general material there; a scope never leaks into another format, event, or card.
6. Use compatible official rulebook material for general rules and official Codex/FAQ material for detail or card clarification. At the same applicable rank, the uniquely newest effective date wins.

A resolved result retains the winning source reference, all displaced or superseded references, and all non-normative provenance. Historical evidence is never silently flattened.

## Unsupported outcomes

Resolution returns `unsupported` with no winner when there is no applicable official authority or when any of these conditions prevents a unique result:

- an unclear or missing effective date;
- an unclear, missing, or mismatched scope, including card data outside a `card:*` scope;
- mixed topics;
- a broken or cyclic/ambiguous supersession claim;
- equal-rank official contenders at the same newest effective date; or
- any otherwise unresolved official conflict.

The contending, superseded, and provenance references remain attached to an unsupported result for review. The engine must not guess, infer a missing ruling, use community behavior as a tiebreaker, or silently choose one contender.

## Implementation evidence

The contract is enforced by `resolveAuthorityPrecedence` and bundle validation. `tests/authority/bundle.test.ts` covers:

- `resolves current official authority and retains winning source references`;
- `resolves explicit official superseded authority by effective date`;
- `applies explicitly selected scoped overlays only in scope`;
- `records equal-rank and ambiguous official conflicts as unsupported`; and
- `never lets community or external-reference sources become normative`.

Source inventory and retrieval evidence belong in the immutable authority bundle. Missing official guidance remains unsupported until a later official source is reviewed and added as a new effective-dated revision.
