# Private authority data and external reuse policy

This policy implements D-08, D-09, and D-11 for the private, local, noncommercial Sorcery Simulator. It is an operating boundary, not legal advice or a claim of legal certainty. Manual provision and hashes do not establish publisher permission, authenticity, or legal clearance. Broader use requires written publisher permission and a separate approved plan.

## Historical evidence and current manual provision

The completed official-2026-08-20 source roots, locks, receipts, and consumed records are immutable historical evidence. Their three closed evidence identities remain eligible only for read-only verification:

- acquisitionMethod user-run-one-shot-powershell with authorizationReference absent;
- acquisitionMethod user-authorized-agent-run-one-shot-powershell with authorizationReference quick-260825-mhh and its durable consumed record; and
- the same agent method with authorizationReference quick-260825-mhh-retry-1 and its distinct durable consumed record.

The unused official-2026-08-27-v3 network authorization and transport path are retired. No project code, agent, test, scheduler, or tool performs new official-source network acquisition.

For official-2026-08-27-v3, the user downloads the seven non-artwork files in a normal browser and saves them beneath this Git-ignored inbox:

    .local/authority/manual-inbox/official-2026-08-27-v3/

The required paths and public source pages are:

| Relative path | Official source URL | Media type |
|---|---|---|
| rulebook/rulebook-current.pdf | https://sorcerytcg.com/news/sorcery-contested-realm-december-2025-rulebook-update | application/pdf |
| formats/constructed-current.html | https://sorcerytcg.com/constructed | text/html |
| codex/codex-current.html | https://curiosa.io/codex | text/html |
| codex/faqs-current.html | https://curiosa.io/faqs | text/html |
| codex/changelog-current.html | https://curiosa.io/codex/changelog | text/html |
| updates/card-updates-2025.html | https://sorcerytcg.com/news/sorcery-contested-realm-card-updates-2025 | text/html |
| cards/cards.raw.json | https://api.sorcerytcg.com/api/cards | application/json |

The rulebook row means download the Standard Rulebook PDF linked from that official release page and save it under the fixed PDF filename. Each HTML file must be the complete saved page whose visible body contains the named official material; an obvious challenge page, login shell, navigation-only shell, script-only shell, or truncation fails closed, while hashes pin the accepted bytes without claiming publisher authenticity. Save the card API response unchanged as cards.raw.json. Do not attach or upload these files to chat.

After the user reports manual-source-set-ready, the executor runs this offline-only command from the repository root:

    pwsh -NoProfile -NonInteractive -File .\scripts\collect-private-authority.ps1 -ImportManualInbox -AcknowledgePrivateUseRisk

The importer has no HTTP client, web request, URL override, scheduler, polling, retry, or evasion surface. It accepts only the fixed inbox, primary, outside-repository sibling backup, and lock locations. It validates exactly seven ordinary path-contained files using per-source byte bounds, PDF page/xref/EOF structure, substantive visible HTML markers, the changelog date, and strict exact-1,100-card adaptation before copying. It stages and verifies independent primary and backup roots, reverifies after publication, and writes the lock last. Failure preserves the manual inbox and never overwrites a canonical destination.

The new lock records acquisitionMethod user-provided-manual-download and authorizationReference phase-01-20260827-manual-provision-1. That reference is workflow provenance only: it is not a network authorization, is not consumed, and grants no transport, redistribution, or legal permission. Its retrieval timestamp records when the offline import attempt began, not when the browser downloaded the files.

## Private source-set lock and backup

The v3 importer writes a seven-entry ignored lock containing the exact manual method/reference identity, absolute private roots, fixed source URLs, intake timestamp, published effective dates, media types, byte lengths, SHA-256 hashes, and canonical source-set root hash. The lock records private/local/noncommercial operation, no redistribution/release/hosting/upload/artwork, acceptance of the identified private-use risk, no legal-permission claim, and no retry or evasion.

The user must maintain a durable, independent, byte-identical backup outside the repository at the fixed sibling location. It may not be the same, nested, linked, junctioned, symlinked, or hard-linked storage identity as the primary. Every relative path, byte length, and SHA-256 must match; private absolute locators appear only in the ignored lock. Missing or mismatched evidence fails closed.

Historical locks retain their immutable historical identities and can be checked only with:

    pwsh -NoProfile -NonInteractive -File .\scripts\collect-private-authority.ps1 -VerifyExistingRoots -LockPath <ignored-lock-path>

Existing-root verification performs no intake, copying, publication, or network access and emits only fixed content-free output.

## Storage and repository boundary

The anchored .local/authority/ ignore rule protects every repository-side inbox file, official input, private lock, normalized snapshot, derived build input, and built official authority revision. All remain private/local, Git-ignored, and excluded from packages. The independent backup remains private and unpackaged outside the repository.

Git may contain only:

- project-owned TypeScript, strict schemas, policy, and documentation;
- synthetic or fact-minimal independently authored fixtures;
- official public source URLs and retrieval/effective dates;
- independent non-content hashes and receipts without private absolute locators; and
- generic local-import, validation, and final-gate tests.

The official PDF, saved HTML pages, and full API corpus may exist only in the private inbox, primary, and backup roots. Raw publisher bytes never enter the built revision, Git, or packages. Normalized official derivatives may exist only inside the ignored private built revision and are forbidden from Git, packages, sharing, hosting, third-party upload, redistribution, public/network APIs, and commercialization without written publisher permission and a separate approved plan.

No official artwork is acquired or shipped. Phase 9 may use only original/project-owned presentation art or user-supplied private local images. Bulk art acquisition, private-CDN access, hotlinking, proxying, hosting, packaging, and redistribution remain prohibited.

## Community and commercial-source audit

No audited community source is an approved gameplay corpus:

| Source | Current classification | Allowed use |
|---|---|---|
| sadkinglabs/sorcery-registry | Technically useful for stable IDs, schema, and checksums, but MIT covers code only; card content is reserved to Erik's Curiosa and needs permission. | Behavioral/schema observation with clean-room evidence only. No card-data import. |
| realms-cards/contested-realms | GPL-3.0 code; its license grants no rights to publisher card content. | Behavioral observation only; no copied or closely translated code, tests, assets, card implementations, or data. |
| JollyGrin/sorcery-tcg-playtest / spells.bar | No reusable license in the audited revision. | Behavioral and UX observation only. |
| sorcery-cards | “No Rights Included”; no reusable license grant. | Reference only; no copying or corpus use. |
| JustTCG | Potential later optional licensed price enrichment, not gameplay authority. | Out of the v1 gameplay catalog; a separate licensed integration would be required. |

Other audited candidates are incomplete, copyleft, unlicensed, or publisher-reserved. None is a substitute corpus and none may become normative authority.

### Private TopDeck competitive-deck snapshots

TopDeck competitive results may be imported manually for private deck research with the existing v2 API adapter. Set `TOPDECK_API_KEY`, then run:

    pnpm decks:import-topdeck -- --last-days 30

The command makes one bounded API request and writes one canonical, content-addressed snapshot beneath ignored `.local/authority/topdeck-candidates/`. It does not scrape HTML, retry, evade access controls, schedule updates, or write a database. The snapshot includes TopDeck attribution, endpoint/API version, retrieval time, raw-response hash, tournament ID/date/participant count, placement, and unresolved deck data. It omits player names, player IDs, tournament names, and location metadata. The current adapter does not expose wins or win rate, so placement is the only retained result evidence. API access and attribution do not grant permission to redistribute the snapshot.

## Clean-room evidence

An external behavioral observation may influence an independent implementation only when its review record contains:

- source project/URL and exact source revision;
- the behavior observed, without copied expression;
- reviewer and review date; and
- an attestation that the resulting project code, tests, assets, and data were independently authored.

Community observations remain provenance/examples only. Official sources alone determine rules and card facts under docs/authority-precedence.md.

## Permission gate for broader use

New project or agent acquisition, recurring updates, sharing, release with publisher content, packaging or publication of official or derived content, hosting, third-party upload, redistribution, commercialization, public/network card APIs, official artwork, or a changed operating scope requires written publisher permission and a separate approved plan before implementation.
