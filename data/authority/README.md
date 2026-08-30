# Private authority bundle

The selected local revision is `bundle:official-2026-08-27-v4`, bound to the exact bundle root recorded in `receipts/official-2026-08-27-v4.json`. Runtime consumers select only that immutable ID and receipt root locally; they do not fetch, poll, scrape, serve, or resolve a mutable latest or date alias.

The v4 revision is derived offline from the fixed manual evidence lock at `.local/authority/locks/official-2026-08-27-v3/source-set-lock.json`. Git alone is insufficient to rebuild it. Rebuild only from either complete independently verified private source set, preserving the original card API bytes and their source hash; the strict in-memory adapter produces derived project card records and never relabels them as publisher bytes.

The earlier `official-2026-08-27-v3` and `official-2026-08-20` receipts, private locks, source roots, authorization records, and ignored revisions remain unchanged historical evidence and are not selected by current consumers.

Official source bytes, normalized corpus, rulebook pages, artwork, private absolute locators, build inputs, and revisions remain outside Git and packages. The operating scope is private, local, noncommercial, and no-redistribution. This repository contains only project code, synthetic tests, and non-content receipts.

Manual provision establishes no legal permission. Written publisher permission and a separate approved plan are required before recurring acquisition, broader agent-run collection, sharing, release, hosting, third-party upload, redistribution, a public API, artwork acquisition, or commercial use.
