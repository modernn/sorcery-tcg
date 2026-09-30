# Rules catalog

[`data/rules/catalog.json`](../data/rules/catalog.json) is the public, human-reviewable inventory of rule behavior currently proved by public `RULE-*` scenarios. It is a versioned file, not a database and not executable rules prose.

Each entry has a stable catalog ID, the related public requirement IDs, a plain-language paraphrase, implementation status, conservative generic fact/action/event hints, and the exact test file and test name that prove it. Add a new entry when a supported behavior gains its direct scenario proof; do not infer support from the private mechanic workload or from preset demand.

Rust alone executes legality; official sources define the required behavior. The catalog helps people review coverage, but changing JSON does not change legality. Official source bytes, normalized snapshots, locks, authority revisions, and private citations remain ignored under `.local/authority/`; do not copy them or official rules prose into this public file. Add an authority-reference field only when a public identifier or citation already exists—never invent one.

## Where rules live

- `data/rules/catalog.json` is the plain-language review and coverage index.
- A validated manifest supplies generic card facts and exact authority identities to Rust.
- Rust loads those immutable facts once into `RulesContext`; compact game positions borrow that shared context during simulation.
- Rust rule helpers interpret generic facts and issue the only legal actions. Card names, deck-site data, prices, and future databases never define legality.

The [private SQLite catalog](card-tracker.md) tracks source reviews, exact Codex references, contextual
rule dependencies, validation jobs, and admission evidence. The local workbook is a
view of that evidence. Keep shared-rule proof, per-card proof, and admission distinct;
a catalog row or broad Codex link cannot unlock a card. Use the
[batch loop](shared-rule-behaviors.md#binding-and-validation-batch-loop) to refresh it.
SQLite stays outside the transition hot path; games execute the reproducible manifest
and immutable Rust `RulesContext`, not database queries.
