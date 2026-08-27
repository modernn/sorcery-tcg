---
gsd_state_version: 1.0
milestone: v1.0
milestone_name: milestone
status: executing
stopped_at: Completed 01-12-PLAN.md
last_updated: "2026-08-27T19:43:03.334Z"
last_activity: 2026-08-27 -- Phase 1 planning complete
progress:
  total_phases: 9
  completed_phases: 0
  total_plans: 15
  completed_plans: 12
  percent: 80
---

# Project State

## Project Reference

See: .planning/PROJECT.md (updated 2026-08-20)

**Core value:** Simulation results must be reproducible and rules-correct enough that deck and model comparisons are trustworthy.
**Current focus:** Phase 01 — rules-and-data-authority

## Current Position

Phase: 01 (rules-and-data-authority) — EXECUTING
Plan: 13 of 15
Status: Ready to execute
Last activity: 2026-08-27 -- Phase 1 planning complete

Progress: [████████░░] 80%

## Performance Metrics

**Velocity:**

- Total plans completed: 5
- Average duration: 28 min
- Total execution time: 2h 20m

**By Phase:**

| Phase | Plans | Total | Avg/Plan |
|-------|-------|-------|----------|
| Phase 1 | 5/8 | 2h 20m | 28 min |

**Recent Trend:**

- Last 5 plans: 15m, 28m, 21m, 34m, 42m
- Trend: Increasing with plan scope; all gates green

*Updated after each plan completion*
| Phase 01 P01 | 15min | 2 tasks | 11 files |
| Phase 01 P02 | 28min | 2 tasks | 5 files |
| Phase 01 P03 | 21min | 2 tasks | 6 files |
| Phase 01 P04 | 34 min | 2 tasks | 6 files |
| Phase 01 P05 | 42 min | 2 tasks | 12 files |
| Phase 01 P06 | 9 min | 2 tasks | 3 files |
| Phase 01 P07 | 5 min | 2 tasks | 3 files |
| Phase 01 P08 | 33 min | 3 tasks | 12 files |
| Phase 01 P12 | 13 min | 3 tasks | 5 files |

## Accumulated Context

### Decisions

Decisions are logged in PROJECT.md Key Decisions table.
Recent decisions affecting current work:

- TypeScript is required across the engine, simulator, agents, optimizer, and browser client.
- The engine is headless and authoritative; every competitor and human client uses its legal-action contract.
- Rules correctness and reproducible simulation precede AI evaluation, optimization, and browser human play.
- External simulator code is reference-only until its license permits the chosen reuse boundary.
- [Phase 01]: Use Node's native TypeScript stripping and node:test without a transpiler or test-runner dependency. — The pinned Node 24 runtime provides the required execution and test facilities.
- [Phase 01]: Keep Wave 0 authority cases as named todo contracts for dependent plans to replace with assertions. — The contract scaffold preserves the complete acceptance boundary without speculative implementations.
- [Phase 01]: Suppress third-party declaration checking for the mandated TypeScript 6.0.3 and @types/node 24.1.0 pins. — Their HTTPS declarations conflict internally; project code remains strictly checked.
- [Phase 01]: Canonical identity uses one bounded project-owned serializer; raw bytes and canonical identity documents are hashed separately. — Duplicate-aware parsing and bounded canonicalization must precede identity hashing.
- [Phase 01]: Source storage is a strict stored-versus-manifest-only union, and normative eligibility is derived only from authorityClass: official. — This prevents ambiguous storage state and caller-controlled promotion of community material.
- [Phase 01]: Recursive graph validation, symlink confinement, stored-byte rehashing, and manifest reference binding remain owned by Plan 01-04. — Plan 01-02 defines shared envelopes while Plan 01-04 owns repository resolution and graph traversal.
- [Phase 01]: Mint card stable IDs from source ID plus source card ID, independent of display fields, array order, and filesystem names. — Stable logical identity must not change with presentation or local storage.
- [Phase 01]: Keep full artifact hashes bound to exact raw-source provenance while normalized payload hashes remain property-order independent. — The D-06 SourceRef binds exact source bytes while semantic normalization remains canonical.
- [Phase 01]: Enforce cross-card identity and printing-slug uniqueness in shared snapshot schemas. — One trust boundary preserves exact diagnostics and prevents duplicated validation logic.
- [Phase 01]: Treat stored-byte rehashing and manifest-only reference verification as distinct successful evidence states. — Stored bytes can be independently rehashed; manifest-only evidence can prove binding but must not claim absent bytes were reverified.
- [Phase 01]: Resolve normative authority from official records only; equal-rank or unclear official outcomes remain unsupported. — Community provenance remains reviewable without becoming normative, and ambiguity must fail closed.
- [Phase 01]: Require explicit license-status metadata for every stored-source record; the D-08 private user-run one-shot seven-source set is the sole risk-accepted exception without approved permission. — Written publisher permission and approved license metadata remain trust-boundary prerequisites before any broader stored-source, recurring acquisition, redistribution, hosting, upload, artwork, or commercial use.
- [Phase 01]: The durable input lock covers the command's fixed local inputs, and its independently supplied root is verified before content parsing. — This makes substitution or stale-lock failures occur before untrusted content reaches normalization.
- [Phase 01]: Stored source paths resolve from the selected revision and only approved raw card bytes may occupy the fixed raw/cards.raw.json path. — Revision-relative confinement matches bundle semantics while the fixed path keeps publication storage narrowly reviewable.
- [Phase 01]: Validation success explicitly reports stored-bytes-rehashed separately from manifest-binding-verified. — Manifest-only records bind provenance without falsely claiming absent raw bytes were independently rehashed.
- [Phase 01]: Byte-locked fixture JSON is pinned to LF in .gitattributes for cross-platform hash stability. — Reviewed byte hashes must remain identical under Windows core.autocrlf checkouts.
- [Phase 01]: Document the implemented official-only precedence order as a project fail-closed policy, not a publisher hierarchy claim. — The resolver must retain evidence and fail closed without overstating publisher guidance.
- [Phase 01]: Permit one fixed user-run one-shot PowerShell collection of the seven official sources plus an independent byte-identical backup, recording `acquisitionMethod: user-run-one-shot-powershell`. — The user accepts the unresolved private-use risk; publisher permission remains absent, and the exception adds no recurring acquisition, retry/evasion, artwork, redistribution, hosting, or public API.
- [Phase 01]: Keep community corpora behavioral-reference only and require written publisher permission plus a separate plan for broader use. — Audited community sources do not grant a complete reusable gameplay corpus.
- [Phase 01]: Accept the exact consumed quick-260825-mhh-retry-1 agent-run evidence pair while preserving all private-use limits. — The current D-08/D-11 contract permits this closed pair and the private lock, consumed record, and bytes independently verify.
- [Phase 01]: Treat source and lock hashes as byte-identity evidence only. — Hashes establish neither publisher authenticity nor legal permission.
- [Phase 01]: Keep DATA-01, DATA-02, and DATA-03 pending through Plan 01-08. — The final private revision and phase gate have not executed.
- [Phase 01]: Adapt unchanged official API bytes through a strict bounded in-memory TypeScript adapter — Original-byte hashes and provenance remain authoritative while derived records are deterministic.
- [Phase 01]: Bind claimed derivation parent hashes to another bundle source — Forged, verbatim-parent, and self-parent claims fail closed.
- [Phase 01]: Keep the real authority revision private and select it only by fixed ID and root hash — Runtime authority stays immutable and offline without public API or corpus distribution.
- [Phase 01]: Windows private locator matching is ASCII case-insensitive while POSIX matching remains exact.
- [Phase 01]: Only exact audited public provenance is exempt from semantic private-locator detection.
- [Phase 01]: Normalized official derivatives may exist only in the ignored private built revision unless a separate approved plan changes that boundary.

### Pending Todos

- 2026-08-20-authorize-card-data-and-catalog-updater.md — Request publisher/community data authorization and decide whether recurring updates justify a separate catalog-maintenance tool.

### Blockers/Concerns

- Phase 1 still lacks publisher permission for broader use. Only the three closed one-shot evidence pairs and independent private backup are accepted without a legal conclusion; recurring acquisition, retry/evasion, artwork collection, redistribution, hosting, third-party upload, public API, and commercialization remain blocked pending written permission and a separate plan.
- Ranked results remain unavailable until the first owned and benchmark card pool passes transitive verified-coverage gates in Phase 6.

### Quick Tasks Completed

| # | Description | Date | Commit | Status | Directory |
|---|-------------|------|--------|--------|-----------|
| 260820-hhu | Build and validate a user-run PowerShell collector for the seven private Sorcery authority sources and independent backup | 2026-08-20 | 302a685 | Verified | [260820-hhu-build-and-validate-a-user-run-powershell](./quick/260820-hhu-build-and-validate-a-user-run-powershell/) |
| 260825-mhh | Allow one explicitly user-authorized agent-run private authority collection, retain private-local/no-redistribution/no-recurring/no-artwork limits, run the collector, and independently validate the source set | 2026-08-26 | 6b05b86 | Verified | [260825-mhh-allow-one-explicitly-user-authorized-age](./quick/260825-mhh-allow-one-explicitly-user-authorized-age/) |

## Deferred Items

Items acknowledged and carried forward from previous milestone close:

| Category | Item | Status | Deferred At |
|----------|------|--------|-------------|
| *(none)* | | | |

## Session Continuity

Last session: 2026-08-27T18:20:26.224Z
Stopped at: Completed 01-12-PLAN.md
Resume file: None
