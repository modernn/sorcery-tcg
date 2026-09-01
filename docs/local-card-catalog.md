# Private local card catalog

Build the immutable SQLite research catalog for an existing private authority revision:

```powershell
pnpm cards:catalog-build -- --revision <revision-id>
```

The command reads the bounded canonical selection receipt in
`data/authority/receipts/<revision-id>.json`, validates the complete selected private bundle,
then atomically creates `.local/authority/catalog/<revision-id>.sqlite3`. It never overwrites a
catalog. Private source and output bytes stay under the ignored `.local/authority/` boundary.

The strict database stores every normalized card fact, ordered elements and subtypes, and the
snapshot's printing slugs in `printing_alias`. Those aliases are not complete printing metadata:
the normalized artifact currently has no official printing ID, set, finish, or release record.

`card_source_mapping` qualifies each official card ID by its source. `price_snapshot` and
`price_quote` are initially empty. A snapshot records an integer Unix-millisecond observation
time; each quote uses the Rust deck-cost identity dimensions: source-qualified card, printing,
variant, condition, currency, and a nonnegative integer minor-unit price. Printing IDs must come
from a separately authorized market mapping until authority normalization contains full records.

This file is a rebuildable deck research/search boundary. The Rust legality engine and rollout
hot path continue to load immutable `RulesContext` data in memory and never query SQLite.
