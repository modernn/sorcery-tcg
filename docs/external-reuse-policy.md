# Private authority data and external reuse policy

This policy implements D-08, D-09, and D-11 for the current private, local, noncommercial Sorcery Simulator. It is an operating boundary, not legal advice or a claim of legal certainty. The authorized v1 path below does not depend on future publisher permission; any broader use does.

## Sole Phase 1 acquisition path

The user makes a manual browser save of exactly one complete official source set under `.local/authority/inputs/official-2026-08-20/primary/`: the current rulebook PDF, base Constructed page, Codex, FAQ, Codex changelog, official card update notice, and full card API JSON.

| Relative path | Official source URL | Media type |
|---|---|---|
| `rulebook/rulebook-current.pdf` | `https://sorcerytcg.com/news/sorcery-contested-realm-december-2025-rulebook-update` | `application/pdf` |
| `formats/constructed-current.html` | `https://sorcerytcg.com/constructed` | `text/html` |
| `codex/codex-current.html` | `https://curiosa.io/codex` | `text/html` |
| `codex/faqs-current.html` | `https://curiosa.io/faqs` | `text/html` |
| `codex/changelog-current.html` | `https://curiosa.io/codex/changelog` | `text/html` |
| `updates/card-updates-2025.html` | `https://sorcerytcg.com/news/sorcery-contested-realm-card-updates-2025` | `text/html` |
| `cards/cards.raw.json` | `https://api.sorcerytcg.com/api/cards` | `application/json` |

There is no second acquisition branch: project code performs no fetch, no scraping, no polling, no API client, and no automated acquisition or update. It does not expose a public/network HTTP card API. Later code reads the selected validated local revision through a local TypeScript catalog/query module.

The official API page describes developer access, intermittent polling, and self-hosting of required data, while the publisher Terms restrict automated access and systematic database construction without written permission. Those statements remain in tension. For v1, only the user-controlled manual browser save above is permitted. Future automation requires written publisher permission and a separate plan.

## Private source-set lock and backup

Plan 07 must block unless it records the user's exact attestation: **private, noncommercial, no redistribution**. It must create a seven-entry source set lock containing, for every row:

- the fixed relative path and official URL above;
- retrieval date/time and effective date when published;
- media type, byte length, and `sha-256` byte hash;
- acquisition/derivation method (`manual-browser-save` for the raw source) and parent hashes for later derivatives; and
- the reviewed rulebook filename/manual-route evidence required by Plan 07.

The lock must also contain the canonical source-set root hash. Hashes prove byte identity, not publisher authenticity or legal permission.

The user must maintain a durable, independent, byte-identical private backup source root outside the repository. It may not be the same, nested, linked, junctioned, symlinked, or hard-linked storage identity as the primary. Every backup relative path, byte length, and SHA-256 must match the primary; its private absolute locator appears only in the Git-ignored lock. Missing or mismatched primary/backup evidence fails closed.

## Storage and repository boundary

The anchored `.local/authority/` ignore rule protects every repository-side official input, private source/input lock, normalized snapshot, derived build input, and built official authority revision. They remain private/local, Git-ignored, and excluded from every package. The independent backup is likewise private and unpackaged outside the repository.

Git may contain only:

- project-owned TypeScript, strict schemas, policy, and documentation;
- synthetic or fact-minimal independently authored fixtures;
- official source URLs and retrieval/effective dates;
- independent non-content hashes and receipts without private absolute locators; and
- generic local-import, validation, and final-gate tests.

The current official rulebook PDF, saved official HTML pages, and full API corpus may exist only in the private primary and backup source roots. They must not enter the built revision, Git, or packages. Card art is excluded entirely. Full corpus bytes, normalized official derivatives, copied community data, and copied external code, tests, assets, or card implementations are also forbidden from Git, packages, and the built revision.

## Community and commercial-source audit

No audited community source is an approved gameplay corpus:

| Source | Current classification | Allowed use |
|---|---|---|
| `sadkinglabs/sorcery-registry` | Technically preferred for stable IDs, schema, and checksums, but MIT covers code only; card content is reserved to Erik's Curiosa and needs permission. | Behavioral/schema observation with clean-room evidence only. No card-data import. |
| `realms-cards/contested-realms` | GPL-3.0 code; its license grants no rights to publisher card content. | Behavioral observation only; no copied or closely translated code, tests, assets, card implementations, or data. |
| `JollyGrin/sorcery-tcg-playtest` / spells.bar | No reusable license in the audited revision. | Behavioral and UX observation only. |
| `sorcery-cards` | “No Rights Included”; no reusable license grant. | Reference only; no copying or corpus use. |
| JustTCG | Potential later optional licensed price enrichment, not gameplay authority. | Out of the v1 gameplay catalog; a separate licensed integration would be required. |

Other audited candidates are incomplete, copyleft, unlicensed, or publisher-reserved. None is a substitute corpus and none may become normative authority.

## Clean-room evidence

An external behavioral observation may influence an independent implementation only when its review record contains:

- source project/URL and exact source revision;
- the behavior observed, without copied expression;
- reviewer and review date; and
- an attestation that the resulting project code/tests/assets/data were independently authored and contain no copied external code, tests, assets, card implementations, or data.

Community observations remain provenance/examples only. Official sources alone determine rules and card facts under `docs/authority-precedence.md`.

## Permission gate for broader use

Sharing, release with publisher content, packaging or publication of official/derived content, automated updating, commercialization, a public/network card API, or any changed operating scope requires written publisher permission and a separate approved plan before implementation. Artwork requires its own explicit grant and remains excluded unless that grant is recorded. Written permission is a future expansion trigger, not a blocker to the private manual-import path documented here.
