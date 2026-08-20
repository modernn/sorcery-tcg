# Phase 1: Rules and Data Authority - Research

**Researched:** 2026-08-20
**Domain:** Immutable rules authority, normalized card data, provenance, and clean-room reuse
**Confidence:** HIGH for architecture and official-source inventory; MEDIUM for the unresolved API-page/Terms conflict governing private personal use; no redistribution or automation permission is claimed

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
- **D-08 (amended 2026-08-20):** The complete official source set, derived normalized snapshot, source/input locks, and built official authority revision remain private under git-ignored `.local/authority/`; Git keeps only project code, schemas/policy, synthetic or minimal fixtures, non-content metadata, and independent hashes/receipts. Card art stays excluded, and official PDF/page/API bytes never enter the built revision, Git, or packages.
- **D-09:** Contested Realms (GPL-3.0) and spells.bar/the playtest project (no reusable license found in the audited revision) are behavioral and UX references only. Do not copy their source, card implementations, assets, or data into this project.
- **D-10:** Phase 1 uses TypeScript and Node standard-library facilities first. Add a dependency only where runtime schema validation or deterministic normalization is materially safer than a small local implementation.

### the agent's Discretion
- Exact directory names, JSON field ordering, command names, and test file layout, provided the offline validation and immutable provenance requirements remain obvious.
- How to preserve exact private rebuild evidence without committing any raw response, normalized snapshot, built revision, or automated retrieval tooling; the resolved answer is an ignored seven-file primary source set plus independent durable private backup set and committed non-content hashes/receipts.

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

Phase 1 should build one private, local, content-addressed authority pipeline, not a rules engine or network service. The user manually supplies the exact seven-file official rulebook/format/Codex-FAQ/changelog/card-update/API source set plus an independent byte-identical backup; project code verifies, derives, and normalizes it under git-ignored `.local/authority/`, while Git retains only code, synthetic/minimal fixtures, policy, safe non-sensitive source metadata, and independent hashes/receipts. Runtime consumers receive only a validated local bundle ID and independently supplied hash and never fetch mutable authority data. [VERIFIED: 01-CONTEXT.md]

The official authority is genuinely multi-versioned. The current official rulebook update is dated 19 December 2025, while the official Codex changelog contains rulings and Updated Cards through 15 July 2026, including reversals of earlier FAQs. The official Constructed baseline is 1 Avatar, a minimum 60-card Spellbook, a minimum 30-card Atlas, and rarity copy limits; event floor rules are separate overlays, not global game rules. [CITED: https://sorcerytcg.com/news/sorcery-contested-realm-december-2025-rulebook-update] [CITED: https://curiosa.io/codex/changelog] [CITED: https://sorcerytcg.com/constructed]

The official API landing page allows developer access and recommends intermittent polling, diffing, and hosting required data, while the site Terms prohibit automated access, systematic retrieval/database construction, and scraper/data-mining activity without written permission. Those official statements remain in tension. The private-local plan avoids making a legal conclusion: the user manually saves the complete rulebook/format/Codex-FAQ/changelog/card-update/API source set through a browser; project code never fetches, polls, scrapes, hosts, commits, packages, or redistributes the corpus. Images/private CDN data remain excluded. [CITED: https://api.sorcerytcg.com/] [CITED: https://api.sorcerytcg.com/api/cards] [CITED: https://sorcerytcg.com/terms]

**Primary recommendation:** Implement a strict offline import/build/validate toolchain over the manually saved, independently hashed complete private source set and its independent durable backup set; git-ignore raw/normalized/built authority data and keep every acquisition/network path outside project code.

## Architectural Responsibility Map

| Capability | Primary Tier | Secondary Tier | Rationale |
|------------|-------------|----------------|-----------|
| Official-source acquisition | Human browser action outside project code | Private filesystem | Seven manually saved official files plus an independent byte-identical backup set; no project fetch/scrape/poll path. [VERIFIED: 01-CONTEXT.md] |
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
| Rulebook | The 19 Dec 2025 official update says it aligns the rulebook with Gothic, changes constructed deck size to 60, and incorporates selected Codex terminology. [CITED: https://sorcerytcg.com/news/sorcery-contested-realm-december-2025-rulebook-update] | Save the current standard PDF manually into the ignored primary/backup source sets and independently hash it; never copy PDF bytes into the built revision, Git, or packages. |
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

The official API landing page explicitly permits developer access and recommends intermittent polling, diffing, and hosting required data; it excludes images and private CDN use. [CITED: https://api.sorcerytcg.com/]

The official Terms cover `sorcerytcg.com`, `play.sorcerytcg.com`, and `curiosa.io`; permit limited personal/noncommercial use; and prohibit automated access, systematic retrieval/database construction, and scraping/data mining without written permission. A hash and URL establish provenance/integrity, not legal permission. [CITED: https://sorcerytcg.com/terms]

### Private-local resolution — 2026-08-20

The two official pages are in tension. This research does not resolve that tension or claim legal certainty. For the user's entirely private, noncommercial, non-released tool, acquisition is one manual browser download; project code has no fetch, scraping, polling, update, hosting, or redistribution path.

No audited community repository supplies a complete permissively licensed gameplay corpus. `sadkinglabs/sorcery-registry` is technically strong, but its MIT license covers code while its card data is explicitly reserved to Erik's Curiosa; other reviewed candidates are GPL, unlicensed, or insufficient. Community sources therefore remain behavioral/provenance references only. [CITED: https://github.com/sadkinglabs/sorcery-registry]

The complete raw source set, source/input locks, normalized snapshot, and built authority revision remain private under git-ignored `.local/authority/`. Git may contain project code/schemas/policies, synthetic or minimal fixtures, source URL/date, independent byte/root hashes and receipts, and generic final-gate tests. Card art is excluded everywhere. Publisher PDFs/pages and the full API response may exist only in the ignored primary/backup source roots; none enters the built revision, Git, packages, or a public/network HTTP card API. Sharing, releasing with content, automated updating, or commercialization requires written publisher permission and a separate plan. [VERIFIED: 01-CONTEXT.md]

`realms-cards/contested-realms` declares GPL-3.0. GPL-covered copying or adaptation, including close translation, can impose GPL obligations on the combined conveyed work; D-09 therefore fixes a behavior-only boundary. Do not copy source, tests, rule implementations, assets, or data. [CITED: https://github.com/realms-cards/contested-realms/blob/main/LICENSE] [CITED: https://www.gnu.org/licenses/gpl] [CITED: https://www.gnu.org/licenses/gpl-faq.en.html]

The audited `JollyGrin/sorcery-tcg-playtest` repository had no LICENSE file even though its README calls the project open source. GitHub documents that absent a license, default copyright applies and others may not reproduce, distribute, or create derivative works. Treat spells.bar/playtest source, tests, assets, and data as non-reusable unless the owner grants a license in writing. [CITED: https://github.com/JollyGrin/sorcery-tcg-playtest] [CITED: https://docs.github.com/en/repositories/managing-your-repositorys-settings-and-features/customizing-your-repository/licensing-a-repository]

Required implementation guardrails:

- Add `docs/external-reuse-policy.md` with allowed behavioral observations, forbidden copying, source URLs, audited revisions, and reviewer sign-off. [VERIFIED: 01-CONTEXT.md]
- Keep raw official bytes, normalized output, and built revisions outside Git and packages unconditionally for this operating model; allow only safe metadata/receipts and synthetic/minimal fixtures. [VERIFIED: 01-CONTEXT.md]
- Put a `licenseStatus`/`storagePolicy` field on each source record so the builder rejects forbidden stored bytes. [ASSUMED]
- Require a blocking human checkpoint to perform/attest the manual browser saves, private/noncommercial/no-redistribution scope, seven-entry metadata/hash lock, canonical source-set root hash, and independent durable private backup root; the checkpoint records an operating decision, not legal permission. [VERIFIED: user clarification 2026-08-20]
- Give every source an `authorityClass`. Reviewed secure community/reference sources may remain as provenance, but only allowlisted `official` sources can enter normative precedence. [VERIFIED: 01-CONTEXT.md]
- The remaining official-data path uses one exact seven-file private source set plus an independent durable backup set. Rehash every corresponding file and require set/root equality before each build. In the private authority bundle, record every publisher source as `manifest-only` with `licenseStatus: permission-required`, prohibited storage, and a SHA-256 URN locator; this preserves the implemented fail-closed permission guard while binding every normalized record to the independently verified private input without copying raw bytes into the revision. [VERIFIED: 01-CONTEXT.md]
- Before DATA-01/02/03 completion, require Plan 07 to confirm all seven fixed relative paths, official URLs/dates/media types, independently computed byte hashes/lengths, matching independent backup identities/bytes, canonical source-set root hash, and no official source/artifact tracked by Git. [VERIFIED: user clarification 2026-08-20]

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
 complete private source-set verification + scoped attestation ---- failed ----> hard stop; phase pending
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
  import-authority.ts     # consumes a clean private materialization matching the recorded input lock
  validate-authority.ts
.local/authority/revisions/<revision-id>/  # private, Git-ignored, never packaged
  bundle.json             # refs/hashes, no copyrighted PDFs/images
  sources.json
  formats.json
  cards.normalized.json
  # no raw input copy; the manifest binds the independently verified private SHA-256
data/authority/receipts/<revision-id>.json # safe non-content URL/date/hash receipt only
data/authority/README.md                   # private-local rebuild/selection policy
docs/
  authority-precedence.md
  external-reuse-policy.md
tests/authority/
  fixtures/               # synthetic/minimal and license-safe
tests/private-authority/
  private-revision.test.ts # opt-in gate; clean-clone default tests remain synthetic
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

`independently verify the exact seven-file primary and backup sets -> require path/metadata/identity/byte equality -> deterministically derive exact importer inputs -> strict parse -> normalize -> strict validate -> canonicalize -> hash -> write a new private-local path -> offline revalidate`. Pass retrieval time/effective date as recorded source metadata; never call the clock, network, locale-sensitive sort, or random functions from normalization. Bind every normalized/card/format/source record to the exact source-set and derived-input hashes, refuse to overwrite an existing revision, and use a same-directory temporary file plus atomic rename for new writes. [VERIFIED: 01-CONTEXT.md]

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
**Avoid:** hash source bytes at acquisition/import, preserve retrieval/effective dates, record a canonical seven-entry source-set lock plus an independently stored byte-identical backup set, and never rebuild an old revision from an unverified fresh fetch.
**Warning signs:** a required official source has only a URL/hash, or the primary and backup resolve to the same/nested/linked storage identity.

### Pitfall 2: Silent “Helpful” Normalization
**What goes wrong:** misspelled fields are dropped, strings are trimmed/coerced, or malformed cards disappear.  
**Avoid:** strict parse both sides; transformations are explicit and tested; diagnostics sort by JSON Pointer, code, then message.  
**Warning signs:** input count and output count differ without a documented mapping result.

### Pitfall 3: Mixing Acquisition with Determinism
**What goes wrong:** timestamps, network response order, locale, or upstream changes alter output.  
**Avoid:** manual browser materialization plus the reusable source-set verifier produces complete private bytes that match the independent source/input locks; normalization is a pure second operation over those verified local inputs.

### Pitfall 4: Legal Contamination
**What goes wrong:** copied GPL implementation or unlicensed playtest assets/tests enter the repository, or official images/raw database are redistributed.  
**Avoid:** clean-room policy, source-review checklist, private-root Git/package leakage scans, and reviewer attestation; written publisher permission is a future trigger for sharing/automation/commercial use, not the current private build gate.

### Pitfall 5: Partial or In-Place Revision Writes
**What goes wrong:** a failed build corrupts an existing authority revision.  
**Avoid:** build in a new revision directory, validate fully, atomically finalize the local revision, and fail if the target exists.

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
  licenseStatus: z.enum(['approved', 'permission-required', 'reference-only', 'unknown']),
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
| A2 | The API landing page and Terms conflict is not legally resolved; manual complete-source-set capture plus private git-ignored use reduces operational exposure but does not establish permission. | Distribution boundary | High; any sharing, release with content, automation, or commercialization still requires written publisher permission and a separate plan. |

## Open Questions (RESOLVED)

1. **The API-page/Terms conflict is operationally avoided, not legally resolved.**
   - The API page supports developer use; the Terms prohibit automated/systematic acquisition without written permission. [CITED: https://api.sorcerytcg.com/] [CITED: https://sorcerytcg.com/terms]
   - Plan 07 records the user's private/noncommercial/no-redistribution operating decision and manual browser source-set saves; it does not claim publisher permission or legal certainty.
   - Expansion to sharing, release with content, automated updating, or commercialization remains blocked on written publisher permission and a separate plan.
2. **Precedence is resolved as a documented project fail-closed policy, not a publisher hierarchy claim.**
   - Official pages establish current rulebook, Codex usage, dated updates, and specific reversals; no audited official page states a complete conflict hierarchy. [CITED: https://curiosa.io/codex/changelog] [VERIFIED: 01-CONTEXT.md]
   - D-01/D-02 therefore require the project ordering documented above. Only `authorityClass: official` may win; equal-rank, unclear-date/scope, and unresolved conflicts remain `unsupported` while community/reference records remain non-normative provenance.
3. **The first authority revision uses one exact seven-file manual source set plus an independent durable private backup set.**
   - The primary root is `.local/authority/inputs/official-2026-08-20/primary/` with fixed rulebook, Constructed, Codex, FAQ, changelog, card-update, and API relative paths; exact absolute roots remain only in the ignored private lock.
   - Plan 07 computes/records every URL/date/media/byte-length/SHA-256 plus the canonical source-set root and rejects same/nested/symlink/junction/hardlink backup identities before requiring all backup bytes to match before producing its summary.
   - Plan 08 independently reverifies both complete sets, derives exact importer inputs from each, builds from each with networking denied, compares outputs byte-for-byte, and keeps every raw/normalized/built byte git-ignored.

## Environment Availability

| Dependency | Required By | Available | Version | Fallback |
|------------|-------------|-----------|---------|----------|
| Node.js 24.19.0 | Runtime/native TS tests | Wrong version | 22.22.2 | Install/pin Node 24 before implementation. [VERIFIED: environment probe] |
| pnpm 11.22.0 | Package management | Wrong version | 9.15.9 | Activate pinned pnpm after Node upgrade. [VERIFIED: environment probe] |
| TypeScript 6.0.3 | Type checking | Missing | — | Install as dev dependency. [VERIFIED: environment probe] |
| Official-source network access | Human browser download only | Available outside project code | — | Save all seven required sources at the fixed git-ignored relative paths and maintain an independent matching private backup set; project code remains offline. |
| Podman | Not required by Phase 1 | Not probed | — | No container should be introduced. [VERIFIED: AGENTS.md] |

**Missing dependencies with no fallback:** Node 24 and the pinned development toolchain must be installed before implementation verification.  
**Missing dependencies with fallback:** Network acquisition remains absent while the pipeline is developed against synthetic fixtures. The private official build starts only after Plan 07 records the user's operating attestation plus matching exact bytes/metadata across the complete fixed primary source set and independent private backup; this checkpoint does not establish legal permission.

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
| DATA-01 | Builds write-once bundle; records every source field/authority class; resolves precedence; rehashes the complete seven-file primary and independent backup sets offline; ambiguous authority fails; Git contains no official corpus bytes. | integration/golden | `node --test tests/authority/bundle.test.ts` plus Plan 08's explicit private-revision gate | No — Wave 0 |
| DATA-02 | Exact complete primary/backup source-set bytes, source-set root, and derived input lock/root hashes are verified before import; two clean no-network builds produce a byte-identical canonical card snapshot; malformed/unknown/duplicate/mismatched records fail. | integration/property loop/release | `node --test tests/authority/card-snapshot.test.ts` plus `pnpm authority:verify-private` | No — Wave 0 plus Plan 08 private gate |
| DATA-03 | Every artifact envelope has stable ID/schema/provenance/hash; canonical vectors pass; tampering, missing refs, cycles, unsupported schema, and recomputed-hash mismatch fail. | unit/integration | `node --test tests/authority/canonical-json.test.ts tests/authority/provenance.test.ts` | No — Wave 0 |

### Required Test Cases

- RFC 8785 key-order, number, Unicode, array-order, and official-example vectors; reject unsupported JS values. [CITED: https://www.rfc-editor.org/rfc/rfc8785.html]
- A one-byte source change changes the raw hash; a payload change changes the artifact hash; formatting alone does not change canonical identity.
- Normalize the same raw input repeatedly and in differently ordered object-property fixtures; canonical bytes/hash stay identical.
- Reject unknown keys, missing fields, invalid dates/hashes, duplicate stable IDs/printing slugs, broken refs, reference cycles, disallowed stored media, and path traversal.
- Accept reviewed HTTPS community/reference records only under non-normative authority classes; reject any attempt for them to win precedence.
- Rehash all seven fixed primary/backup files independently, reject linked/non-independent identities, and require exact set/metadata/byte equality, then tamper source hashes, `SourceRef` bindings, and copied build artifacts to prove canonical/root validation fails without mutating the selected private revision.
- Refuse a changed input-lock hash, missing/extra file, or one-byte raw mismatch before normalization; record the verified input-root hash and independently calculated bundle-root hash.
- Sort all validation issues deterministically by JSON Pointer/code/message and assert exact paths.
- Run successful bundle validation with network calls disabled/stubbed to throw.
- Refuse overwrite of an existing revision and prove a failed build leaves no partial selected local revision.
- Resolve dated/specific fixtures and emit explicit `unsupported` for ambiguous/equal-rank conflicts.

### Sampling Rate

- **Per task commit:** relevant authority test file plus `pnpm typecheck`.
- **Per wave merge:** `pnpm typecheck && pnpm lint && pnpm test`.
- **Phase gate:** full suite green; private-use/clean-room/manual-input/backup evidence complete; explicit fetch/http/https/net denial active; two clean rebuilds from independently rehashed private primary/backup input locks are byte-identical; expected bundle root is captured outside and recomputed independently; raw-input/manifest-binding/artifact tamper cases fail; restricted-content scans pass; selected private revision bytes remain unchanged.

### Wave 0 Gaps

- [ ] `package.json`, `pnpm-lock.yaml`, `tsconfig.json`, and flat ESLint config with pinned scripts/dependencies.
- [ ] `tests/authority/canonical-json.test.ts` and RFC-derived vectors.
- [ ] `tests/authority/provenance.test.ts` for envelope/reference/tamper behavior.
- [ ] `tests/authority/card-snapshot.test.ts` with synthetic, malformed, and approved minimal fixtures.
- [ ] `tests/authority/bundle.test.ts` with current/superseded/ambiguous source graphs, canonical input-lock validation cases, and offline guard; Plan 08 adds a separate opt-in private-revision test so clean-clone default tests remain synthetic.
- [ ] License-safe fixture policy and a test ensuring prohibited media/raw source bytes cannot enter Git or packages.

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
| Source substitution | Spoofing | Require reviewed HTTPS URL plus authority class; allowlist publisher hosts for normative official records; bind every complete private source set to independent per-file hashes plus source-set and derived-input root hashes; hashes alone do not authenticate origin. |
| Artifact/reference tampering | Tampering | Recompute every canonical and private-source hash, compare every primary file with its independent durable backup counterpart, and recursively validate references/cycles against an independently supplied root hash. |
| Restricted asset leakage | Information disclosure / legal | Git-ignore all real inputs, normalized snapshots, and built revisions; deny images/PDF/full-corpus bytes from Git and packages; make the final gate scan tracked and staged content. |

## Sources

### Primary (HIGH confidence)

- https://sorcerytcg.com/how-to-play — official rules entry point.
- https://sorcerytcg.com/news/sorcery-contested-realm-december-2025-rulebook-update — current rulebook release and changes.
- https://sorcerytcg.com/constructed — current base Constructed rules.
- https://curiosa.io/codex, https://curiosa.io/faqs, https://curiosa.io/codex/changelog — official Codex/FAQ/change history.
- https://sorcerytcg.com/news/sorcery-contested-realm-card-updates-2025 — official Updated Cards policy.
- https://api.sorcerytcg.com/api/cards — audited official card payload.
- https://api.sorcerytcg.com/ — official developer-access/polling/self-host guidance and image/private-CDN exclusion.
- https://sorcerytcg.com/terms — official terms governing site/Curiosa/database/media use.
- https://nodejs.org/download/release/latest-v24.x/docs/api/typescript.html, https://nodejs.org/download/release/latest-v24.x/docs/api/test.html, https://nodejs.org/download/release/latest-v24.x/docs/api/crypto.html — runtime behavior.
- https://zod.dev/api, https://zod.dev/error-customization — strict schemas and diagnostic paths.
- https://www.rfc-editor.org/rfc/rfc8785.html — canonical JSON semantics/vectors.

### Secondary (MEDIUM confidence)

- https://github.com/realms-cards/contested-realms/blob/main/LICENSE and https://www.gnu.org/licenses/gpl — GPL boundary.
- https://github.com/JollyGrin/sorcery-tcg-playtest and https://docs.github.com/en/repositories/managing-your-repositorys-settings-and-features/customizing-your-repository/licensing-a-repository — no-license boundary.
- https://github.com/sadkinglabs/sorcery-registry — community audit: MIT code, card data reserved to Erik's Curiosa; not a reusable gameplay corpus.

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
