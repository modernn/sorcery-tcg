# Rules catalog

[`data/rules/catalog.json`](../data/rules/catalog.json) is the public, human-reviewable inventory of rule behavior currently proved by public `RULE-*` scenarios. It is a versioned file, not a database and not executable rules prose.

Each entry has a stable catalog ID, the related public requirement IDs, a plain-language paraphrase, implementation status, conservative generic fact/action/event hints, and the exact test file and test name that prove it. Add a new entry when a supported behavior gains its direct scenario proof; do not infer support from the private mechanic workload or from preset demand.

Rust engine code is the executable authority. The catalog helps people review coverage, but changing JSON does not change legality. Official source bytes, normalized snapshots, locks, authority revisions, and private citations remain ignored under `.local/authority/`; do not copy them or official rules prose into this public file. Add an authority-reference field only when a public identifier or citation already exists—never invent one.

## Where rules live

- `data/rules/catalog.json` is the plain-language review and coverage index.
- A validated manifest supplies generic card facts and exact authority identities to Rust.
- Rust loads those immutable facts once into `RulesContext`; compact game positions borrow that shared context during simulation.
- Rust rule helpers interpret generic facts and issue the only legal actions. Card names, deck-site data, prices, and future databases never define legality.

A database may later retain deck provenance, price history, and aggregate simulation results. Keep it outside the transition hot path: measurements can justify an index or cache, but not moving authoritative rules out of the reproducible manifest plus `RulesContext` boundary.
