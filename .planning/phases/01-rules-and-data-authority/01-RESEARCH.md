# Phase 1: Rules and Data Authority - Research

**Researched:** 2026-08-20
**Domain:** Immutable rules authority, normalized card data, provenance, and clean-room reuse
**Confidence:** HIGH for architecture and official-source inventory; MEDIUM for redistribution rights pending written permission

<user_constraints>
## User Constraints (from CONTEXT.md)

### Locked Decisions

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
- **D-08:** Store derived normalized data and source manifests in the repository. Do not bundle copyrighted rulebook PDFs or card images unless their redistribution terms clearly allow it; keep retrieval instructions and verified hashes instead.
- **D-09:** Contested Realms (GPL-3.0) and spells.bar/the playtest project (no reusable license found in the audited revision) are behavioral and UX references only. Do not copy their source, card implementations, assets, or data into this project.
- **D-10:** Phase 1 uses TypeScript and Node standard-library facilities first. Add a dependency only where runtime schema validation or deterministic normalization is materially safer than a small local implementation.

### the agent's Discretion
- Exact directory names, JSON field ordering, command names, and test file layout, provided the offline validation and immutable provenance requirements remain obvious.
- Whether raw official API responses can be committed after the source/licensing audit; otherwise commit only the normalized snapshot, manifest, and reproducible retrieval tooling.

### Deferred Ideas (OUT OF SCOPE)
- Owned collection import and common online deck acquisition belong to Phase 6.
- Game-rule execution belongs to Phases 3 and 4.
- Model competitors belong to Phase 7; browser human play belongs to Phase 9.
</user_constraints>

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|------------------|
| DATA-01 | A developer can build an immutable authority bundle containing the pinned official rulebook, format rules, Codex/FAQ/card updates, precedence policy, source URLs, retrieval dates, and SHA-256 hashes. | Source inventory, precedence model, manifest schema, immutable build pipeline, and bundle validation below. |
| DATA-02 | A developer can build and validate a versioned normalized card snapshot without live network access during a game or experiment. | Official API audit, strict input/output schemas, pure normalization, local fixtures, and offline validation tests below. |
| DATA-03 | Every canonical card, rule, format, deck, collection, behavior, and experiment artifact has a stable ID, schema version, provenance record, and content hash. | Common artifact envelope and canonical JSON/SHA-256 identity contract below. |
</phase_requirements>

## Summary

Phase 1 should build one local, content-addressed authority pipeline, not a rules engine. Each source is captured with byte hash, reviewed HTTPS URL, authority class, retrieval time, effective date, and legal/storage status. Stored-source records point to confined permitted bytes that offline validation rehashes; manifest-only records bind an immutable locator or approved acquisition-procedure fingerprint and expected byte hash without claiming absent bytes were reread. A pure normalizer produces strict canonical artifacts, and a bundle manifest binds them under a documented precedence policy. Runtime consumers receive only a validated bundle ID and independently supplied hash and never fetch mutable authority data. [VERIFIED: 01-CONTEXT.md]

The official authority is genuinely multi-versioned. The current official rulebook update is dated 19 December 2025, while the official Codex changelog contains rulings and Updated Cards through 15 July 2026, including reversals of earlier FAQs. The official Constructed baseline is 1 Avatar, a minimum 60-card Spellbook, a minimum 30-card Atlas, and rarity copy limits; event floor rules are separate overlays, not global game rules. [CITED: https://sorcerytcg.com/news/sorcery-contested-realm-december-2025-rulebook-update] [CITED: https://curiosa.io/codex/changelog] [CITED: https://sorcerytcg.com/constructed]

The official card API is useful as an acquisition input, but it is mutable and exposes no observed top-level snapshot version or canonical card ID. More importantly, the site terms cover Curiosa and related media, reserve database/graphics rights, and prohibit automated access and systematic database extraction without permission. No API-specific open-data or image redistribution grant was found. Therefore the plan must add a human/legal checkpoint before automated retrieval or committing raw API responses, and must never commit rulebook PDFs or images without written permission. [CITED: https://api.sorcerytcg.com/api/cards] [CITED: https://sorcerytcg.com/terms]

**Primary recommendation:** Implement a strict offline import/build/validate toolchain whose canonical output is reproducible from a Plan 07-approved durable exact-byte input lock; keep network acquisition permission-gated and outside the runtime.

## Architectural Responsibility Map

| Capability | Primary Tier | Secondary Tier | Rationale |
|------------|-------------|----------------|-----------|
| Official-source acquisition | Developer command / external boundary | Filesystem | Network access is an explicit maintenance action, never runtime behavior. [VERIFIED: 01-CONTEXT.md] |
| Strict parsing and normalization | Local TypeScript library | — | Pure functions make rebuilds deterministic and testable. [VERIFIED: 01-CONTEXT.md] |
| Artifact identity and provenance | Local TypeScript library | Filesystem | One envelope and canonical hash contract must serve all later artifact kinds. [VERIFIED: 01-CONTEXT.md] |
| Immutable authority storage | Filesystem | Local TypeScript library | Versioned JSON/manifests are inspectable and need no database service. [VERIFIED: .planning/research/STACK.md] |
| Bundle selection | Simulator/API boundary in later phases | Filesystem | Later consumers select a validated local bundle by ID and hash. [VERIFIED: 01-CONTEXT.md] |

## Project Constraints (from AGENTS.md)

- Use TypeScript throughout; keep official rules and rulings authoritative and never tune balance by changing a rule. [VERIFIED: AGENTS.md]
- Fail closed: unsupported or ambiguous behavior must be explicit, never a silent no-op or guessed ruling. [VERIFIED: AGENTS.md]
- Preserve deterministic, reproducible artifacts; later runs bind to exact inputs. [VERIFIED: AGENTS.md]
- Copy external material only when license obligations are compatible and accepted; otherwise use it as behavioral reference only. [VERIFIED: AGENTS.md]
- Use Podman rather than Docker if containers become necessary; Phase 1 needs no container. [VERIFIED: AGENTS.md]
- Prefer MCP sources, small verified commits, and the smallest meaningful automated test in every implementation task. [VERIFIED: AGENTS.md]
- No project skills were present under `.codex/skills` or `.agents/skills` during this audit. [VERIFIED: codebase inspection]

## Official Authority Audit

| Authority | Current audited source | Planning consequence |
|-----------|------------------------|----------------------|
| Rulebook | The 19 Dec 2025 official update says it aligns the rulebook with Gothic, changes constructed deck size to 60, and incorporates selected Codex terminology. [CITED: https://sorcerytcg.com/news/sorcery-contested-realm-december-2025-rulebook-update] | Store the standard and annotated PDFs as separate URL/hash records; do not store PDF bytes without permission. |
| Constructed format | 1 Avatar; Spellbook ≥60; Atlas ≥30; copy limits 4 Ordinary/3 Exceptional/2 Elite/1 Unique. [CITED: https://sorcerytcg.com/constructed] | Encode a versioned base `constructed` format artifact. Preserve source wording; do not silently repair source typos. |
| Collection/event policy | Gothic-era tournament guidance permits a Collection up to 10 cards; event floor rules can add registration, legality, or procedure policy. [CITED: https://sorcerytcg.com/news/what-you-carry-with-you-a-first-look-at-the-collection-and-deck-sizes-in-gothic] [CITED: https://sorcerytcg.com/news/everything-you-need-to-know-for-sorcery-at-gen-con-2026] | Model event rules as explicitly scoped overlays referencing the base format, never global authority. |
| Codex/FAQ | Curiosa instructs readers to start with the rulebook, then use the Codex for detail and card FAQs for specific cards. [CITED: https://curiosa.io/codex] [CITED: https://curiosa.io/faqs] | Capture Codex concepts, FAQ/card pages, and changelog revision independently. |
| Updates/errata | The 2025 card-update notice calls the new text official and supersedes the old “treat as” FAQ approach; the 15 Jul 2026 changelog reverses earlier FAQs for named cards. [CITED: https://sorcerytcg.com/news/sorcery-contested-realm-card-updates-2025] [CITED: https://curiosa.io/codex/changelog] | Effective date and supersession references are required; never flatten history into one undated blob. |
| Official card data | Curiosa links the official API. The audited payload contains `guardian` current card data and printing `sets[].variants[].slug`, but no observed canonical top-level ID, image URL, or snapshot-version envelope. [CITED: https://curiosa.io/codex] [CITED: https://api.sorcerytcg.com/api/cards] | Retain printing slugs as source IDs; mint stable project card IDs deterministically; hash the complete raw input; do not hard-code a card count. |

### Project Precedence Policy

Implement the locked rule as data, not scattered conditionals: a later official clarification or erratum supersedes older text; a specifically scoped format/event rule applies only in that scope; equal-rank or unclear conflicts produce an `unsupported` record. This is a project policy required by D-02, not a claim that the publisher has published a complete conflict hierarchy. [VERIFIED: 01-CONTEXT.md]

Recommended ordered evaluation:

1. Apply explicit official supersession/reversal records by effective date.
2. Apply card-specific Updated Cards/current official card characteristics to that card.
3. Apply selected format/event overlays only to their declared scope.
4. Use the rulebook for general rules and the Codex/FAQ for compatible detail and card clarification.
5. Treat printed text and community/reference behavior as non-normative provenance.
6. Fail closed when effective dates, scope, or authority rank cannot resolve a conflict.

Every resolution result should retain the winning source reference and all superseded/contending references. [VERIFIED: 01-CONTEXT.md]

## Distribution and Clean-Room Boundary

The official Terms cover `sorcerytcg.com`, `play.sorcerytcg.com`, and `curiosa.io`; reserve rights in site databases, text, photographs, and graphics; grant only limited personal/noncommercial use; and prohibit automated access, systematic retrieval, and database construction without permission. No audited API page supplied a separate open license. Treat automated API ingestion, raw API redistribution, PDFs, and card images as permission-required. A hash and URL are not redistribution. [CITED: https://sorcerytcg.com/terms]

`realms-cards/contested-realms` declares GPL-3.0. GPL-covered copying or adaptation, including close translation, can impose GPL obligations on the combined conveyed work; D-09 therefore fixes a behavior-only boundary. Do not copy source, tests, rule implementations, assets, or data. [CITED: https://github.com/realms-cards/contested-realms/blob/main/LICENSE] [CITED: https://www.gnu.org/licenses/gpl] [CITED: https://www.gnu.org/licenses/gpl-faq.en.html]

The audited `JollyGrin/sorcery-tcg-playtest` repository had no LICENSE file even though its README calls the project open source. GitHub documents that absent a license, default copyright applies and others may not reproduce, distribute, or create derivative works. Treat spells.bar/playtest source, tests, assets, and data as non-reusable unless the owner grants a license in writing. [CITED: https://github.com/JollyGrin/sorcery-tcg-playtest] [CITED: https://docs.github.com/en/repositories/managing-your-repositorys-settings-and-features/customizing-your-repository/licensing-a-repository]

Required implementation guardrails:

- Add `docs/external-reuse-policy.md` with allowed behavioral observations, forbidden copying, source URLs, audited revisions, and reviewer sign-off. [VERIFIED: 01-CONTEXT.md]
- Keep publisher bytes outside git by default; commit source manifests, normalized output only after permission review, and fixtures that are synthetic or minimal facts rather than copied corpora. [VERIFIED: 01-CONTEXT.md]
- Put a `licenseStatus`/`storagePolicy` field on each source record so the builder rejects forbidden stored bytes. [ASSUMED]
- Require a human checkpoint before first automated retrieval or redistribution; written permission should state API use, normalized-data redistribution, caching, images, attribution, and revocation/update expectations. [ASSUMED]
- Give every source an `authorityClass`. Reviewed secure community/reference sources may remain as provenance, but only allowlisted `official` sources can enter normative precedence. [VERIFIED: 01-CONTEXT.md]
- Separate `stored` sources from `manifest-only` sources. Rehash permitted stored bytes offline; for absent bytes, validate the canonical durable-locator/procedure and source-reference binding, with the raw byte hash established during import. [VERIFIED: 01-CONTEXT.md]
- Before DATA-02 publication, require Plan 07 to approve one durable exact-byte path and an independent canonical input-root hash; a mutable executor-local directory is not sufficient. [ASSUMED]

## Standard Stack

### Core

| Library | Version | Purpose | Why Standard |
|---------|---------|---------|--------------|
| Node.js | 24.19.0 LTS | Runtime, test runner, crypto, filesystem | Native TypeScript stripping is stable in Node 24.12+, but does not type-check; pair it with `tsc --noEmit`. [CITED: https://nodejs.org/download/release/latest-v24.x/docs/api/typescript.html] |
| TypeScript | 6.0.3 | Static types and build-time checking | Project-selected current compiler with conventional tool API. [VERIFIED: npm registry] |
| pnpm | 11.22.0 | Pinned package manager/lockfile | Project standard; pin in `packageManager`. [VERIFIED: npm registry] |
| `node:test`, `node:assert/strict` | Node 24 bundled | Unit, golden, integration, and offline tests | Node 24 directly discovers `.test.ts` under native stripping. [CITED: https://nodejs.org/download/release/latest-v24.x/docs/api/test.html] |
| `node:crypto` | Node 24 bundled | SHA-256 identity and byte hashes | Use `createHash('sha256')`; a digest proves byte identity, not publisher authenticity. [CITED: https://nodejs.org/download/release/latest-v24.x/docs/api/crypto.html#createhash] |
| `node:fs`, `node:path` | Node 24 bundled | Atomic local reads/writes and path confinement | No database or network service is needed. [VERIFIED: 01-CONTEXT.md] |

### Supporting

| Library | Version | Purpose | When to Use |
|---------|---------|---------|-------------|
| `zod` | 4.4.3 | Strict runtime schemas and exact issue paths | At raw-input, normalized-output, manifest, and bundle boundaries. Use `z.strictObject`; ordinary `z.object` strips unknown keys. [CITED: https://zod.dev/api] [CITED: https://zod.dev/error-customization] |
| `@types/node` | 24.1.0 | Node type declarations | Development only, aligned to Node 24. [VERIFIED: npm registry] |
| ESLint | 10.8.1 | Static checks | Development verification. [VERIFIED: npm registry] |
| `typescript-eslint` | 8.67.0 | Type-aware lint integration | Development verification with flat config. [CITED: https://typescript-eslint.io/getting-started/] |

### Alternatives Considered

| Instead of | Could Use | Tradeoff |
|------------|-----------|----------|
| Zod strict schemas | Hand validators | Rejected: silent omissions and inconsistent paths are high-risk at the authority boundary. |
| Project-owned canonicalizer | Canonical-JSON package | Rejected by D-07/D-10: the required narrow routine can follow RFC 8785 and be locked with official test vectors. |
| JSON/JSONL files | Database | Rejected: Phase 1 is immutable, local, inspectable, and query-light. |
| Permission-gated local import | Live API reads | Rejected: violates offline runtime and may violate source terms. |

**Installation:**

```bash
pnpm add zod@4.4.3
pnpm add -D typescript@6.0.3 @types/node@24.1.0 eslint@10.8.1 typescript-eslint@8.67.0
```

## Package Legitimacy Audit

`slopcheck 0.6.1` was run against the npm ecosystem and returned OK for every proposed package; registry existence, version, publish date, source repository, downloads, and postinstall metadata were also checked. No package had a postinstall script. [VERIFIED: npm registry]

| Package | Registry | Published | Downloads/week | Source Repo | slopcheck | Disposition |
|---------|----------|-----------|----------------|-------------|-----------|-------------|
| zod 4.4.3 | npm | 2026-05-04 | 223,672,922 | github.com/colinhacks/zod | OK | Approved |
| typescript 6.0.3 | npm | 2026-04-16 | 225,722,105 | github.com/microsoft/TypeScript | OK | Approved |
| @types/node 24.1.0 | npm | 2025-07-22 | 348,951,646 | github.com/DefinitelyTyped/DefinitelyTyped | OK | Approved |
| eslint 10.8.1 | npm | 2026-08-07 | 133,827,823 | github.com/eslint/eslint | OK | Approved |
| typescript-eslint 8.67.0 | npm | 2026-08-10 | 75,088,819 | github.com/typescript-eslint/typescript-eslint | OK | Approved |
| pnpm 11.22.0 | npm | 2026-08-15 | 139,053,204 | github.com/pnpm/pnpm | OK | Approved |

**Packages removed due to slopcheck [SLOP] verdict:** none  
**Packages flagged as suspicious [SUS]:** none

## Architecture Patterns

### System Architecture Diagram

```text
Official URL / manually supplied bytes
                 |
                 v
 permission + durable input review ---- denied ----> hard stop; phase pending
                 |
                 v
 exact input lock/root hash + strict parse
                 |
                 v
       pure deterministic normalizer
                 |
                 v
       strict normalized validation
                 |
                 v
       canonical JSON -> SHA-256 identity
                 |
                 v
     write-once revision + bundle manifest
                 |
                 v
       recursive offline bundle validator
          | valid                 | invalid/ambiguous
          v                       v
 later consumer selects       fail closed with sorted,
 bundle ID + hash             path-specific diagnostics
```

### Recommended Project Structure

```text
src/authority/
  schemas.ts              # strict boundary and artifact schemas
  canonical-json.ts       # one RFC-8785-compatible narrow serializer
  hash.ts                 # raw-byte and canonical-artifact SHA-256
  normalize-cards.ts      # pure source -> project card mapping
  validate-bundle.ts      # recursive, offline integrity validation
src/commands/
  import-authority.ts     # consumes a clean materialization of the approved durable input lock
  validate-authority.ts
data/authority/<revision-id>/
  bundle.json             # refs/hashes, no copyrighted PDFs/images
  sources.json
  formats.json
  cards.normalized.json
  raw/cards.raw.json      # only when written permission explicitly approves repository storage
docs/
  authority-precedence.md
  external-reuse-policy.md
tests/authority/
  fixtures/               # synthetic/minimal and license-safe
```

### Pattern 1: Hashable Artifact Envelope

Use one envelope for every later artifact kind. Avoid a self-referential digest by hashing the canonical identity document with `contentHash` omitted, then store the digest beside it.

```typescript
type ArtifactRef = Readonly<{ artifactKind: string; stableId: string; contentHash: `sha256:${string}` }>;
type SourceRef = Readonly<{ sourceId: string; byteHash: `sha256:${string}` }>;

type IdentityDocument<T> = Readonly<{
  artifactKind: string;
  stableId: string;
  schemaVersion: number;
  parentRefs: readonly ArtifactRef[];
  sourceRefs: readonly SourceRef[];
  payload: T;
}>;

type CanonicalArtifact<T> = Readonly<{
  identity: IdentityDocument<T>;
  contentHash: `sha256:${string}`; // hash(canonicalJson(identity))
}>;
```

Stable IDs identify the logical project entity; immutable revision identity is the content hash. Do not replace one with the other. [VERIFIED: 01-CONTEXT.md]

### Pattern 2: Strict, Pure Build Pipeline

`read independent input-root hash + canonical input lock -> require exact file set and byte hashes -> strict parse -> normalize -> strict validate -> canonicalize -> hash -> write new path -> offline revalidate`. Pass retrieval time/effective date as recorded source metadata; never call the clock, network, locale-sensitive sort, or random functions from normalization. Bind raw bytes at import. Offline validators rehash stored bytes and verify manifest-only canonical bindings without claiming absent bytes were reverified. Refuse to overwrite an existing revision and use a same-directory temporary file plus atomic rename for new writes. [VERIFIED: 01-CONTEXT.md]

### Pattern 3: Canonical JSON Contract

Implement the project-owned serializer over already validated JSON values. Follow RFC 8785: UTF-8 output, recursive object-key sorting by UTF-16 code units, array-order preservation, ECMAScript primitive serialization, and no Unicode normalization. Reject non-finite/unsafe numeric values, `undefined`, sparse arrays, unsupported prototypes, and duplicate-key input before canonicalization. Lock behavior with RFC vectors. [CITED: https://www.rfc-editor.org/rfc/rfc8785.html]

### Anti-Patterns to Avoid

- **One “current rules” blob:** destroys temporal provenance and cannot represent reversals.
- **Hashing pretty-printed JSON:** whitespace/key order changes identity; hash only canonical bytes.
- **Self-referential hashes:** define exactly which document excludes the digest.
- **Permissive parsing:** `z.object` strips unknown keys; use strict objects and no coercion/repair.
- **Network-capable runtime validator:** validation must be complete with networking unavailable.
- **Event policy as base rules:** overlays must be explicitly scoped and parent-linked.
- **Treating API set metadata as historical printed text:** the audited API includes updated effective text; retain source semantics conservatively.
- **Hash as authenticity proof:** SHA-256 detects changed bytes but does not prove publisher origin.

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| Boundary validation | Ad hoc property checks | Zod strict schemas | Unknown-key rejection, unions, refinements, and exact issue paths. |
| Cryptographic hashing | Custom checksum/hash | Node `createHash('sha256')` | Standard implementation and stable byte digest. |
| Copyright/license judgment | Implicit developer guess | Written permission + recorded legal review | Repository visibility and “open source” wording are not licenses. |
| Rules interpretation | Code inferred from community behavior | Explicit official-source records and unsupported conflicts | Keeps authority auditable and fail-closed. |
| Canonical serialization contract | Ordinary `JSON.stringify` calls scattered around | One small project module implementing the locked RFC-based profile | One auditable identity boundary is safer than inconsistent call sites. |

## Common Pitfalls

### Pitfall 1: Mutable Upstream Masquerades as a Snapshot
**What goes wrong:** a later API/Codex update changes the same URL and old experiments cannot be rebuilt.  
**Avoid:** hash source bytes at acquisition/import, preserve retrieval/effective dates, record an immutable locator or approved mismatch-refusing acquisition procedure, and never rebuild an old revision from an unverified fresh fetch.
**Warning signs:** manifest contains only a URL/hash while the exact bytes exist solely in an executor-local directory.

### Pitfall 2: Silent “Helpful” Normalization
**What goes wrong:** misspelled fields are dropped, strings are trimmed/coerced, or malformed cards disappear.  
**Avoid:** strict parse both sides; transformations are explicit and tested; diagnostics sort by JSON Pointer, code, then message.  
**Warning signs:** input count and output count differ without a documented mapping result.

### Pitfall 3: Mixing Acquisition with Determinism
**What goes wrong:** timestamps, network response order, locale, or upstream changes alter output.  
**Avoid:** approved acquisition/materialization produces bytes that match the independent input lock/root; normalization is a pure second operation over those verified local inputs.

### Pitfall 4: Legal Contamination
**What goes wrong:** copied GPL implementation or unlicensed playtest assets/tests enter the repository, or official images/raw database are redistributed.  
**Avoid:** clean-room policy, source-review checklist, permission checkpoint, and reviewer attestation in each source manifest.

### Pitfall 5: Partial or In-Place Revision Writes
**What goes wrong:** a failed build corrupts an existing authority revision.  
**Avoid:** build in a new revision directory, validate fully, atomically publish, and fail if the target exists.

## Code Examples

### Exact-Path Strict Validation

```typescript
// Source: https://zod.dev/api and https://zod.dev/error-customization
import { z } from 'zod';

const SourceFields = {
  sourceId: z.string().min(1),
  url: z.url().refine((value) => value.startsWith('https:')),
  authorityClass: z.enum(['official', 'community-provenance', 'external-reference']),
  retrievedAt: z.iso.datetime(),
  effectiveDate: z.iso.date().nullable(),
  mediaType: z.string().min(1),
  byteHash: z.string().regex(/^sha256:[0-9a-f]{64}$/),
  licenseStatus: z.enum(['approved', 'manifest-only', 'permission-required']),
} as const;

const StoredSourceRecord = z.strictObject({
  ...SourceFields,
  storageMode: z.literal('stored'),
  relativePath: z.string().min(1),
});

const ManifestOnlySourceRecord = z.strictObject({
  ...SourceFields,
  storageMode: z.literal('manifest-only'),
  durableLocator: z.string().min(1),
  acquisitionProcedureHash: z.string().regex(/^sha256:[0-9a-f]{64}$/).nullable(),
});

const SourceRecord = z.discriminatedUnion('storageMode', [
  StoredSourceRecord,
  ManifestOnlySourceRecord,
]);

export function validateSource(input: unknown) {
  const result = SourceRecord.safeParse(input);
  if (result.success) return result.data;
  const issues = result.error.issues.map((issue) => ({
    path: '/' + issue.path.map(String).map((p) => p.replaceAll('~', '~0').replaceAll('/', '~1')).join('/'),
    code: issue.code,
    message: issue.message,
  })).sort((a, b) => a.path.localeCompare(b.path) || a.code.localeCompare(b.code) || a.message.localeCompare(b.message));
  throw new AggregateError(issues.map((i) => new Error(`${i.path}: ${i.code}: ${i.message}`)), 'Invalid source');
}
```

After schema validation, allowlist publisher hosts only for `authorityClass: official` and exclude all other authority classes from normative precedence. Offline validation rehashes `StoredSourceRecord.relativePath`; it validates a `ManifestOnlySourceRecord`'s canonical locator/procedure, expected byte hash, and `SourceRef` binding without claiming the absent bytes were reread.

### Canonical Artifact Hash

```typescript
// Sources: https://nodejs.org/download/release/latest-v24.x/docs/api/crypto.html#createhash
//          https://www.rfc-editor.org/rfc/rfc8785.html
import { createHash } from 'node:crypto';
import { canonicalJson } from './canonical-json.ts';

export function sha256(bytes: Uint8Array): `sha256:${string}` {
  return `sha256:${createHash('sha256').update(bytes).digest('hex')}`;
}

export function identityHash(value: unknown): `sha256:${string}` {
  return sha256(Buffer.from(canonicalJson(value), 'utf8'));
}
```

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|--------------|--------|
| 40-card Spellbook / 20-card Atlas | 60-card Spellbook / 30-card Atlas | Gothic release / Dec 2025 rulebook | Version format rules; do not keep old sizes as current defaults. [CITED: https://sorcerytcg.com/news/what-you-carry-with-you-a-first-look-at-the-collection-and-deck-sizes-in-gothic] |
| FAQ “treat as” card text | Official Updated Cards/current text | Nov 2025 onward | Store explicit supersession and effective text provenance. [CITED: https://sorcerytcg.com/news/sorcery-contested-realm-card-updates-2025] |
| Earlier Plague of Frogs/Vindictive Nation FAQs | Reversed FAQ rulings | 15 Jul 2026 | Changelog can be newer than rulebook; revision dates matter. [CITED: https://curiosa.io/codex/changelog] |

## Assumptions Log

| # | Claim | Section | Risk if Wrong |
|---|-------|---------|---------------|
| A1 | `licenseStatus`/`storagePolicy` fields are the best enforcement mechanism. | Distribution boundary | Low; exact field design can change without changing the required guardrail. |
| A2 | Written publisher permission is required before automated API retrieval or normalized-data redistribution because no API-specific grant was found and the site terms prohibit automated/systematic extraction. | Distribution boundary | High; legal review or publisher permission must settle scope before acquisition tooling runs. |

## Open Questions (RESOLVED)

1. **Publisher permission and redistribution scope resolve only at Plan 07's blocking human gate.**
   - Current site terms do not grant API/database/image redistribution and prohibit automated/systematic extraction. [CITED: https://sorcerytcg.com/terms]
   - This research does not claim permission exists. Synthetic-fixture pipeline work may proceed, but official acquisition, normalized publication, or permitted stored raw bytes require explicit written scope and clean-room attestation in Plan 07.
   - If that scope is absent or narrower than Plan 08, Phase 1 and DATA-01/02/03 remain pending and Plans 07/08 produce no completion summaries.
2. **Precedence is resolved as a documented project fail-closed policy, not a publisher hierarchy claim.**
   - Official pages establish current rulebook, Codex usage, dated updates, and specific reversals; no audited official page states a complete conflict hierarchy. [CITED: https://curiosa.io/codex/changelog] [VERIFIED: 01-CONTEXT.md]
   - D-01/D-02 therefore require the project ordering documented above. Only `authorityClass: official` may win; equal-rank, unclear-date/scope, and unresolved conflicts remain `unsupported` while community/reference records remain non-normative provenance.
3. **The first card snapshot requires one Plan 07-approved durable exact-byte path.**
   - The gate must select permitted fixed repository bytes, an immutable approved archive/custodian with exact locator and SHA-256 values, or an approved acquisition procedure with exact expected hashes that refuses mismatched bytes.
   - A caller-local `AUTHORITY_INPUT_DIR` by itself is not durable. Plan 08 must verify the independent canonical input-root hash, rebuild twice from a clean materialization with networking denied, and halt if no such path is approved.

## Environment Availability

| Dependency | Required By | Available | Version | Fallback |
|------------|-------------|-----------|---------|----------|
| Node.js 24.19.0 | Runtime/native TS tests | Wrong version | 22.22.2 | Install/pin Node 24 before implementation. [VERIFIED: environment probe] |
| pnpm 11.22.0 | Package management | Wrong version | 9.15.9 | Activate pinned pnpm after Node upgrade. [VERIFIED: environment probe] |
| TypeScript 6.0.3 | Type checking | Missing | — | Install as dev dependency. [VERIFIED: environment probe] |
| Official-source network access | Acquisition only | Available during research | — | Plan 07-approved repository/archive/procedure plus exact input lock; an executor-local directory alone is not a fallback. |
| Podman | Not required by Phase 1 | Not probed | — | No container should be introduced. [VERIFIED: AGENTS.md] |

**Missing dependencies with no fallback:** Node 24 and the pinned development toolchain must be installed before implementation verification.  
**Missing dependencies with fallback:** Network acquisition can remain disabled while the pipeline is developed against synthetic fixtures; official publication still halts until Plan 07 approves permission and one durable exact-byte path.

## Validation Architecture

### Test Framework

| Property | Value |
|----------|-------|
| Framework | `node:test` + `node:assert/strict`, bundled with Node 24.19.0 |
| Config file | none — add package scripts in Wave 0 |
| Quick run command | `node --test tests/authority/*.test.ts` |
| Full suite command | `pnpm typecheck && pnpm lint && pnpm test` |

### Phase Requirements -> Test Map

| Req ID | Behavior | Test Type | Automated Command | File Exists? |
|--------|----------|-----------|-------------------|-------------|
| DATA-01 | Builds write-once bundle; records every source field/authority class; resolves precedence; rehashes stored bytes and verifies manifest-only locator/hash/ref bindings offline; ambiguous authority fails. | integration/golden | `node --test tests/authority/bundle.test.ts` | No — Wave 0 |
| DATA-02 | Exact durable input lock/root hash is verified before import; two clean no-network builds produce a byte-identical canonical card snapshot; malformed/unknown/duplicate/mismatched records fail. | integration/property loop/release | `node --test tests/authority/card-snapshot.test.ts tests/authority/official-revision.test.ts` | No — Wave 0 plus Plan 08 release test |
| DATA-03 | Every artifact envelope has stable ID/schema/provenance/hash; canonical vectors pass; tampering, missing refs, cycles, unsupported schema, and recomputed-hash mismatch fail. | unit/integration | `node --test tests/authority/canonical-json.test.ts tests/authority/provenance.test.ts` | No — Wave 0 |

### Required Test Cases

- RFC 8785 key-order, number, Unicode, array-order, and official-example vectors; reject unsupported JS values. [CITED: https://www.rfc-editor.org/rfc/rfc8785.html]
- A one-byte source change changes the raw hash; a payload change changes the artifact hash; formatting alone does not change canonical identity.
- Normalize the same raw input repeatedly and in differently ordered object-property fixtures; canonical bytes/hash stay identical.
- Reject unknown keys, missing fields, invalid dates/hashes, duplicate stable IDs/printing slugs, broken refs, reference cycles, disallowed stored media, and path traversal.
- Accept reviewed HTTPS community/reference records only under non-normative authority classes; reject any attempt for them to win precedence.
- Rehash permitted stored-source bytes. For manifest-only records, tamper the durable locator/procedure, expected byte hash, and `SourceRef` and prove canonical/root validation fails without claiming absent bytes were reread.
- Refuse a changed input-lock hash, missing/extra file, or one-byte raw mismatch before normalization; record the verified input-root hash and independently calculated bundle-root hash.
- Sort all validation issues deterministically by JSON Pointer/code/message and assert exact paths.
- Run successful bundle validation with network calls disabled/stubbed to throw.
- Refuse overwrite of an existing revision and prove a failed build leaves no published partial revision.
- Resolve dated/specific fixtures and emit explicit `unsupported` for ambiguous/equal-rank conflicts.

### Sampling Rate

- **Per task commit:** relevant authority test file plus `pnpm typecheck`.
- **Per wave merge:** `pnpm typecheck && pnpm lint && pnpm test`.
- **Phase gate:** full suite green; permission/clean-room/durable-input review complete; explicit fetch/http/https/net denial active; two clean rebuilds from the approved input lock are byte-identical; expected bundle root is captured outside and recomputed independently; stored/manifest/artifact tamper copies fail; restricted-content scans pass; published bytes remain unchanged.

### Wave 0 Gaps

- [ ] `package.json`, `pnpm-lock.yaml`, `tsconfig.json`, and flat ESLint config with pinned scripts/dependencies.
- [ ] `tests/authority/canonical-json.test.ts` and RFC-derived vectors.
- [ ] `tests/authority/provenance.test.ts` for envelope/reference/tamper behavior.
- [ ] `tests/authority/card-snapshot.test.ts` with synthetic, malformed, and approved minimal fixtures.
- [ ] `tests/authority/bundle.test.ts` with current/superseded/ambiguous source graphs, canonical input lock, stored-source and manifest-only validation cases, and offline guard.
- [ ] License-safe fixture policy and a test ensuring prohibited media/raw source bytes are not publishable.

## Security Domain

### Applicable ASVS Categories

| ASVS Category | Applies | Standard Control |
|---------------|---------|-----------------|
| V2 Authentication | no | No users or authentication in Phase 1. |
| V3 Session Management | no | No sessions. |
| V4 Access Control | no | Local developer tool; constrain filesystem paths. |
| V5 Input Validation | yes | Zod strict schemas, allowlisted source schemes/hosts/media, size/count/depth limits, exact-path errors. |
| V6 Cryptography | yes | Node SHA-256 for integrity identity; do not describe an unsigned hash as source authentication. |

### Known Threat Patterns for TypeScript File Import

| Pattern | STRIDE | Standard Mitigation |
|---------|--------|---------------------|
| Malformed or malicious JSON | Tampering / DoS | Strict schemas, bounded bytes/records/depth, no coercion, deterministic error cap. |
| Path traversal/symlink escape | Tampering | Resolve and verify every input/output remains under configured authority roots; refuse absolute and `..` paths. |
| Source substitution | Spoofing | Require reviewed HTTPS URL plus authority class; allowlist publisher hosts for normative official records; bind exact bytes to an independent durable input-root hash; hashes alone do not authenticate origin. |
| Artifact/reference tampering | Tampering | Recompute every canonical/stored-source hash, verify manifest-only locator/hash/ref bindings, and recursively validate references/cycles against an independently supplied root hash. |
| Restricted asset publication | Information disclosure / legal | Manifest storage policy, deny images/PDF/raw corpus by default, human approval checkpoint. |

## Sources

### Primary (HIGH confidence)

- https://sorcerytcg.com/how-to-play — official rules entry point.
- https://sorcerytcg.com/news/sorcery-contested-realm-december-2025-rulebook-update — current rulebook release and changes.
- https://sorcerytcg.com/constructed — current base Constructed rules.
- https://curiosa.io/codex, https://curiosa.io/faqs, https://curiosa.io/codex/changelog — official Codex/FAQ/change history.
- https://sorcerytcg.com/news/sorcery-contested-realm-card-updates-2025 — official Updated Cards policy.
- https://api.sorcerytcg.com/api/cards — audited official card payload.
- https://sorcerytcg.com/terms — official terms governing site/Curiosa/database/media use.
- https://nodejs.org/download/release/latest-v24.x/docs/api/typescript.html, https://nodejs.org/download/release/latest-v24.x/docs/api/test.html, https://nodejs.org/download/release/latest-v24.x/docs/api/crypto.html — runtime behavior.
- https://zod.dev/api, https://zod.dev/error-customization — strict schemas and diagnostic paths.
- https://www.rfc-editor.org/rfc/rfc8785.html — canonical JSON semantics/vectors.

### Secondary (MEDIUM confidence)

- https://github.com/realms-cards/contested-realms/blob/main/LICENSE and https://www.gnu.org/licenses/gpl — GPL boundary.
- https://github.com/JollyGrin/sorcery-tcg-playtest and https://docs.github.com/en/repositories/managing-your-repositorys-settings-and-features/customizing-your-repository/licensing-a-repository — no-license boundary.

### Tertiary (LOW confidence)

- None; unresolved legal and precedence questions are explicitly gated above.

## Metadata

**Confidence breakdown:**
- Standard stack: HIGH — versions verified on npm; APIs checked in official documentation; slopcheck passed.
- Architecture: HIGH — directly implements locked offline, immutable, canonical, fail-closed decisions.
- Official authority inventory: HIGH — publisher/official Curiosa sources audited through 2026-08-20.
- Redistribution/API permission: MEDIUM — restrictive terms are clear, but only written publisher clarification can settle project-specific permission.
- Pitfalls/validation: HIGH — derived from locked invariants, source behavior, and official serialization/schema/runtime contracts.

**Research date:** 2026-08-20  
**Valid until:** 2026-09-19 for technical stack; recheck Codex/API/terms immediately before acquiring a new authority revision.
