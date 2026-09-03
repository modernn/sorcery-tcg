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
| `ed643a3` | Lint-only unused import cleanup after Cloud City. |
| `7df79c6` | Document overnight SetupCtx migration progress and remaining call counts. |
| `4057b2b` | Fix Defend legality for non-surface combat regions (void path Defend). |
| `857a643` | Migrate RULE-02 Geomancer / settlement / edge-wrap compounds to SetupCtx. |
| `2cd0931` | Migrate `game-setup-05` drag projectile proofs to SetupCtx (batch 2/3). |
| `637d79a` | Document SetupCtx progress after RULE-02 compounds and drag batch 2. |
| `87fd0cf` | Migrate `game-setup-05` remaining proofs to SetupCtx (batch 3/3). |
| `e05133d` | Migrate setup-02 Leap Attack / Magic targets / Lash to SetupCtx. |
| `285c394` | Migrate setup-02 Freeze disable proof onto SetupCtx. |
| `0ea6877` | Migrate setup-02 subsurface disable, Lightning Bolt, Lucky Charm to SetupCtx. |
| `170a183` | Migrate setup-02 Minor Explosion, Chain Magic, Rain of Arrows to SetupCtx. |
| `6ae4ed2` | Migrate setup-02 Charge and Overpower proofs to SetupCtx. |
| `bcd9d7d` | Migrate setup-02 nearby-allies and controlled Mortal power to SetupCtx. |
| `d861152` | Migrate remaining setup-02 aura-loss and magic proofs (Deathrite→Bury). |
| `9d8d49e` | Migrate setup-07 RULE-05 Deathrite family (Bladderblimp→healing). |

Tip: run `git log -1 --oneline` (expected near this handoff commit).

## Gate status at tip

- `pnpm verify` — **404 tests, 0 fail** (green after setup-02 finish + setup-07 Deathrite family).
- Do **not** apply `stash@{0}` (`wip-parallel`): incomplete/broken SetupCtx rewrites of setup-03/04/06 + novelty-rollout left by a parallel agent; tip TS versions of those files still pass.

## Boundary cutover status

Done:
- Demo / batch / play / parity / public views / fail-closed demo agent (prior).
- SetupCtx bridge + helpers (`toNorthSecondMain`, `takeAction`, `withNorthAttacksAtC2`, …).
- RULE-01 opening proofs (setup, mulligan, first player, empty-deck) + stale rejection.
- Core RULE-02 spatial proofs (expansion, zero-domain recovery, draw-site, forged actions, Cloud City).
- RULE-02 compounds: Geomancer rubble, region settlement (+ void Defend engine fix), top/bottom edge wrap.
- RULE-03 opening draw-spell + Spellcaster summon.
- `game-setup-05` **complete** (batches 1–3): genesis sleep through ranged/drag, Granary Rats, Airborne/Mountain Pass, Updraft Ridge, Stealth, Sly Fox.
- `game-setup-02` **play-path complete**: Leap through Bury. Remaining refs are seed peeks + forged-state probes (Chain Magic mana/region/stealth; Blink empty-atlas steps).
- `game-setup-07` RULE-05 Deathrite family: Bladderblimp, chained area damage, moved-last-location Ward/Lethal, power snapshot, healing.

Still present — `src/engine/game.ts` (~525KB):
- Still exports `createGameSession` / `legalGameActions` / `stepGame` because most split setup files and other callers still use them.
- Keep types / `hashGameState` / `createGameManifest` / `observeGame` as the thin TS boundary.
- Geomancer / Granary Rats / Chain Magic / Blink empty-atlas still use TS legality only for forged-state probes.
- Seed-search loops may still peek opening hands via `createGameSession` (cheap); play paths use SetupCtx.
- Note: mid-combat Rust journals can diverge from TS `resumeGameCheckpoint`; use `SetupCtx.resumeCheckpoint` / `ctx.resume` for those roundtrips.

## Remaining TS legality surface (estimate)

`createGameSession(` / `legalGameActions(` / `stepGame(` call counts in setup tree ≈ **424** total:

| File | ~calls |
| --- | ---: |
| game-setup-01 | 50 |
| game-setup-02 | 19 |
| game-setup-03 | 61 |
| game-setup-04 | 83 |
| game-setup-05 | 1 |
| game-setup-06 | 70 |
| game-setup-07 | 53 |
| game-setup-08 | 80 |
| helpers | 7 |

Other public/private callers still needing Rust cutover later:
- `tests/engine/game-novelty-rollout.test.ts` + `src/simulator/novelty-rollout.ts`
- Sync `resumeGameCheckpoint` in `src/engine/checkpoint.ts`
- Private: `run-private-game-check.ts`, `run-private-novelty-gauntlet.ts`
- `benchmarks/typescript-engine.ts` (intentional until deletion)

`game.ts` legality exports still required.

## Next exact step

1. Continue `game-setup-07` from the next unmigrated test:
   - Next: `RULE-04 Move and Attack stages movement before an undefended enemy-site strike`
   - Then: Defend split damage; takes-less prevention; Intercept window; damage persistence; Death's Door pair; then Artifact family (Pick Up/Drop → Siege Ballista → Payload → Boulder → Mesmerism).
2. Then setup-08, or remaining RULE-02/03 in setup-03/04/06.
3. Prefer helpers in `game-setup-helpers.ts` (`withNorthAttacksAtC2`, …); do **not** re-run archived one-shot rewrite scripts under `.local/archive/`.
4. After public tests no longer call TS legality, gut `createGameSession` / `legalGameActions` / `stepGame` in `game.ts`.
5. Drop or ignore `stash@{0}` after confirming tip does not need it (`git stash drop` only when ready).

## Do not

- Commit anything under `.local/` (whole directory is ignored; `.local/authority/` holds private authority bytes).
- Run more than one agent against this working tree at a time. Parallel WIP corrupted setup-04/06 migrations tonight (stashed as `wip-parallel`).
- Push unless explicitly asked.
- Reintroduce TypeScript as a second legality engine.
