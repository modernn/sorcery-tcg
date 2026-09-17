# Overnight handoff

Canonical line: `cursor/source-bound-control-0005` @ `0b71534fe02bd3226ba6cc04e9a953df2b31591c`. Lanes through `RULE-CATALOG-0622` are on this branch. TypeScript admits 2×2 token/wrap/outer-column (`0371`–`0376`); 2×2 Landbound composes with Flood and Drought (`0507`–`0508`, `0511`); 2×2 Waterbound composes with Drought and Flood (`0509`–`0510`, `0512`); this-turn, stealth-bound, source-bound Genesis, sacrifice-to-steal Artifact, nearby-Avatar discard, and previous-player turn control are bound (`0513`–`0524`); draw-then-may-play land or water site Magic is bound (`0525`–`0526`); measured disable-until-damaged Magic is bound (`0527`–`0528`); grant-+2-this-turn then draw-spell Magic is bound (`0529`–`0530`); grant-Lethal-this-turn then draw-spell Magic is bound (`0531`–`0532`); grant-Airborne-this-turn then draw-spell Magic is bound (`0533`–`0534`); grant-+1-movement-this-turn then draw-spell Magic is bound (`0535`–`0536`); Genesis untap-adjacent-allies is bound (`0537`–`0538`); grant-Stealth-to-allied-minions then draw-spell Magic is bound (`0539`–`0540`); grant-Stealth-to-an-allied-minion occupying an enemy site then draw-spell Magic is bound (`0541`–`0542`); summon-token-to-allied-minion then draw-spell Magic is bound (`0543`–`0544`); ward-each-allied-minion-at-target-water-site Magic is bound (`0545`–`0546`); pull-adjacent-aboveground-unit-to-target-water-site then draw-spell Magic is bound (`0547`–`0548`); return-up-to-three-cemetery-cards-to-deck-bottom then draw-spell Magic is bound (`0549`–`0550`); ward-nearby-minion-or-site Magic is bound (`0551`–`0552`); silence-and-tap-nearby-minion then may-draw-spell Magic is bound (`0553`–`0554`); ally-submerges-target-nearby-minion Magic is bound (`0555`–`0556`); ally-strikes-each-enemy-at-its-location Magic is bound (`0557`–`0558`); burrow-target-adjacent-minion Magic is bound (`0559`–`0560`); ally-takes-up-to-two-steps Magic is bound (`0561`–`0562`); teleport-target-one-diagonal Magic is bound (`0563`–`0564`); grant-double-damage-to-ally-next-strike-this-turn Magic is bound (`0565`–`0566`); kill-mortal-minions-at-location-within-two-steps Magic is bound (`0567`–`0568`); destroy-artifacts-and-auras-at-location-within-two-steps Magic is bound (`0569`–`0570`); destroy-minions-at-water-site-within-two-steps Magic is bound (`0571`–`0572`); banish-demon-and-undead-minions-at-location-within-two-steps Magic is bound (`0573`–`0574`); destroy-undead-minions-and-artifacts-at-location-within-two-steps Magic is bound (`0575`–`0576`); destroy-own-artifact-at-location-for-area-damage Magic is bound (`0577`–`0578`); summon-token-to-each-controlled-site-bordering-enemy-site Magic is bound (`0579`–`0580`); disable-target-nearby-minion-until-next-turn Magic is bound (`0581`–`0582`); summon-random-minion-from-any-cemetery Magic is bound (`0583`–`0584`); burrow-target-minion-or-artifact Magic is bound (`0585`–`0586`); burrow-all-minions-and-artifacts-at-target-land-site Magic is bound (`0587`–`0588`); damage-each-unit-at-location-within-two-steps Magic is bound (`0589`–`0590`); submerge-target-minion Magic is bound (`0591`–`0592`); return-minion-from-own-cemetery Magic is bound (`0593`–`0594`); damage-target-unit Magic is bound (`0595`–`0596`); grant-charge-to-ally-this-turn Magic is bound (`0597`–`0598`); leap-attack-ally Magic is bound (`0599`–`0600`); lure-enemy-minion-one-step-closer Magic is bound (`0601`–`0602`); fight-ally-with-adjacent-enemy Magic is bound (`0603`–`0604`); damage-each-aboveground-minion Magic is bound (`0605`–`0606`); damage-random-unit-at-location Magic is bound (`0607`–`0608`); kill-target-wounded-minion Magic is bound (`0609`–`0610`); tap-target-minion Magic is bound (`0611`–`0612`); untap-target-minion Magic is bound (`0613`–`0614`); grant-Ward-to-target-minion Magic is bound (`0615`–`0616`); grant-Stealth-to-target-minion Magic is bound (`0617`–`0618`); kill-target-minion Magic is bound (`0619`–`0620`); return-target-minion-to-owner-hand Magic is bound (`0621`–`0622`). Branch new Phase 3 work as `cursor/<one-family>-0005` from here. Do not grow already-merged landings.

Lucky Charm extra-random discard-here damage, Sparkmage/Aramos random-discard honor, and the 2×2 occupy proofs through `RULE-CATALOG-0370` are merged on `master` (#54). Ballista range occupy, helper occupy, Flood/Drought water-cast proofs, Updraft Ridge occupy, measured-range Magic, and Secret Tunnel hops stay unchanged. Compact demo/batch stdout is unchanged.

2×2 Voidwalk is bound (`RULE-CATALOG-0316`–`0317`). Token occupancy (`0371`–`0372`), top/bottom wraparound (`0373`–`0374`), outer-column casts (`0375`–`0376`), deck-pair harness (`0377`–`0378`), oversized start-turn random teleport (`0379`–`0380`), and token Genesis draw-site/disable/adjacent-damage (`0381`–`0384`) are admitted.

Stale cutover branches were retired locally. `cursor/phase3-drown-bury-artifacts-36d3` is archived as `archive/cursor/phase3-drown-bury-artifacts-36d3` (`git tag -l 'archive/*'`). Do not branch from `codex/rust-cutover` or other superseded cutover lines.

Do not fast-forward `master` from a checkout that cannot run `pnpm verify` with authority fixtures and `pwsh`.

## Catalog count

`data/rules/catalog.json`: **622 rust-supported / 0 typescript-supported** out of 622.

Latest catalog proofs: Grant-Ward Magic marks an unwarded minion and no-ops on already-warded targets (`RULE-CATALOG-0615`–`0616`), Grant-Stealth Magic hides an enemy minion and cannot target already-stealthed enemies (`RULE-CATALOG-0617`–`0618`), Kill-target-minion Magic destroys healthy minions while Ward absorbs the kill (`RULE-CATALOG-0619`–`0620`), and Bounce Magic returns minions to hand while Ward absorbs the return (`RULE-CATALOG-0621`–`0622`).

`magic_rules.rs` still hosts **66** catalog entries awaiting dedicated backfill (low-ID entries were not repointed).

## TypeScript legality cutover

Done — do not redo:

- Game-setup proofs run through `SetupCtx`. Checkpoints resume only through Rust.
- `createGameSession` / `legalGameActions` / `stepGame` / `replayGame` / `verifyGameReplay` / `observeGame` call a blocking `session-json` client (`src/engine/rust-legality-sync.ts`).
- `actionDescriptors` / `applyDescriptor` and their observation helpers are deleted from `src/engine/game.ts`. That file keeps types, manifest validation, and the thin Rust wrappers.
- Async `RustGameSessionHandle` snapshots bind their checkpoints so `SetupCtx.observe` / `observeGame` resume on the shared worker.
- Private-check Chain Lightning extra-target mana uses a real 3-mana sibling (draw Atlas, do not play the fourth site). No constructed `GameSession` spreads remain.

Still TypeScript (not a second legality, observation, or agent engine):

- Manifest validation, authority ingestion, server, and browser UI.
- `run-private-game-check.ts` still *calls* the sync wrappers; it cannot be executed in this cloud checkout (no `.local/authority/`).
- `run-private-novelty-gauntlet.ts` still orchestrates four private lesson jobs and writes authority-private reports. Single-root frontier search is `runNoveltyFrontierSearch`.
- Production agents call `selectPolicyAction` on the live Rust session.

## Gate status on master

- `pnpm typecheck` / `pnpm lint` — green.
- Monument playthrough (`RULE-04 a Monument cannot be conjured onto a unit or picked up`), catalog `0314`–`0315`, and the catalog linker — green.
- Belfry playthrough remains green.
- Remaining `pnpm test` engine and ingestion files — green. Full `pnpm verify` still fails authority-collector / some DATA-01 bundle cases (`pwsh` missing, no private authority bundle). Those are environment gaps, not rules gaps.
- PR #2 stays frozen. Rebuild `session-json` after engine fact changes.

## Pending merge (ready on worktrees, not on canonical)

None.

## Next exact step

1. Continue `magic_rules.rs` backfill for remaining engine-complete families still pointing at the shared file. Suggested iteration 5 four-lane batch: `destroy-site` (`0623`–`0624`, source `0207`–`0208`), `return-site` (`0625`–`0626`, source `0209` + protected edge), `mill-spells` (`0627`–`0628`, source `0225`–`0226`), `mill-sites` (`0629`–`0630`, source `0227`–`0228`). Next tier after that: `returnTargetArtifactToOwnerHand`, `destroyTargetArtifact`, cemetery-return families.
2. `pnpm game:check-private` and `pnpm game:selfplay-acceptance` are green on `master` with local `.local/authority/`. `pnpm authority:verify-private` still fails the production boundary-scan tests on this machine; that is an environment gap, not a rules regression. Finished games with passing TEST-04 gates now classify `unranked_unverified_authority` (rules-complete, authority still unverified). Full ranked waits on verified authority binding.
3. Retire this handoff when ranked-ready gates and private authority verification are both green.

## Do not

- Commit anything under `.local/` (whole directory is ignored; `.local/authority/` holds private authority bytes).
- Reintroduce TypeScript as a second legality engine.
- Re-migrate `tests/engine/game-setup.test.ts` or restore `applyDescriptor`.
- Acquire official artwork.
