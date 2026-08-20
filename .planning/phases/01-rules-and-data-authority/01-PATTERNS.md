# Phase 1: Rules and Data Authority — Current Pattern Map

**Originally mapped:** 2026-08-20
**Updated:** 2026-08-20 after Plans 01-01 through 01-05 and the private-local source-set decision

Plans 01-01 through 01-05 established the package, strict authority schemas, canonical hashing, normalization, offline validation, and atomic import/validate commands. Remaining Plans 01-06 through 01-08 must extend those files and conventions; external projects remain behavior-only references and are never code/data analogs.

## Current File Map

| File or private path | Role | Current analog/contract |
|---|---|---|
| `src/authority/schemas.ts` | Strict trust-boundary schemas and common provenance envelopes | Existing implementation; use its official-host, source-record, stable-ID, diagnostic, and size-bound conventions |
| `src/authority/canonical-json.ts` | One deterministic JSON serializer | Existing implementation; all identity/root hashes use it |
| `src/authority/hash.ts` | Raw-byte and canonical identity SHA-256 | Existing implementation; hashes prove integrity, not publisher authenticity |
| `src/authority/normalize-cards.ts` | Pure deterministic card normalization | Existing implementation; no clock/network/randomness/repair |
| `src/authority/validate-bundle.ts` | Recursive offline bundle/path/reference/precedence/storage validation | Existing implementation; callers supply expected stable ID/root hash externally |
| `src/commands/import-authority.ts` | Exact locked-input, write-once local revision builder | Existing Plan 05 command and programmatic `importAuthority()` contract |
| `src/commands/validate-authority.ts` | Thin offline validation CLI | Existing Plan 05 relative-root/bundle contract |
| `src/authority/private-source-set.ts` | Complete primary/backup source-set verifier | New Plan 07 helper; reuse existing diagnostics, canonicalization, and hash conventions |
| `docs/authority-precedence.md` | D-01/D-02 official precedence policy | Plan 06 |
| `docs/external-reuse-policy.md` | D-08/D-09/D-11 private-local/no-leakage/clean-room policy | Plan 06 |
| `.local/authority/inputs/official-2026-08-20/primary/**` | Seven manually saved official primary files | Private, Git-ignored, never packaged |
| user-selected independent backup root | Byte-identical seven-file backup tree | Private, outside repository; exact locator only in ignored lock |
| `.local/authority/locks/official-2026-08-20/source-set-lock.json` | Exact private roots, seven-entry metadata/hash map, source-set root | Private, Git-ignored |
| `.local/authority/build-inputs/official-2026-08-20/{primary,backup}/` | Derived four-file importer inputs | Private, Git-ignored; independently generated from each source root |
| `.local/authority/revisions/official-2026-08-20/` | Selected immutable authority revision | Private, Git-ignored, selected locally by stable ID/root hash |
| `data/authority/receipts/official-2026-08-20.json` | Safe non-content source/revision receipt | Tracked; relative paths/URLs/dates/media/hashes and non-sensitive labels only |
| `data/authority/README.md` | Private rebuild/runtime/no-release instructions | Tracked; explicitly says Git alone is insufficient |
| `tests/authority/private-source-set.test.ts` | Generic source-set identity/path/bounds tests | Default clean-clone suite |
| `tests/private-authority/private-revision.test.ts` | Full-set double-build/tamper/immutability gate | Explicit private-data command only |
| `tests/private-authority/repository-boundary.test.ts` | No-network and Git/staged/package leakage gate | Explicit private-data command only |
| `tests/authority/fixtures/**` | Synthetic/minimal fixtures | Existing license-safe default tests only |

## Private Authority Layout

```text
.local/authority/                           # anchored Git-ignore
  inputs/official-2026-08-20/primary/
    rulebook/rulebook-current.pdf
    formats/constructed-current.html
    codex/codex-current.html
    codex/faqs-current.html
    codex/changelog-current.html
    updates/card-updates-2025.html
    cards/cards.raw.json
  locks/official-2026-08-20/
    source-set-lock.json                    # includes exact private roots; never tracked
  build-inputs/official-2026-08-20/
    primary/{cards.raw.json,formats.json,sources.json,input-lock.json}
    backup/{cards.raw.json,formats.json,sources.json,input-lock.json}
  revisions/official-2026-08-20/
    bundle.json
    sources.json
    formats.json
    cards.normalized.json

data/authority/
  README.md                                 # safe tracked instructions
  receipts/official-2026-08-20.json         # safe tracked metadata/hashes only
```

Publisher PDF/HTML/API bytes are permitted only in the two private source roots for the current operating model. They are never copied into the selected revision, Git, or packages. Card art remains excluded everywhere.

## Source-Set Verification Pattern

`verifyPrivateSourceSet` is the reusable trust boundary for Plans 07–08:

- Require exactly the seven fixed canonical relative paths and complete URL/retrieval/effective-date/media/byte-length/SHA-256 metadata.
- Allow normative URLs only on audited official Sorcery/Curiosa HTTPS hosts with no credentials.
- Resolve repository, primary root, backup root, and every file through `realpath`; use platform-aware `path.relative`, not textual prefix checks.
- Reject same/nested roots, backup beneath the repository, symlinks, junction/reparse aliases, non-files, duplicate identities, and hardlinks using bigint `stat.dev`/`stat.ino`.
- Compare exact sorted path sets, byte lengths, and streaming SHA-256 values. `cards/cards.raw.json` must also satisfy the importer's 10,000,000-byte JSON limit; all source reads remain bounded.
- Compute a deterministic `sourceSetRootHash` over canonical metadata/byte evidence while excluding machine-specific root locators.
- Repeat verification in Plan 08 and private tests; never trust Plan 07 summary values alone.

## Completed Importer Contract

The importer input root contains exactly four files:

```text
cards.raw.json
formats.json
sources.json
input-lock.json
```

`input-lock.json` covers exactly the first three in that order. Storage modes are `manifest-only`, `stored`, `stored`. Each JSON input is at most 10,000,000 bytes and the three total at most 32,000,000 bytes.

Use the CLI exactly:

```text
pnpm authority:import -- --input <input-root> --input-lock input-lock.json --expected-input-root-hash <sha256:...> --output-root .local/authority/revisions --revision-id official-2026-08-20
```

Import stdout emits candidate `verifiedInputRootHash` and `bundleRootHash`, not `bundleId`. The deterministic stable ID is `bundle:official-2026-08-20`; independently read/recompute it after the build.

Every format row must reference a derived format-input source record whose byte hash equals canonical `formats.json`. Its derivation parent hashes bind the locked official rulebook/Constructed/Codex-FAQ/changelog/card-update evidence. All publisher-source records remain manifest-only/permission-required/prohibited-storage so no raw source is copied into the revision.

## Completed Validator Contract

Use revision root plus a root-relative bundle path:

```text
pnpm authority:validate -- --root .local/authority/revisions/official-2026-08-20 --bundle bundle.json --id bundle:official-2026-08-20 --hash <independently captured root>
```

Do not duplicate the revision path between `--root` and `--bundle`. Validation accepts no source/input directory and performs no fetch, repair, rewrite, or data acquisition.

## Testing and Commit Pattern

- Default clean-clone suite: `pnpm verify`; all `tests/authority/*.test.ts` use project code and synthetic/minimal fixtures.
- Plan 07 focused test: `node --test tests/authority/private-source-set.test.ts`.
- Explicit private gate: `pnpm authority:verify-private`, running both files under `tests/private-authority/` and failing rather than skipping when private evidence is absent.
- Deterministic integrity and tamper/immutability stay in `private-revision.test.ts`.
- Network denial and Git/staged/package leakage stay in `repository-boundary.test.ts`.
- Commit implementation/helper, policy, receipt/docs, and test/script slices atomically after their focused checks and `pnpm verify` pass. Never force-add `.local/authority/`.

## Clean-Room and Future-Use Boundary

- Official sources are normative; community projects are provenance/examples only.
- Do not copy Contested Realms GPL code/tests/assets/card implementations/data or unlicensed spells.bar/playtest material.
- Current Phase 1 human action is manual private source provision and scoped attestation—not a publisher-permission adjudication.
- Sharing, releasing with publisher content, automated updating, commercialization, or a public/network card API requires written publisher permission and a separate approved plan.
- Hashes prove byte identity, not legal permission or publisher authenticity.

## Scope Fence

Do not add a network acquisition client, public card API, database, container, rules engine, deck/collection code, simulator, model integration, or GUI in Phase 1. Use Podman, not Docker, if a later phase genuinely needs containers.
