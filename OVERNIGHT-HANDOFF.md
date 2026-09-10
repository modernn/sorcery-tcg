# Overnight handoff

Live line: `cursor/rust-cutover-setup-tests-0005` (PR https://github.com/modernn/sorcery-tcg/pull/2).

Do not start from `master`'s copy of this file. That copy still says to migrate `game-setup.test.ts` and gut `game.ts`; that work is already on this branch. Do not open a second cutover branch.

`cursor/phase3-drown-bury-artifacts-36d3` was identical to `master` with PR #1 closed. It is archived as `archive/cursor/phase3-drown-bury-artifacts-36d3` (`git tag -l 'archive/*'`).

Do not fast-forward `master` from a checkout that cannot run `pnpm verify` with authority fixtures and `pwsh`.

## Catalog count

`data/rules/catalog.json`: **289 rust-supported / 0 typescript-supported** out of 289.

Latest catalog proofs: target-player library draw (`RULE-CATALOG-0286`–`0289`) and start-of-controller-turn controller mana gain (`RULE-CATALOG-0284`–`0285`).
Grant-Airborne this turn remains `RULE-CATALOG-0274`–`0275`.
Cemetery Aura return remains `RULE-CATALOG-0272`–`0273`.
Destroy- and return-target Aura Magic remains `RULE-CATALOG-0270`–`0271`.
Start-of-controller-turn controller life gain remains `RULE-CATALOG-0268`–`0269`.
Flood and Drought terrain Auras remain `RULE-CATALOG-0266`–`0267`.
Site-granted start-of-controller-turn life loss plus this-turn mana remains `RULE-CATALOG-0264`–`0265`.
End-of-each-turn wandering Aura remains `RULE-CATALOG-0262`–`0263`.
Start-of-controller-turn controller life loss remains `RULE-CATALOG-0260`–`0261`.
Start-of-controller-turn occupied-site Aura destruction remains `RULE-CATALOG-0258`–`0259`.
Start-of-controller-turn nearby enemy lure remains `RULE-CATALOG-0256`–`0257`.
Nearby double damage on Ranged and Genesis strikes remains `RULE-CATALOG-0252`–`0255`.
Mask of Mayhem walk-away and nearby combat double damage remain `RULE-CATALOG-0249`–`0251`.
Nearby-must-attack remains `RULE-CATALOG-0247`–`0248`.
Enemies-must-attack-this remains `RULE-CATALOG-0245`–`0246`.
Self-printed must-attack-a-unit remains `RULE-CATALOG-0243`–`0244`.
Start-of-controller-turn Atlas draw remains `RULE-CATALOG-0241`–`0242`.
Target-player discard Magic remains `RULE-CATALOG-0239`–`0240`.
Start-of-controller-turn Spellbook draw remains `RULE-CATALOG-0237`–`0238`.
Player-chosen additional Magic discard remains `RULE-CATALOG-0235`–`0236`.
Cemetery Site return remains `RULE-CATALOG-0233`–`0234`.
Cemetery Artifact return remains `RULE-CATALOG-0231`–`0232`.
Cemetery Magic return remains `RULE-CATALOG-0229`–`0230`.
Target-player mill Magic remains `RULE-CATALOG-0225`–`0228`.
Grant-Stealth remains `RULE-CATALOG-0223`–`0224`.
Grant-Ward remains `RULE-CATALOG-0221`–`0222`.
Tap-minion remains `RULE-CATALOG-0219`–`0220`.
Untap-minion remains `RULE-CATALOG-0217`–`0218`.
Target-player life-gain remains `RULE-CATALOG-0215`–`0216`.
Target-player life-loss remains `RULE-CATALOG-0213`–`0214`.
Destroy- and return-artifact remain `RULE-CATALOG-0211`–`0212`.
Return-site remains `RULE-CATALOG-0209`–`0210`.
Standalone destroy-site remains `RULE-CATALOG-0207`–`0208`.
Defending first strike remains `RULE-CATALOG-0204`–`0206`.

## TypeScript legality cutover

Done — do not redo:

- Game-setup proofs run through `SetupCtx`. Checkpoints resume only through Rust.
- `createGameSession` / `legalGameActions` / `stepGame` / `replayGame` / `verifyGameReplay` / `observeGame` call a blocking `session-json` client (`src/engine/rust-legality-sync.ts`).
- `actionDescriptors` / `applyDescriptor` and their observation helpers are deleted from `src/engine/game.ts`. That file keeps types, manifest validation, and the thin Rust wrappers.
- Async `RustGameSessionHandle` snapshots bind their checkpoints so `SetupCtx.observe` / `observeGame` resume on the shared worker.
- Private-check Chain Lightning extra-target mana uses a real 3-mana sibling (draw Atlas, do not play the fourth site). No constructed `GameSession` spreads remain.

Still TypeScript (not a second legality or observation engine):

- Manifest validation, authority ingestion, server, and browser UI.
- `run-private-game-check.ts` still *calls* the sync wrappers; it cannot be executed in this cloud checkout (no `.local/authority/`).

## Gate status on this branch

- `cargo fmt` / `clippy -D warnings` / `cargo test --workspace --all-features --locked` — green.
- `pnpm typecheck` / `pnpm lint` — green.
- `tests/engine` + catalog proofs — 255 engine + 1 catalog pass, including catalog 0167–0289.
- Full `pnpm test` in this checkout still has authority-collector / DATA-01 failures (`pwsh` missing, no private authority bundle). Those are environment gaps, not the cutover.

## Next exact step

1. Continue on this branch only. Next high-value official-rules work is leftover 2×2 fail-closed combinations (do not lift Voidwalk or tokens blindly) or more official start/end-turn slices. Target-player library draw on an empty library is a deck-out; mill remains a paid no-op. Atlantean Fate is a different flood that strips other abilities — do not conflate it with Flood. Do not invent MTG keywords.
2. Run `pnpm verify` and `pnpm game:check-private` on a machine that has `.local/authority/` and `pwsh`.
3. Retire this handoff and fast-forward `master` only after that private-check run is green.

## Do not

- Commit anything under `.local/` (whole directory is ignored; `.local/authority/` holds private authority bytes).
- Reintroduce TypeScript as a second legality engine.
- Re-migrate `tests/engine/game-setup.test.ts` or restore `applyDescriptor`.
- Acquire official artwork.
