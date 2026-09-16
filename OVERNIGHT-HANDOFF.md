# Overnight handoff

Canonical line: `cursor/source-bound-control-0005` @ `c715cb5`. Lanes through `RULE-CATALOG-0530` are on this branch. TypeScript admits 2×2 token/wrap/outer-column (`0371`–`0376`); 2×2 Landbound composes with Flood and Drought (`0507`–`0508`, `0511`); 2×2 Waterbound composes with Drought and Flood (`0509`–`0510`, `0512`); this-turn, stealth-bound, source-bound Genesis, sacrifice-to-steal Artifact, nearby-Avatar discard, and previous-player turn control are bound (`0513`–`0524`); draw-then-may-play land or water site Magic is bound (`0525`–`0526`); measured disable-until-damaged Magic is bound (`0527`–`0528`); grant-+2-this-turn then draw-spell Magic is bound (`0529`–`0530`). Branch new Phase 3 work as `cursor/<one-family>-0005` from here. Do not grow already-merged landings.

Lucky Charm extra-random discard-here damage, Sparkmage/Aramos random-discard honor, and the 2×2 occupy proofs through `RULE-CATALOG-0370` are merged on `master` (#54). Ballista range occupy, helper occupy, Flood/Drought water-cast proofs, Updraft Ridge occupy, measured-range Magic, and Secret Tunnel hops stay unchanged. Compact demo/batch stdout is unchanged.

2×2 Voidwalk is bound (`RULE-CATALOG-0316`–`0317`). Token occupancy (`0371`–`0372`), top/bottom wraparound (`0373`–`0374`), outer-column casts (`0375`–`0376`), deck-pair harness (`0377`–`0378`), oversized start-turn random teleport (`0379`–`0380`), and token Genesis draw-site/disable/adjacent-damage (`0381`–`0384`) are admitted.

Stale cutover branches were retired locally. `cursor/phase3-drown-bury-artifacts-36d3` is archived as `archive/cursor/phase3-drown-bury-artifacts-36d3` (`git tag -l 'archive/*'`). Do not branch from `codex/rust-cutover` or other superseded cutover lines.

Do not fast-forward `master` from a checkout that cannot run `pnpm verify` with authority fixtures and `pwsh`.

## Catalog count

`data/rules/catalog.json`: **530 rust-supported / 0 typescript-supported** out of 530.

Latest catalog proofs: grant-+2-this-turn then draw-spell Magic offers only allied minions, applies shared this-turn power, draws one hidden spell, expires at End Phase, and decks out on an empty Spellbook (`RULE-CATALOG-0529`–`0530`), measured disable-until-damaged Magic offers same-region minions within two cardinal steps, wakes on damage, lets enemy Ward absorb, and survives the caster’s next Start Phase (`RULE-CATALOG-0527`–`0528`), draw-then-may-play land or water site Magic draws a hidden site and then offers an extra matching site play that does not tap the Avatar (`RULE-CATALOG-0525`–`0526`), previous-player Genesis control lets each player’s next turn be submitted by the previous player, then expires (`RULE-CATALOG-0523`–`0524`), nearby Avatars may discard a card to take permanent control of the granting minion (`RULE-CATALOG-0521`–`0522`), sacrifice-to-steal Artifact control transfers a co-located enemy minion to the bearer until that bearer leaves (`RULE-CATALOG-0519`–`0520`), source-bound Genesis control steals tapped minions sharing the newcomer's footprint without targeting and reverts when that source leaves (`RULE-CATALOG-0517`–`0518`), stealth-bound enemy-minion control steals, taps, and hides a distant enemy, then reverts when Stealth is lost (`RULE-CATALOG-0515`–`0516`), this-turn enemy-minion control transfers a distant tapped enemy and reverts at End Phase (`RULE-CATALOG-0513`–`0514`), 2×2 Landbound becomes enabled when Drought covers its whole Water footprint (`RULE-CATALOG-0511`), 2×2 Waterbound becomes enabled when Flood covers its whole land footprint (`RULE-CATALOG-0512`), 2×2 Waterbound stays enabled under partial Drought and disables under full-footprint Drought (`RULE-CATALOG-0509`–`0510`), 2×2 Landbound stays enabled under partial Flood and disables under full-footprint Flood (`RULE-CATALOG-0507`–`0508`), ranked-ready TEST-04 eligibility on finished synthetic demo (`RULE-CATALOG-0506`), terminal synthetic deck-pair win with verified replay and batch transcript stability (`RULE-CATALOG-0504`–`0505`), extended synthetic deck-pair combat with verified replay (`RULE-CATALOG-0503`), stacked minion Genesis draw-spell, life loss, and controller heal on token and spellbook entry (`RULE-CATALOG-0501`–`0502`), targeted Genesis adjacent damage composes with local-minion sacrifice summon payment on decline and target branches (`RULE-CATALOG-0499`–`0500`), mixed unconditional plus first-copy-only site Genesis mana (`RULE-CATALOG-0497`–`0498`), Genesis disable-until-damaged composes with here-area damage on token and spellbook entry (`RULE-CATALOG-0495`–`0496`), Genesis disable-until-damaged composes with strike-each-enemy-here on token and spellbook entry (`RULE-CATALOG-0493`–`0494`), Genesis disable-until-damaged composes with controller life loss on token and spellbook entry (`RULE-CATALOG-0491`–`0492`), Genesis disable-until-damaged composes with controller heal on token and spellbook entry (`RULE-CATALOG-0489`–`0490`), Genesis disable-until-damaged composes with optional adjacent damage on decline and target branches (`RULE-CATALOG-0487`–`0488`), Genesis disable-until-damaged composes with draw-spell on token and spellbook entry (`RULE-CATALOG-0485`–`0486`), Genesis disable-until-damaged composes with draw-site on token and spellbook entry (`RULE-CATALOG-0483`–`0484`), targeted Genesis adjacent damage composes with random-card alternative summon payment on decline and target branches (`RULE-CATALOG-0481`–`0482`), Genesis disable-until-damaged strips Stealth on token and spellbook entry (`RULE-CATALOG-0479`–`0480`), first-copy-only gain-mana plus token-stealth-adjacent, heal-may-bottom, and immobilize-reorder stacks (`RULE-CATALOG-0471`–`0478`), first-copy-only gain-mana plus adjacent draw stacked with paid-token, may-bottom, reorder, and discard-top (`RULE-CATALOG-0463`–`0470`), first-copy-only gain-mana plus adjacent draw, stealth strip, heal, and immobilize (`RULE-CATALOG-0455`–`0462`), first-copy-only gain-mana plus discard-top, may-bottom, reorder, and paid-token (`RULE-CATALOG-0449`–`0454`), paid-token plus stealth strip (`RULE-CATALOG-0443`–`0444`), heal plus may-bottom (`RULE-CATALOG-0445`–`0446`), immobilize plus reorder (`RULE-CATALOG-0447`–`0448`), adjacent draw plus immobilize (`RULE-CATALOG-0441`–`0442`), adjacent draw plus heal (`RULE-CATALOG-0439`–`0440`), adjacent draw plus stealth strip (`RULE-CATALOG-0437`–`0438`), paid-token plus adjacent draw (`RULE-CATALOG-0435`–`0436`), adjacent draw plus reorder (`RULE-CATALOG-0433`–`0434`), adjacent draw plus may-bottom (`RULE-CATALOG-0431`–`0432`), gain-mana plus adjacent draw (`RULE-CATALOG-0429`–`0430`), discard-top plus reorder (`RULE-CATALOG-0427`–`0428`), discard-top plus may-bottom (`RULE-CATALOG-0425`–`0426`), paid-token plus reorder (`RULE-CATALOG-0423`–`0424`), may-bottom plus reorder chain (`RULE-CATALOG-0421`–`0422`), paid-token plus discard-top-spells (`RULE-CATALOG-0419`–`0420`), paid-token plus may-bottom (`RULE-CATALOG-0417`–`0418`), gain-mana plus discard-top-spells (`RULE-CATALOG-0415`–`0416`), gain-mana plus reorder-next-spells (`RULE-CATALOG-0413`–`0414`), gain-mana plus may-bottom-next-spell (`RULE-CATALOG-0411`–`0412`), paid-token plus gain-mana (`RULE-CATALOG-0409`–`0410`), 2×2 during-movement Ranged plus post-strike step combo (`RULE-CATALOG-0407`–`0408`), stacked site Genesis discard and adjacent draw (`RULE-CATALOG-0405`–`0406`), library + lure and teleport mixes (`RULE-CATALOG-0401`–`0404`), stacked start-turn exclusive pulses (`RULE-CATALOG-0399`–`0400`), library + mana/here mix (`RULE-CATALOG-0397`–`0398`), triple end-turn pulse stack (`RULE-CATALOG-0395`–`0396`), start-turn library + life mix (`RULE-CATALOG-0393`–`0394`), cross-zone start-turn library stacks (`RULE-CATALOG-0391`–`0392`), stacked end-turn life and here-damage pulses (`RULE-CATALOG-0389`–`0390`), stacked start-turn draw-then-mill (`RULE-CATALOG-0387`–`0388`), 2×2 Spellcaster Chain Magic footprint hops (`RULE-CATALOG-0385`–`0386`), token Genesis adjacent-damage choice (`RULE-CATALOG-0383`–`0384`), draw-site and disable-until-damaged on entry (`RULE-CATALOG-0381`–`0382`), 2×2 token footprint occupancy and banish (`RULE-CATALOG-0371`–`0372`), wraparound summon and movement (`0373`–`0374`), outer-column cast restriction combo (`0375`–`0376`), and deterministic deck-pair opening plus batch transcript replay (`0377`–`0378`). Lucky Charm extra-random for discard-here uses a 2×2 source’s occupied cells (`RULE-CATALOG-0369`–`0370`). A B3 occupant of an A3-anchored square is offered and takes the chosen damage; a C1 minion is not. Tap-pair Artifact range walks from every occupied bearer cell remain `RULE-CATALOG-0367`–`0368`. Tap-pair helpers standing with a 2×2 bearer remain `RULE-CATALOG-0365`–`0366`. Flood and Drought water-layer casts remain `RULE-CATALOG-0363`–`0364`. Updraft Ridge occupy remains `RULE-CATALOG-0361`–`0362`. Measured-range Magic walks from every occupied caster cell remain `RULE-CATALOG-0359`–`0360`. Fate Genesis 2×2 occupants remain `RULE-CATALOG-0355`–`0356`. Cave-In 2×2 occupants remain `RULE-CATALOG-0353`–`0354`. Site flight onto a void surfaces occupants remains `RULE-CATALOG-0351`–`0352`. Geomancer adjacent rubble surfaces void occupants remains `RULE-CATALOG-0349`–`0350`. Return-site 2×2 footprint banishment remains `RULE-CATALOG-0347`–`0348`. Return-site lower-layer banishment remains `RULE-CATALOG-0345`–`0346`. Play-site overlay conversion remains `RULE-CATALOG-0343`–`0344`. Site-flight layer conversion remains `RULE-CATALOG-0341`–`0342`. Overlay leave conversion remains `RULE-CATALOG-0339`–`0340`. Overlay enter conversion remains `RULE-CATALOG-0337`–`0338`. Fate Lose strips a covered non-Ordinary Tower bonus remains `RULE-CATALOG-0335`–`0336`. Fate Lose strips Genesis paid tokens remains `RULE-CATALOG-0333`–`0334`. Token power-threshold entry remains `RULE-CATALOG-0331`–`0332`.
Official Drought enabling Landbound remains `RULE-CATALOG-0322`. A Landbound minion on a Water site is Disabled; Drought makes that site land and the minion becomes enabled in place.
Official Pay Life remains `RULE-CATALOG-0320`–`0321`. Paying life is an additional Magic cost, not losing life. The caster may pay only when current life is at least the printed amount, so Death's Door cannot pay.
Official Landbound remains `RULE-CATALOG-0318`–`0319`. A Landbound minion is Disabled while it occupies no land location. Flood and mixed Water sites are not land. The Landbound ability itself still applies while Disabled.
Official 2×2 Voidwalk remains `RULE-CATALOG-0316`–`0317`. A 2×2 Voidwalk minion summons and steps only on all-void squares. Existing helpers do not offer a surface step, because a one-cell translation overlaps.
Official Monument carry prohibition remains `RULE-CATALOG-0314`–`0315`. A Monument can be conjured onto a site but cannot be conjured onto a unit or picked up. An ordinary Artifact on the same square stays carryable.
Belfry nearby untap remains `RULE-CATALOG-0312`–`0313`.
Deterministic `powered-movement` now uses public temporary-power identities on the seat observation. It does not add a new catalog family.
Production callers now ask `session-json` `selectPolicyAction` for the shared baseline policy. The TypeScript `selectDeterministicGameAction` selector is deleted.
Atlantean Fate remains `RULE-CATALOG-0310`–`0311`.
Start Phase doesn't-untap remains `RULE-CATALOG-0308`–`0309`.
End-of-controller-turn Avatar life remains `RULE-CATALOG-0304`–`0307`.
Deathrite library mill remains `RULE-CATALOG-0300`–`0303`.
Heal-target-minion Magic remains `RULE-CATALOG-0298`–`0299`.
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

## Next exact step

1. Continue remaining Phase 3 rule families listed in `03-PROGRESS.md` §Still required.
2. `pnpm game:check-private` and `pnpm game:selfplay-acceptance` are green on `master` with local `.local/authority/`. `pnpm authority:verify-private` still fails the production boundary-scan tests on this machine; that is an environment gap, not a rules regression. Finished games with passing TEST-04 gates now classify `unranked_unverified_authority` (rules-complete, authority still unverified). Full ranked waits on verified authority binding.
3. Retire this handoff when ranked-ready gates and private authority verification are both green.

## Do not

- Commit anything under `.local/` (whole directory is ignored; `.local/authority/` holds private authority bytes).
- Reintroduce TypeScript as a second legality engine.
- Re-migrate `tests/engine/game-setup.test.ts` or restore `applyDescriptor`.
- Acquire official artwork.
