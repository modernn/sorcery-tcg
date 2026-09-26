---
name: sorcery-playtest
description: Play and improve at Sorcery using this repository's local Rust simulator, compare deck or policy variations, and preserve tested tactical lessons. Use for Sorcery simulation and deck testing; legality comes from the engine and private official authority.
---

# Sorcery playtesting

Work from the Sorcery repository root. Read its `README.md` for current commands and
`docs/shared-rule-behaviors.md` when a card is unsupported. Keep official source text,
card facts, deck lists, match records, and derived learning receipts under ignored
`.local/authority/`. This skill contains procedures and independently learned lessons,
not a card corpus. Do not acquire artwork.

For engine expansion, analyze complete corpus clauses before choosing abstractions.
A keyword occurrence is not a semantic review: negation, conditional grants, targeting,
timing, source identity, and cost/effect roles can reverse what the same words mean.
Do not preserve a card-specific runtime design merely because it already exists.

## Establish what can run

Export a new catalog with a unique output ID:

```sh
pnpm game:experiment-private --catalog --output-id review-catalog
```

Read the resulting private catalog. Corpus presence, reviewed binding, exercised
mechanic, format legality, and competitive strength are different claims. Never strip
an unsupported clause or submit replacement facts to get a deck admitted. Review
complete text against implementation: timing such as turn end versus your next turn
must match, even when an existing field looks almost suitable.

Use counted `cardId`/`copies` rows or expanded card IDs in a deck-only JSON request.
Prepare it with `game:experiment-private --decks <path-within-authority> --output-id
<unique-id>`, then feed that generated request to `pnpm --silent game:experiment`.
The interface takes both decks, seeds, and workers. Read both the rejection diagnostics
and result limitations. Failed admission is evidence of missing support, not a loss.

## Play a position

Reuse `RustSessionClient` in `src/engine/rust-engine.ts` or the persistent
`pnpm --silent game:session` JSON-lines service. Start from the prepared `baseManifest`.
Use `publicView` for the acting seat and request its engine-issued legal actions.
Choose by issued action ID and current state version. Never invent a mutation.

Separate fair play from debugging. Do not use opponent hidden hands, deck order, or
full replay state to choose moves in a fair-play experiment. Checkpoint branches of
the true state can expose hidden outcomes; label those omniscient diagnostics unless
the position is fully public. An unfinished search branch is unknown, not a win.

Before a tactical probe, save a checkpoint and state the hypothesis. Compare issued
alternatives, keep the root unchanged, resume it exactly, and verify the completed
replay. Record whether the intended mechanic actually occurred. Merely drawing or
summoning a card does not exercise every ability on it.

## Learn through controlled batches

Change one to three cards at a time. Preserve deck size, copy limits, mana curve, and
element requirements unless one of those is the hypothesis. Hold opponent policies,
seeds, and orientation schedule fixed. Swap both seats; when comparing policies across
different decks, also assign each policy to each deck so deck quality is not a confound.

Use `game:engine batch-json` for explicit `northPolicy`/`southPolicy` comparisons;
`experiment-json` currently selects the baseline internally. Reuse canonical policy
and manifest hashes. Keep screening seeds separate from a preregistered held-out set,
evaluate the chosen candidate once on that set, and report negative results too.

The baseline is an experiment control, not an expert teacher. Inspect missed spells,
activations, attacks, defenses, and positional opportunities before trusting its deck
rankings. A few wins or guided opening seeds establish neither strength nor value.
Track paired outcomes and uncertainty, not just a winning percentage.

Run independent games with native workers, bounded by jobs and actual CPU/memory
availability. Check memory pressure before concurrent builds or agents. Use the release
worker benchmark in the README to choose concurrency; do not infer it from swap size.
Check output identity across worker counts after changes to scheduling or determinism.

## Keep useful knowledge

Append private evidence to `.local/authority/learning/`: hypothesis, source commit and
binary identity, authority/deck/policy hashes, training and held-out seeds, actual
mechanics exercised, result limitations, and a reproducible command or driver path.
Promote a tactical lesson into this skill only after it survives an independent check.
Preserve counterexamples and scope; never learn a rule by changing official legality.

For best-value deck recommendations, compare current priced, complete lists and source
dates. Keep unknown prices explicit, separate purchase cost from simulation outcomes,
and distinguish tournament evidence from provisional community claims. Unsupported
decks can be research candidates but cannot receive simulated win rates.
