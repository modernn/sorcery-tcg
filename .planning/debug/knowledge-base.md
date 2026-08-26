# GSD Debug Knowledge Base

Resolved debug sessions. Used by `gsd-debugger` to surface known-pattern hypotheses at the start of new investigations.

---

## collector-changelog-date — Collector rejected the official day-first changelog date
- **Date:** 2026-08-25
- **Error patterns:** first visible changelog date, missing or invalid, collection stops before publication, day-first date
- **Root cause:** `Get-ChangelogDate` accepted only ISO and month-first English dates while the official changelog rendered `19 May 2026` in day-first full-month form.
- **Fix:** Added explicit invariant `d MMMM yyyy` matching and strict parsing, with regression coverage for the official form and an impossible date.
- **Files changed:** scripts/collect-private-authority.ps1, tests/authority/private-authority-collector.test.ts
---
