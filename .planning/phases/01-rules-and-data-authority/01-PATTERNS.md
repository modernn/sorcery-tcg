# Phase 1: Rules and Data Authority - Pattern Map

**Mapped:** 2026-08-20
**Files analyzed:** 22 expected new files/file groups
**Analogs found:** 0 / 22

This repository contains planning artifacts and `AGENTS.md`, but no production TypeScript, package configuration, command, data, or test files. Consequently, every assignment below is a research-derived starting contract, not an in-repository implementation analog. Do not treat either audited external project as a code analog: Contested Realms is GPL-3.0 and the audited spells.bar/playtest revision has no reusable license.

## File Classification

| New/Modified File | Role | Data Flow | Closest Analog | Match Quality |
|---|---|---|---|---|
| `package.json` | config | batch | none; `01-RESEARCH.md:119-155`, `01-VALIDATION.md:16-24` | no analog |
| `pnpm-lock.yaml` | config | batch | none; generated from pinned dependencies | no analog |
| `tsconfig.json` | config | transform | none; `01-RESEARCH.md:123-130` | no analog |
| `eslint.config.js` | config | transform | none; `01-RESEARCH.md:132-139` | no analog |
| `src/authority/schemas.ts` | model | transform | none; `01-RESEARCH.md:310-336` | no analog |
| `src/authority/canonical-json.ts` | utility | transform | none; `01-RESEARCH.md:259-261` | no analog |
| `src/authority/hash.ts` | utility | transform | none; `01-RESEARCH.md:338-353` | no analog |
| `src/authority/normalize-cards.ts` | service | batch/transform | none; `01-RESEARCH.md:255-257` | no analog |
| `src/authority/validate-bundle.ts` | service | file-I/O/batch | none; `01-RESEARCH.md:175-204` | no analog |
| `src/commands/import-authority.ts` | controller | file-I/O/batch | none; `01-RESEARCH.md:206-228` | no analog |
| `src/commands/validate-authority.ts` | controller | file-I/O/request-response | none; `01-RESEARCH.md:206-228` | no analog |
| `data/authority/<revision-id>/bundle.json` | config | file-I/O | none; generated immutable artifact | no analog |
| `data/authority/<revision-id>/sources.json` | model | file-I/O | none; generated immutable artifact | no analog |
| `data/authority/<revision-id>/formats.json` | model | file-I/O | none; generated immutable artifact | no analog |
| `data/authority/<revision-id>/cards.normalized.json` | model | file-I/O | none; generated immutable artifact | no analog |
| `docs/authority-precedence.md` | config | request-response | none; `01-CONTEXT.md` D-01 through D-03 | no analog |
| `docs/external-reuse-policy.md` | config | request-response | none; `01-RESEARCH.md:104-117` | no analog |
| `tests/authority/canonical-json.test.ts` | test | transform | none; `01-VALIDATION.md:41,54` | no analog |
| `tests/authority/provenance.test.ts` | test | file-I/O/transform | none; `01-VALIDATION.md:42,55` | no analog |
| `tests/authority/card-snapshot.test.ts` | test | batch/transform | none; `01-VALIDATION.md:43,56` | no analog |
| `tests/authority/bundle.test.ts` | test | file-I/O/batch | none; `01-VALIDATION.md:44-45,57` | no analog |
| `tests/authority/fixtures/**` | config | file-I/O | none; synthetic/minimal test data only | no analog |

The paths above come directly from the recommended structure in `01-RESEARCH.md:206-228` and the Wave 0 contract in `01-VALIDATION.md:51-57`. Do not add separate precedence, repository, network-client, database, container, or rules-engine modules in this phase unless implementation proves one of these files cannot hold the required behavior.

## Pattern Assignments

### `package.json`, `pnpm-lock.yaml`, `tsconfig.json`, `eslint.config.js` (config)

**Analog:** None. The repository has no package/tooling implementation.

**Research fallback:** `01-RESEARCH.md:123-139`; commands are locked by `01-VALIDATION.md:20-23`.

Use one pnpm package, pin `packageManager` and the researched versions, use ESM-compatible native TypeScript, and keep type checking separate because Node's type stripping does not type-check. Required command surface:

```text
node --test tests/authority/*.test.ts
pnpm typecheck && pnpm lint && pnpm test
```

The test runner is `node:test` with `node:assert/strict`; no test-runner config or additional test framework is needed. The only Phase 1 production dependency identified by research is Zod for strict trust-boundary schemas. Use Node built-ins for hashing, filesystem operations, paths, and tests. `pnpm-lock.yaml` is generated, never manually patterned.

---

### `src/authority/schemas.ts` (model, transform)

**Analog:** None.

**Research fallback — strict validation pattern** (`01-RESEARCH.md:312-335`):

```typescript
import { z } from 'zod';

const SourceRecord = z.strictObject({
  sourceId: z.string().min(1),
  url: z.url(),
  retrievedAt: z.iso.datetime(),
  effectiveDate: z.iso.date().nullable(),
  mediaType: z.string().min(1),
  byteHash: z.string().regex(/^sha256:[0-9a-f]{64}$/),
  licenseStatus: z.enum(['approved', 'manifest-only', 'permission-required']),
});

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

Apply this shape to raw input, normalized cards, artifact envelopes, manifests, and bundles: strict objects, no coercion or repair, JSON-Pointer paths, and deterministic issue ordering. Add explicit size/count/depth limits and reject unknown keys, duplicate stable IDs/source IDs/printing slugs, invalid dates/hashes, and prohibited storage policy values.

---

### `src/authority/canonical-json.ts` (utility, transform)

**Analog:** None.

**Research fallback:** `01-RESEARCH.md:259-261`.

Implement one project-owned serializer over already validated JSON values. It must use UTF-8, recursively sort object keys by UTF-16 code units, preserve array order, use ECMAScript primitive serialization, and perform no Unicode normalization. Reject non-finite or unsafe numbers, `undefined`, sparse arrays, unsupported prototypes, and duplicate-key input before canonicalization. Do not scatter ordinary `JSON.stringify` calls across identity-producing code.

The canonical serializer is the single shared identity boundary for source manifests, normalized snapshots, bundles, and all later canonical artifacts. Lock it with RFC 8785-derived vectors before other authority code depends on it.

---

### `src/authority/hash.ts` (utility, transform)

**Analog:** None.

**Research fallback — Node hashing pattern** (`01-RESEARCH.md:340-352`):

```typescript
import { createHash } from 'node:crypto';
import { canonicalJson } from './canonical-json.ts';

export function sha256(bytes: Uint8Array): `sha256:${string}` {
  return `sha256:${createHash('sha256').update(bytes).digest('hex')}`;
}

export function identityHash(value: unknown): `sha256:${string}` {
  return sha256(Buffer.from(canonicalJson(value), 'utf8'));
}
```

Hash raw source bytes directly. Hash canonical artifact identity documents with `contentHash` omitted, then store the result beside the identity. A hash proves byte integrity, not publisher authenticity.

---

### `src/authority/normalize-cards.ts` (service, batch/transform)

**Analog:** None.

**Research fallback:** the pure pipeline at `01-RESEARCH.md:255-257`:

```text
read bytes -> hash bytes -> strict parse -> normalize -> strict validate
-> canonicalize -> hash -> write new path -> offline revalidate
```

Keep normalization pure: accept pinned bytes plus explicit retrieval/effective metadata and return normalized values. Do not read the clock, network, locale, directory order, or randomness. Preserve official source identifiers when present, mint stable project IDs deterministically, and fail if input/output counts differ without an explicit mapping result. Do not interpret card rules text or implement card behavior in Phase 1.

---

### `src/authority/validate-bundle.ts` (service, file-I/O/batch)

**Analog:** None.

**Research fallback:** the architecture flow at `01-RESEARCH.md:177-203` and threats at `01-RESEARCH.md:453-461`.

Recursively validate a selected local bundle without network access. Recompute raw and canonical hashes; validate schema versions, stable IDs, parent/source references, cycles, precedence results, storage policy, and path confinement. Resolve every path under configured authority roots and reject absolute paths, `..`, and symlink escapes. Sort diagnostics by JSON Pointer, code, then message. Ambiguous or equal-rank official conflicts become explicit `unsupported` records rather than guessed outcomes.

---

### `src/commands/import-authority.ts` (controller, file-I/O/batch)

**Analog:** None.

**Research fallback:** `01-RESEARCH.md:178-203,255-257`.

This is an imperative filesystem shell around the pure authority functions. It consumes approved local files; it is not a live acquisition client. Build into a new revision path, use a same-directory temporary path and atomic rename, recursively validate before publication, refuse to overwrite an existing revision, and leave no published partial revision after failure. Any future network acquisition remains a separate human-permission-gated maintenance action.

---

### `src/commands/validate-authority.ts` (controller, file-I/O/request-response)

**Analog:** None.

Keep the command thin: parse arguments, call the offline bundle validator, print deterministically ordered diagnostics, and set a non-zero exit status on every invalid, tampered, ambiguous, unsupported, or policy-prohibited bundle. Do not fetch, repair, rewrite, or drop data during validation.

---

### `data/authority/<revision-id>/{bundle,sources,formats,cards.normalized}.json` (immutable data, file-I/O)

**Analog:** None.

These are generated outputs, not hand-maintained examples. The common identity pattern from `01-RESEARCH.md:234-250` is:

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
  contentHash: `sha256:${string}`;
}>;
```

`sources.json` records URL, retrieval timestamp, effective date when available, media type, byte hash, derivation metadata, and legal/storage status. `bundle.json` binds exact source/artifact references and precedence policy. `formats.json` keeps base formats separate from explicitly scoped overlays. `cards.normalized.json` contains deterministic canonical records, not executable behavior.

Do not publish a real revision until its input and redistribution status are approved. Until then, only license-safe synthetic/minimal fixtures should exercise the pipeline.

---

### `docs/authority-precedence.md` and `docs/external-reuse-policy.md` (config/policy)

**Analog:** None.

`authority-precedence.md` must record the locked precedence and effective-date policy: explicit official supersession/reversal; current card-specific updates; explicitly selected scoped overlays; compatible rulebook/Codex/FAQ detail; then `unsupported` when rank, scope, or dates cannot resolve a conflict. Retain both winning and superseded/contending source references.

`external-reuse-policy.md` must name allowed behavioral observations, forbidden copying, source URLs, audited revisions, and reviewer sign-off. It must state:

- Official sources are normative; community projects are provenance/examples only.
- Do not copy code, tests, assets, card implementations, or data from Contested Realms without an explicit GPL product decision.
- Do not copy code, tests, assets, or data from the unlicensed spells.bar/playtest revision without written permission.
- Do not commit publisher PDFs, images, raw API corpora, or normalized derivatives until permission/storage review allows it.
- Fixtures remain synthetic or minimal factual records; independent implementation and reviewer attestation are required.

---

### `tests/authority/canonical-json.test.ts` (test, transform)

**Analog:** None.

Use `node:test` and `node:assert/strict`. Cover RFC-derived key ordering, numeric encoding, Unicode, array ordering, official examples, and rejection of unsupported JS values. Assert that formatting/property insertion order does not change canonical bytes or identity, while a one-byte/payload change changes its respective hash.

---

### `tests/authority/provenance.test.ts` (test, file-I/O/transform)

**Analog:** None.

Cover strict artifact/source envelopes, unknown fields, unsupported schema versions, stable IDs, tampering, recomputed-hash mismatch, missing and broken references, reference cycles, path traversal, absolute paths, and symlink escape. Assert exact, deterministically sorted diagnostic paths.

---

### `tests/authority/card-snapshot.test.ts` (test, batch/transform)

**Analog:** None.

Normalize the same pinned synthetic fixture twice and from differently ordered object-property inputs; canonical bytes and hashes must match. Reject malformed/unknown records, duplicate stable IDs and printing slugs, invalid dates/hashes, and unexplained input/output count differences. Stub or disable networking so any accidental access fails the test.

---

### `tests/authority/bundle.test.ts` (test, file-I/O/batch)

**Analog:** None.

Cover current, superseded, explicitly scoped, and ambiguous source graphs; full recursive offline validation; source fields and hashes; prohibited publisher media/raw storage; immutable write-once publication; atomic-failure cleanup; tampering; and no-network execution. Two clean rebuilds from identical inputs must produce byte-identical output.

---

### `tests/authority/fixtures/**` (test data, file-I/O)

**Analog:** None.

Keep fixtures compact, synthetic, deterministic, and license-safe. Include only the minimal valid/malformed records, graph edges, and factual fields needed by the four tests. Do not copy external repository tests or publisher corpora as “fixtures.”

## Shared Patterns

### Functional Core, Imperative Shell

**Source:** `.planning/research/ARCHITECTURE.md:377-383`; `01-RESEARCH.md:255-257`  
**Apply to:** normalization and hashing as pure functions; commands as the only filesystem shell.

No clock, network, locale-sensitive ordering, random input, or filesystem enumeration may influence canonical values.

### Strict Boundary Validation

**Source:** `01-RESEARCH.md:310-335`  
**Apply to:** raw inputs, normalized cards, source records, artifact envelopes, and bundles.

Use `z.strictObject`, no coercion or silent repair, exact JSON-Pointer paths, and deterministic issue sorting.

### Canonical Identity

**Source:** `01-RESEARCH.md:230-253,259-261,338-353`  
**Apply to:** every canonical artifact and later deck, collection, behavior, and experiment artifact.

Stable ID names the logical entity; canonical content hash names the immutable revision. Never conflate them or hash a document containing its own digest.

### Offline, Write-Once File Safety

**Source:** `01-RESEARCH.md:255-257,304-306,453-461`  
**Apply to:** import command, validator, bundle publication, and bundle tests.

Constrain paths, reject escapes, build at a new location, validate before atomic rename, refuse overwrite, and never make runtime validation network-capable.

### Fail-Closed Error Handling

**Source:** `01-CONTEXT.md` D-02 and D-07; `01-RESEARCH.md:291-306`  
**Apply to:** all schemas, normalization, bundle validation, precedence, and commands.

Malformed data, ambiguity, restricted storage, and broken integrity stop the operation with stable diagnostics. No “best effort,” dropped cards, inferred ruling, or automatic repair is authoritative.

### Native Node Test and Verification Commands

**Source:** `01-VALIDATION.md:16-33`  
**Apply to:** all implementation tasks.

```text
# directly relevant test, then typecheck after every task commit
node --test tests/authority/<area>.test.ts
pnpm typecheck

# after every wave and at the phase gate
pnpm typecheck && pnpm lint && pnpm test
```

At the phase gate, run the full suite with networking unavailable and confirm two clean rebuilds are byte-identical.

### Clean-Room Review Gate

**Source:** `01-VALIDATION.md:61-67`; `01-RESEARCH.md:104-117`  
**Apply to:** all code, tests, fixtures, data, and source manifests.

Publisher permission and external-reuse attestation are manual gates and cannot be replaced by automated tests. Record reviewer, date, source/revision, permitted storage/use, attribution, and any revocation/update expectations.

## No Analog Found

All 22 classified files/file groups have no codebase analog. The planner should use the cited `01-RESEARCH.md`, `01-VALIDATION.md`, and locked `01-CONTEXT.md` contracts, not external implementation source.

| Area | Reason |
|---|---|
| Tooling/config | Repository has no `package.json`, TypeScript config, ESLint config, or lockfile. |
| Authority library | Repository has no `src/` directory or TypeScript implementation. |
| Commands | Repository has no CLI/command implementation. |
| Data artifacts | Repository has no committed authority revision. |
| Tests/fixtures | Repository has no `tests/` directory or fixture convention. |
| Documentation | Existing planning documents define decisions, but no production policy document exists to copy structurally. |

## Metadata

**Analog search scope:** entire repository via hidden-file inventory; `.codex/skills/` and `.agents/skills/` were also checked and do not exist.  
**Files scanned:** 15 repository files; all are planning documents or `AGENTS.md`.  
**External-source policy:** external repositories were not used as code analogs and no external code was copied.  
**Pattern extraction date:** 2026-08-20
