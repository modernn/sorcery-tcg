# Private collection photo scanning

This local-only slice detects cards in user-supplied photos, matches them against reviewer-confirmed private reference images, and proposes exact collection count changes. It does not download artwork, call a network service, or use an LLM.

## Private files

Keep every input and output below the ignored `.local/collection/` directory:

```text
.local/collection/
  catalog.json
  references/
  photos/
  saved.json
  review.json
  reports/
```

The normalized authority artifact passed with `--authority` must remain below `.local/authority/`.

`catalog.json` binds private reference photos to canonical authority identities:

```json
{"schemaVersion":1,"entries":[{"cardId":"card:example","printingSlug":"example-printing","finish":"standard","referenceImages":["references/example.jpg"]}]}
```

`saved.json` stores confirmed counts:

```json
{"schemaVersion":1,"counts":[{"cardId":"card:example","printingSlug":"example-printing","finish":"standard","count":2}]}
```

## Scan and reconcile

Python 3 with OpenCV is required locally. For a 3-column by 3-row binder page:

```powershell
pnpm collection:scan -- --authority .local/authority/revisions/REVISION/cards.normalized.json --catalog .local/collection/catalog.json --photos .local/collection/photos --saved .local/collection/saved.json --report .local/collection/reports/scan-1.json --collection-out .local/collection/collection-1.json --grid 3x3 --mode add
```

Omit `--grid` for separated cards on a contrasting tabletop. `--threshold`, `--margin`, `--min-area`, and `--max-area` calibrate matching and contour detection. Outputs are create-only; existing files are never overwritten.

When a report says `review-required`, copy its `scanBinding` and each unresolved detection into `review.json`, then rerun with `--review`. Reviews are rejected if the photos or detector result changed. The detection box and ordered candidates are in the private report:

```json
{"schemaVersion":1,"scanBinding":"sha256:0000000000000000000000000000000000000000000000000000000000000000","decisions":[{"detectionId":"photo-0001:0","decision":"card","cardId":"card:example","printingSlug":"example-printing","finish":"standard"}]}
```

Replace the zero hash with the report value. Use `{"detectionId":"photo-0001:0","decision":"not-card"}` to dismiss an empty grid cell or false detection. No collection file is written until every detection is confirmed or dismissed. `add` adds the photographed cards to saved counts; `replace` treats the complete scan as the new collection.
