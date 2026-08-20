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
