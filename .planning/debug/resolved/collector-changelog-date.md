---
status: resolved
trigger: "Collector throws: The first visible changelog date is missing or invalid"
created: 2026-08-25
updated: 2026-08-25T18:00:00-08:00
---

# Debug Session: Collector changelog date

## Symptoms

- expected: The fixed official changelog source is validated and collected.
- actual: Collection stops before publication because the first visible changelog date is rejected.
- error: `scripts/collect-private-authority.ps1:323 — The first visible changelog date is missing or invalid`
- timeline: First observed against the current official response on 2026-08-25; prior fixtures passed.
- reproduction: Run the private authority collector with a new absolute backup root and acknowledge the private-use risk.

## Current Focus

- hypothesis: Confirmed — `Get-ChangelogDate` rejects the official day-first `d MMMM yyyy` text because its allowlist accepts only ISO and month-first forms.
- test: Focused regression, full repository verification, human verification, archival, and knowledge-base capture are complete.
- expecting: No further debug action is required.
- next_action: None — resolved and archived.
- reasoning_checkpoint:
    hypothesis: `Get-ChangelogDate` causes collection to stop because the public page's first visible date is day-first (`19 May 2026`) while the regex and ParseExact format only allow ISO or month-first text.
    confirming_evidence:
      - Sanitized public-page observation places `19 May 2026` after the exact marker.
      - Repository trace shows the parser accepts only ISO and `MMMM d, yyyy`, and its only fixture uses the latter.
    falsification_test: The hypothesis would be false if the current parser accepted `19 May 2026` or if that text appeared before the marker.
    fix_rationale: Adding only the explicit invariant `d MMMM yyyy` alternative at the shared parse boundary accepts the publisher syntax while leaving all other text rejected.
    blind_spots: No production collector or private response was inspected; the regression uses the independently sanitized public date syntax.
- tdd_checkpoint:

## Evidence

- timestamp: 2026-08-25T18:00:00-08:00
  checked: `Get-ChangelogDate` implementation and repository-wide callers.
  found: The parser starts at the exact `Codex Changelog` marker, normalizes visible text, accepts only ISO `yyyy-MM-dd` or full English `MMMM d, yyyy`, and has one caller in the `effectiveDatePolicy` switch.
  implication: The failure is isolated to the shared changelog parse boundary.

- timestamp: 2026-08-25T18:00:00-08:00
  checked: `tests/authority/private-authority-collector.test.ts` and parser history.
  found: The sole success fixture uses `August 20, 2026`; the parser and fixture were introduced together and the parser has not evolved.
  implication: Tests cover only one hard-coded human-readable syntax and missed publisher presentation drift.

- timestamp: 2026-08-25T18:00:00-08:00
  checked: Sanitized public-page observation supplied without collector invocation or private artifact access.
  found: The first visible date after the exact marker is `19 May 2026`.
  implication: The valid official day-first form cannot match the parser regex, directly explaining the reported exception.

- timestamp: 2026-08-25T18:00:00-08:00
  checked: Focused loopback regression before implementation change.
  found: Both `19 May 2026` and `31 February 2026` fail at line 323 as missing/invalid because neither reaches `ParseExact`.
  implication: The regression reproduces the reported mechanism; adding the syntax to the regex should make the valid case pass and route impossible dates through strict calendar validation.

- timestamp: 2026-08-25T18:00:00-08:00
  checked: Focused loopback regression after the minimal parser change.
  found: The official day-first case normalized to `2026-05-19`, the impossible day-first case failed at strict calendar parsing, and all 3 focused test nodes passed.
  implication: The fix addresses the observed format while preserving fail-closed validation.

- timestamp: 2026-08-25T18:00:00-08:00
  checked: Full `pnpm verify` after the fix.
  found: Typecheck, lint, and all 152 test nodes passed with 0 failures.
  implication: The focused parser change introduces no detected repository regression.

- timestamp: 2026-08-25T18:00:00-08:00
  checked: Atomic code/test commit and post-commit worktree status.
  found: Commit `5cdb9d3` contains only the parser and regression test; unrelated `01-CONTEXT.md` and `01-07-PLAN.md` edits remain unstaged, and the debug artifact remains separate.
  implication: The verified fix is durable without absorbing unrelated work or private artifacts.

- timestamp: 2026-08-25T18:00:00-08:00
  checked: Human verification checkpoint.
  found: The fix was confirmed.
  implication: The session may be marked resolved and archived without rerunning production collection.

## Eliminated

## Resolution

- root_cause: `Get-ChangelogDate` hard-codes ISO and month-first English dates, but the official changelog currently renders the first visible date as day-first full English month (`19 May 2026`), so the regex rejects it before `ParseExact`.
- fix: Added only the explicit invariant day-first full-month syntax to the shared changelog regex and ParseExact format selection, plus one loopback regression covering the official syntax and an impossible date.
- verification: Focused regression passed 3/3; full `pnpm verify` passed typecheck, lint, and 152/152 test nodes. Production collection and external network were intentionally not invoked.
- files_changed: [scripts/collect-private-authority.ps1, tests/authority/private-authority-collector.test.ts]
