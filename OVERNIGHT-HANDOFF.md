# Overnight handoff

Live line: `cursor/rust-cutover-setup-tests-0005` (PR https://github.com/modernn/sorcery-tcg/pull/2), tip `0090408`.

Do not start from `master`'s copy of this file. That copy still says to migrate `game-setup.test.ts` and gut `game.ts`; that work is already on this branch. Do not open a second cutover branch.

`cursor/phase3-drown-bury-artifacts-36d3` was identical to `master` with PR #1 closed. It is archived as `archive/cursor/phase3-drown-bury-artifacts-36d3` (`git tag -l 'archive/*'`).

Do not fast-forward `master` from a checkout that cannot run `pnpm verify` with authority fixtures and `pwsh`.

## Catalog count

`data/rules/catalog.json`: **161 rust-supported / 0 typescript-supported** out of 161.

## TypeScript legality cutover

Done — do not redo:

- Game-setup proofs run through `SetupCtx`. Checkpoints resume only through Rust.
- `createGameSession` / `legalGameActions` / `stepGame` / `replayGame` / `verifyGameReplay` call a blocking `session-json` client (`src/engine/rust-legality-sync.ts`).
- `actionDescriptors` / `applyDescriptor` and their helpers are deleted from `src/engine/game.ts`. That file keeps types, manifest validation, `observeGame`, and the thin Rust wrappers.
- Private-check Chain Lightning extra-target mana uses a real 3-mana sibling (draw Atlas, do not play the fourth site). No constructed `GameSession` spreads remain.

Still TypeScript (not a second legality engine):

- Manifest validation, observation, authority ingestion, server, and browser UI.
- `run-private-game-check.ts` still *calls* the sync wrappers; it cannot be executed in this cloud checkout (no `.local/authority/`).

## Gate status at tip `0090408`

- `cargo fmt` / `clippy -D warnings` / `cargo test --workspace --all-features --locked` — green.
- `pnpm typecheck` / `pnpm lint` — green.
- `tests/engine` + `tests/ingestion` — 221 pass, 0 fail.
- Full `pnpm test` in this checkout: 31 authority-collector / DATA-01 failures (`pwsh` missing, no private authority bundle). Those are environment gaps, not the cutover.

## Next exact step

1. Continue on this branch only. Run `pnpm verify` and `pnpm game:check-private` on a machine that has `.local/authority/` and `pwsh`.
2. Retire this handoff and fast-forward `master` only after that private-check run is green.
3. Phase 4+ product surfaces stay out of scope until that private verification lands.

## Do not

- Commit anything under `.local/` (whole directory is ignored; `.local/authority/` holds private authority bytes).
- Reintroduce TypeScript as a second legality engine.
- Re-migrate `tests/engine/game-setup.test.ts` or restore `applyDescriptor`.
- Acquire official artwork.
