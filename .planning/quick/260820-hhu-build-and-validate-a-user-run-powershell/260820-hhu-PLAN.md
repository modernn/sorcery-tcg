---
phase: quick-260820-hhu-build-and-validate-a-user-run-powershell
plan: "01"
type: execute
wave: 1
depends_on: []
files_modified:
  - scripts/collect-private-authority.ps1
  - tests/authority/private-authority-collector.test.ts
  - docs/external-reuse-policy.md
  - docs/card-data-authorization-outreach.md
  - .planning/phases/01-rules-and-data-authority/01-07-PLAN.md
autonomous: true
requirements: [DATA-01, DATA-02, DATA-03]
must_haves:
  truths:
    - "The supported direct CLI exposes only `-BackupRoot` and `-AcknowledgePrivateUseRisk`; it collects exactly the seven fixed official authority sources and exposes no supported production URL, manifest, or artwork override."
    - "The collector stops without retry, fallback, evasion, or a final receipt on 401, 403, 429, CAPTCHA/block evidence, disallowed redirects, response-size violations, malformed content, or any filesystem/verifier failure."
    - "Existing primary, backup, or lock destinations are never overwritten, downloads are bounded and staged, exact response bytes are preserved, and the private lock is published last only after verifyPrivateSourceSet accepts both final trees."
    - "The ignored lock records the fixed `acquisitionMethod: user-run-one-shot-powershell`, and Plan 01-07 rejects any other or missing method before accepting the source set."
    - "A fully synthetic loopback test suite proves the happy path, fixed provenance, standard-not-annotated rulebook selection, strict content checks, redirect controls, stop conditions, bounds, backup independence, and no-overwrite behavior without contacting the internet or storing publisher bytes."
    - "Among the three exported commands, only `Invoke-PrivateAuthorityCollectionForLoopbackTest` accepts replacement repository, primary, lock, descriptor, timeout, or byte-limit inputs, and it rejects non-loopback request and redirect hosts before filesystem mutation. Deliberate private-module invocation or modification/copying of the readable script by the machine owner is outside this interface boundary."
    - "Policy and Plan 01-07 truthfully describe the user-run one-shot collector, preserve private/noncommercial/no-redistribution and no-legal-conclusion limits, keep recurring acquisition permission-gated, and permit only user-supplied private local images in a separately approved GUI task."
  artifacts:
    - path: "scripts/collect-private-authority.ps1"
      provides: "Fixed-manifest, user-invoked PowerShell 7 collector with bounded transport, staging, independent backup, acknowledgment, and verifier-gated private lock"
      contains: "AcknowledgePrivateUseRisk"
    - path: "tests/authority/private-authority-collector.test.ts"
      provides: "Node test harness that drives the collector against a loopback synthetic HTTP server"
      contains: "verifyPrivateSourceSet"
    - path: "docs/external-reuse-policy.md"
      provides: "Truthful accepted operating boundary for the one-shot collector and private local art"
    - path: ".planning/phases/01-rules-and-data-authority/01-07-PLAN.md"
      provides: "Updated human checkpoint and attestation for running and validating the collector"
    - path: ".planning/quick/260820-hhu-build-and-validate-a-user-run-powershell/260820-hhu-VALIDATION.md"
      provides: "DATA-01/02/03 mapping to named loopback-only collector tests and release gates"
  key_links:
    - from: "tests/authority/private-authority-collector.test.ts"
      to: "scripts/collect-private-authority.ps1"
      via: "spawned pwsh process and dot-sourced loopback-only descriptor seam"
      pattern: "pwsh.*collect-private-authority"
    - from: "scripts/collect-private-authority.ps1"
      to: "src/authority/private-source-set.ts"
      via: "fixed Node bridge calling verifyPrivateSourceSet before final lock publication"
      pattern: "verifyPrivateSourceSet"
    - from: "scripts/collect-private-authority.ps1"
      to: ".local/authority/locks/official-2026-08-20/source-set-lock.json"
      via: "BOM-free staged JSON moved without overwrite only after final-tree re-verification"
      pattern: "sourceSetRootHash"
    - from: ".planning/phases/01-rules-and-data-authority/01-07-PLAN.md"
      to: "scripts/collect-private-authority.ps1"
      via: "blocking user command with an absolute backup root and explicit risk acknowledgment"
      pattern: "AcknowledgePrivateUseRisk"
---

<objective>
Build and validate the content-free, user-run one-shot collector that acquires the seven fixed private Sorcery authority sources, makes an independent backup, and publishes a verifier-approved private lock without ever exercising publisher endpoints during implementation.

Purpose: Replace the contradictory manual-browser-only path with the exact acquisition method the user selected while retaining the existing deterministic identity, private-use, fail-closed, no-artwork, and no-redistribution boundaries.
Output: One PowerShell 7 collector, one offline Node integration test file, and reconciled policy/Plan 01-07 instructions.
</objective>

<execution_context>
@C:/Users/Dad/.codex/get-shit-done/workflows/execute-plan.md
@C:/Users/Dad/.codex/get-shit-done/templates/summary.md
</execution_context>

<context>
@AGENTS.md
@.planning/STATE.md
@.planning/phases/01-rules-and-data-authority/01-CONTEXT.md
@.planning/quick/260820-hhu-build-and-validate-a-user-run-powershell/260820-hhu-RESEARCH.md
@.planning/quick/260820-hhu-build-and-validate-a-user-run-powershell/260820-hhu-LEGAL-RESEARCH.md
@.planning/quick/260820-hhu-build-and-validate-a-user-run-powershell/260820-hhu-VALIDATION.md
@.planning/phases/01-rules-and-data-authority/01-07-PLAN.md
@.planning/phases/01-rules-and-data-authority/01-08-PLAN.md
@docs/external-reuse-policy.md
@docs/card-data-authorization-outreach.md
@src/authority/private-source-set.ts
@tests/authority/private-source-set.test.ts
@package.json

<interfaces>
The collector must consume the existing authority contract without modifying or reimplementing it:

From `src/authority/private-source-set.ts`:

```typescript
export const PRIVATE_AUTHORITY_SOURCE_PATHS: readonly [
  'rulebook/rulebook-current.pdf',
  'formats/constructed-current.html',
  'codex/codex-current.html',
  'codex/faqs-current.html',
  'codex/changelog-current.html',
  'updates/card-updates-2025.html',
  'cards/cards.raw.json',
];

export type PrivateAuthoritySourceEntry = Readonly<{
  relativePath: (typeof PRIVATE_AUTHORITY_SOURCE_PATHS)[number];
  url: string;
  retrievedAt: string;
  effectiveDate: string | null;
  mediaType: 'application/pdf' | 'text/html' | 'application/json';
  byteLength: number;
  byteHash: `sha256:${string}`;
}>;

export async function verifyPrivateSourceSet(options: {
  primaryRoot: string;
  backupRoot: string;
  repositoryRoot: string;
  entries: readonly PrivateAuthoritySourceEntry[];
}): Promise<{
  entries: readonly PrivateAuthoritySourceEntry[];
  sourceSetRootHash: `sha256:${string}`;
}>;
```

The verifier already enforces the exact seven paths; official HTTPS provenance hosts; strict timestamps, dates, media types, lengths, and lowercase SHA-256 values; 10,000,000 card-JSON bytes; 256 MiB for each other source; 768 MiB total; ordinary independent primary/backup files; byte equality; valid card JSON; deterministic sorting; and canonical root hashing.
</interfaces>
</context>

<source_coverage>

| Source | ID | Feature / constraint | Task | Status | Notes |
|---|---|---|---|---|---|
| GOAL | — | User-run PowerShell collector for seven private authority sources and an independent backup | 1-3 | COVERED | Built and tested without a live run; Plan 01-07 owns the later user invocation. |
| REQ | DATA-01 | Immutable source set with fixed official URLs, dates, hashes, and backup | 1 | COVERED | Exact staged bytes and verifier-approved metadata/lock. |
| REQ | DATA-02 | Full raw card JSON usable offline | 1 | COVERED | One bounded exact-byte JSON acquisition; no game/runtime live access. |
| REQ | DATA-03 | Stable provenance and canonical content identity | 1 | COVERED | Reuses verifyPrivateSourceSet for sorted entries and sourceSetRootHash. |
| CONTEXT | D-01, D-02 | Official-only authority, precedence evidence, and effective dates | 1 | COVERED | Fixed official manifest; known dates plus parsed latest changelog date; ambiguity fails. |
| CONTEXT | D-03, D-04 | No mutable live authority during games; immutable no-overwrite revisions | 1 | COVERED | Collector is a separate explicit acquisition command and refuses existing destinations. |
| CONTEXT | D-05, D-06, D-07 | Stable IDs/provenance, deterministic canonical hashing, exact validation | 1 | COVERED | Raw acquisition preserves the existing entry/root contract; normalization remains downstream and unchanged. |
| CONTEXT | D-08 | Private source set and independent backup | 1-3 | COVERED | The amended decision records the fixed one-shot method/risk boundary; all private storage limits remain. |
| CONTEXT | D-09 | Community projects are reference-only | 1-3 | COVERED | No community code, data, tests, assets, or endpoints are used. |
| CONTEXT | D-10 | TypeScript/Node standard library first | 1 | COVERED | PowerShell 7/.NET and Node standard library only; no install. |
| CONTEXT | D-11 | Private/local/noncommercial, no public API or broader use | 1-3 | COVERED | Explicit acknowledgment and policy; no recurring updater, service, or redistribution. |
| RESEARCH | — | Fixed endpoints, explicit one-shot acknowledgment, no generic production URL input, no artwork | 1 | COVERED | Exact production wrapper and seven-entry descriptor set. |
| RESEARCH | — | Manual HTTPS redirect allowlists and exact standard rulebook resolution | 1 | COVERED | Per-hop validation, five-hop cap, exact non-annotated anchor, Drive evidence remains non-normative. |
| RESEARCH | — | Bounded streaming, strict content checks, stop/no retry/evasion conditions | 1 | COVERED | Header and streamed bounds, timeouts, signatures/markers/JSON shape, terminal failure. |
| RESEARCH | — | Sibling staging, no overwrite, independent copies, receipt last | 1 | COVERED | CreateNew/copy-without-overwrite/move-without-overwrite plus final verifier call. |
| RESEARCH | — | Loopback-only synthetic security/integration tests | 1 | COVERED | node:test and http.createServer; no internet or publisher bytes. |
| VALIDATION | — | DATA-01/02/03 map to named offline collector checks | 1-3 | COVERED | Quick validation matrix identifies commands, threats, and no-live-run gate. |
| RESEARCH | — | Reconcile manual-only documentation while making no legal conclusion | 3 | COVERED | Policy, outreach boundary, and Plan 01-07 are updated together. |
| LEGAL RESEARCH | — | User accepts identified private-use risk; no circumvention or continued access after a block | 1-3 | COVERED | Acknowledgment records risk without claiming permission; terminal stop conditions are enforced. |
| LEGAL RESEARCH | — | Artwork remains outside collector; local GUI art is user-supplied only pending written permission | 1-3 | COVERED | No image endpoints; documents preserve the separate local-image boundary. |

Deferred collection/deck import, rules execution, model competitors, browser play, recurring updates, public hosting/API, content redistribution, commercial use, and bulk artwork acquisition remain excluded.
</source_coverage>

<tasks>

<task type="auto" tdd="true">
  <name>Task 1: Lock and test the inert wrapper, production manifest, and bounded transport</name>
  <read_first>
    - AGENTS.md
    - .planning/quick/260820-hhu-build-and-validate-a-user-run-powershell/260820-hhu-RESEARCH.md
    - .planning/quick/260820-hhu-build-and-validate-a-user-run-powershell/260820-hhu-LEGAL-RESEARCH.md
    - src/authority/private-source-set.ts
    - tests/authority/private-source-set.test.ts
    - package.json
  </read_first>
  <files>tests/authority/private-authority-collector.test.ts, scripts/collect-private-authority.ps1</files>
  <behavior>
    - Dot-sourcing the script defines functions but performs zero requests and zero filesystem writes; direct execution without `-BackupRoot` or `-AcknowledgePrivateUseRisk` fails before either side effect.
    - An offline accessor exposes exactly seven production descriptors with the locked relative paths, normative URLs, request URLs, media types, source markers, effective-date policies, size caps, and redirect allowlists; the direct wrapper passes only this set to the tested collection core.
    - The direct production parameter block has no URL, manifest, base-URI, artwork, scheduling, concurrency, retry, timeout, or update override.
    - A dot-sourced `Invoke-PrivateAuthorityCollectionForLoopbackTest` seam accepts temporary repository/primary/lock destinations and reduced transport limits only with all request/redirect hosts on loopback; the direct wrapper cannot pass or select this seam.
    - The loopback transport selects the exact visible-text standard December 2025 rulebook anchor even when the annotated link appears first, follows only allowed redirects, preserves exact response bytes, and never requests embedded/card-art URLs.
    - Every redirect hop requires an absolute or safely resolved URI with the allowed scheme, no credentials, an approved per-source host, no loop, and at most five hops; unexpected host, malformed/missing Location, or downgrade fails before that target is requested.
    - 401, 403, 429, other non-success status, a challenge/block page, disconnect, header/body timeout, declared/streamed/aggregate oversize content, wrong media/signature, missing marker, wrong/missing/duplicate rulebook anchor, wrong PDF filename, malformed/empty/wrong-root card JSON, or an empty card name fails immediately with no retry, fallback, evasion, or later-source request.
  </behavior>
  <action>Write the transport/manifest cases in `tests/authority/private-authority-collector.test.ts` first and commit the failing RED contract as `test(quick-260820-hhu): lock private collector transport contract`. Use only `node:test`, `node:assert/strict`, `node:http`, `node:child_process`, and Node filesystem/path/crypto APIs. Start `http.createServer()` on `127.0.0.1` with an OS-assigned port and synthetic bodies only: five marked HTML authorities, a release page with annotated and standard anchors, a redirect chain to a small `%PDF-`/`%%EOF` file with `Content-Disposition: attachment; filename=SorceryRulebook.pdf`, and a nonempty JSON array of synthetic objects with string `name`. Invoke `pwsh -NoProfile -NonInteractive` and use resolved-temp-root assertions around cleanup. First prove dot-sourcing performs zero requests/writes. Then call only `Invoke-PrivateAuthorityCollectionForLoopbackTest`, supplying a test-owned temporary repository root, primary root beneath it, lock path beneath it, independent backup root outside it, loopback request descriptors separated from fixed official `provenanceUrl` values, and reduced time/byte caps. This seam must reject any non-loopback request/redirect before filesystem mutation. Assert the direct script parameter names and AST expose no repository, primary, lock, request, descriptor, timeout, or byte-limit override and cannot dispatch the loopback-test function. Assert request counts, exact saved bytes, fail-stop ordering, rulebook selection, URI validation, status/challenge handling, strict content validation, and that no route representing artwork is requested.

Implement the transport half of `scripts/collect-private-authority.ps1` with `#requires -Version 7.0`, a non-mandatory syntactic `param([string]$BackupRoot, [switch]$AcknowledgePrivateUseRisk)` block, strict mode, terminating errors, function definitions, and the exact final guard `if ($MyInvocation.InvocationName -ne '.') { ... }`. Inside that guard, validate both direct arguments before calling `Invoke-PrivateAuthorityCollection (Get-ProductionSourceDescriptors)`; dot-sourcing must only define functions. `Get-ProductionSourceDescriptors` is the sole production manifest owner and returns exactly: rulebook release URL/path/PDF/release marker `Sorcery: Contested Realm December 2025 Rulebook Update`/`2025-12-19`; Constructed URL/path/HTML/marker `Constructed Format`/null date; Codex URL/path/HTML/marker `Welcome to the Codex`/null; FAQs URL/path/HTML/marker `FAQs`/null; changelog URL/path/HTML/marker `Codex Changelog` with derived date; card-update URL/path/HTML/marker `Sorcery: Contested Realm Card Updates 2025`/`2025-11-25`; and cards API URL/path/JSON/null. Add an offline test that serializes this accessor, compares every field to the locked matrix, and proves the direct guard calls only this accessor—no endpoint string may exist in the direct wrapper.

Per D-09/D-10, use no community source and no added dependency. Implement manual transport with `System.Net.Http.HttpClient`, automatic redirects disabled, `ResponseHeadersRead`, a fixed buffer, a 30-second header timeout, a separate 15-minute body timeout, Content-Length prechecks, production caps of 10,000,000 bytes for JSON, 256 MiB for every other body, and 768 MiB total. Allow a maximum of five redirect hops, HTTPS/no credentials/no loop, and only `sorcerytcg.com` plus `www.sorcerytcg.com` for Sorcery pages, `curiosa.io` plus `www.curiosa.io` for Codex pages, and `api.sorcerytcg.com` for card JSON. Resolve the rulebook by first requiring a 200 `text/html` release page with strict UTF-8, its exact release marker, and visible publication date `2025-12-19`; HTML-decode and require exactly one visible anchor normalized to `Sorcery: Contested Realm Rulebook (December 2025)`, reject annotated/duplicate/missing matches, require a `drive.google.com/file/d/{id}/view` locator, derive only `drive.google.com/uc?export=download&amp;id={id}`, allow its redirects only through `drive.google.com` and `drive.usercontent.google.com`, require PDF or octet-stream media, filename `SorceryRulebook.pdf`, `%PDF-` prefix, and `%%EOF` suffix. For HTML require `text/html` ignoring parameters, strict UTF-8, an HTML signature, and its exact marker. For changelog, strip tags/decode visible text after the `Codex Changelog` marker, take the first displayed ISO or invariant `MMMM d, yyyy` date, normalize to `yyyy-MM-dd`, and fail when absent/unparseable. For card data require `application/json` ignoring parameters, strict UTF-8, a nonempty array root whose every member is an object with a nonempty string `name`, while saving the original bytes unchanged. Reject 401/403/429 before body read, every other non-200, and challenge-shaped HTML whose title/elements identify `captcha`, `cf-chl-`, `challenge-platform`, `Access Denied`, `Request Blocked`, `Attention Required`, or `unusual traffic`; never retry, fall back, poll, evade, or request embedded/image URLs. Commit the passing transport as `feat(quick-260820-hhu): add fixed bounded collector transport`. Do not execute the direct guard or contact publisher endpoints.</action>
  <verify>
    <automated>node --test tests/authority/private-authority-collector.test.ts</automated>
  </verify>
  <done>The dot-source seam is inert, the direct wrapper is locked to the exact audited seven-source manifest, and synthetic loopback tests prove bounded fail-closed transport, content validation, and no-artwork behavior without a live request.</done>
</task>

<task type="auto" tdd="true">
  <name>Task 2: Stage, back up, reverify, and publish the private lock safely</name>
  <read_first>
    - .planning/quick/260820-hhu-build-and-validate-a-user-run-powershell/260820-hhu-RESEARCH.md
    - src/authority/private-source-set.ts
    - tests/authority/private-source-set.test.ts
    - scripts/collect-private-authority.ps1
    - tests/authority/private-authority-collector.test.ts
  </read_first>
  <files>tests/authority/private-authority-collector.test.ts, scripts/collect-private-authority.ps1</files>
  <behavior>
    - The happy loopback run writes exactly seven primary files and seven ordinary byte-identical but identity-distinct backup files, preserves card JSON response bytes exactly, and publishes a private lock accepted by `verifyPrivateSourceSet`.
    - Existing primary, backup, destination file, or lock is never changed and fails before acquisition; a backup equal to, nested beneath, or containing the temporary repository is rejected with zero requests; a valid absolute backup containing spaces, apostrophes, ampersands, parentheses, dollar signs, and brackets is handled as data rather than executable text.
    - Staging trees pass `verifyPrivateSourceSet` before publication, final trees pass it again, and the BOM-free lock is moved into place last only after a third lock-candidate verification returns the same sorted entries/root hash.
    - A staged failure cleans only run-owned staging paths; a forced verifier or lock-publication failure after either final move atomically renames only this invocation's published trees/lock candidate to run-ID failure quarantine siblings and leaves the canonical destinations absent for a safe retry.
    - The lock records `acquisitionMethod: user-run-one-shot-powershell`, exact final roots, entries, sourceSetRootHash, exact acknowledgment, and non-normative rulebook evidence without official content, excerpts, artwork locators, or legal/authenticity claims.
  </behavior>
  <action>Extend `tests/authority/private-authority-collector.test.ts` with failing publication tests and commit them as `test(quick-260820-hhu): add private collector publication contract`. Every case must use the loopback-only temporary repository/primary/lock destination seam from Task 1; direct production paths remain untouched. The happy case must assert exact seven-file trees/no extras, distinct primary/backup dev+ino identities, exact API response bytes, sorted official provenance entries, fixed/derived effective dates, rulebook evidence, `acquisitionMethod: user-run-one-shot-powershell`, acknowledgment, and final acceptance through the imported TypeScript `verifyPrivateSourceSet`. Add table-driven preflight/no-overwrite cases for existing primary, backup, any destination, and lock. Add equal/nested/containing-repository backup cases and assert they fail before the loopback server observes any request. Use a valid outside-repository backup pathname containing spaces and legal Windows shell metacharacters/apostrophes to prove no code/shell interpolation. Add controlled internal fault points available only in loopback test mode after staged verification, after each final move, during final verifier invocation, and before lock move; assert no receipt is published, pre-existing paths remain unchanged, and only invocation-owned outputs move to uniquely named failure quarantine siblings.

Finish `Invoke-PrivateAuthorityCollection`. Per the user-scoped D-08 amendment and D-11, reject missing acknowledgment before side effects and persist top-level `acquisitionMethod` with the only accepted value `user-run-one-shot-powershell`, plus this exact semantic record: private/local/noncommercial use; no redistribution, release, hosting, third-party upload, or artwork; the API guidance/Terms/robots conflict and private-use risk are accepted; this is not legal permission; 401/403/429/CAPTCHA/block or publisher objection requires stopping without retry or evasion. The direct guard must obtain repository root from the script path and derive the canonical primary `.local/authority/inputs/official-2026-08-20/primary` and lock `.local/authority/locks/official-2026-08-20/source-set-lock.json`; only the dot-sourced loopback-test function may supply temporary replacements, and it must prove every request/redirect host is loopback before accepting them. Before any request, resolve all roots and reject a backup that is not absolute/outside the repository, equals or nests under the repository, contains the repository, aliases either root, or already exists; also require primary and lock destinations absent. Stage primary and ordinary byte-copied backup trees at unique same-parent sibling paths using run IDs, `FileMode.CreateNew`, `File.Copy(..., false)`, and no-overwrite `Directory.Move`; record the `Z` UTC completion timestamp only after each exact response passes transport/content validation, then compute its length and lowercase `sha256:` digest.

Invoke Node with `System.Diagnostics.ProcessStartInfo` using `UseShellExecute = false` and `ArgumentList`: fixed non-interpolated JavaScript source plus an owned draft-JSON path passed as an argument. The fixed JavaScript imports `verifyPrivateSourceSet`, parses the owned JSON, passes only its roots/entries, and emits JSON; never place `BackupRoot`, any path, JSON, or user value inside JavaScript source, a command string, or `Invoke-Expression`. Run this bridge against the complete staging roots before publication. Publish backup then primary without overwrite, run the bridge again against final roots, and require identical sorted entries/root hash. Write a BOM-free lock candidate with `schemaVersion`, `acquisitionMethod: user-run-one-shot-powershell`, final absolute roots, returned entries/root hash, structured `operatingAcknowledgment`, and `rulebookAcquisitionEvidence` containing normative release URL, relative path/hash/timestamp, observed filename, non-normative Drive locator evidence, and `privateLocatorIsNormative: false`; re-read it, assert the exact acquisition method, run the bridge a third time against its values, compare the result, then move it without overwrite to the final lock path. Do not reproduce canonical JSON/identity hashing in PowerShell.

Track ownership flags for every staging/final move. On failure before publication, remove only paths whose run ID and resolved parent match the invocation's expected staging paths. On failure after publication begins, move only final paths whose ownership flag proves this invocation created them to absent same-parent `.failed-{runId}` quarantine siblings; never delete, overwrite, or rename a pre-existing path. If quarantine itself fails, preserve the evidence, report every occupied path, and leave the canonical lock absent so Plan 01-07 cannot accept the run. Commit the passing publication slice as `feat(quick-260820-hhu): verify and publish private source receipt`. Do not run the production entry point or contact publisher endpoints.</action>
  <verify>
    <automated>node --test tests/authority/private-authority-collector.test.ts &amp;&amp; node --test tests/authority/private-source-set.test.ts</automated>
  </verify>
  <done>The loopback suite proves independent no-overwrite publication, injection-safe verifier invocation, recoverable invocation-owned failure quarantine, and receipt-last acceptance by verifyPrivateSourceSet without any live request.</done>
</task>

<task type="auto">
  <name>Task 3: Reconcile policy, outreach, and Plan 01-07 with the collector</name>
  <read_first>
    - .planning/quick/260820-hhu-build-and-validate-a-user-run-powershell/260820-hhu-LEGAL-RESEARCH.md
    - .planning/phases/01-rules-and-data-authority/01-07-PLAN.md
    - docs/external-reuse-policy.md
    - docs/card-data-authorization-outreach.md
    - scripts/collect-private-authority.ps1
    - tests/authority/private-authority-collector.test.ts
  </read_first>
  <files>docs/external-reuse-policy.md, docs/card-data-authorization-outreach.md, .planning/phases/01-rules-and-data-authority/01-07-PLAN.md</files>
  <action>Update `docs/external-reuse-policy.md` so the sole accepted Phase 1 acquisition path is the user's explicit one-shot invocation of `scripts/collect-private-authority.ps1` with an absolute independent `-BackupRoot` and `-AcknowledgePrivateUseRisk`. Per the user-amended D-08 and D-11, retain the exact seven-source table, ignored private roots, immutable/no-overwrite set, independent backup, private/noncommercial/no-redistribution/no-upload scope, no public card API, and the statement that hashes prove identity rather than authenticity or permission. State plainly that the user accepts the identified risk arising from the unresolved API-page/Terms/robots conflict; the collector does not establish legal permission; it stops on 401/403/429/CAPTCHA/block or publisher objection and has no retry, evasion, scheduler, or recurring update. Replace every manual-browser-only/no-acquisition statement that would now be false. Preserve D-09's clean-room/community-source limits. Preserve the artwork boundary exactly: this collector acquires no artwork and exposes no art endpoint; pending written permission, a browser GUI may support only images the user separately supplies from private local storage through a separate permission-reviewed task, while bulk art acquisition, private-CDN access, hotlinking, proxying, hosting, packaging, and redistribution remain prohibited.

Update `docs/card-data-authorization-outreach.md` only where it calls the current boundary manual-only: describe the accepted private one-shot user-run collector as a risk-accepted path without implying permission, explicitly name the unresolved API/Terms/robots conflict and stop/no-retry/no-evasion boundary, and retain the outreach request plus written-permission gate for recurring/scheduled or agent-run acquisition, sharing, publication, hosting, commercial use, bulk art, and private-CDN access. Do not weaken or delete the pending authorization request.

Revise `.planning/phases/01-rules-and-data-authority/01-07-PLAN.md` so its user setup, truths, context, checkpoint action, verification, acceptance criteria, resume signal, success criteria, threat boundaries, and D-08/D-11 must-have all consume and reverify the collector-produced private lock instead of asking for manual browser saves or manual metadata transcription. The blocking human action must tell the user to choose an absent absolute backup root outside the repository and personally run `pwsh -NoProfile -NonInteractive -File scripts/collect-private-authority.ps1 -BackupRoot <absolute-path> -AcknowledgePrivateUseRisk`; it must not let the executor or test suite run the production command. Replace the false manual attestation with the exact collector-recorded acknowledgment: private/local/noncommercial use; no redistribution, release, hosting, upload, or artwork; accepted API/Terms/robots risk; no legal-permission claim; stop without retry or evasion on 401/403/429/CAPTCHA/block or objection. After the user reports completion, Plan 01-07 must independently load the ignored lock, require `acquisitionMethod === 'user-run-one-shot-powershell'`, assert the acknowledgment and exact seven entries, call `verifyPrivateSourceSet` against final roots, match the root hash/rulebook evidence, and prove `.local/authority` remains ignored/untracked/unstaged. Its safe summary must record the same acquisition method without private locators. Only then may it release the already reconciled Plan 01-08; any missing/conflicting method or evidence preserves the existing no-summary/fail-closed behavior. Commit these three reconciled documents atomically as `docs(quick-260820-hhu): accept user-run private collector boundary` after the focused suite and full repository gate pass.</action>
  <verify>
    <automated>node --test tests/authority/private-authority-collector.test.ts &amp;&amp; node -e "const fs=require('node:fs'),read=p=&gt;fs.readFileSync(p,'utf8'),norm=s=&gt;s.toLowerCase().replace(/[^a-z0-9]+/g,' ').trim();const raw={policy:read('docs/external-reuse-policy.md'),outreach:read('docs/card-data-authorization-outreach.md'),plan:read('.planning/phases/01-rules-and-data-authority/01-07-PLAN.md')},files=Object.fromEntries(Object.entries(raw).map(([k,v])=&gt;[k,norm(v)]));const req={policy:['acknowledgeprivateuserisk','one shot','api','terms','robots','risk','401','403','429','captcha','stop','no retry','no evasion','recurring','written publisher permission','bulk art','private cdn','user supplied','local images','does not establish legal permission'],outreach:['one shot','user run','api','terms','robots','risk','recurring','written permission','bulk art','private cdn'],plan:['acknowledgeprivateuserisk','collect private authority ps1','acquisitionmethod user run one shot powershell','private','noncommercial','no redistribution','does not establish legal permission','no card art','401','403','429','captcha','no retry','no evasion']};for(const [k,terms] of Object.entries(req))for(const term of terms)if(!files[k].includes(term))throw Error(k+' missing '+term);for(const [k,text] of Object.entries(files))for(const stale of ['authorized v1 path','saved every required file manually through my browser','only the user controlled manual browser save','manual browser only','manual import boundary','manual private snapshot','private manual path','project code performs no fetch','no automated acquisition'])if(text.includes(stale))throw Error(k+' stale '+stale)" &amp;&amp; pnpm verify</automated>
  </verify>
  <done>All three documents truthfully accept the user-run one-shot collector without a legal conclusion, Plan 01-07 validates its output instead of requesting manual acquisition, the local-art-only boundary is explicit, and `pnpm verify` passes.</done>
</task>

</tasks>

<threat_model>
## Trust Boundaries

| Boundary | Description |
|---|---|
| user invocation -> collector | The backup locator and explicit risk acknowledgment cross into a command that can create private evidence. |
| official/loopback HTTP -> staging | Status, headers, redirects, and bodies are untrusted and may be malicious, blocked, misleading, oversized, or truncated. |
| release page -> Drive PDF | Only the exact standard rulebook link may cross to the tightly allowed acquisition hosts; the locator is not normative authority. |
| staging -> final primary/backup roots | Partial writes, path aliases, pre-existing evidence, or overwrite could corrupt or misrepresent an immutable revision. |
| PowerShell receipt -> TypeScript verifier | Transport metadata and byte hashes are untrusted until the existing canonical verifier accepts both final trees. |
| private roots/lock -> Git, packages, GUI | Official corpus bytes, private locators, and artwork must not escape private storage. |
| same-owner PowerShell/source control -> out of scope | Module-private functions are implementation encapsulation, not a sandbox against code already executing as the machine owner in the same runspace. |

## STRIDE Threat Register

| Threat ID | Category | Component | Disposition | Mitigation Plan |
|---|---|---|---|---|
| T-QH-01 | Spoofing / Information disclosure | redirects and rulebook resolution | mitigate | Disable auto-redirect, cap at five, validate scheme/credentials/host at every hop, require the exact standard anchor and PDF filename/magic, and keep Drive evidence non-normative. |
| T-QH-02 | Tampering | response body and metadata | mitigate | Stream exact bytes once, enforce media/signature/UTF-8/marker/JSON-shape checks, derive timestamps/hashes after completion, and delegate final identity to verifyPrivateSourceSet. |
| T-QH-03 | Denial of service | remote response | mitigate | Header and streamed byte caps, per-request header/body cancellation, bounded buffer, sequential requests, and terminal failure with no retry. |
| T-QH-04 | Tampering / Repudiation | filesystem publication | mitigate | Absent-target preflight, CreateNew, ordinary no-overwrite copies, sibling staging, no-overwrite directory moves, re-verification of final roots, and lock publication last. |
| T-QH-05 | Elevation of privilege | user backup path | mitigate | Require an absolute outside-repository root and reuse realpath/dev/inode/symlink/junction/hardlink checks in verifyPrivateSourceSet. |
| T-QH-06 | Information disclosure | official content and artwork | mitigate | Fixed seven-source manifest, no image URL traversal, ignored private roots/lock, no official bytes in fixtures/Git/packages, and user-supplied private local art only under a separate permission-reviewed GUI task. |
| T-QH-07 | Repudiation / legal | user acknowledgment | mitigate | Require the named switch before side effects and persist exact private-scope, risk, stop, and no-permission language in the ignored lock and Plan 01-07 verification. |
| T-QH-SC | Tampering | package installs | accept | No package-manager install occurs; PowerShell/.NET and Node standard-library facilities plus the existing verifier are sufficient. |
</threat_model>

<verification>
1. Run `node --test tests/authority/private-authority-collector.test.ts` and confirm every network interaction targets the test-owned loopback server.
2. Run `pnpm verify` for typecheck, lint, the existing verifier suite, and the new collector suite.
3. Run `git diff --check` and inspect `git diff -- scripts/collect-private-authority.ps1 tests/authority/private-authority-collector.test.ts docs/external-reuse-policy.md docs/card-data-authorization-outreach.md .planning/phases/01-rules-and-data-authority/01-07-PLAN.md` for fixed production URLs, no live-test invocation, no official bytes, and no artwork acquisition surface.
4. Confirm `git ls-files .local/authority` is empty and no `.local/authority` path is staged. Do not run the production collector during this quick task.
</verification>

<success_criteria>
- The direct PowerShell 7 entry point has only an absolute backup-root input and explicit risk-acknowledgment switch; every production source/relative path is fixed and artwork is absent.
- Offline loopback tests prove the complete success path and every required stop, redirect, bound, strict-content, staging, no-overwrite, independent-backup, exact-byte, metadata/hash, and receipt behavior.
- The collector calls the existing `verifyPrivateSourceSet` against final trees and publishes the exact private lock last; PowerShell does not duplicate canonical hashing.
- Policy, outreach, and Plan 01-07 consistently describe the user-run collector, risk acceptance, private/noncommercial/no-redistribution/no-legal-conclusion scope, stop-without-evasion behavior, and user-supplied-private-local-art-only boundary.
- The focused suite and `pnpm verify` pass, only synthetic fixture bytes are committed, and no live publisher endpoint is contacted.
- Each RED, GREEN, and documentation slice is committed separately with the descriptive commit messages specified in its task.
</success_criteria>

<output>
Create `.planning/quick/260820-hhu-build-and-validate-a-user-run-powershell/260820-hhu-SUMMARY.md` only after all three tasks and every verification criterion pass. Do not create or modify `.planning/phases/01-rules-and-data-authority/01-07-SUMMARY.md`; that remains blocked on the user's later live collector invocation under the revised Plan 01-07 checkpoint.
</output>
