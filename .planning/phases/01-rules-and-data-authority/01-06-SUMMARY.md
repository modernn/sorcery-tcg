---
phase: 01-rules-and-data-authority
plan: "06"
subsystem: authority-policy
tags: [precedence, private-data, clean-room, licensing, git-boundary]
dependency-graph:
  requires:
    - phase: 01-04
      provides: official-only fail-closed precedence resolver and bundle validation
    - phase: 01-05
      provides: offline write-once authority import and validation commands
  provides:
    - reviewable official authority precedence matching the implemented resolver
    - one private manual seven-file source-set and independent-backup operating path
    - Git/package and clean-room boundaries for official and community material
  affects: [01-07, 01-08, phase-02-engine-contract]
tech-stack:
  added: []
  patterns:
    - official-only effective-dated scoped resolution that retains conflict evidence
    - private manual authority acquisition with no project network path
    - written-permission gate for broader content use or automation
key-files:
  created:
    - docs/authority-precedence.md
    - docs/external-reuse-policy.md
  modified:
    - .gitignore
key-decisions:
  - "Document the implemented official-only precedence order as a project fail-closed policy, not a publisher hierarchy claim."
  - "Permit only one manually browser-saved seven-file private source set plus an independent byte-identical backup for v1."
  - "Keep community corpora behavioral-reference only and require written publisher permission plus a separate plan for broader use."
patterns-established:
  - "Private authority boundary: all repository-side real authority inputs, locks, normalized data, and revisions live below ignored .local/authority/."
  - "Clean-room evidence: external observations require source revision, reviewer, review date, and independent-authorship attestation."
requirements-completed: [DATA-01, DATA-03]
metrics:
  duration: 9 min
  completed: 2026-08-20
---

# Phase 1 Plan 6: Authority and External Reuse Policy Summary

**Official-only fail-closed precedence plus a Git-excluded, manually acquired private authority source set with explicit clean-room and future-permission gates.**

## Performance

- **Duration:** 9 min
- **Started:** 2026-08-20T19:34:38Z
- **Completed:** 2026-08-20T19:43:40Z
- **Tasks:** 2
- **Files modified:** 3

## Accomplishments

- Documented the five normative official source classes and the exact date, scope, supersession, ranking, and unsupported behavior enforced by `resolveAuthorityPrecedence`.
- Fixed the only v1 acquisition path to seven manual browser saves, a complete metadata/hash lock, and an independent byte-identical private backup with no fetch, scraper, poller, API client, or public HTTP service.
- Recorded exact reuse classifications for Sorcery Registry, Contested Realms, spells.bar/playtest, sorcery-cards, and JustTCG while excluding artwork and all protected corpus/derivative bytes from Git and packages.

## Task Commits

Each task was committed atomically:

1. **Task 1: Document normative authority and precedence** - `b9a3729` (`docs`)
2. **Task 2: Enforce the private-local and clean-room boundary** - `0bbb616` (`docs`)

## Files Created/Modified

- `docs/authority-precedence.md` - Official source inventory and implemented fail-closed conflict-resolution policy.
- `docs/external-reuse-policy.md` - Seven-file private source set, backup evidence, repository exclusions, reuse audit, and permission triggers.
- `.gitignore` - Anchored exclusion for `.local/authority/`.

## Decisions Made

- Treat precedence as a documented project resolution policy and retain winning, superseded, contending, and provenance references rather than flattening history.
- Keep the current simulator private, local, noncommercial, and no-redistribution; written permission is not a blocker to the documented manual path.
- Prefer a local TypeScript catalog/query module over a public/network API; automation, sharing, packaging/publication with content, commercialization, or changed scope requires written publisher permission and a separate plan.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] Corrected SDK progress frontmatter**

- **Found during:** Plan tracking updates
- **Issue:** `state.advance-plan` set the frontmatter percentage to `0` while disk summaries and the rendered progress field correctly reported 6/8 (75%).
- **Fix:** Corrected the frontmatter percentage to 75 after the SDK update and verified it against the summary count.
- **Files modified:** `.planning/STATE.md`
- **Verification:** STATE frontmatter and rendered progress both report 75%; roadmap reports 6/8 plans.
- **Committed in:** final tracking commit

---

**Total deviations:** 1 auto-fixed (1 bug).
**Impact on plan:** Tracking metadata was made internally consistent; policy scope and behavior were unchanged.

## Issues Encountered

- The Windows patch helper intermittently failed to update existing files. The one-line ignore change was applied with a command-scoped Git patch, and one policy wording correction used an exact guarded replacement; scoped diffs and all verification gates passed afterward.
- The installed GSD SDK requires named arguments for metrics and decisions despite the executor reference showing positional examples; the failed calls made no changes and were retried with the installed handler's supported flags.

## Authentication Gates

None.

## Known Stubs

None. The policies contain no placeholders, TODOs, FIXME markers, mock data, or deferred behavior that prevents the plan goal.

## User Setup Required

None for this plan. Plan 07 owns the blocking manual source-set provision and attestation checkpoint.

## Verification Results

- Exact precedence document assertion - passed.
- Exact private-local/reuse document assertion - passed.
- `git check-ignore -q .local/authority/probe` - passed.
- `git ls-files .local/authority` - empty.
- `pnpm verify` - passed: typecheck, lint, and 63 tests (0 failed, 0 skipped, 0 todo).
- Modified-file stub scan - no findings.
- Threat-surface scan - no new endpoint, auth path, schema trust boundary, or file-access implementation; policy only.

## Next Phase Readiness

- Plan 07 can enforce the exact seven-entry primary/backup lock and private/noncommercial/no-redistribution attestation before any official bytes are supplied.
- The pending authorization/catalog-updater todo remains intact. It gates future automation, release, publication, sharing, or commercialization, not the private manual-import path.

## Self-Check: PASSED

- `.gitignore`, `docs/authority-precedence.md`, and `docs/external-reuse-policy.md` exist.
- Task commits `b9a3729` and `0bbb616` are present in repository history.
- All task acceptance and plan verification commands passed.
