# Private card tracker

The tracker is a rebuildable SQLite index for the current private card snapshot, Codex snapshot, reviewed bindings, and card-to-rule requirement feed. It does not admit cards or decide game legality. Run its commands from the repository root:

- `pnpm cards:tracker-sync` validates pinned source identities and transactionally refreshes `.local/authority/catalog/card-tracker.sqlite3`.
- `pnpm cards:tracker-export` writes the workbook JSON feed to `.local/authority/catalog/card-tracker-workbook.json`.
- `pnpm cards:tracker-export-csv` writes the equivalent card-level CSV to `.local/authority/catalog/card-tracker-workbook.csv`.

All database and export files remain under the ignored `.local/authority/` directory. The database retains official printed text and Codex article/subentry text for local review. Workbook exports contain card/status annotations, source hashes and locators, requirements, and exact Codex references; they omit printed text, card facts, and Codex article text. Never move these files outside the private authority boundary.

Sync reads the current binding catalog at `.local/authority/catalog/current-bindings.catalog.json`, guarded by the feed's exact catalog hash. Refresh this catalog from accepted bindings when promoting cards, preserving earlier catalogs and receipts as historical evidence.

The tracker keeps separate meanings for three kinds of status. `bindingStatus` reflects whether the current private catalog has a binding. `sourceReviewStatus` is `source-reviewed` only when a guarded reviewed-binding row covers the exact current card source hash and attests to reviewing the complete printed rules text; `source-review-required` marks remaining feed rows, and is not a review result. `status`, `reviewStatus`, `validationStatus`, and `feedValidationReadyAnnotation` are imported feed annotations, not admission or proof. Manual review status defaults to `pending` and survives syncs.

Imported native proof references are stored as `referenced-unverified`. A reference, Codex link, admission, or feed annotation does not count as an exercised Rust proof. The tracker does not currently expose a readiness or automatic validation-job queue command; proof readiness and card-level job fingerprints need explicit, current source, rule-slice, and engine evidence before that workflow is enabled.

Sync verifies the normalized authority revision, catalog-byte hash, per-card source hashes, reviewed-source hashes, and exact Codex ID/title/URL joins before changing rows. Current rows refresh in one SQLite transaction. Manual statuses, proof metadata, and validation-job results are stored separately and are not overwritten by source refresh; changed source/rule/engine hashes must be checked before any future consumer treats prior evidence as current.
