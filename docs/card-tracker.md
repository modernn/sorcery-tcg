# Private card tracker

The tracker is a rebuildable SQLite index for the current private card snapshot, Codex snapshot, reviewed bindings, and card-to-rule requirement feed. It does not admit cards or decide game legality. Run its commands from the repository root:

- `pnpm cards:tracker-sync` validates pinned source identities and transactionally refreshes `.local/authority/catalog/card-tracker.sqlite3`.
- `pnpm cards:tracker-export` writes the workbook JSON feed to `.local/authority/catalog/card-tracker-workbook.json`.
- `pnpm cards:tracker-export-csv` writes the equivalent card-level CSV to `.local/authority/catalog/card-tracker-workbook.csv`.
- `pnpm cards:tracker-audit [authority-relative-packet-path]` prints the read-only candidate readiness report as JSON; the default packet is `.local/authority/catalog/readiness-packet.json`.

All database and export files remain under the ignored `.local/authority/` directory. The database retains official printed text and Codex article/subentry text for local review. Workbook exports contain card/status annotations, source hashes and locators, requirements, and exact Codex references; they omit printed text, card facts, and Codex article text. Never move these files outside the private authority boundary.

Sync reads the current binding catalog at `.local/authority/catalog/current-bindings.catalog.json`, guarded by the feed's exact catalog hash. Refresh this catalog from accepted bindings when promoting cards, preserving earlier catalogs and receipts as historical evidence.

The tracker keeps separate meanings for three kinds of status. `bindingStatus` reflects whether the current private catalog has a binding. `sourceReviewStatus` is `source-reviewed` only when a guarded reviewed-binding row covers the exact current card source hash and attests to reviewing the complete printed rules text; `source-review-required` marks remaining feed rows, and is not a review result. `status`, `reviewStatus`, `validationStatus`, and `feedValidationReadyAnnotation` are imported feed annotations, not admission or proof. Manual review status defaults to `pending` and survives syncs.

Imported native proof references are stored as `referenced-unverified`. A reference, Codex link, admission, or feed annotation does not count as an exercised Rust proof. The read-only audit reports evidence-based readiness; it does not create or lease automatic validation jobs.

Sync verifies the normalized authority revision, catalog-byte hash, per-card source hashes, reviewed-source hashes, and exact Codex ID/title/URL joins before changing rows. Current rows refresh in one SQLite transaction. Manual statuses, proof metadata, and validation-job results are stored separately and are not overwritten by source refresh; changed source/rule/engine hashes must be checked before any future consumer treats prior evidence as current.

## Read-only readiness audit

The audit reads the default packet `.local/authority/catalog/readiness-packet.json`, or one supplied authority-relative packet path, verifies every referenced artifact under `.local/authority/`, and prints a deterministic JSON report to stdout. It opens `card-tracker.sqlite3` read-only in one read transaction. It does not sync the tracker, create jobs, promote bindings, or write receipts. Keep the packet, referenced artifacts, and printed report private; they contain local evidence identities and executable paths are committed into fingerprints.

The packet has this exact shape:

```json
{
  "schemaVersion": 1,
  "candidateBindings": { "path": "bindings/candidates.json", "sha256": "sha256:<64 lowercase hex digits>" },
  "runtime": { "path": "qualification/runtime.json", "sha256": "sha256:<64 lowercase hex digits>" },
  "cards": [{
    "cardId": "<current normalized card ID>",
    "contextReview": { "path": "reviews/card-context.json", "sha256": "sha256:<64 lowercase hex digits>" },
    "sharedProofs": [],
    "validation": null
  }]
}
```

Every reference path is relative to `.local/authority/`; its hash covers the exact file bytes. Candidate bindings use the reviewed binding file shape and printed-fact guards, but omit `replacesFactsHash` because replacement authority belongs to later registry promotion. Context receipts separately identify the reviewer, current authority and Codex snapshot, source and candidate-fact hashes, the complete sorted requirement set, unresolved items, and at least one evidence reference. A reviewed empty requirement list is explicit; a missing review is not empty.

The runtime receipt pins a passed or failed qualification, current engine source hash, and the canonical absolute path plus SHA-256 of the preserved native `session-json` executable. On this Linux host the executable must be a regular ELF binary. Its referenced qualification evidence records the build/toolchain, command, exit status, and logs. Runtime and proof receipts include nonempty referenced evidence. A shared-rule proof pins its card, requirement slice, current source and candidate facts, engine, and executable. Each current requirement needs exactly one matching passed proof. A card-validation receipt contains its status, card ID, computed dependency fingerprint, and nonempty evidence references, including direct native scenario outputs reviewed for the complete card. Failed, missing, or stale card validation remains `candidate-ready` when every prevalidation obligation is current; missing or stale prevalidation evidence is `blocked`.

`currently-validated` means the receipt identity and referenced evidence match the present audit inputs. The audit verifies hashes and identities, but it cannot decide whether a human's context review or scenario review was semantically complete. Runtime qualification applies to the recorded host environment; it does not prove OS/shared-library closure or cross-platform compatibility. Admission is reported separately and does not grant readiness.
