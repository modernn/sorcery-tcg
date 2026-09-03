# Overnight handoff

Branch: `cursor/phase3-drown-bury-artifacts-36d3` is the integration line. `master` is fast-forwarded to it. Superseded branches and stashes live only as `archive/*` tags (`git tag -l 'archive/*'`).

## Catalog count

`data/rules/catalog.json`: **161 rust-supported / 0 typescript-supported** out of 161.

## Commits landed (this overnight SetupCtx line)

| Hash | Change |
| --- | --- |
| `41dc56a` | Migrate core RULE-02 spatial setup proofs onto Rust SetupCtx. |
| `cfe2398` | Migrate RULE-03 draw-spell + Spellcaster summon proofs to SetupCtx. |
| `f80732b` | Split monolithic `game-setup.test.ts` into eight files + shared helpers. |
| `7d0c208` | Migrate `game-setup-05` proofs to SetupCtx (batch 1/3). |
| `5fa0ce3` | Migrate remaining RULE-01 empty-deck + stale-rejection proofs to SetupCtx. |
| `af0b6f4` | Migrate RULE-02 Cloud City flight proof onto SetupCtx. |
| `…` | Lint-only unused import cleanup after Cloud City. |

Tip: run `git log -1 --oneline` (expected near `af0b6f4` + lint fix).

## Gate status at tip

- `pnpm verify` — **404 tests, 0 fail** (re-run green after Cloud City + RULE-01 leftovers).
- Rust workspace gates not re-run this turn (no crate changes).
- Do **not** apply `stash@{0}` (`wip-parallel`): incomplete/broken SetupCtx rewrites of setup-03/04/06 + novelty-rollout left by a parallel agent; tip TS versions of those files still pass.

## Boundary cutover status

Done:
- Demo / batch / play / parity / public views / fail-closed demo agent (prior).
- SetupCtx bridge + helpers (`toNorthSecondMain`, `takeAction`, `withNorthAttacksAtC2`, …).
- RULE-01 opening proofs (setup, mulligan, first player, empty-deck) + stale rejection.
- Core RULE-02 spatial proofs (expansion, zero-domain recovery live path, draw-site, forged actions, Cloud City).
- RULE-03 opening draw-spell + Spellcaster summon.
- `game-setup-05` batch 1/3 (genesis sleep through ranged strike family).

Still present — `src/engine/game.ts` (~525KB):
- Still exports `createGameSession` / `legalGameActions` / `stepGame` because most split setup files and other callers still use them.
- Keep types / `hashGameState` / `createGameManifest` / `observeGame` as the thin TS boundary.

## Remaining TS legality surface (estimate)

`createGameSession(` / `legalGameActions(` / `stepGame(` call counts in setup tree ≈ **578** total:

| File | ~calls |
| --- | ---: |
| game-setup-01 | 50 |
| game-setup-02 | 107 |
| game-setup-03 | 67 |
| game-setup-04 | 88 |
| game-setup-05 | 44 |
| game-setup-06 | 73 |
| game-setup-07 | 62 |
| game-setup-08 | 80 |
| helpers | 7 |

Other public/private callers still needing Rust cutover later:
- `tests/engine/game-novelty-rollout.test.ts` + `src/simulator/novelty-rollout.ts`
- Sync `resumeGameCheckpoint` in `src/engine/checkpoint.ts`
- Private: `run-private-game-check.ts`, `run-private-novelty-gauntlet.ts`
- `benchmarks/typescript-engine.ts` (intentional until deletion)

`game.ts` legality exports still required.

## Next exact step

1. Continue SetupCtx migration by file/family, one verified commit each:
   - Next: remaining **RULE-02/03–04** compound proofs in setup-03/04/06 (Geomancer, region settlement, top/bottom edges), **or** `game-setup-05` batch 2/3 starting at drag projectile (use `createGameCheckpoint(ctx.session)` + `ctx.resume` for branches).
   - Then: `game-setup-02` (largest remaining RULE-03 magic cluster), then 07/08 artifacts/Deathrites.
2. Prefer helpers in `game-setup-helpers.ts`; do **not** re-run archived one-shot rewrite scripts under `.local/archive/`.
3. After public tests no longer call TS legality, gut `createGameSession` / `legalGameActions` / `stepGame` in `game.ts`.
4. Drop or ignore `stash@{0}` after confirming tip does not need it (`git stash drop` only when ready).

## Do not

- Commit anything under `.local/` (whole directory is ignored; `.local/authority/` holds private authority bytes).
- Run more than one agent against this working tree at a time. Parallel WIP corrupted setup-04/06 migrations tonight (stashed as `wip-parallel`).
- Push unless explicitly asked.
- Reintroduce TypeScript as a second legality engine.
