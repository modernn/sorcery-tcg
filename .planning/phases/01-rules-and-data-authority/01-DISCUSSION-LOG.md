# Phase 1: Rules and Data Authority - Discussion Log

> **Audit trail only.** Do not use as input to planning, research, or execution agents.
> Decisions are captured in CONTEXT.md — this log preserves the alternatives considered.

**Date:** 2026-08-20
**Phase:** 1-Rules and Data Authority
**Areas discussed:** official authority, immutable snapshots, artifact identity, distribution and reuse

---

## Official authority

| Option | Description | Selected |
|--------|-------------|----------|
| Official-only, fail closed | Official sources are normative; ambiguity becomes unsupported | ✓ |
| Community-assisted rulings | Fill official gaps with community interpretations | |
| Prototype compatibility | Preserve handoff behavior where authorities differ | |

**User's choice:** Auto-selected the recommended official-only, fail-closed policy.
**Notes:** Trustworthy deck and model comparisons require rule disagreements to be visible, not guessed.

## Immutable snapshots

| Option | Description | Selected |
|--------|-------------|----------|
| Pinned local revisions | Validate immutable offline bundles selected by hash | ✓ |
| Live API at runtime | Read current card data for every run | |
| Mutable local cache | Refresh one local dataset in place | |

**User's choice:** Auto-selected pinned local revisions.
**Notes:** This preserves exact experiment replay as official sources change.

## Artifact identity

| Option | Description | Selected |
|--------|-------------|----------|
| Shared provenance envelope | Stable ID, schema version, hash, and source lineage for every artifact | ✓ |
| Type-specific ad hoc metadata | Each artifact defines unrelated identity fields | |
| File path identity | Treat paths and names as stable identities | |

**User's choice:** Auto-selected one minimal shared provenance envelope.
**Notes:** Canonical serialization and validation must be project-owned and deterministic.

## Distribution and reuse

| Option | Description | Selected |
|--------|-------------|----------|
| Clean-room references | Use external projects only for observed behavior and UX ideas | ✓ |
| Reuse GPL engine code | Adopt GPL-covered implementation code | |
| Copy unlicensed assets | Import code/data where no license was found | |

**User's choice:** Auto-selected the clean-room boundary.
**Notes:** Official PDFs/images are not redistributed unless terms clearly permit it.

## the agent's Discretion

- Directory and command naming, deterministic JSON implementation details, and whether source terms permit committing raw official API responses.

## Deferred Ideas

- Collection/deck importing, common online deck acquisition, rule execution, model play, and browser human play remain in their roadmap phases.

---

## Private-local card-data clarification — 2026-08-20

| Option | Description | Selected |
|--------|-------------|----------|
| Manual official snapshot, private local storage | User saves one full official API JSON response through the browser; project code imports it locally; raw and derived authority data stay git-ignored and private | ✓ |
| Automated official API polling | Project fetches intermittently, diffs, and self-hosts data as suggested by the API landing page | |
| Community gameplay corpus | Import a complete permissively licensed third-party card corpus | |
| Public/network card API | Expose card queries through an HTTP service | |

**User's choice:** The tool is entirely private, local, noncommercial, and will not be released to anyone else. Use the manual official snapshot path.

**Evidence considered:**
- The official API landing page permits developer access and recommends intermittent polling, diffing, and hosting required data, while excluding images/private CDN access.
- The site Terms permit limited personal noncommercial use but prohibit automated access, systematic retrieval/database creation, and scraping/data mining without written permission. The project does not claim to resolve that conflict legally.
- No audited community repository provides a complete permissively licensed gameplay corpus. `sadkinglabs/sorcery-registry` is technically strong, but its MIT license covers code while its card data remains reserved to Erik's Curiosa; other candidates were GPL, unlicensed, or incomplete.

**Operating decision, refined after verification:** The user manually saves the current rulebook PDF, base Constructed page, Codex, FAQs, Codex changelog, official card-update notice, and full API JSON to the seven fixed relative paths beneath `.local/authority/inputs/official-2026-08-20/primary/`. The user maintains an independent byte-identical private backup tree outside the repository. Every entry is locked by official URL, retrieval/effective date, media type, byte length, and SHA-256; absolute locators remain only in a git-ignored lock. Project code never fetches, scrapes, polls, or hosts upstream material. All source, normalized, and built bytes remain private/ignored and never packaged; private rulebook/page bytes are allowed only in the two source roots, while card art remains excluded everywhere.

**Future trigger:** Sharing, releasing with content, automated updating, or commercialization requires written publisher permission and a separate plan. Later code queries validated local JSON through TypeScript; no public/network HTTP card API is planned.
