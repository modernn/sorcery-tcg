# Quick Task: User-Run Private Authority Collector - Research

**Researched:** 2026-08-20  
**Domain:** One-shot Windows PowerShell acquisition, validation, backup, and provenance receipt  
**Confidence:** HIGH for the current endpoints and existing verifier contract; MEDIUM for future Google Drive download behavior

<user_constraints>
## User Constraints (from CONTEXT.md)

### Locked Decisions

Source: `.planning/phases/01-rules-and-data-authority/01-CONTEXT.md`, copied verbatim. [VERIFIED: codebase read]

### Official authority and precedence
- **D-01:** Official Sorcery rulebooks, official format rules, the official Codex/FAQ, official card updates, and official card data are the only normative authorities. Community tools and deck sites may supply examples or provenance, never rules.
- **D-02:** The bundle must document an explicit precedence order and effective date. A later official clarification or erratum overrides older text; unresolved conflicts and ambiguous rulings are recorded as unsupported instead of guessed.
- **D-03:** Games and experiments never read mutable live authority data. They select one validated local bundle by ID and content hash.

### Snapshot and artifact identity
- **D-04:** Authority updates create a new immutable revision; existing revisions are never edited in place. Each revision records source URL, retrieval timestamp, effective date when available, media type, SHA-256, and derivation metadata.
- **D-05:** Normalized cards retain an official source identifier when available plus a stable project ID. Normalization is deterministic, schema-versioned, and reproducible from the pinned raw input.
- **D-06:** Rules, cards, formats, decks, collections, behaviors, and experiments use the same minimal provenance envelope: artifact kind, stable ID, schema version, content hash, and parent/source references.
- **D-07:** Canonical JSON uses one project-owned deterministic serialization routine before SHA-256 hashing. Validation reports exact paths and never silently repairs or drops malformed data.

### Distribution and reuse boundary
- **D-08 (amended 2026-08-20, refined after verification):** For this private, noncommercial tool, the user manually saves one complete official source set beneath `.local/authority/inputs/official-2026-08-20/primary/`: current rulebook PDF, base Constructed page/export, Codex, FAQs, Codex changelog, official card-update notice, and full API JSON, at the fixed relative paths recorded by Plan 07. The user maintains an independent byte-identical private backup set outside the repository. Every entry records its official URL, retrieval/effective date, media type, byte length, and SHA-256; project code performs no fetch, scraping, polling, or automated acquisition. All raw sources, private locks, normalized snapshot, and built authority revision remain under git-ignored private roots and are never committed or packaged. Git may contain only project code/schemas/policies, synthetic or minimal fixtures, safe relative source metadata and independent hashes/receipts, and generic local-import/final-gate tests; card art and all publisher/API corpus bytes remain excluded from Git and packages.
- **D-09:** Contested Realms (GPL-3.0) and spells.bar/the playtest project (no reusable license found in the audited revision) are behavioral and UX references only. Do not copy their source, card implementations, assets, or data into this project.
- **D-10:** Phase 1 uses TypeScript and Node standard-library facilities first. Add a dependency only where runtime schema validation or deterministic normalization is materially safer than a small local implementation.
- **D-11 (added 2026-08-20):** The current operating scope is private, local, and noncommercial. Sharing, releasing with publisher content, automated updating, or commercialization requires written publisher permission and a separate approved plan. No public/network HTTP card API is built; later consumers query the selected validated local JSON revision through a TypeScript module.

### the agent's Discretion

Source: `.planning/phases/01-rules-and-data-authority/01-CONTEXT.md`, copied verbatim. [VERIFIED: codebase read]

- Exact directory names, JSON field ordering, command names, and test file layout, provided the offline validation and immutable provenance requirements remain obvious.
- Exact safe committed receipt fields and non-sensitive backup label/fingerprint; absolute private source/backup locators stay only in the git-ignored source-set lock.

### Deferred Ideas (OUT OF SCOPE)

Source: `.planning/phases/01-rules-and-data-authority/01-CONTEXT.md`, copied verbatim. [VERIFIED: codebase read]

- Owned collection import and common online deck acquisition belong to Phase 6.
- Game-rule execution belongs to Phases 3 and 4.
- Model competitors belong to Phase 7; browser human play belongs to Phase 9.

### Latest Task Amendment

The current quick-task brief explicitly asks for and accepts a **user-run, one-shot PowerShell collector**. That supersedes D-08's manual-browser/no-automation implementation choice only for this scoped task; it does not grant publisher permission or expand the private, noncommercial, no-redistribution boundary. [VERIFIED: quick-task brief]

The implementation must update the contradictory manual-only wording in `docs/external-reuse-policy.md`, Plan 01-07/checkpoint instructions, and the attestation before treating the collector path as accepted. It must not preserve the sentence claiming every file was manually browser-saved, because that would be false after the collector runs. [VERIFIED: codebase read; VERIFIED: quick-task brief]
</user_constraints>

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|---|---|---|
| DATA-01 | Build an immutable authority bundle with pinned official rules sources, URLs, dates, and hashes. | The fixed seven-source map, fail-closed download flow, and verifier-compatible receipt provide the required pinned evidence. [VERIFIED: `.planning/REQUIREMENTS.md`; VERIFIED: codebase read] |
| DATA-02 | Build and validate a versioned normalized card snapshot without live network access during games or experiments. | The collector stores one bounded raw JSON snapshot and the existing verifier parses it before later offline import. [VERIFIED: `.planning/REQUIREMENTS.md`; VERIFIED: `src/authority/private-source-set.ts`] |
| DATA-03 | Give canonical artifacts stable IDs, schemas, provenance, and content hashes. | The receipt preserves the existing seven-entry schema and delegates canonical source-set hashing to `verifyPrivateSourceSet`. [VERIFIED: `.planning/REQUIREMENTS.md`; VERIFIED: `src/authority/private-source-set.ts`] |
</phase_requirements>

## Summary

Implement one PowerShell 7 script that owns only acquisition and staging; reuse `verifyPrivateSourceSet` for the final trust decision and source-set root hash. The script should perform one bounded GET per ordinary source, one GET of the official rulebook release page to discover the standard rulebook, and one bounded PDF GET. It should expose no general URL arguments, should never request artwork, and should publish no receipt until both source trees pass the existing verifier. [VERIFIED: official endpoint probes; VERIFIED: codebase read]

The exact rulebook case is important: the normative URL is an HTML release page dated 19 December 2025. Its standard link points to a Google Drive viewer, not PDF bytes; the current download route ends at an `application/octet-stream` attachment named `SorceryRulebook.pdf` whose bytes start `%PDF-1.4`. The annotated link is a separate file and must not be selected. Keep the release page URL in `entries[].url` and `rulebookAcquisitionEvidence.sourceUrl`; record the Drive locator only as non-normative acquisition evidence. [VERIFIED: official endpoint probes 2026-08-20; CITED: https://sorcerytcg.com/news/sorcery-contested-realm-december-2025-rulebook-update]

The publisher's public API page invites card-data access and intermittent self-hosting, while the Terms prohibit automated/non-human access and systematic retrieval without written permission. The user's decision to run the collector does not resolve that tension or establish permission, so the result must remain private/noncommercial/unshared and the receipt must say so plainly. [CITED: https://api.sorcerytcg.com/; CITED: https://sorcerytcg.com/terms]

**Primary recommendation:** use a fixed-manifest, one-shot PowerShell 7 collector with manual redirect allowlists, streaming size limits, whole-tree staging, `CreateNew`/no-overwrite publication, an ordinary copied backup, and a final call to the existing TypeScript verifier. [RECOMMENDED]

## Architectural Responsibility Map

| Capability | Primary Tier | Secondary Tier | Rationale |
|---|---|---|---|
| User authorization to run collection | Local CLI / user | — | The user initiates a single explicit run; there is no scheduler or background updater. [VERIFIED: quick-task brief] |
| Source acquisition and redirect checks | Local PowerShell tooling | External official sites | The script owns bounded HTTP transport; official services remain untrusted inputs. [RECOMMENDED] |
| Raw content validation | Local PowerShell tooling | — | Reject wrong status, redirect, size, signature, media type, and JSON shape before publication. [RECOMMENDED] |
| Primary source persistence | Repository-local ignored storage | Filesystem | The fixed primary root remains under `.local/authority/` and is already ignored. [VERIFIED: `.gitignore`; VERIFIED: codebase read] |
| Independent backup | Filesystem / storage | Local PowerShell tooling | The user supplies an absolute root outside the repository; the collector creates ordinary copied files and the verifier checks identities. [VERIFIED: `src/authority/private-source-set.ts`] |
| Receipt and source-set root hash | Existing TypeScript authority layer | Local PowerShell tooling | PowerShell collects byte evidence; `verifyPrivateSourceSet` remains the canonical validation/hash owner. [VERIFIED: `src/authority/private-source-set.ts`] |

## Project Constraints (from AGENTS.md)

- Use Podman rather than Docker; no container is needed for this task. [VERIFIED: `AGENTS.md`]
- Prefer MCP/authoritative sources; current endpoint evidence came from the official sites and direct endpoint probes. [VERIFIED: `AGENTS.md`]
- Use independent agents in parallel when practical; endpoint verification was delegated independently. [VERIFIED: `AGENTS.md`]
- Commit small verified changes with descriptive messages. [VERIFIED: `AGENTS.md`]
- Include the smallest meaningful automated test in implementation plans. [VERIFIED: `AGENTS.md`]
- Keep official content private, ignored, untracked, unstaged, and unpackaged; exclude card art. [VERIFIED: `AGENTS.md`; VERIFIED: `docs/external-reuse-policy.md`]
- The PowerShell script is local acquisition tooling, not a change to the TypeScript requirement for the engine, simulator, agents, or GUI. [VERIFIED: `AGENTS.md`; VERIFIED: quick-task brief]

## Standard Stack

### Core

| Component | Version | Purpose | Why Standard |
|---|---:|---|---|
| PowerShell (`pwsh`) | 7.6.5 available | User-run Windows entry point | Already installed; use `#requires -Version 7.0` and invoke with `pwsh -NoProfile -NonInteractive -File`. [VERIFIED: environment probe] |
| `System.Net.Http.HttpClient` | .NET 10 assembly in pwsh 7.6.5 | Streaming GETs and explicit redirect handling | `AllowAutoRedirect = $false` exposes every 3xx response so the script can enforce HTTPS and host allowlists. [VERIFIED: environment probe; CITED: https://learn.microsoft.com/en-us/dotnet/api/system.net.http.httpclienthandler.allowautoredirect] |
| `System.IO` | bundled | `CreateNew` files, staging directories, flush, copy, and no-overwrite moves | `FileMode.CreateNew` and `Directory.Move` fail when destinations already exist. [CITED: https://learn.microsoft.com/en-us/dotnet/api/system.io.filemode; CITED: https://learn.microsoft.com/en-us/dotnet/api/system.io.directory.move] |
| `Get-FileHash` | bundled | SHA-256 byte receipts | It supports explicit SHA-256; lowercase its returned hex and prefix `sha256:` for the verifier. [CITED: https://learn.microsoft.com/en-us/powershell/module/microsoft.powershell.utility/get-filehash] |
| Existing `verifyPrivateSourceSet` | repository commit `45d59f8` | Final root, identity, equality, JSON, metadata, and canonical root verification | It already enforces exactly seven paths, strict entry fields, bounded bytes, independent roots, ordinary files, matching hashes, and JSON parsing. [VERIFIED: codebase read] |
| Node.js / `node:test` | 24.19.0 | Verifier bridge and local HTTP integration tests | Already pinned and installed; no Pester dependency is necessary. [VERIFIED: `package.json`; VERIFIED: environment probe] |

### Supporting

| Component | Purpose | When to Use |
|---|---|---|
| `System.Text.UTF8Encoding($false, $true)` | Strict UTF-8 decoding and BOM-free JSON receipt writing | HTML/JSON validation and atomic receipt staging. [CITED: https://learn.microsoft.com/en-us/dotnet/api/system.text.utf8encoding] |
| `ConvertFrom-Json` / `ConvertTo-Json` | Parse API JSON and serialize the private receipt | Use for syntactic/shape validation and transport only; do not reproduce the project's canonical identity hash in PowerShell. [CITED: https://learn.microsoft.com/en-us/powershell/module/microsoft.powershell.utility/convertfrom-json; CITED: https://learn.microsoft.com/en-us/powershell/module/microsoft.powershell.utility/convertto-json] |

### Alternatives Considered

| Instead of | Could Use | Tradeoff |
|---|---|---|
| Manual `HttpClient` redirects | `Invoke-WebRequest -MaximumRedirection` | Shorter, but it hides intermediate redirect targets and makes per-hop host enforcement harder; use `HttpClient` here. [CITED: https://learn.microsoft.com/en-us/dotnet/api/system.net.http.httpclienthandler.allowautoredirect] |
| Existing Node verifier | Reimplement canonical JSON/root hashing in PowerShell | Duplicates a security-critical contract and risks incompatible receipts; do not do it. [VERIFIED: `src/authority/private-source-set.ts`] |
| `node:test` | Installed Pester 3.4.0 | Pester is unnecessary and the installed version is old; the repository already runs `node:test`. [VERIFIED: environment probe; VERIFIED: `package.json`] |
| Fixed one-shot manifest | Generic downloader/updater | A generic URL surface broadens SSRF and provenance risk and would exceed the accepted one-shot scope. [RECOMMENDED; VERIFIED: quick-task brief] |

**Installation:** none. No package legitimacy audit is required because this task adds no package. [VERIFIED: environment audit]

## Current Source Manifest and Download Behavior

The following probes were performed on 2026-08-20. Observed lengths and ETags are diagnostics only and must **not** be hardcoded; the verifier binds whatever accepted bytes the user actually retrieves. [VERIFIED: official endpoint probes 2026-08-20]

| Locked relative path | Normative URL | Current response / validation | Effective date |
|---|---|---|---|
| `rulebook/rulebook-current.pdf` | `https://sorcerytcg.com/news/sorcery-contested-realm-december-2025-rulebook-update` | Release page: 200 `text/html`, 77,990 bytes. Select exactly the non-annotated December 2025 anchor, download its PDF, accept `application/pdf` or `application/octet-stream`, and require `%PDF-`. [VERIFIED: official endpoint probe; CITED: https://sorcerytcg.com/news/sorcery-contested-realm-december-2025-rulebook-update] | `2025-12-19` [CITED: https://sorcerytcg.com/news/sorcery-contested-realm-december-2025-rulebook-update] |
| `formats/constructed-current.html` | `https://sorcerytcg.com/constructed` | 200 `text/html`, 33,825 bytes, HTML signature; require stable marker `Constructed Format`. [VERIFIED: official endpoint probe; CITED: https://sorcerytcg.com/constructed] | `null` (no page-wide effective date found). [VERIFIED: official page read] |
| `codex/codex-current.html` | `https://curiosa.io/codex` | 200 `text/html`, 494,736 bytes, HTML signature; require stable marker `Welcome to the Codex`. [VERIFIED: official endpoint probe; CITED: https://curiosa.io/codex] | `null` (no page-wide effective date found). [VERIFIED: official page read] |
| `codex/faqs-current.html` | `https://curiosa.io/faqs` | 200 `text/html`, 1,137,176 bytes, HTML signature; require stable marker `FAQs`. [VERIFIED: official endpoint probe; CITED: https://curiosa.io/faqs] | `null` (no page-wide effective date found). [VERIFIED: official page read] |
| `codex/changelog-current.html` | `https://curiosa.io/codex/changelog` | 200 `text/html`, 629,757 bytes, HTML signature; require `Codex Changelog` and parse the first displayed changelog date. [VERIFIED: official endpoint probe; CITED: https://curiosa.io/codex/changelog] | Observed latest entry `2026-07-15`; derive it from accepted content so a later update is not mislabeled. [CITED: https://curiosa.io/codex/changelog] |
| `updates/card-updates-2025.html` | `https://sorcerytcg.com/news/sorcery-contested-realm-card-updates-2025` | 200 `text/html`, 79,225 bytes, HTML signature; require stable article title. [VERIFIED: official endpoint probe; CITED: https://sorcerytcg.com/news/sorcery-contested-realm-card-updates-2025] | `2025-11-25` [CITED: https://sorcerytcg.com/news/sorcery-contested-realm-card-updates-2025] |
| `cards/cards.raw.json` | `https://api.sorcerytcg.com/api/cards` | 200 `application/json`, 1,409,089 bytes, raw nonempty JSON array beginning with card objects. The endpoint advertised a three-request rate limit and ignored Range, so issue exactly one GET and stream it once. [VERIFIED: official endpoint probe; CITED: https://api.sorcerytcg.com/api/cards] | `null` (no snapshot-wide effective date in the response). [VERIFIED: official endpoint read] |

### Safe Rulebook Resolution

1. GET only the exact official release URL and require 200, UTF-8 HTML, the expected article title, and the published date. [RECOMMENDED; CITED: https://sorcerytcg.com/news/sorcery-contested-realm-december-2025-rulebook-update]
2. Extract anchor candidates from the accepted HTML, HTML-decode both `href` and visible text, and require exactly one whose normalized text is `Sorcery: Contested Realm Rulebook (December 2025)`. Explicitly reject text containing `Annotated`. [VERIFIED: official release-page HTML]
3. Require an HTTPS `drive.google.com` URI with no credentials and a path matching `/file/d/{id}/view`; never accept a user-supplied locator. The current standard viewer is `private-rulebook-locator-redacted`. [VERIFIED: official release-page HTML]
4. Convert only that validated Drive file ID to `https://drive.google.com/uc?export=download&id={escaped-id}`. The current route returns 303 to `https://drive.usercontent.google.com/download?...` and then 200. Follow at most five redirects manually, allow only `drive.google.com` then `drive.usercontent.google.com`, and reject HTTP downgrade, credentials, missing/relative-invalid `Location`, loops, and every other host. [RECOMMENDED; VERIFIED: current Drive probe; CITED: https://learn.microsoft.com/en-us/dotnet/api/system.net.http.httpclienthandler.allowautoredirect]
5. Require final status 200, a bounded streamed body, `Content-Disposition` filename `SorceryRulebook.pdf`, and `%PDF-` magic. Save it only to the locked local name `rulebook-current.pdf`; remote filenames never control a path. [VERIFIED: current Drive probe]
6. Record the official release page as normative `url`/`sourceUrl`; record the Drive viewer/download evidence only in the ignored `rulebookAcquisitionEvidence` with `privateLocatorIsNormative: false`. [VERIFIED: Plan 01-07; VERIFIED: existing lock contract]

The current standard download is 72,701,511 bytes and begins `%PDF-1.4`; the annotated PDF is a different 72,702,841-byte file named `SorceryRulebook-Annotated.pdf`. These observations are useful test fixtures for selection logic but are not stable production pins. [VERIFIED: official endpoint probes 2026-08-20]

## Architecture Patterns

### System Architecture Diagram

```text
User runs pwsh with absolute backup root
                  |
                  v
Preflight: fixed repo + absent final roots + absent receipt
                  |
                  v
Fixed seven-source manifest (no URL parameters, no artwork)
                  |
                  v
Manual redirect loop ---- bad scheme/host/status/limit ---> fail
                  |
                  v
Sibling staging trees ---- oversize/signature/JSON error --> fail
                  |
                  v
Ordinary byte copies to backup staging
                  |
                  v
No-overwrite directory publication: backup, then primary
                  |
                  v
Existing verifyPrivateSourceSet (roots, identities, bytes, JSON, root hash)
                  |
                  v
Atomic private receipt publication (commit marker)
```

[RECOMMENDED; VERIFIED: existing verifier contract]

### Recommended Project Structure

```text
scripts/
└── collect-private-authority.ps1             # fixed one-shot production wrapper + script-local helpers
tests/authority/
└── private-authority-collector.test.ts        # node:test local HTTP integration coverage
```

[RECOMMENDED]

### Pattern 1: Stream, Bound, Validate, Then Publish

Use `HttpCompletionOption.ResponseHeadersRead`, check status/headers first, then copy with a fixed buffer while incrementing a byte counter. Cancel the body read separately because `HttpClient.Timeout` ends at headers in this mode. Write with `FileMode.CreateNew`, flush, close, validate, and only then move the complete staging directory to its absent final name. [CITED: https://learn.microsoft.com/en-us/dotnet/api/system.net.http.httpcompletionoption; CITED: https://learn.microsoft.com/en-us/dotnet/api/system.io.filemode; CITED: https://learn.microsoft.com/en-us/dotnet/api/system.io.directory.move]

```powershell
# Source: Microsoft HttpCompletionOption and FileMode documentation.
$response = $client.SendAsync(
  $request,
  [Net.Http.HttpCompletionOption]::ResponseHeadersRead,
  $cancellation.Token
).GetAwaiter().GetResult()

$output = [IO.FileStream]::new(
  $stagedPath,
  [IO.FileMode]::CreateNew,
  [IO.FileAccess]::Write,
  [IO.FileShare]::None
)
# Read response stream in a bounded loop; fail immediately once bytes exceed the per-source cap.
```

Use the existing limits exactly: 10,000,000 bytes for card JSON, 256 MiB for each other file, and 768 MiB total. [VERIFIED: `src/authority/private-source-set.ts`]

Resolve the repository root from the script location, not the caller's current directory. The production primary root is exactly `.local/authority/inputs/official-2026-08-20/primary/`, the receipt is exactly `.local/authority/locks/official-2026-08-20/source-set-lock.json`, and the backup root is a mandatory absolute user parameter outside the repository. [VERIFIED: Plan 01-07; RECOMMENDED]

Set each `retrievedAt` only after its complete body passes transport and content validation, using `[DateTime]::UtcNow.ToString("yyyy-MM-dd'T'HH:mm:ss.fff'Z'", [Globalization.CultureInfo]::InvariantCulture)`. This produces the `Z`-terminated UTC form accepted by the existing verifier. [VERIFIED: `src/authority/private-source-set.ts`; RECOMMENDED]

### Pattern 2: Receipt Is Written Last and Reverified

Build entries with **exactly** the seven fields accepted by the verifier: `relativePath`, `url`, `retrievedAt`, `effectiveDate`, `mediaType`, `byteLength`, and lowercase `byteHash`. Extra entry fields fail validation. [VERIFIED: `src/authority/private-source-set.ts`]

After both trees are published, have the PowerShell script invoke Node from the repository root with a short in-memory module that reads the draft, calls `verifyPrivateSourceSet`, and returns its sorted entries plus `sourceSetRootHash`. PowerShell then writes the complete private lock to a sibling temporary file with BOM-free UTF-8, re-reads it, invokes the verifier again, and moves it to the absent final receipt path. Do not reproduce `identityHash` or canonical JSON in PowerShell. [RECOMMENDED; VERIFIED: `src/authority/private-source-set.ts`; VERIFIED: Plan 01-07]

The private lock shape should remain:

```json
{
  "schemaVersion": 1,
  "primaryRoot": "<absolute ignored primary root>",
  "backupRoot": "<absolute outside-repository root>",
  "entries": ["<seven strict verifier entries>"],
  "sourceSetRootHash": "sha256:<canonical verifier result>",
  "rulebookAcquisitionEvidence": {
    "sourceUrl": "https://sorcerytcg.com/news/sorcery-contested-realm-december-2025-rulebook-update",
    "relativePath": "rulebook/rulebook-current.pdf",
    "byteHash": "sha256:<PDF bytes>",
    "observedFilename": "SorceryRulebook.pdf",
    "retrievedAt": "<UTC timestamp>",
    "privateLocatorEvidence": "<non-normative Drive viewer locator>",
    "privateLocatorIsNormative": false
  }
}
```

[VERIFIED: Plan 01-07 lock contract]

### Pattern 3: Test the Core Without Exposing Arbitrary Production URLs

Keep fixed production descriptors in the CLI wrapper. Put transport/validation orchestration in script-local functions that accept a descriptor list only so the test harness can dot-source the script and supply loopback fixtures; do not expose `-Url`, `-Manifest`, or `-BaseUri` in the production parameter set. [RECOMMENDED]

### Anti-Patterns to Avoid

- **Saving the Drive viewer:** it returns HTML, not the rulebook PDF. [VERIFIED: official endpoint probe]
- **Matching the first Drive link:** the page also links an annotated PDF; match exact visible text and reject `Annotated`. [VERIFIED: official release page]
- **Trusting `Content-Type` alone:** the real PDF currently arrives as `application/octet-stream`; validate `%PDF-`, HTML signatures/markers, and JSON syntax/shape. [VERIFIED: official endpoint probes]
- **Writing directly to final paths:** a crash can leave a partial set; stage siblings and publish complete directories with no-overwrite moves. [RECOMMENDED; CITED: https://learn.microsoft.com/en-us/dotnet/api/system.io.directory.move]
- **Copying with overwrite:** `Copy-Item -Force` can destroy existing evidence. Use `File.Copy(..., $false)` and fail when anything exists. [CITED: https://learn.microsoft.com/en-us/dotnet/api/system.io.file.copy]
- **Pretty-printing API JSON:** the receipt must hash the exact response bytes, not a parsed/reserialized representation. [VERIFIED: existing verifier byte-hash contract]
- **Using hashes as authenticity proof:** hashes prove equality with the recorded bytes, not publisher identity or legal permission. [VERIFIED: Plan 01-07; CITED: https://sorcerytcg.com/terms]
- **Reusing the manual-browser attestation:** it would contradict the actual collection method. [VERIFIED: codebase read; VERIFIED: quick-task brief]

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---|---|---|---|
| SHA-256 file hashing | Custom digest code | `Get-FileHash -Algorithm SHA256` | Bundled and produces the exact digest material required after lowercasing/prefixing. [CITED: https://learn.microsoft.com/en-us/powershell/module/microsoft.powershell.utility/get-filehash] |
| Canonical source-set root | PowerShell JSON canonicalizer | Existing `verifyPrivateSourceSet` / `identityHash` | Prevents a second, divergent identity contract. [VERIFIED: codebase read] |
| Backup independence checks | Path string comparison | Existing realpath + dev/inode verifier | It already rejects aliases, nesting, symlinks, junctions, and hardlinks. [VERIFIED: `src/authority/private-source-set.ts`] |
| Generic HTML parser | A reusable DOM/parser subsystem | One strict extraction for the exact named rulebook anchor, followed by URI and PDF validation | The task has one known discovery case; a parser dependency adds more surface than value. [RECOMMENDED] |
| Test framework | New Pester dependency | Existing `node:test` + native PowerShell exit codes | Keeps `pnpm verify` as the single project gate. [VERIFIED: `package.json`] |
| Retry/scheduler/updater | Background polling service | One explicit run; fail and report on transient errors | Recurring automation is out of scope and API requests are rate-limited. [VERIFIED: quick-task brief; VERIFIED: official API probe] |

## Common Pitfalls

### Pitfall 1: Redirect Confusion / SSRF
**What goes wrong:** an official page or redirect points the collector to an unexpected host or HTTP URL. [RECOMMENDED threat model]  
**How to avoid:** disable automatic redirects, cap hops, validate every absolute resolved URI, require HTTPS/no credentials, and use per-source host allowlists. [CITED: https://learn.microsoft.com/en-us/dotnet/api/system.net.http.httpclienthandler.allowautoredirect]  
**Warning signs:** unexpected host, missing `Location`, redirect loop, downgrade, or more than five hops. [RECOMMENDED]

### Pitfall 2: Error Page Saved as Authority
**What goes wrong:** a CDN/login/error HTML response is saved with `.pdf` or `.json`. [VERIFIED: Drive viewer behavior demonstrates this class]  
**How to avoid:** combine final status, bounded bytes, header expectations, magic bytes, strict UTF-8, source-specific HTML markers, and JSON root/shape checks. [RECOMMENDED]  
**Warning signs:** PDF does not start `%PDF-`; JSON root is not a nonempty array of objects with a string `name`; HTML lacks its stable title marker. [RECOMMENDED; VERIFIED: current API shape]

### Pitfall 3: Partial Publication or Overwrite
**What goes wrong:** interrupted downloads leave a seemingly complete final tree, or reruns replace prior evidence. [RECOMMENDED threat model]  
**How to avoid:** require absent final roots/receipt, stage under unique sibling directories with `CreateNew`, publish complete trees using no-overwrite `Directory.Move`, and write the verified receipt last. [CITED: https://learn.microsoft.com/en-us/dotnet/api/system.io.filemode; CITED: https://learn.microsoft.com/en-us/dotnet/api/system.io.directory.move]  
**Warning signs:** a final root exists without a receipt, a `.collecting-*` directory remains, or a destination already exists. [RECOMMENDED]

### Pitfall 4: Backup Is Only an Alias
**What goes wrong:** symlink, junction, hardlink, or nested paths masquerade as an independent backup. [VERIFIED: existing verifier tests]  
**How to avoid:** create ordinary copies, require an absolute backup root outside the repository, and gate the receipt on `verifyPrivateSourceSet`. [VERIFIED: `src/authority/private-source-set.ts`]  
**Warning signs:** verifier codes `filesystem_alias`, `linked_file`, `root_overlap`, or `backup_inside_repository`. [VERIFIED: existing verifier]

### Pitfall 5: Metadata Does Not Match the Verifier
**What goes wrong:** timestamps end in `+00:00`, hashes are uppercase, media type follows the octet-stream header, or entry objects include acquisition fields. [VERIFIED: existing strict field validation]  
**How to avoid:** emit UTC as `yyyy-MM-dd'T'HH:mm:ss.fff'Z'`, lowercase SHA-256 with `sha256:`, use canonical media types by locked path, and keep acquisition evidence at the receipt top level. [VERIFIED: `src/authority/private-source-set.ts`]  
**Warning signs:** `invalid_timestamp`, `invalid_hash`, `invalid_media_type`, or `unknown_field`. [VERIFIED: existing verifier]

## Validation Architecture

### Test Framework

| Property | Value |
|---|---|
| Framework | Node 24.19.0 `node:test` + `node:assert/strict`; PowerShell is spawned as the system under test. [VERIFIED: environment; VERIFIED: package scripts] |
| Config file | None; existing `package.json` glob runs `tests/authority/*.test.ts`. [VERIFIED: `package.json`] |
| Quick run command | `node --test tests/authority/private-authority-collector.test.ts` [RECOMMENDED] |
| Full suite command | `pnpm verify` [VERIFIED: `package.json`] |

### Phase Requirements -> Test Map

| Req ID | Behavior | Test Type | Automated Command | File Exists? |
|---|---|---|---|---|
| DATA-01 | Exact seven paths, rulebook selection, validation, no-overwrite publication, provenance fields | Local HTTP integration | `node --test tests/authority/private-authority-collector.test.ts` | No - Wave 0 [VERIFIED: codebase scan] |
| DATA-02 | Raw JSON bytes preserved, bounded, parsed, and accepted by existing verifier from both roots | Local HTTP integration | same | No - Wave 0 [VERIFIED: codebase scan] |
| DATA-03 | Lowercase hashes, UTC dates, deterministic sorted verifier entries, source-set root, rulebook evidence | Integration | same | No - Wave 0 [VERIFIED: codebase scan] |

### Required Local HTTP Cases

Use `http.createServer()` on `127.0.0.1` and an OS-assigned port, then spawn `pwsh -NoProfile -NonInteractive`. The fixture server supplies six HTML/JSON routes, a release page with standard and annotated links, a 303 PDF redirect, and a valid synthetic `%PDF-` body. No test contacts the internet or stores publisher bytes. [RECOMMENDED; VERIFIED: Node standard library availability]

The minimum meaningful cases are: [RECOMMENDED]

1. Happy path creates exactly seven primary files, seven ordinary backup copies, and a receipt that `verifyPrivateSourceSet` accepts.
2. Standard rulebook is selected even when annotated appears first; wrong/missing/duplicate standard anchors fail.
3. Unexpected redirect host, HTTP downgrade, loop, too many hops, non-200 status, or mid-stream disconnect fails with no receipt.
4. Oversized declared or streamed body, HTML-as-PDF, PDF-as-HTML, malformed/empty/wrong-root JSON, and missing content marker fail closed.
5. Existing primary root, backup root, destination file, or receipt is never modified.
6. Backup inside the repository and linked/aliased backup files are rejected by the existing verifier.
7. API route is requested exactly once and the saved JSON bytes equal the server bytes exactly.

### Sampling Rate

- **Per task commit:** `node --test tests/authority/private-authority-collector.test.ts` [RECOMMENDED]
- **Per wave merge:** `pnpm verify` [VERIFIED: project convention]
- **Phase gate:** focused integration test, full suite, ignored/untracked checks, then one user-run collection. [RECOMMENDED; VERIFIED: Plan 01-07]

### Wave 0 Gaps

- [ ] `tests/authority/private-authority-collector.test.ts` - local HTTP/spawn integration test. [VERIFIED: codebase scan]
- [ ] `scripts/collect-private-authority.ps1` - production collector. [VERIFIED: codebase scan]
- [ ] Update Plan 01-07 and `docs/external-reuse-policy.md` so the accepted collector method and attestation no longer contradict the implementation. [VERIFIED: codebase read]

## Environment Availability

| Dependency | Required By | Available | Version | Fallback |
|---|---|---:|---:|---|
| PowerShell Core (`pwsh`) | Collector | Yes | 7.6.5 | None needed. [VERIFIED: environment probe] |
| Windows PowerShell | Optional legacy runtime | Yes | 5.1.26100.9168 | Do not target initially; run the documented `pwsh` command. [VERIFIED: environment probe; RECOMMENDED] |
| Node.js | Verifier bridge and tests | Yes | 24.19.0 | None needed. [VERIFIED: environment probe] |
| Pester | Not required | Present but unused | 3.4.0 | Existing `node:test`. [VERIFIED: environment probe] |
| Network access to current sources | User-run collection | Yes during research | All current endpoints responded | Fail with diagnostics; no retry loop. [VERIFIED: official endpoint probes] |
| Absolute independent backup root | Publication | User-supplied at run time | — | No safe default; make it mandatory. [VERIFIED: quick-task brief; VERIFIED: existing verifier] |

**Missing dependencies with no fallback:** the user must supply an absent absolute backup destination outside the repository when running the collector. [VERIFIED: task scope]

## Security Domain

### Applicable ASVS Categories

| ASVS Category | Applies | Standard Control |
|---|---:|---|
| V2 Authentication | No | No authenticated endpoint is used. [VERIFIED: source manifest] |
| V3 Session Management | No | No session is created or persisted. [VERIFIED: architecture] |
| V4 Access Control | Yes | Fixed destination roots, absent-target checks, realpath containment, ignored private lock. [VERIFIED: existing verifier; RECOMMENDED] |
| V5 Input Validation | Yes | Fixed URLs, manual redirect allowlists, size limits, signatures, strict UTF-8/JSON, source-specific markers. [RECOMMENDED] |
| V6 Cryptography | Yes | Built-in SHA-256 for integrity/equality only; never call it publisher authentication. [CITED: https://learn.microsoft.com/en-us/powershell/module/microsoft.powershell.utility/get-filehash; VERIFIED: Plan 01-07] |

### Known Threat Patterns for the Collector

| Pattern | STRIDE | Standard Mitigation |
|---|---|---|
| Redirect to attacker-controlled host | Spoofing / Information disclosure | Manual redirect loop with HTTPS and per-source hosts. [RECOMMENDED] |
| Oversized or endless response | Denial of service | Header precheck plus streamed byte counter and body cancellation timeout. [CITED: https://learn.microsoft.com/en-us/dotnet/api/system.net.http.httpcompletionoption] |
| Wrong content behind correct URL | Spoofing / Tampering | Status, media expectation, magic bytes, source marker, JSON shape, exact hashes. [RECOMMENDED] |
| Existing evidence overwritten | Tampering / Repudiation | `CreateNew`, absent final roots, no-overwrite moves, receipt last. [CITED: https://learn.microsoft.com/en-us/dotnet/api/system.io.filemode; CITED: https://learn.microsoft.com/en-us/dotnet/api/system.io.directory.move] |
| Backup aliases primary | Tampering | Ordinary copies plus existing dev/inode/realpath verifier. [VERIFIED: existing verifier] |
| Private bytes or locators enter Git | Information disclosure | Keep all bytes/lock under ignored roots; test `git check-ignore`, `git ls-files`, and staged paths. [VERIFIED: `.gitignore`; VERIFIED: Plan 01-07] |
| Artwork fetched transitively | Information disclosure / scope violation | Fixed seven-request manifest; never follow card image/variant/CDN links. [CITED: https://api.sorcerytcg.com/; VERIFIED: task scope] |

## State of the Art / Decision Change

| Old Approach | Current Approach | Changed By | Impact |
|---|---|---|---|
| Manual browser save only; project code performs no acquisition | User explicitly runs a one-shot PowerShell collector | Current quick-task brief, 2026-08-20 | Update policy/plan/attestation before acceptance; keep later automated updating out of scope. [VERIFIED: quick-task brief; VERIFIED: codebase read] |
| User manually reports timestamps/media | Collector records response-completion UTC time, canonical media type, byte length, and hash | Current quick-task brief | Removes transcription errors while preserving the verifier schema. [RECOMMENDED] |
| Executor builds lock after manual checkpoint | Collector stages evidence, calls the existing verifier, and publishes the private lock last | Current quick-task brief | The same verifier remains authoritative; acquisition does not get to self-attest. [RECOMMENDED] |

## Assumptions Log

| # | Claim | Section | Risk if Wrong |
|---|---|---|---|
| A1 | Google Drive will continue accepting a download URI derived from the validated public file ID. [ASSUMED] | Safe Rulebook Resolution | Future Drive changes may make collection fail; fail closed and update the resolver from fresh official-page evidence rather than falling back to arbitrary scraping. |

## Open Questions

1. **Written publisher permission remains unresolved.**
   - What we know: the public API page describes developer card-data access, but the Terms prohibit automated access and systematic collection without written permission. [CITED: https://api.sorcerytcg.com/; CITED: https://sorcerytcg.com/terms]
   - What's unclear: whether the publisher authorizes this specific user-run collector. [VERIFIED: source comparison]
   - Recommendation: proceed only within the user's explicitly accepted private/noncommercial/no-redistribution scope, state that no permission is established, and keep the existing publisher-authorization follow-up open. [RECOMMENDED; VERIFIED: `.planning/STATE.md`]

## Sources

### Primary (HIGH confidence)

- [Official December 2025 rulebook release](https://sorcerytcg.com/news/sorcery-contested-realm-december-2025-rulebook-update) - standard/annotated links and publication date.
- [Official Constructed format](https://sorcerytcg.com/constructed) - live HTML source.
- [Official Codex](https://curiosa.io/codex), [FAQs](https://curiosa.io/faqs), and [Codex changelog](https://curiosa.io/codex/changelog) - live authority pages and latest visible changelog date.
- [Official 2025 card updates](https://sorcerytcg.com/news/sorcery-contested-realm-card-updates-2025) - live HTML and publication date.
- [Official card endpoint](https://api.sorcerytcg.com/api/cards) and [API landing page](https://api.sorcerytcg.com/) - live JSON shape, rate-limit behavior, and artwork exclusion statement.
- [Publisher Terms](https://sorcerytcg.com/terms) - automation, systematic retrieval, personal/noncommercial access, and permission caveats.
- [Microsoft `HttpClientHandler.AllowAutoRedirect`](https://learn.microsoft.com/en-us/dotnet/api/system.net.http.httpclienthandler.allowautoredirect) and [`HttpCompletionOption`](https://learn.microsoft.com/en-us/dotnet/api/system.net.http.httpcompletionoption) - redirect and streaming semantics.
- [Microsoft `FileMode`](https://learn.microsoft.com/en-us/dotnet/api/system.io.filemode), [`Directory.Move`](https://learn.microsoft.com/en-us/dotnet/api/system.io.directory.move), and [`File.Copy`](https://learn.microsoft.com/en-us/dotnet/api/system.io.file.copy) - no-overwrite staging/publication behavior.
- [Microsoft `Get-FileHash`](https://learn.microsoft.com/en-us/powershell/module/microsoft.powershell.utility/get-filehash), [`ConvertFrom-Json`](https://learn.microsoft.com/en-us/powershell/module/microsoft.powershell.utility/convertfrom-json), and [`ConvertTo-Json`](https://learn.microsoft.com/en-us/powershell/module/microsoft.powershell.utility/convertto-json) - hashing and JSON APIs.
- `src/authority/private-source-set.ts`, `tests/authority/private-source-set.test.ts`, Plan 01-07, `docs/external-reuse-policy.md`, `.planning/REQUIREMENTS.md`, and `.gitignore` - repository contract and current policy conflict. [VERIFIED: codebase read]

### Secondary (MEDIUM confidence)

- None; external implementation claims were checked against official documentation or live primary endpoints. [VERIFIED: research log]

### Tertiary (LOW confidence)

- Future Google Drive download URL stability is unknown and is isolated in Assumption A1. [ASSUMED]

## Metadata

**Confidence breakdown:**
- Standard stack: HIGH - all components are bundled, pinned, or already present. [VERIFIED: environment and codebase]
- Architecture: HIGH - it reuses the implemented verifier and platform no-overwrite primitives. [VERIFIED: codebase; CITED: https://learn.microsoft.com/en-us/dotnet/api/system.io.filemode; CITED: https://learn.microsoft.com/en-us/dotnet/api/system.io.directory.move]
- Endpoint behavior: HIGH for 2026-08-20, MEDIUM for future Drive behavior - every endpoint was probed, but third-party routing can change. [VERIFIED: official endpoint probes; ASSUMED for future]
- Legal scope: MEDIUM - official pages conflict and this research makes no legal conclusion. [CITED: https://api.sorcerytcg.com/; CITED: https://sorcerytcg.com/terms]

**Research date:** 2026-08-20  
**Valid until:** 2026-08-27 for endpoint/Drive behavior; 2026-09-19 for local architecture. [RECOMMENDED]
