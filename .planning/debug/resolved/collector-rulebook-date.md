---
status: resolved
trigger: "The first live collector run failed at line 352 with: Rulebook release date 2025-12-19 is missing"
created: 2026-08-20
updated: 2026-08-20
---

# Collector Rulebook Date

## Symptoms

- expected_behavior: The acknowledged one-shot collector validates the official December 2025 rulebook release page, downloads the standard rulebook, and completes the seven-source publication.
- actual_behavior: The collector stops while validating the rulebook release page before downloading the PDF.
- error_messages: `Rulebook release date 2025-12-19 is missing` at `scripts/collect-private-authority.ps1:352`.
- timeline: First user-run live collection on 2026-08-20; the synthetic collector suite had passed previously.
- reproduction: Run `pwsh -NoProfile -NonInteractive -File .\scripts\collect-private-authority.ps1 -BackupRoot 'C:\Users\Dad\Documents\SorceryAuthorityBackup\official-2026-08-20' -AcknowledgePrivateUseRisk` from the repository root.

## Current Focus

- hypothesis: Confirmed and fixed.
- test: Fixtures now match the publisher's visible date, and a wrong-date case proves stale pages fail closed.
- expecting: The valid publisher date is accepted while a stale date is rejected before PDF download.
- next_action: User reruns the supported collector command.
- reasoning_checkpoint: The source marker, exact standard-rulebook anchor, redirect, PDF, bounded-transfer, and publication checks remain unchanged.
- tdd_checkpoint: Regression added; focused and full suites pass.

## Evidence

- timestamp: 2026-08-20T00:00:00-08:00
  observation: The exception occurs immediately after `Get-NormalizedVisibleText` and before anchor selection or PDF download.

## Eliminated

- hypothesis: The failure is caused by filesystem permissions or an existing destination.
  reason: The collector reached release-page content validation and left no authority output tree.

## Resolution

- root_cause: Visible-text normalization removes HTML attributes, while the publisher visibly renders 19 Dec 2025; implementation and fixtures incorrectly required visible ISO 2025-12-19.
- fix: Derive the invariant visible date from the locked descriptor effective date and retain every independent source and publication check.
- verification: Focused collector suite passed 54/54; full typecheck, lint, and test suite passed 128/128; independent debug review found the patch sufficient.
- files_changed: scripts/collect-private-authority.ps1, tests/authority/private-authority-collector.test.ts
