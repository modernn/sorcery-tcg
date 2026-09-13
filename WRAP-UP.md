# Wrap-up mode

Faster pipeline for finishing the simulator without duplicating proof work on every rule slice.

## Ship bars

| Gate | Command | When |
| --- | --- | --- |
| Public PR | `pnpm verify` | Every change (typecheck, lint, public tests) |
| Engine / release | `pnpm verify:release` | Touches `crates/sorcery-engine/`, merge to `master`, tags |
| Private proofs | `pnpm game:check-private` | Real-card teaching scenarios; needs `.local/authority/` |
| Authority bundle | `pnpm authority:verify-private` | Authority ingestion; needs ignored local inputs + `pwsh` |

Do not block ordinary PRs on private checks or full private authority unless the change touches those surfaces.

## One proof per rule slice

For new or changed rule behavior:

1. Implement in Rust (`crates/sorcery-engine/`).
2. Add one direct scenario test in the matching `*_rules.rs` or SetupCtx setup file.
3. Add or update one row in `data/rules/catalog.json`.

Do **not** add parallel playthroughs to `run-private-game-check.ts` unless the rule needs a real-card private teaching deck. Do **not** grow forged-state TS legality probes once the play path runs through SetupCtx / Rust.

## Remaining cutover (this branch)

- Finish SetupCtx migration in `tests/engine/game-setup-*.ts` (see `OVERNIGHT-HANDOFF.md`).
- Then delete TS legality exports from `src/engine/game.ts` when no public caller remains.
- Consolidate parity capture scripts and private-check seed scans when cutover is green (ponytail audit targets).

## Explicitly deferred for v1 wrap-up

- Roadmap phases 4–9 (full card matrix UI, AI competitors, deck optimization, browser polish).
- 2×2 fail-closed combinations for token, wraparound, and outer-column unless a card requires them before v1.

## v1 done checklist

- [ ] Public `pnpm verify` green in CI
- [ ] `pnpm verify:release` green locally
- [ ] No TS legality callers in public setup tests
- [ ] Catalog linker matches Rust-supported proofs
- [ ] One green `pnpm game:check-private` on release machine (when `.local/authority/` present)
