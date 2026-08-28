import assert from 'node:assert/strict';
import test from 'node:test';

import { runPrivateGameCheck } from '../../src/commands/run-private-game-check.ts';

test('private actual-card decks complete deterministic combat, Earth, Air, Fire, and Water scenarios', async () => {
  const result = await runPrivateGameCheck();
  assert.equal(result.classification, 'private-local_actual-cards_unranked-partial-rules');
  assert.equal(result.airGenesisSpell.genesisMinion, 'Apprentice Wizard');
  assert.equal(result.airGenesisSpell.acceptedActionCount, 15);
  assert.equal(result.airGenesisSpell.drewSpell, true);
  assert.equal(result.airGenesisSpell.handSizePreserved, true);
  assert.equal(result.airGenesisSpell.hiddenFromOpponent, true);
  assert.equal(result.airGenesisSpell.deck.atlas.reduce((total, card) => total + card.copies, 0), 30);
  assert.equal(result.airGenesisSpell.deck.spellbook.reduce((total, card) => total + card.copies, 0), 60);
  assert.equal(result.airGenesisSpell.deck.spellbook
    .find(({ name }) => name === 'Apprentice Wizard')?.copies, 4);
  assert.equal(result.airGenesisSpell.replayVerified, true);
  assert.equal(result.airborne.airborneMinion, 'Plumed Pegasus');
  assert.equal(result.airborne.groundMinion, 'Ghoul');
  assert.equal(result.airborne.diagonalMove, true);
  assert.equal(result.airborne.airborneCanAttackGround, true);
  assert.equal(result.airborne.groundCannotIntercept, true);
  assert.equal(result.airborne.groundCannotAttackAirborne, true);
  assert.equal(result.airborne.deck.atlas.reduce((total, card) => total + card.copies, 0), 30);
  assert.equal(result.airborne.deck.spellbook.reduce((total, card) => total + card.copies, 0), 60);
  assert.equal(result.airborne.deck.spellbook
    .find(({ name }) => name === 'Plumed Pegasus')?.copies, 4);
  assert.equal(result.airborne.deck.spellbook
    .find(({ name }) => name === 'Ghoul')?.copies, 4);
  assert.equal(result.airborne.replayVerified, true);
  assert.equal(result.stealth.stealthMinion, 'Band of Thieves');
  assert.equal(result.stealth.groundMinion, 'Snow Leopard');
  assert.equal(result.stealth.enteredStealthed, true);
  assert.equal(result.stealth.groundCouldNotAttack, true);
  assert.equal(result.stealth.attackSkippedDefend, true);
  assert.equal(result.stealth.stealthLostAfterAttack, true);
  assert.equal(result.stealth.groundMinionDied, true);
  assert.equal(result.stealth.deck.atlas.reduce((total, card) => total + card.copies, 0), 30);
  assert.equal(result.stealth.deck.spellbook.reduce((total, card) => total + card.copies, 0), 60);
  assert.equal(result.stealth.deck.spellbook
    .find(({ name }) => name === 'Band of Thieves')?.copies, 4);
  assert.equal(result.stealth.deck.spellbook
    .find(({ name }) => name === 'Snow Leopard')?.copies, 4);
  assert.deepEqual(result.stealth.deck, result.airborne.deck);
  assert.equal(result.stealth.replayVerified, true);
  assert.equal(result.airMovement.movementMinion, 'Snallygaster');
  assert.equal(result.airMovement.twoStepDefend, true);
  assert.equal(result.airMovement.twoStepMoveAndAttack, true);
  assert.equal(result.airMovement.deck.atlas.reduce((total, card) => total + card.copies, 0), 30);
  assert.equal(result.airMovement.deck.spellbook.reduce((total, card) => total + card.copies, 0), 60);
  assert.equal(result.airMovement.replayVerified, true);
  assert.equal(result.airMovementTwo.movementMinion, 'Cloud Spirit');
  assert.equal(result.airMovementTwo.threeStepAirbornePath, true);
  assert.equal(result.airMovementTwo.attackAvailableAfterThreeSteps, true);
  assert.equal(result.airMovementTwo.returningPathAvailable, true);
  assert.equal(result.airMovementTwo.repeatedStepUnavailable, true);
  assert.equal(result.airMovementTwo.deck.atlas.reduce((total, card) => total + card.copies, 0), 30);
  assert.equal(result.airMovementTwo.deck.spellbook.reduce((total, card) => total + card.copies, 0), 60);
  assert.equal(result.airMovementTwo.deck.spellbook
    .find(({ name }) => name === 'Cloud Spirit')?.copies, 4);
  assert.deepEqual(result.airMovementTwo.deck, result.airborne.deck);
  assert.equal(result.airMovementTwo.replayVerified, true);
  assert.equal(result.airSummoning.roamingMinion, 'Roaming Monster');
  assert.equal(result.airSummoning.ordinaryRestricted, true);
  assert.equal(result.airSummoning.summonedAtEnemySite, true);
  assert.equal(result.airSummoning.deck.atlas.reduce((total, card) => total + card.copies, 0), 30);
  assert.equal(result.airSummoning.deck.spellbook.reduce((total, card) => total + card.copies, 0), 60);
  assert.deepEqual(result.airSummoning.deck, result.airMovement.deck);
  assert.equal(result.airSummoning.replayVerified, true);
  assert.equal(result.airVoidwalk.voidwalkMinion, 'Spectral Stalker');
  assert.equal(result.airVoidwalk.forsaken, 'Forsaken');
  assert.equal(result.airVoidwalk.acceptedActionCount, 17);
  assert.equal(result.airVoidwalk.forsakenOuterVoidAvailable, true);
  assert.equal(result.airVoidwalk.forsakenInnerVoidUnavailable, true);
  assert.equal(result.airVoidwalk.forsakenInnerSurfaceUnavailable, true);
  assert.equal(result.airVoidwalk.targetWasVoid, true);
  assert.equal(result.airVoidwalk.surfaceSummonAvailable, true);
  assert.equal(result.airVoidwalk.voidSummonAvailable, true);
  assert.equal(result.airVoidwalk.nonVoidSurfaceAvailable, true);
  assert.equal(result.airVoidwalk.nonVoidVoidUnavailable, true);
  assert.equal(result.airVoidwalk.summonedInVoid, true);
  assert.equal(result.airVoidwalk.voidMoveAvailable, true);
  assert.equal(result.airVoidwalk.surfaceExitAvailable, true);
  assert.equal(result.airVoidwalk.subsurfaceExitUnavailable, true);
  assert.equal(result.airVoidwalk.siteTargetAvailableAfterExit, true);
  assert.equal(result.airVoidwalk.deck.atlas.reduce((total, card) => total + card.copies, 0), 30);
  assert.equal(result.airVoidwalk.deck.spellbook.reduce((total, card) => total + card.copies, 0), 60);
  assert.equal(result.airVoidwalk.deck.spellbook
    .find(({ name }) => name === 'Spectral Stalker')?.copies, 4);
  assert.equal(result.airVoidwalk.deck.spellbook
    .find(({ name }) => name === 'Forsaken')?.copies, 4);
  assert.equal(result.airVoidwalk.replayVerified, true);
  assert.equal(result.airZap.zap, 'Zap!');
  assert.equal(result.airZap.snowLeopard, 'Snow Leopard');
  assert.equal(result.airZap.acceptedActionCount, 10);
  assert.equal(result.airZap.damageDealt, 1);
  assert.equal(result.airZap.manaPaid, 1);
  assert.equal(result.airZap.snowLeopardSurvived, true);
  assert.equal(result.airZap.spellLeftHand, true);
  assert.equal(result.airZap.spellEnteredCemetery, true);
  assert.equal(result.airZap.deck.atlas.reduce((total, card) => total + card.copies, 0), 30);
  assert.equal(result.airZap.deck.spellbook
    .reduce((total, card) => total + card.copies, 0), 60);
  assert.equal(result.airZap.deck.spellbook
    .find(({ name }) => name === 'Zap!')?.copies, 4);
  assert.equal(result.airZap.deck.spellbook
    .find(({ name }) => name === 'Snow Leopard')?.copies, 4);
  assert.equal(result.airZap.replayVerified, true);
  assert.equal(result.airArcLightning.arcLightning, 'Arc Lightning');
  assert.equal(result.airArcLightning.snowLeopard, 'Snow Leopard');
  assert.equal(result.airArcLightning.acceptedActionCount, 26);
  assert.equal(result.airArcLightning.nearbyTargetAvailable, true);
  assert.equal(result.airArcLightning.farSameRegionUnitUnavailable, true);
  assert.equal(result.airArcLightning.manaPaid, 4);
  assert.equal(result.airArcLightning.snowLeopardDied, true);
  assert.equal(result.airArcLightning.spellEnteredCemetery, true);
  assert.equal(result.airArcLightning.deck.atlas
    .reduce((total, card) => total + card.copies, 0), 30);
  assert.equal(result.airArcLightning.deck.spellbook
    .reduce((total, card) => total + card.copies, 0), 60);
  assert.equal(result.airArcLightning.deck.spellbook
    .find(({ name }) => name === 'Arc Lightning')?.copies, 4);
  assert.equal(result.airArcLightning.deck.spellbook
    .find(({ name }) => name === 'Snow Leopard')?.copies, 4);
  assert.equal(result.airArcLightning.replayVerified, true);
  assert.equal(result.airLeyline.henge, 'Leyline Henge');
  assert.equal(result.airLeyline.acceptedActionCount, 9);
  assert.equal(result.airLeyline.firstHengeDrewNothing, true);
  assert.equal(result.airLeyline.genesisDrewOne, true);
  assert.equal(result.airLeyline.hiddenFromOpponent, true);
  assert.equal(result.airLeyline.deck.atlas.reduce((total, card) => total + card.copies, 0), 30);
  assert.equal(result.airLeyline.deck.spellbook
    .reduce((total, card) => total + card.copies, 0), 60);
  assert.equal(result.airLeyline.deck.atlas
    .find(({ name }) => name === 'Leyline Henge')?.copies, 4);
  assert.equal(result.airLeyline.replayVerified, true);
  assert.equal(result.waterEdgeConnection.polarBears, 'Polar Bears');
  assert.equal(result.waterEdgeConnection.acceptedActionCount, 16);
  assert.equal(result.waterEdgeConnection.wrapMoveAvailable, true);
  assert.equal(result.waterEdgeConnection.avatarWrapUnavailable, true);
  assert.equal(result.waterEdgeConnection.siteTargetAvailable, true);
  assert.equal(result.waterEdgeConnection.deck.atlas.reduce((total, card) => total + card.copies, 0), 30);
  assert.equal(result.waterEdgeConnection.deck.spellbook
    .reduce((total, card) => total + card.copies, 0), 60);
  assert.equal(result.waterEdgeConnection.deck.spellbook
    .find(({ name }) => name === 'Polar Bears')?.copies, 4);
  assert.equal(result.waterEdgeConnection.replayVerified, true);
  assert.equal(result.avatarSpellDrawn, true);
  assert.equal(result.charge.activatedOnSummon, true);
  assert.equal(result.genesis.siteDrawn, true);
  assert.equal(result.lethal.tougherMinionKilled, true);
  assert.equal(result.provider.affinityAdded, true);
  assert.equal(result.decks.north.atlas.reduce((total, card) => total + card.copies, 0), 30);
  assert.equal(result.decks.south.atlas.reduce((total, card) => total + card.copies, 0), 30);
  assert.equal(result.decks.north.spellbook.reduce((total, card) => total + card.copies, 0), 60);
  assert.equal(result.decks.south.spellbook.reduce((total, card) => total + card.copies, 0), 60);
  assert.equal(result.earthBurrowing.burrowingMinion, 'Cave Trolls');
  assert.equal(result.earthBurrowing.acceptedActionCount, 28);
  assert.equal(result.earthBurrowing.targetIsLandSite, true);
  assert.equal(result.earthBurrowing.surfaceSummonAvailable, true);
  assert.equal(result.earthBurrowing.undergroundSummonAvailable, true);
  assert.equal(result.earthBurrowing.nonBurrowingSurfaceAvailable, true);
  assert.equal(result.earthBurrowing.nonBurrowingUndergroundUnavailable, true);
  assert.equal(result.earthBurrowing.movedUnderground, true);
  assert.equal(result.earthBurrowing.siteTargetUnavailableUnderground, true);
  assert.equal(result.earthBurrowing.surfaced, true);
  assert.equal(result.earthBurrowing.siteTargetAvailableAfterSurfacing, true);
  assert.equal(result.earthBurrowing.deck.atlas.reduce((total, card) => total + card.copies, 0), 30);
  assert.equal(result.earthBurrowing.deck.spellbook.reduce((total, card) => total + card.copies, 0), 60);
  assert.equal(result.earthBurrowing.deck.spellbook
    .find(({ name }) => name === 'Cave Trolls')?.copies, 4);
  assert.equal(result.earthBurrowing.replayVerified, true);
  assert.equal(result.earthEntombed.entombed, 'Entombed');
  assert.equal(result.earthEntombed.boskTroll, 'Bosk Troll');
  assert.equal(result.earthEntombed.acceptedActionCount, 10);
  assert.equal(result.earthEntombed.entombedSurfaceUnavailable, true);
  assert.equal(result.earthEntombed.entombedUndergroundAvailable, true);
  assert.equal(result.earthEntombed.boskTrollSurfaceAvailable, true);
  assert.equal(result.earthEntombed.boskTrollUndergroundUnavailable, true);
  assert.equal(result.earthEntombed.summonedUnderground, true);
  assert.equal(result.earthEntombed.deck.atlas.reduce((total, card) => total + card.copies, 0), 30);
  assert.equal(result.earthEntombed.deck.spellbook
    .reduce((total, card) => total + card.copies, 0), 60);
  assert.equal(result.earthEntombed.deck.spellbook
    .find(({ name }) => name === 'Entombed')?.copies, 4);
  assert.equal(result.earthEntombed.replayVerified, true);
  assert.equal(result.earthForwardMovement.phalanx, 'Dalcean Phalanx');
  assert.equal(result.earthForwardMovement.acceptedActionCount, 22);
  assert.equal(result.earthForwardMovement.forwardPathAvailable, true);
  assert.equal(result.earthForwardMovement.backwardPathUnavailable, true);
  assert.equal(result.earthForwardMovement.sidewaysPathUnavailable, true);
  assert.equal(result.earthForwardMovement.siteTargetAvailable, true);
  assert.equal(result.earthForwardMovement.deck.atlas
    .reduce((total, card) => total + card.copies, 0), 30);
  assert.equal(result.earthForwardMovement.deck.spellbook
    .reduce((total, card) => total + card.copies, 0), 60);
  assert.equal(result.earthForwardMovement.deck.spellbook
    .find(({ name }) => name === 'Dalcean Phalanx')?.copies, 3);
  assert.equal(result.earthForwardMovement.replayVerified, true);
  assert.equal(result.earthImmobile.pudgeButcher, 'Pudge Butcher');
  assert.equal(result.earthImmobile.comparatorMinion, 'Bosk Troll');
  assert.equal(result.earthImmobile.acceptedActionCount, 30);
  assert.equal(result.earthImmobile.nearbySitePresent, true);
  assert.equal(result.earthImmobile.positiveStepMoveUnavailable, true);
  assert.equal(result.earthImmobile.sameLocationAttackAvailable, true);
  assert.equal(result.earthImmobile.localDefendAvailable, true);
  assert.equal(result.earthImmobile.deck.atlas
    .reduce((total, card) => total + card.copies, 0), 30);
  assert.equal(result.earthImmobile.deck.spellbook
    .reduce((total, card) => total + card.copies, 0), 60);
  assert.equal(result.earthImmobile.deck.spellbook
    .find(({ name }) => name === 'Pudge Butcher')?.copies, 3);
  assert.equal(result.earthImmobile.deck.spellbook
    .find(({ name }) => name === 'Bosk Troll')?.copies, 4);
  assert.equal(result.earthImmobile.replayVerified, true);
  assert.equal(result.earthSecretTunnel.secretTunnel, 'Secret Tunnel');
  assert.equal(result.earthSecretTunnel.caveTrolls, 'Cave Trolls');
  assert.equal(result.earthSecretTunnel.acceptedActionCount, 21);
  assert.equal(result.earthSecretTunnel.physicalMoveAvailable, true);
  assert.equal(result.earthSecretTunnel.directTunnelMoveAvailable, true);
  assert.equal(result.earthSecretTunnel.directOpponentUnavailable, true);
  assert.equal(result.earthSecretTunnel.avatarPhysicalAvailable, true);
  assert.equal(result.earthSecretTunnel.avatarDirectUnavailable, true);
  assert.equal(result.earthSecretTunnel.movedUnderground, true);
  assert.equal(result.earthSecretTunnel.deck.atlas
    .reduce((total, card) => total + card.copies, 0), 30);
  assert.equal(result.earthSecretTunnel.deck.spellbook
    .reduce((total, card) => total + card.copies, 0), 60);
  assert.equal(result.earthSecretTunnel.deck.atlas
    .find(({ name }) => name === 'Secret Tunnel')?.copies, 3);
  assert.equal(result.earthSecretTunnel.deck.spellbook
    .find(({ name }) => name === 'Cave Trolls')?.copies, 4);
  assert.equal(result.earthSecretTunnel.replayVerified, true);
  assert.equal(result.earthRamp.affinityAdded, true);
  assert.equal(result.earthRamp.manaUnavailableWhileSick, true);
  assert.equal(result.earthRamp.manaGained, 2);
  assert.equal(result.earthRamp.rampPaidFive, true);
  assert.equal(result.earthRamp.payoffCanMoveAndAttack, true);
  assert.equal(result.earthRamp.movingDefendUnavailable, true);
  assert.equal(result.earthRamp.genesisSiteDrawn, true);
  assert.equal(result.earthRamp.ghostTown, 'Ghost Town');
  assert.equal(result.earthRamp.ghostTownBonusMana, 1);
  assert.equal(result.earthRamp.ghostTownUnusedManaExpired, true);
  assert.equal(result.earthRamp.deathriteSiteDrawnBeforeCemetery, true);
  assert.equal(result.earthRamp.deck.atlas.reduce((total, card) => total + card.copies, 0), 30);
  assert.equal(result.earthRamp.deck.spellbook.reduce((total, card) => total + card.copies, 0), 60);
  assert.equal(result.earthRamp.replayVerified, true);
  assert.equal(result.earthFirstStrike.firstStrikeMinion, 'Albespine Pikemen');
  assert.equal(result.earthFirstStrike.targetMinion, 'Bosk Troll');
  assert.equal(result.earthFirstStrike.attackerSurvivedUndamaged, true);
  assert.equal(result.earthFirstStrike.targetDiedBeforeReturn, true);
  assert.equal(result.earthFirstStrike.deck.atlas.reduce((total, card) => total + card.copies, 0), 30);
  assert.equal(result.earthFirstStrike.deck.spellbook.reduce((total, card) => total + card.copies, 0), 60);
  assert.equal(result.earthFirstStrike.deck.spellbook
    .find(({ name }) => name === 'Albespine Pikemen')?.copies, 3);
  assert.equal(result.earthFirstStrike.deck.spellbook
    .find(({ name }) => name === 'Bosk Troll')?.copies, 4);
  assert.deepEqual(result.earthFirstStrike.deck, result.earthRamp.deck);
  assert.equal(result.earthFirstStrike.replayVerified, true);
  assert.equal(result.earthRanged.rangedMinion, 'Belmotte Longbowmen');
  assert.equal(result.earthRanged.rangedOneStep, true);
  assert.equal(result.earthRanged.rangedShooterStayedSafe, true);
  assert.equal(result.earthRanged.rangedTargetDied, true);
  assert.equal(result.earthRanged.deck.atlas.reduce((total, card) => total + card.copies, 0), 30);
  assert.equal(result.earthRanged.deck.spellbook.reduce((total, card) => total + card.copies, 0), 60);
  assert.deepEqual(result.earthRanged.deck, result.earthRamp.deck);
  assert.equal(result.earthRanged.replayVerified, true);
  assert.equal(result.earthWard.rangedMinion, 'Belmotte Longbowmen');
  assert.equal(result.earthWard.wardMinion, 'Holy Warrior');
  assert.equal(result.earthWard.wardBroke, true);
  assert.equal(result.earthWard.wardPreventedDamage, true);
  assert.equal(result.earthWard.wardTargetSurvived, true);
  assert.equal(result.earthWard.wardTargetDiedAfterSecondShot, true);
  assert.equal(result.earthWard.deck.atlas.reduce((total, card) => total + card.copies, 0), 30);
  assert.equal(result.earthWard.deck.spellbook.reduce((total, card) => total + card.copies, 0), 60);
  assert.deepEqual(result.earthWard.deck, result.earthRamp.deck);
  assert.equal(result.earthWard.replayVerified, true);
  assert.equal(result.fireResponse.lumberingGiant, 'Lumbering Giant');
  assert.equal(result.fireResponse.monstrousLion, 'Monstrous Lion');
  assert.equal(result.fireResponse.chargeMoveAndAttack, true);
  assert.equal(result.fireResponse.unitTargetAvailable, true);
  assert.equal(result.fireResponse.siteTargetUnavailable, true);
  assert.equal(result.fireResponse.defendUnavailable, true);
  assert.equal(result.fireResponse.interceptUnavailable, true);
  assert.equal(result.fireResponse.deck.atlas.reduce((total, card) => total + card.copies, 0), 30);
  assert.equal(result.fireResponse.deck.spellbook.reduce((total, card) => total + card.copies, 0), 60);
  assert.equal(result.fireResponse.replayVerified, true);
  assert.equal(result.combat.northMinionDied, true);
  assert.equal(result.combat.southMinionDied, true);
  assert.equal(result.waterDrowned.drowned, 'Drowned');
  assert.equal(result.waterDrowned.slyFox, 'Sly Fox');
  assert.equal(result.waterDrowned.acceptedActionCount, 10);
  assert.equal(result.waterDrowned.drownedSurfaceUnavailable, true);
  assert.equal(result.waterDrowned.drownedUnderwaterAvailable, true);
  assert.equal(result.waterDrowned.slyFoxSurfaceAvailable, true);
  assert.equal(result.waterDrowned.slyFoxUnderwaterUnavailable, true);
  assert.equal(result.waterDrowned.summonedUnderwater, true);
  assert.equal(result.waterDrowned.deck.atlas.reduce((total, card) => total + card.copies, 0), 30);
  assert.equal(result.waterDrowned.deck.spellbook
    .reduce((total, card) => total + card.copies, 0), 60);
  assert.equal(result.waterDrowned.deck.spellbook
    .find(({ name }) => name === 'Drowned')?.copies, 4);
  assert.equal(result.waterDrowned.replayVerified, true);
  assert.equal(result.waterLugbog.lugbogCat, 'Lugbog Cat');
  assert.equal(result.waterLugbog.slyFox, 'Sly Fox');
  assert.equal(result.waterLugbog.acceptedActionCount, 16);
  assert.equal(result.waterLugbog.enemyWaterAvailable, true);
  assert.equal(result.waterLugbog.enemyLandUnavailable, true);
  assert.equal(result.waterLugbog.slyFoxControlledWaterAvailable, true);
  assert.equal(result.waterLugbog.slyFoxEnemyWaterUnavailable, true);
  assert.equal(result.waterLugbog.summonedToEnemyWater, true);
  assert.equal(result.waterLugbog.deck.atlas
    .reduce((total, card) => total + card.copies, 0), 30);
  assert.equal(result.waterLugbog.deck.spellbook
    .reduce((total, card) => total + card.copies, 0), 60);
  assert.equal(result.waterLugbog.deck.spellbook
    .find(({ name }) => name === 'Lugbog Cat')?.copies, 3);
  assert.equal(result.waterLugbog.deck.spellbook
    .find(({ name }) => name === 'Sly Fox')?.copies, 4);
  assert.equal(result.waterLugbog.replayVerified, true);
  assert.equal(result.waterEndTurnStealth.slyFox, 'Sly Fox');
  assert.equal(result.waterEndTurnStealth.acceptedActionCount, 23);
  assert.equal(result.waterEndTurnStealth.summonedUnstealthed, true);
  assert.equal(result.waterEndTurnStealth.gainedStealthAtEndOfTurn, true);
  assert.equal(result.waterEndTurnStealth.coLocatedReadyAttacker, true);
  assert.equal(result.waterEndTurnStealth.attackSiteAvailable, true);
  assert.equal(result.waterEndTurnStealth.slyFoxAttackUnavailable, true);
  assert.equal(result.waterEndTurnStealth.deck.atlas
    .reduce((total, card) => total + card.copies, 0), 30);
  assert.equal(result.waterEndTurnStealth.deck.spellbook
    .reduce((total, card) => total + card.copies, 0), 60);
  assert.equal(result.waterEndTurnStealth.deck.spellbook
    .find(({ name }) => name === 'Sly Fox')?.copies, 4);
  assert.equal(result.waterEndTurnStealth.replayVerified, true);
  assert.equal(result.waterSidewaysMovement.sedgeCrabs, 'Sedge Crabs');
  assert.equal(result.waterSidewaysMovement.acceptedActionCount, 22);
  assert.equal(result.waterSidewaysMovement.sidewaysPathAvailable, true);
  assert.equal(result.waterSidewaysMovement.forwardPathUnavailable, true);
  assert.equal(result.waterSidewaysMovement.backwardPathUnavailable, true);
  assert.equal(result.waterSidewaysMovement.deck.atlas
    .reduce((total, card) => total + card.copies, 0), 30);
  assert.equal(result.waterSidewaysMovement.deck.spellbook
    .reduce((total, card) => total + card.copies, 0), 60);
  assert.equal(result.waterSidewaysMovement.deck.spellbook
    .find(({ name }) => name === 'Sedge Crabs')?.copies, 4);
  assert.deepEqual(result.waterSidewaysMovement.deck, result.waterEndTurnStealth.deck);
  assert.equal(result.waterSidewaysMovement.replayVerified, true);
  assert.equal(result.waterSubmerge.submergeMinion, 'Coral-Reef Kelpie');
  assert.equal(result.waterSubmerge.acceptedActionCount, 16);
  assert.equal(result.waterSubmerge.targetIsWaterSite, true);
  assert.equal(result.waterSubmerge.surfaceSummonAvailable, true);
  assert.equal(result.waterSubmerge.underwaterSummonAvailable, true);
  assert.equal(result.waterSubmerge.nonSubmergeSurfaceAvailable, true);
  assert.equal(result.waterSubmerge.nonSubmergeUnderwaterUnavailable, true);
  assert.equal(result.waterSubmerge.summonedUnderwater, true);
  assert.equal(result.waterSubmerge.deck.atlas.reduce((total, card) => total + card.copies, 0), 30);
  assert.equal(result.waterSubmerge.deck.spellbook.reduce((total, card) => total + card.copies, 0), 60);
  assert.equal(result.waterSubmerge.deck.spellbook
    .find(({ name }) => name === 'Coral-Reef Kelpie')?.copies, 4);
  assert.equal(result.waterSubmerge.replayVerified, true);
  assert.equal(result.waterHealing.healingMinion, 'Muddy Pigs');
  assert.equal(result.waterHealing.healed, 3);
  assert.equal(result.waterHealing.healedBeforeCemetery, true);
  assert.equal(result.waterHealing.healingMinionDied, true);
  assert.equal(result.waterHealing.opponentMinionDied, true);
  assert.equal(result.waterHealing.deck.atlas.reduce((total, card) => total + card.copies, 0), 30);
  assert.equal(result.waterHealing.deck.spellbook.reduce((total, card) => total + card.copies, 0), 60);
  assert.deepEqual(result.waterEndTurnStealth.deck, result.waterHealing.deck);
  assert.equal(result.waterHealing.replayVerified, true);
  assert.equal(result.replayVerified, true);
});
