# Private authority data and external reuse policy

This policy implements D-08, D-09, and D-11 for the current private, local, noncommercial Sorcery Simulator. It is an operating boundary, not legal advice or a claim of legal certainty. The narrow Phase 1 exception below records the user's risk acceptance; it does not establish legal permission. Any broader use requires written publisher permission and a separate approved plan.

## Exactly four closed Phase 1 acquisition evidence identities

The project recognizes exactly four closed acquisition method/reference identities:

- `acquisitionMethod: user-run-one-shot-powershell` with `authorizationReference` absent; this is historical evidence only.
- `acquisitionMethod: user-authorized-agent-run-one-shot-powershell` with `authorizationReference: quick-260825-mhh` and its durable ignored consumed record; this initial 2026-08-25 attempt remains consumed forever.
- `acquisitionMethod: user-authorized-agent-run-one-shot-powershell` with `authorizationReference: quick-260825-mhh-retry-1` and its distinct durable ignored consumed record; this later 2026-08-25 attempt remains consumed forever.
- `acquisitionMethod: user-authorized-user-run-one-shot-powershell` with `authorizationReference: phase-01-20260827-private-reacquisition-1` and `.local/authority/authorizations/phase-01-20260827-private-reacquisition-1.consumed.json`; this is the one user-personal attempt for `official-2026-08-27-v3`.

The first three identities and every `official-2026-08-20` root, lock, receipt, and authorization record are immutable historical evidence. Historical locks that satisfy the existing verifier remain eligible only for read-only verification; none can authorize new transport. Production acquisition now accepts only the exact fourth identity together with `-AcknowledgePrivateUseRisk`.

The user explicitly authorized one additional private-local, noncommercial, no-redistribution acquisition of the fixed seven official non-artwork sources and acknowledged the private-use risk. The user—not an agent, test, scheduled job, or recurring process—must personally run this exact command once from the repository root:

```powershell
$backupRoot = [IO.Path]::GetFullPath((Join-Path (Split-Path -Parent (Get-Location).Path) 'sorcery-tcg-authority-backup-official-2026-08-27-v3')); pwsh -NoProfile -NonInteractive -File .\scripts\collect-private-authority.ps1 -BackupRoot $backupRoot -AcknowledgePrivateUseRisk -UserAuthorizationReference phase-01-20260827-private-reacquisition-1
```

The collector hard-binds the repository-local primary to `.local/authority/inputs/official-2026-08-27-v3/primary/`, the lock to `.local/authority/locks/official-2026-08-27-v3/source-set-lock.json`, and the independent backup to the fixed sibling named `sorcery-tcg-authority-backup-official-2026-08-27-v3` beneath the repository parent. All four fresh destinations must be absent before the command runs.

The fourth authorization is atomically consumed before request construction, descriptor or destination preflight, or transport. Success, failure, cleanup, or deletion of primary, backup, lock, or other outputs never renews it; the consumed record must not be removed, reset, changed, or reused. A failed attempt is a hard stop requiring a separate future explicit authorization and plan, never a rerun or evasion.

The collector acquires exactly one complete official source set: the current standard rulebook PDF, base Constructed page, Codex, FAQ, Codex changelog, official card update notice, and full card API JSON. The authorization and resulting hashes establish neither publisher permission, publisher authenticity, nor legal clearance and create no standing or broader permission.

| Relative path | Official source URL | Media type |
|---|---|---|
| `rulebook/rulebook-current.pdf` | `https://sorcerytcg.com/news/sorcery-contested-realm-december-2025-rulebook-update` | `application/pdf` |
| `formats/constructed-current.html` | `https://sorcerytcg.com/constructed` | `text/html` |
| `codex/codex-current.html` | `https://curiosa.io/codex` | `text/html` |
| `codex/faqs-current.html` | `https://curiosa.io/faqs` | `text/html` |
| `codex/changelog-current.html` | `https://curiosa.io/codex/changelog` | `text/html` |
| `updates/card-updates-2025.html` | `https://sorcerytcg.com/news/sorcery-contested-realm-card-updates-2025` | `text/html` |
| `cards/cards.raw.json` | `https://api.sorcerytcg.com/api/cards` | `application/json` |

There is no configurable or recurring acquisition branch. The production wrapper exposes no URL, manifest, artwork, scheduling, polling, concurrency, retry, timeout, update, or authorization-record override. It makes one bounded sequential pass over the fixed seven-source manifest, publishes no lock on failure, and stops on HTTP 401, 403, 429, CAPTCHA/block evidence, or publisher objection. It performs no retry and no evasion. It exposes no public/network HTTP card API. Later code reads only the selected validated local revision through a local TypeScript catalog/query module.

The official API page describes developer access, intermittent polling, and self-hosting of required data, while the publisher Terms restrict automated access and systematic database construction without written permission and the API host's `robots.txt` disallows bots. The user accepts the identified private-use risk from this unresolved API/Terms/robots conflict by supplying `-AcknowledgePrivateUseRisk`. The collector does not establish legal permission. All other acquisition remains blocked; written publisher permission and a separate plan are required, as they are for recurring, scheduled, or unattended acquisition.

## Private source-set lock and backup

The collector must block unless the user supplies the explicit risk acknowledgment and exact current reference. A current v3 run creates a seven-entry source set lock containing the exact fourth method/reference identity above and exact absolute private roots; historical locks retain their immutable historical identities for read-only verification. Every lock records:

- the fixed relative path and official URL above;
- retrieval date/time and effective date when published;
- media type, byte length, and `sha-256` byte hash;
- exact response-completion evidence produced by the fixed collector; and
- the reviewed standard-rulebook filename and non-normative acquisition-route evidence required by Plan 07.

The lock also contains the canonical source-set root hash and the exact operating acknowledgment: private/local/noncommercial use; no redistribution, release, hosting, third-party upload, or artwork; acceptance of the unresolved API/Terms/robots risk; no legal-permission claim; and stop without retry or evasion on 401/403/429/CAPTCHA/block or publisher objection. Hashes prove byte identity, not publisher authenticity or legal permission.

The user must maintain a durable, independent, byte-identical private backup source root outside the repository. It may not be the same, nested, linked, junctioned, symlinked, or hard-linked storage identity as the primary. Every backup relative path, byte length, and SHA-256 must match the primary; its private absolute locator appears only in the Git-ignored lock. Missing or mismatched primary/backup evidence fails closed.

## Storage and repository boundary

The anchored `.local/authority/` ignore rule protects every repository-side official input, private source/input lock, normalized snapshot, derived build input, and built official authority revision. Normalized official derivatives may exist only inside the ignored private built revision selected for offline importer and runtime use. Every private authority artifact remains private/local, Git-ignored, and excluded from every package. The independent backup is likewise private and unpackaged outside the repository. Broader use requires written publisher permission and a separate plan approved for that scope.

Git may contain only:

- project-owned TypeScript, strict schemas, policy, and documentation;
- synthetic or fact-minimal independently authored fixtures;
- official source URLs and retrieval/effective dates;
- independent non-content hashes and receipts without private absolute locators; and
- generic local-import, validation, and final-gate tests.

The current official rulebook PDF, saved official HTML pages, and full API corpus may exist only in the private primary and backup source roots. Those raw publisher PDF, HTML, and API corpus bytes must not enter the built revision, Git, or packages. The collector acquires no artwork and exposes no art endpoint. Pending written permission, a later browser GUI may display only user-supplied local images from private storage through a separate permission-reviewed task. Bulk art acquisition, private-CDN access, hotlinking, proxying, hosting, packaging, and redistribution remain prohibited. Normalized official derivatives are forbidden from Git, packages, sharing, hosting, third-party upload, redistribution, public or network APIs, and commercialization without written publisher permission and a separate approved plan. Copied community data and copied external code, tests, assets, or card implementations remain excluded from Git, packages, and the built revision.

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

Sharing, release with publisher content, packaging or publication of official/derived content, recurring acquisition, all other acquisition, commercialization, a public/network card API, bulk art, private-CDN access, or any changed operating scope requires written publisher permission and a separate approved plan before implementation. There is no redistribution and no artwork acquisition under this exception; artwork requires its own explicit grant. Until then, the only GUI allowance is separately supplied private local images under the limited boundary above. Written permission is a future expansion trigger, not a claim that any narrow one-shot evidence pair is legally cleared.
