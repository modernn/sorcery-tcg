import { readFile } from 'node:fs/promises';
import { basename, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

import { canonicalJson, parseJsonWithDuplicateKeyCheck, type JsonValue } from '../authority/canonical-json.ts';
import { identityHash } from '../authority/hash.ts';
import {
  canonicalArtifactSchema,
  formatArtifactSchema,
  normalizedCardSnapshotSchema,
  type FormatDefinition,
  type Hash,
  type NormalizedCard,
} from '../authority/schemas.ts';
import {
  createGameManifest,
  createGameSession,
  hashGameState,
  legalGameActions,
  observeGame,
  stepGame,
  verifyGameReplay,
  type GameCardDefinition,
  type GameDeckSpec,
  type GameElement,
  type GameLegalAction,
  type GameManifest,
  type GameSeat,
  type GameSession,
} from '../engine/game.ts';

function ruleTextDigest(rulesText: string): Hash {
  return identityHash(rulesText as JsonValue);
}

const REPOSITORY_ROOT = resolve(import.meta.dirname, '..', '..');
const DEFAULT_SCENARIO = resolve(
  REPOSITORY_ROOT,
  '.local',
  'authority',
  'scenarios',
  'vanilla-constructed.json',
);
type ScenarioConfig = Readonly<{
  airborneSeed: number;
  airSeed: number;
  avatar: Readonly<{ drawSpell: boolean; stableId: string }>;
  cannotDefendMinionStableId: string;
  chargeMinionStableId: string;
  deathriteMinionStableId: string;
  earthProviderMinionStableId: string;
  earthFirstStrikeSeed: number;
  earthRangedSeed: number;
  earthSeed: number;
  earthWardSeed: number;
  fireSeed: number;
  firstStrikeMinionStableId: string;
  firstStrikeTargetMinionStableId: string;
  genesisMinionStableId: string;
  ghostTownSiteStableId: string;
  healingMinionStableId: string;
  lethalMinionStableId: string;
  lumberingMinionStableId: string;
  manaMinionStableId: string;
  monstrousLionStableId: string;
  movementMinionStableId: string;
  movementTwoSeed: number;
  providerMinionStableId: string;
  rangedMinionStableId: string;
  revisionId: string;
  roamingMinionStableId: string;
  roamingSeed: number;
  seed: number;
  slyFoxSeed: number;
  stealthSeed: number;
  waterSeed: number;
  wardMinionStableId: string;
}>;

type DeckList = Readonly<{
  atlas: readonly Readonly<{ copies: number; name: string }>[];
  avatar: string;
  spellbook: readonly Readonly<{ copies: number; name: string }>[];
}>;

export type PrivateGameCheck = Readonly<{
  acceptedActionCount: number;
  airborne: Readonly<{
    acceptedActionCount: number;
    airborneCanAttackGround: boolean;
    airborneMinion: string;
    deck: DeckList;
    diagonalMove: boolean;
    groundCannotAttackAirborne: boolean;
    groundCannotIntercept: boolean;
    groundMinion: string;
    replayVerified: boolean;
    seed: number;
  }>;
  airMovement: Readonly<{
    acceptedActionCount: number;
    deck: DeckList;
    movementMinion: string;
    replayVerified: boolean;
    seed: number;
    twoStepDefend: boolean;
    twoStepMoveAndAttack: boolean;
  }>;
  airMovementTwo: Readonly<{
    acceptedActionCount: number;
    attackAvailableAfterThreeSteps: boolean;
    deck: DeckList;
    movementMinion: string;
    replayVerified: boolean;
    seed: number;
    threeStepAirbornePath: boolean;
  }>;
  airSummoning: Readonly<{
    acceptedActionCount: number;
    deck: DeckList;
    ordinaryRestricted: boolean;
    replayVerified: boolean;
    roamingMinion: string;
    seed: number;
    summonedAtEnemySite: boolean;
  }>;
  authorityHash: Hash;
  avatarSpellDrawn: boolean;
  charge: Readonly<{ activatedOnSummon: boolean; minion: string }>;
  classification: 'private-local_actual-cards_unranked-partial-rules';
  combat: Readonly<{
    northMinion: string;
    northMinionDied: boolean;
    southMinion: string;
    southMinionDied: boolean;
  }>;
  decks: Readonly<Record<GameSeat, DeckList>>;
  finalStateHash: Hash;
  formatStableId: string;
  genesis: Readonly<{ minion: string; siteDrawn: boolean }>;
  earthRamp: Readonly<{
    acceptedActionCount: number;
    affinityAdded: boolean;
    deck: DeckList;
    deathriteMinion: string;
    deathriteSiteDrawnBeforeCemetery: boolean;
    genesisSiteDrawn: boolean;
    ghostTown: string;
    ghostTownBonusMana: number;
    ghostTownUnusedManaExpired: boolean;
    manaGained: number;
    manaMinion: string;
    manaUnavailableWhileSick: boolean;
    movingDefendUnavailable: boolean;
    payoffCanMoveAndAttack: boolean;
    rampPaidFive: boolean;
    rampPayoffMinion: string;
    replayVerified: boolean;
    seed: number;
  }>;
  earthFirstStrike: Readonly<{
    acceptedActionCount: number;
    attackerSurvivedUndamaged: boolean;
    deck: DeckList;
    firstStrikeMinion: string;
    replayVerified: boolean;
    seed: number;
    targetDiedBeforeReturn: boolean;
    targetMinion: string;
  }>;
  earthRanged: Readonly<{
    acceptedActionCount: number;
    deck: DeckList;
    rangedMinion: string;
    rangedOneStep: boolean;
    rangedShooterStayedSafe: boolean;
    rangedTargetDied: boolean;
    replayVerified: boolean;
    seed: number;
  }>;
  earthWard: Readonly<{
    acceptedActionCount: number;
    deck: DeckList;
    rangedMinion: string;
    replayVerified: boolean;
    seed: number;
    wardBroke: boolean;
    wardMinion: string;
    wardPreventedDamage: boolean;
    wardTargetDiedAfterSecondShot: boolean;
    wardTargetSurvived: boolean;
  }>;
  fireResponse: Readonly<{
    acceptedActionCount: number;
    chargeMoveAndAttack: boolean;
    deck: DeckList;
    defendUnavailable: boolean;
    interceptUnavailable: boolean;
    lumberingGiant: string;
    monstrousLion: string;
    replayVerified: boolean;
    seed: number;
    siteTargetUnavailable: boolean;
    unitTargetAvailable: boolean;
  }>;
  lethal: Readonly<{ minion: string; tougherMinionKilled: boolean }>;
  provider: Readonly<{ affinityAdded: boolean; minion: string }>;
  replayVerified: boolean;
  revisionId: string;
  seed: number;
  stealth: Readonly<{
    acceptedActionCount: number;
    attackSkippedDefend: boolean;
    deck: DeckList;
    enteredStealthed: boolean;
    groundCouldNotAttack: boolean;
    groundMinion: string;
    groundMinionDied: boolean;
    replayVerified: boolean;
    seed: number;
    stealthLostAfterAttack: boolean;
    stealthMinion: string;
  }>;
  waterEndTurnStealth: Readonly<{
    acceptedActionCount: number;
    attackSiteAvailable: boolean;
    coLocatedReadyAttacker: boolean;
    deck: DeckList;
    gainedStealthAtEndOfTurn: boolean;
    replayVerified: boolean;
    seed: number;
    slyFox: string;
    slyFoxAttackUnavailable: boolean;
    summonedUnstealthed: boolean;
  }>;
  waterHealing: Readonly<{
    acceptedActionCount: number;
    deck: DeckList;
    healed: number;
    healedBeforeCemetery: boolean;
    healingMinionDied: boolean;
    healingMinion: string;
    opponentMinionDied: boolean;
    replayVerified: boolean;
    seed: number;
  }>;
}>;

function isJsonRecord(value: unknown): value is Readonly<Record<string, JsonValue>> {
  return value !== null && typeof value === 'object' && !Array.isArray(value);
}

function scenarioConfig(value: JsonValue): ScenarioConfig {
  if (!isJsonRecord(value)) {
    throw new Error('private game scenario must be an object');
  }
  const avatar = value.avatar;
  if (!isJsonRecord(avatar)
    || !Number.isSafeInteger(value.airborneSeed)
    || typeof value.airborneSeed !== 'number'
    || value.airborneSeed < 0
    || !Number.isSafeInteger(value.airSeed)
    || typeof value.airSeed !== 'number'
    || value.airSeed < 0
    || typeof avatar.stableId !== 'string'
    || typeof avatar.drawSpell !== 'boolean'
    || typeof value.cannotDefendMinionStableId !== 'string'
    || typeof value.chargeMinionStableId !== 'string'
    || typeof value.deathriteMinionStableId !== 'string'
    || typeof value.earthProviderMinionStableId !== 'string'
    || !Number.isSafeInteger(value.earthFirstStrikeSeed)
    || typeof value.earthFirstStrikeSeed !== 'number'
    || value.earthFirstStrikeSeed < 0
    || !Number.isSafeInteger(value.earthRangedSeed)
    || typeof value.earthRangedSeed !== 'number'
    || value.earthRangedSeed < 0
    || !Number.isSafeInteger(value.earthSeed)
    || typeof value.earthSeed !== 'number'
    || value.earthSeed < 0
    || !Number.isSafeInteger(value.earthWardSeed)
    || typeof value.earthWardSeed !== 'number'
    || value.earthWardSeed < 0
    || !Number.isSafeInteger(value.fireSeed)
    || typeof value.fireSeed !== 'number'
    || value.fireSeed < 0
    || typeof value.firstStrikeMinionStableId !== 'string'
    || typeof value.firstStrikeTargetMinionStableId !== 'string'
    || typeof value.genesisMinionStableId !== 'string'
    || typeof value.ghostTownSiteStableId !== 'string'
    || typeof value.healingMinionStableId !== 'string'
    || typeof value.lethalMinionStableId !== 'string'
    || typeof value.lumberingMinionStableId !== 'string'
    || typeof value.manaMinionStableId !== 'string'
    || typeof value.monstrousLionStableId !== 'string'
    || typeof value.movementMinionStableId !== 'string'
    || !Number.isSafeInteger(value.movementTwoSeed)
    || typeof value.movementTwoSeed !== 'number'
    || value.movementTwoSeed < 0
    || typeof value.providerMinionStableId !== 'string'
    || typeof value.rangedMinionStableId !== 'string'
    || typeof value.revisionId !== 'string'
    || typeof value.roamingMinionStableId !== 'string'
    || !Number.isSafeInteger(value.roamingSeed)
    || typeof value.roamingSeed !== 'number'
    || value.roamingSeed < 0
    || !Number.isSafeInteger(value.seed)
    || typeof value.seed !== 'number'
    || value.seed < 0
    || !Number.isSafeInteger(value.slyFoxSeed)
    || typeof value.slyFoxSeed !== 'number'
    || value.slyFoxSeed < 0
    || !Number.isSafeInteger(value.stealthSeed)
    || typeof value.stealthSeed !== 'number'
    || value.stealthSeed < 0
    || !Number.isSafeInteger(value.waterSeed)
    || typeof value.waterSeed !== 'number'
    || value.waterSeed < 0
    || typeof value.wardMinionStableId !== 'string'
    || Object.keys(value).sort().join(',') !== 'airSeed,airborneSeed,avatar,cannotDefendMinionStableId,chargeMinionStableId,deathriteMinionStableId,earthFirstStrikeSeed,earthProviderMinionStableId,earthRangedSeed,earthSeed,earthWardSeed,fireSeed,firstStrikeMinionStableId,firstStrikeTargetMinionStableId,genesisMinionStableId,ghostTownSiteStableId,healingMinionStableId,lethalMinionStableId,lumberingMinionStableId,manaMinionStableId,monstrousLionStableId,movementMinionStableId,movementTwoSeed,providerMinionStableId,rangedMinionStableId,revisionId,roamingMinionStableId,roamingSeed,seed,slyFoxSeed,stealthSeed,wardMinionStableId,waterSeed'
    || Object.keys(avatar).sort().join(',') !== 'drawSpell,stableId') {
    throw new Error('private game scenario has an unsupported shape');
  }
  return {
    airborneSeed: value.airborneSeed,
    airSeed: value.airSeed,
    avatar: { drawSpell: avatar.drawSpell, stableId: avatar.stableId },
    cannotDefendMinionStableId: value.cannotDefendMinionStableId,
    chargeMinionStableId: value.chargeMinionStableId,
    deathriteMinionStableId: value.deathriteMinionStableId,
    earthFirstStrikeSeed: value.earthFirstStrikeSeed,
    earthProviderMinionStableId: value.earthProviderMinionStableId,
    earthRangedSeed: value.earthRangedSeed,
    earthSeed: value.earthSeed,
    earthWardSeed: value.earthWardSeed,
    fireSeed: value.fireSeed,
    firstStrikeMinionStableId: value.firstStrikeMinionStableId,
    firstStrikeTargetMinionStableId: value.firstStrikeTargetMinionStableId,
    genesisMinionStableId: value.genesisMinionStableId,
    ghostTownSiteStableId: value.ghostTownSiteStableId,
    healingMinionStableId: value.healingMinionStableId,
    lethalMinionStableId: value.lethalMinionStableId,
    lumberingMinionStableId: value.lumberingMinionStableId,
    manaMinionStableId: value.manaMinionStableId,
    monstrousLionStableId: value.monstrousLionStableId,
    movementMinionStableId: value.movementMinionStableId,
    movementTwoSeed: value.movementTwoSeed,
    providerMinionStableId: value.providerMinionStableId,
    rangedMinionStableId: value.rangedMinionStableId,
    revisionId: value.revisionId,
    roamingMinionStableId: value.roamingMinionStableId,
    roamingSeed: value.roamingSeed,
    seed: value.seed,
    slyFoxSeed: value.slyFoxSeed,
    stealthSeed: value.stealthSeed,
    waterSeed: value.waterSeed,
    wardMinionStableId: value.wardMinionStableId,
  };
}

async function readPrivateInputs(path: string): Promise<Readonly<{
  airborneMinion: NormalizedCard;
  airborneTargetMinion: NormalizedCard;
  authorityHash: Hash;
  cards: readonly NormalizedCard[];
  cannotDefendMinion: NormalizedCard;
  chargeMinion: NormalizedCard;
  config: ScenarioConfig;
  deathriteMinion: NormalizedCard;
  earthProviderMinion: NormalizedCard;
  format: FormatDefinition;
  formatStableId: string;
  firstStrikeMinion: NormalizedCard;
  firstStrikeTargetMinion: NormalizedCard;
  genesisMinion: NormalizedCard;
  ghostTownSite: NormalizedCard;
  healingMinion: NormalizedCard;
  lethalMinion: NormalizedCard;
  lumberingMinion: NormalizedCard;
  manaMinion: NormalizedCard;
  monstrousLion: NormalizedCard;
  movementMinion: NormalizedCard;
  movementTwoMinion: NormalizedCard;
  providerMinion: NormalizedCard;
  rangedMinion: NormalizedCard;
  roamingMinion: NormalizedCard;
  slyFox: NormalizedCard;
  stealthMinion: NormalizedCard;
  stealthTargetMinion: NormalizedCard;
  wardMinion: NormalizedCard;
}>> {
  const config = scenarioConfig(parseJsonWithDuplicateKeyCheck(await readFile(path, 'utf8')));
  const revisionRoot = resolve(REPOSITORY_ROOT, '.local', 'authority', 'revisions', config.revisionId);
  if (basename(revisionRoot) !== config.revisionId) throw new Error('private revision ID must be one path segment');

  const artifact = canonicalArtifactSchema.parse(parseJsonWithDuplicateKeyCheck(
    await readFile(resolve(revisionRoot, 'cards.normalized.json'), 'utf8'),
  ));
  if (artifact.identity.artifactKind !== 'card-snapshot'
    || identityHash(artifact.identity as unknown as JsonValue) !== artifact.contentHash) {
    throw new Error('private normalized card artifact identity is invalid');
  }
  const snapshot = normalizedCardSnapshotSchema.parse(artifact.identity.payload);
  const airborneMinion = snapshot.cards.find(({ name }) => name === 'Plumed Pegasus');
  if (!airborneMinion
    || airborneMinion.cardType !== 'minion'
    || ruleTextDigest(airborneMinion.rulesText) !== 'sha256:6e79df3e3cb30888c18998c6c630cf8bff79b8977b2d020ab3a8701617177d44'
    || airborneMinion.manaCost !== 3
    || airborneMinion.attack !== 3
    || airborneMinion.defense !== 3
    || airborneMinion.elements.length !== 1
    || airborneMinion.elements[0] !== 'air'
    || airborneMinion.thresholds.air !== 1
    || airborneMinion.thresholds.earth !== 0
    || airborneMinion.thresholds.fire !== 0
    || airborneMinion.thresholds.water !== 0
    || airborneMinion.rarity !== 'ordinary') {
    throw new Error('private Airborne minion no longer matches its supported facts');
  }
  const airborneTargetMinion = snapshot.cards.find(({ name }) => name === 'Ghoul');
  if (!airborneTargetMinion
    || airborneTargetMinion.cardType !== 'minion'
    || airborneTargetMinion.rulesText.trim() !== ''
    || airborneTargetMinion.manaCost !== 3
    || airborneTargetMinion.attack !== 3
    || airborneTargetMinion.defense !== 3
    || airborneTargetMinion.elements.length !== 1
    || airborneTargetMinion.elements[0] !== 'air'
    || airborneTargetMinion.thresholds.air !== 1
    || airborneTargetMinion.thresholds.earth !== 0
    || airborneTargetMinion.thresholds.fire !== 0
    || airborneTargetMinion.thresholds.water !== 0
    || airborneTargetMinion.rarity !== 'ordinary') {
    throw new Error('private Airborne target minion no longer matches its supported facts');
  }
  const stealthMinion = snapshot.cards.find(({ name }) => name === 'Band of Thieves');
  if (!stealthMinion
    || stealthMinion.cardType !== 'minion'
    || ruleTextDigest(stealthMinion.rulesText) !== 'sha256:72a55afdb2b2d9624948393c357d58ee59673b285dd30c70dad3056b49aa4288'
    || stealthMinion.manaCost !== 3
    || stealthMinion.attack !== 3
    || stealthMinion.defense !== 3
    || stealthMinion.elements.length !== 1
    || stealthMinion.elements[0] !== 'air'
    || stealthMinion.thresholds.air !== 2
    || stealthMinion.thresholds.earth !== 0
    || stealthMinion.thresholds.fire !== 0
    || stealthMinion.thresholds.water !== 0
    || stealthMinion.rarity !== 'ordinary') {
    throw new Error('private Stealth minion no longer matches its supported facts');
  }
  const stealthTargetMinion = snapshot.cards.find(({ name }) => name === 'Snow Leopard');
  if (!stealthTargetMinion
    || stealthTargetMinion.cardType !== 'minion'
    || stealthTargetMinion.rulesText.trim() !== ''
    || stealthTargetMinion.manaCost !== 1
    || stealthTargetMinion.attack !== 2
    || stealthTargetMinion.defense !== 2
    || stealthTargetMinion.elements.length !== 1
    || stealthTargetMinion.elements[0] !== 'air'
    || stealthTargetMinion.thresholds.air !== 1
    || stealthTargetMinion.thresholds.earth !== 0
    || stealthTargetMinion.thresholds.fire !== 0
    || stealthTargetMinion.thresholds.water !== 0
    || stealthTargetMinion.rarity !== 'ordinary') {
    throw new Error('private Stealth target minion no longer matches its supported facts');
  }
  const slyFox = snapshot.cards.find(({ name }) => name === 'Sly Fox');
  if (!slyFox
    || slyFox.cardType !== 'minion'
    || ruleTextDigest(slyFox.rulesText) !== 'sha256:3c776ca4c65ff111ce71d34582be9cdffcd9468177ade70c831c5062e101f26b'
    || slyFox.manaCost !== 1
    || slyFox.attack !== 1
    || slyFox.defense !== 1
    || slyFox.elements.length !== 1
    || slyFox.elements[0] !== 'water'
    || slyFox.thresholds.air !== 0
    || slyFox.thresholds.earth !== 0
    || slyFox.thresholds.fire !== 0
    || slyFox.thresholds.water !== 1
    || slyFox.rarity !== 'ordinary') {
    throw new Error('private end-turn Stealth minion no longer matches its supported facts');
  }
  const cannotDefendMinion = snapshot.cards
    .find(({ stableId }) => stableId === config.cannotDefendMinionStableId);
  if (!cannotDefendMinion
    || cannotDefendMinion.cardType !== 'minion'
    || ruleTextDigest(cannotDefendMinion.rulesText) !== 'sha256:fdefbee8ed48ee2b3052920a26709b40ca4f25b05013d5512d708f300c108082'
    || cannotDefendMinion.attack === null
    || cannotDefendMinion.defense === null
    || cannotDefendMinion.manaCost === null
    || cannotDefendMinion.rarity === null) {
    throw new Error('private moving-Defend restriction minion no longer matches its supported facts');
  }
  const chargeMinion = snapshot.cards.find(({ stableId }) => stableId === config.chargeMinionStableId);
  if (!chargeMinion
    || chargeMinion.cardType !== 'minion'
    || ruleTextDigest(chargeMinion.rulesText) !== 'sha256:85c4ac28604c87578297367ac49adfd2139a72edeadc491fe9383ef50e127c07'
    || chargeMinion.attack === null
    || chargeMinion.defense === null
    || chargeMinion.manaCost === null
    || chargeMinion.rarity === null) {
    throw new Error('private Charge minion no longer matches its supported facts');
  }
  const deathriteMinion = snapshot.cards.find(({ stableId }) => stableId === config.deathriteMinionStableId);
  if (!deathriteMinion
    || deathriteMinion.cardType !== 'minion'
    || ruleTextDigest(deathriteMinion.rulesText) !== 'sha256:3684fafcb97b44bf47cb7fc4221a94e76445b17f77af6dc211c8e4dc92a56344'
    || deathriteMinion.attack === null
    || deathriteMinion.defense === null
    || deathriteMinion.manaCost === null
    || deathriteMinion.rarity === null) {
    throw new Error('private Deathrite minion no longer matches its supported facts');
  }
  const providerMinion = snapshot.cards.find(({ stableId }) => stableId === config.providerMinionStableId);
  if (!providerMinion
    || providerMinion.cardType !== 'minion'
    || ruleTextDigest(providerMinion.rulesText) !== 'sha256:7b3ab7708f281accdae1f925e91090a0268493c0c30783c989d7bd3bbd875c20'
    || providerMinion.attack === null
    || providerMinion.defense === null
    || providerMinion.manaCost === null
    || providerMinion.rarity === null) {
    throw new Error('private affinity provider no longer matches its supported facts');
  }
  const rangedMinion = snapshot.cards.find(({ stableId }) => stableId === config.rangedMinionStableId);
  if (!rangedMinion
    || rangedMinion.cardType !== 'minion'
    || ruleTextDigest(rangedMinion.rulesText) !== 'sha256:95fb48373a18ac8f27d4825f331a8d262e5f6199d4873bce6ac876803514449b'
    || rangedMinion.attack === null
    || rangedMinion.defense === null
    || rangedMinion.manaCost === null
    || rangedMinion.rarity === null) {
    throw new Error('private Ranged minion no longer matches its supported facts');
  }
  const lethalMinion = snapshot.cards.find(({ stableId }) => stableId === config.lethalMinionStableId);
  if (!lethalMinion
    || lethalMinion.cardType !== 'minion'
    || ruleTextDigest(lethalMinion.rulesText) !== 'sha256:e80bc5195a9c887e88d38c4d42f2aa9caff3a1c0240dafacb15a49f09eb25bb7'
    || lethalMinion.attack === null
    || lethalMinion.defense === null
    || lethalMinion.manaCost === null
    || lethalMinion.rarity === null) {
    throw new Error('private Lethal minion no longer matches its supported facts');
  }
  const lumberingMinion = snapshot.cards.find(({ stableId }) => stableId === config.lumberingMinionStableId);
  if (!lumberingMinion
    || lumberingMinion.cardType !== 'minion'
    || ruleTextDigest(lumberingMinion.rulesText) !== 'sha256:7f0e8e1a70fae8546468458b7cf4cedb3c8a4a55220cfc310242a813ea18756f'
    || lumberingMinion.attack === null
    || lumberingMinion.defense === null
    || lumberingMinion.manaCost === null
    || lumberingMinion.rarity === null) {
    throw new Error('private Defend-or-Intercept prohibition minion no longer matches its supported facts');
  }
  const monstrousLion = snapshot.cards.find(({ stableId }) => stableId === config.monstrousLionStableId);
  if (!monstrousLion
    || monstrousLion.cardType !== 'minion'
    || ruleTextDigest(monstrousLion.rulesText) !== 'sha256:deb371d2f58d8c61fdcd1c6fe7b66d351aa29b8da3f6a9b2a289bada52ad8103'
    || monstrousLion.attack === null
    || monstrousLion.defense === null
    || monstrousLion.manaCost === null
    || monstrousLion.rarity === null) {
    throw new Error('private Charge and site-attack restriction minion no longer matches its supported facts');
  }
  const genesisMinion = snapshot.cards.find(({ stableId }) => stableId === config.genesisMinionStableId);
  if (!genesisMinion
    || genesisMinion.cardType !== 'minion'
    || ruleTextDigest(genesisMinion.rulesText) !== 'sha256:d3dcab14c64eed981dfd003402716da68b7510c09b7419f7c8f344c330a9bb0f'
    || genesisMinion.attack === null
    || genesisMinion.defense === null
    || genesisMinion.manaCost === null
    || genesisMinion.rarity === null) {
    throw new Error('private Genesis minion no longer matches its supported facts');
  }
  const ghostTownSite = snapshot.cards.find(({ stableId }) => stableId === config.ghostTownSiteStableId);
  if (!ghostTownSite
    || ghostTownSite.cardType !== 'site'
    || ruleTextDigest(ghostTownSite.rulesText) !== 'sha256:88d7d0059b0cb0b591affdaf84626949782c50f2fdd0d332a4424662f4fef791'
    || ghostTownSite.rarity === null) {
    throw new Error('private site Genesis mana card no longer matches its supported facts');
  }
  const healingMinion = snapshot.cards.find(({ stableId }) => stableId === config.healingMinionStableId);
  if (!healingMinion
    || healingMinion.cardType !== 'minion'
    || ruleTextDigest(healingMinion.rulesText) !== 'sha256:7ba046800e281cf208a39389a94bf1ffdf92343bd0fe0417356a69d7bcd85372'
    || healingMinion.attack === null
    || healingMinion.defense === null
    || healingMinion.manaCost === null
    || healingMinion.rarity === null) {
    throw new Error('private Deathrite healing minion no longer matches its supported facts');
  }
  const earthProviderMinion = snapshot.cards
    .find(({ stableId }) => stableId === config.earthProviderMinionStableId);
  if (!earthProviderMinion
    || earthProviderMinion.cardType !== 'minion'
    || ruleTextDigest(earthProviderMinion.rulesText) !== 'sha256:a2f49f1ea2e2cace3fc57c535c8dac21efcdd52fe267f953d2a986ab3f342db8'
    || earthProviderMinion.attack === null
    || earthProviderMinion.defense === null
    || earthProviderMinion.manaCost === null
    || earthProviderMinion.rarity === null) {
    throw new Error('private Earth affinity provider no longer matches its supported facts');
  }
  const manaMinion = snapshot.cards.find(({ stableId }) => stableId === config.manaMinionStableId);
  if (!manaMinion
    || manaMinion.cardType !== 'minion'
    || ruleTextDigest(manaMinion.rulesText) !== 'sha256:9ffe2c2dfc8f5fde05a94e196c567ad9b3ba79f4a313e7711bb00d00c8c2c29a'
    || manaMinion.attack === null
    || manaMinion.defense === null
    || manaMinion.manaCost === null
    || manaMinion.rarity === null) {
    throw new Error('private mana minion no longer matches its supported facts');
  }
  const movementMinion = snapshot.cards.find(({ stableId }) => stableId === config.movementMinionStableId);
  if (!movementMinion
    || movementMinion.cardType !== 'minion'
    || ruleTextDigest(movementMinion.rulesText) !== 'sha256:9dc4252080d544e6615711ac7db3bfb97f3b9a549a80485036927d2a444fb17c'
    || movementMinion.attack === null
    || movementMinion.defense === null
    || movementMinion.manaCost === null
    || movementMinion.rarity === null) {
    throw new Error('private Movement +1 minion no longer matches its supported facts');
  }
  const movementTwoMinion = snapshot.cards.find(({ name }) => name === 'Cloud Spirit');
  if (!movementTwoMinion
    || movementTwoMinion.cardType !== 'minion'
    || ruleTextDigest(movementTwoMinion.rulesText) !== 'sha256:09974cf8ccd5af0cb1d62ee69213f78f27673326989b0dd4114ce6b8e1256324'
    || movementTwoMinion.manaCost !== 2
    || movementTwoMinion.attack !== 2
    || movementTwoMinion.defense !== 2
    || movementTwoMinion.elements.length !== 1
    || movementTwoMinion.elements[0] !== 'air'
    || movementTwoMinion.thresholds.air !== 2
    || movementTwoMinion.thresholds.earth !== 0
    || movementTwoMinion.thresholds.fire !== 0
    || movementTwoMinion.thresholds.water !== 0
    || movementTwoMinion.rarity !== 'ordinary') {
    throw new Error('private Movement +2 minion no longer matches its supported facts');
  }
  const roamingMinion = snapshot.cards.find(({ stableId }) => stableId === config.roamingMinionStableId);
  if (!roamingMinion
    || roamingMinion.cardType !== 'minion'
    || ruleTextDigest(roamingMinion.rulesText) !== 'sha256:2f6fd360844dc6a53f414c2fa5a3bc9d0294aa5b68bea0981cb1793737ff64e7'
    || roamingMinion.attack === null
    || roamingMinion.defense === null
    || roamingMinion.manaCost === null
    || roamingMinion.rarity === null) {
    throw new Error('private unrestricted-site summon minion no longer matches its supported facts');
  }
  const wardMinion = snapshot.cards.find(({ stableId }) => stableId === config.wardMinionStableId);
  if (!wardMinion
    || wardMinion.name !== 'Holy Warrior'
    || wardMinion.cardType !== 'minion'
    || ruleTextDigest(wardMinion.rulesText) !== 'sha256:c0041315fc144f7a9f09f98eb1d50f513fc3fd83308bb816606ac4b689733316'
    || wardMinion.manaCost !== 2
    || wardMinion.attack !== 2
    || wardMinion.defense !== 2
    || wardMinion.elements.length !== 1
    || wardMinion.elements[0] !== 'earth'
    || wardMinion.thresholds.earth !== 1
    || wardMinion.thresholds.air !== 0
    || wardMinion.thresholds.fire !== 0
    || wardMinion.thresholds.water !== 0
    || wardMinion.rarity !== 'ordinary') {
    throw new Error('private Ward minion no longer matches its supported facts');
  }
  const firstStrikeMinion = snapshot.cards
    .find(({ stableId }) => stableId === config.firstStrikeMinionStableId);
  if (!firstStrikeMinion
    || firstStrikeMinion.name !== 'Albespine Pikemen'
    || firstStrikeMinion.cardType !== 'minion'
    || ruleTextDigest(firstStrikeMinion.rulesText) !== 'sha256:3b875d6f0d654a614982a30692bcbdbb5a74d06f39d30a26377b913103e04bd7'
    || firstStrikeMinion.manaCost !== 3
    || firstStrikeMinion.attack !== 3
    || firstStrikeMinion.defense !== 3
    || firstStrikeMinion.elements.length !== 1
    || firstStrikeMinion.elements[0] !== 'earth'
    || firstStrikeMinion.thresholds.earth !== 2
    || firstStrikeMinion.thresholds.air !== 0
    || firstStrikeMinion.thresholds.fire !== 0
    || firstStrikeMinion.thresholds.water !== 0
    || firstStrikeMinion.rarity !== 'exceptional') {
    throw new Error('private attacking first-strike minion no longer matches its supported facts');
  }
  const firstStrikeTargetMinion = snapshot.cards
    .find(({ stableId }) => stableId === config.firstStrikeTargetMinionStableId);
  if (!firstStrikeTargetMinion
    || firstStrikeTargetMinion.name !== 'Bosk Troll'
    || firstStrikeTargetMinion.cardType !== 'minion'
    || firstStrikeTargetMinion.rulesText.trim() !== ''
    || firstStrikeTargetMinion.manaCost !== 2
    || firstStrikeTargetMinion.attack !== 3
    || firstStrikeTargetMinion.defense !== 3
    || firstStrikeTargetMinion.elements.length !== 1
    || firstStrikeTargetMinion.elements[0] !== 'earth'
    || firstStrikeTargetMinion.thresholds.earth !== 1
    || firstStrikeTargetMinion.thresholds.air !== 0
    || firstStrikeTargetMinion.thresholds.fire !== 0
    || firstStrikeTargetMinion.thresholds.water !== 0
    || firstStrikeTargetMinion.rarity !== 'ordinary') {
    throw new Error('private first-strike target minion no longer matches its supported facts');
  }

  const formatsValue = parseJsonWithDuplicateKeyCheck(await readFile(resolve(revisionRoot, 'formats.json'), 'utf8'));
  if (!isJsonRecord(formatsValue)
    || !Array.isArray(formatsValue.formats)) {
    throw new Error('private format artifact collection is invalid');
  }
  const formats = formatsValue.formats.map((value) => formatArtifactSchema.parse(value));
  formats.forEach((value) => {
    if (identityHash(value.identity as unknown as JsonValue) !== value.contentHash) {
      throw new Error('private format artifact identity is invalid');
    }
  });
  const selected = formats
    .filter(({ identity }) => identity.payload.name === 'Constructed')
    .sort((left, right) => right.identity.payload.effectiveDate.localeCompare(left.identity.payload.effectiveDate))[0];
  if (!selected) throw new Error('private authority has no Constructed format');
  return {
    airborneMinion,
    airborneTargetMinion,
    authorityHash: artifact.contentHash,
    cards: snapshot.cards,
    cannotDefendMinion,
    chargeMinion,
    config,
    deathriteMinion,
    earthProviderMinion,
    format: selected.identity.payload,
    formatStableId: selected.identity.stableId,
    firstStrikeMinion,
    firstStrikeTargetMinion,
    genesisMinion,
    ghostTownSite,
    healingMinion,
    lethalMinion,
    lumberingMinion,
    manaMinion,
    monstrousLion,
    movementMinion,
    movementTwoMinion,
    providerMinion,
    rangedMinion,
    roamingMinion,
    slyFox,
    stealthMinion,
    stealthTargetMinion,
    wardMinion,
  };
}

function ordered<T extends { stableId: string }>(values: readonly T[], reverse: boolean): readonly T[] {
  return [...values].sort((left, right) => {
    const order = left.stableId < right.stableId ? -1 : left.stableId > right.stableId ? 1 : 0;
    return reverse ? -order : order;
  });
}

function fillZone(
  candidates: readonly NormalizedCard[],
  count: number,
  format: FormatDefinition,
  reverse: boolean,
): readonly string[] {
  const result: string[] = [];
  for (const card of ordered(candidates, reverse)) {
    if (!card.rarity) throw new Error('supported deck card lacks rarity');
    const copies = format.copyLimits[card.rarity];
    result.push(...Array.from({ length: Math.min(copies, count - result.length) }, () => card.stableId));
    if (result.length === count) return result;
  }
  throw new Error(`supported actual cards can fill only ${result.length} of ${count} required cards`);
}

function gameDefinition(
  card: NormalizedCard,
  drawSpell: boolean,
  charge = false,
  genesisDrawSite = false,
  lethal = false,
  provides?: GameElement,
  tapForMana?: number,
  deathriteDrawSite = false,
  cannotDefend = false,
  movementBonus: 0 | 1 | 2 = 0,
  deathriteHeal = 0,
  siteGenesisGainMana = 0,
  summonToAnySite = false,
  cannotDefendOrIntercept = false,
  cannotAttackSites = false,
  ranged = false,
  strikesFirstWhileAttacking = false,
  ward = false,
  airborne = false,
  stealth = false,
  gainsStealthAtEndOfTurn = false,
): GameCardDefinition {
  if (card.cardType === 'avatar'
    && card.attack !== null
    && card.defense !== null
    && card.life !== null) {
    return {
      attack: card.attack,
      cardType: 'avatar',
      defense: card.defense,
      drawSpell,
      life: card.life,
    };
  }
  if (card.cardType === 'site') {
    return {
      cardType: 'site',
      elements: card.elements,
      ...(siteGenesisGainMana ? { genesisGainMana: siteGenesisGainMana } : {}),
    };
  }
  if (card.cardType === 'minion'
    && card.attack !== null
    && card.defense !== null
    && card.manaCost !== null) {
    return {
      airborne,
      attack: card.attack,
      cardType: 'minion',
      cannotAttackSites,
      cannotDefend,
      cannotDefendOrIntercept,
      charge,
      deathriteDrawSite,
      ...(deathriteHeal ? { deathriteHeal } : {}),
      defense: card.defense,
      genesisDrawSite,
      gainsStealthAtEndOfTurn,
      lethal,
      manaCost: card.manaCost,
      ...(movementBonus ? { movementBonus } : {}),
      ...(provides ? { provides } : {}),
      ranged,
      stealth,
      strikesFirstWhileAttacking,
      summonToAnySite,
      ...(tapForMana ? { tapForMana } : {}),
      thresholds: card.thresholds,
      ward,
    };
  }
  throw new Error(`actual card ${card.stableId} lacks required supported facts`);
}

function buildManifest(
  input: Awaited<ReturnType<typeof readPrivateInputs>>,
  seed: number,
  scenario: 'air' | 'airborne' | 'combat' | 'earth' | 'earth-first-strike' | 'earth-ward' | 'fire' | 'movement-two' | 'stealth' | 'water' | 'water-stealth' = 'combat',
): Readonly<{ manifest: GameManifest; names: ReadonlyMap<string, string> }> {
  const avatar = input.cards.find(({ stableId }) => stableId === input.config.avatar.stableId);
  if (!avatar || avatar.cardType !== 'avatar') throw new Error('private scenario Avatar is missing');
  const sites = input.cards.filter((card) =>
    card.cardType === 'site' && card.rulesText.trim() === '' && card.life === null);
  const minions = input.cards.filter((card) =>
    card.cardType === 'minion'
      && card.rulesText.trim() === ''
      && card.attack !== null
      && card.defense !== null
      && card.manaCost !== null);
  const deck = (reverse: boolean, includeCharge: boolean): GameDeckSpec => {
    const chargeCopies = includeCharge
      ? input.format.copyLimits[input.chargeMinion.rarity!]
      : 0;
    const providerCopies = includeCharge
      ? input.format.copyLimits[input.providerMinion.rarity!]
      : 0;
    const lethalCopies = includeCharge
      ? input.format.copyLimits[input.lethalMinion.rarity!]
      : 0;
    const genesisCopies = includeCharge
      ? input.format.copyLimits[input.genesisMinion.rarity!]
      : 0;
    return {
    atlas: fillZone(sites, input.format.atlasMinimum, input.format, reverse),
    avatar: avatar.stableId,
    spellbook: [
      ...Array.from({ length: chargeCopies }, () => input.chargeMinion.stableId),
      ...Array.from({ length: providerCopies }, () => input.providerMinion.stableId),
      ...Array.from({ length: lethalCopies }, () => input.lethalMinion.stableId),
      ...Array.from({ length: genesisCopies }, () => input.genesisMinion.stableId),
      ...fillZone(
        minions,
        input.format.spellbookMinimum
          - chargeCopies
          - providerCopies
          - lethalCopies
          - genesisCopies,
        input.format,
        reverse,
      ),
    ],
    };
  };
  const elementalDeck = (
    element: GameElement,
    featuredMinions: readonly NormalizedCard[],
    featuredSites: readonly NormalizedCard[] = [],
  ): GameDeckSpec => {
    const featuredSiteCards = featuredSites.flatMap((card) =>
      Array.from({ length: input.format.copyLimits[card.rarity!] }, () => card.stableId));
    const featuredSiteIds = new Set(featuredSites.map(({ stableId }) => stableId));
    const elementSites = sites.filter((card) =>
      card.rarity && card.elements.includes(element) && !featuredSiteIds.has(card.stableId));
    const elementSiteCount = Math.min(
      input.format.atlasMinimum - featuredSiteCards.length,
      elementSites.reduce((total, card) => total + input.format.copyLimits[card.rarity!], 0),
    );
    const elementSiteIds = new Set(elementSites.map(({ stableId }) => stableId));
    const featuredMinionCards = featuredMinions.flatMap((card) =>
      Array.from({ length: input.format.copyLimits[card.rarity!] }, () => card.stableId));
    const featuredMinionIds = new Set(featuredMinions.map(({ stableId }) => stableId));
    return {
      atlas: [
        ...featuredSiteCards,
        ...fillZone(elementSites, elementSiteCount, input.format, false),
        ...fillZone(
          sites.filter(({ stableId }) =>
            !featuredSiteIds.has(stableId) && !elementSiteIds.has(stableId)),
          input.format.atlasMinimum - featuredSiteCards.length - elementSiteCount,
          input.format,
          false,
        ),
      ],
      avatar: avatar.stableId,
      spellbook: [
        ...featuredMinionCards,
        ...fillZone(
          minions.filter(({ stableId }) => !featuredMinionIds.has(stableId)),
          input.format.spellbookMinimum - featuredMinionCards.length,
          input.format,
          false,
        ),
      ],
    };
  };
  const earthDeck = elementalDeck('earth', [
    input.earthProviderMinion,
    input.manaMinion,
    input.genesisMinion,
    input.deathriteMinion,
    input.cannotDefendMinion,
    input.rangedMinion,
    input.wardMinion,
    input.firstStrikeMinion,
    input.firstStrikeTargetMinion,
  ], [input.ghostTownSite]);
  const airborneDeck = elementalDeck('air', [
    input.movementMinion,
    input.roamingMinion,
    input.airborneMinion,
    input.airborneTargetMinion,
    input.stealthMinion,
    input.stealthTargetMinion,
    input.movementTwoMinion,
  ]);
  const waterDeck = elementalDeck('water', [
    input.healingMinion,
    input.slyFox,
  ]);
  const decks = {
    north: scenario === 'airborne' || scenario === 'movement-two' || scenario === 'stealth'
      ? airborneDeck
      : scenario === 'earth' || scenario === 'earth-first-strike' || scenario === 'earth-ward'
      ? earthDeck
      : scenario === 'air'
        ? elementalDeck('air', [input.movementMinion, input.roamingMinion])
        : scenario === 'fire'
          ? elementalDeck('fire', [input.lumberingMinion, input.monstrousLion])
        : scenario === 'water' || scenario === 'water-stealth'
          ? waterDeck
          : deck(false, true),
    south: scenario === 'airborne' || scenario === 'movement-two' || scenario === 'stealth'
      ? airborneDeck
      : scenario === 'earth-first-strike' || scenario === 'earth-ward' ? earthDeck : deck(true, false),
  };
  const referenced = new Set([
    decks.north.avatar,
    ...decks.north.atlas,
    ...decks.north.spellbook,
    ...decks.south.atlas,
    ...decks.south.spellbook,
  ]);
  const selectedCards = input.cards.filter(({ stableId }) => referenced.has(stableId));
  const definitions = Object.fromEntries(selectedCards.map((card) => [
    card.stableId,
    gameDefinition(
      card,
      card.stableId === avatar.stableId && input.config.avatar.drawSpell,
      card.stableId === input.chargeMinion.stableId
        || card.stableId === input.monstrousLion.stableId,
      card.stableId === input.genesisMinion.stableId,
      card.stableId === input.lethalMinion.stableId,
      card.stableId === input.providerMinion.stableId
        ? 'fire'
        : card.stableId === input.earthProviderMinion.stableId
          ? 'earth'
          : undefined,
      card.stableId === input.manaMinion.stableId ? 2 : undefined,
      card.stableId === input.deathriteMinion.stableId,
      card.stableId === input.cannotDefendMinion.stableId,
      card.stableId === input.movementMinion.stableId
        ? 1
        : card.stableId === input.movementTwoMinion.stableId ? 2 : 0,
      card.stableId === input.healingMinion.stableId ? 3 : 0,
      card.stableId === input.ghostTownSite.stableId ? 1 : 0,
      card.stableId === input.roamingMinion.stableId,
      card.stableId === input.lumberingMinion.stableId,
      card.stableId === input.monstrousLion.stableId,
      card.stableId === input.rangedMinion.stableId,
      card.stableId === input.firstStrikeMinion.stableId,
      card.stableId === input.wardMinion.stableId,
      card.stableId === input.airborneMinion.stableId
        || card.stableId === input.movementTwoMinion.stableId,
      card.stableId === input.stealthMinion.stableId,
      card.stableId === input.slyFox.stableId,
    ),
  ]));
  return {
    manifest: createGameManifest({
      authority: {
        contentHash: input.authorityHash,
        mode: 'private-local',
        revisionId: input.config.revisionId,
      },
      cards: definitions,
      decks,
      firstSeat: 'north',
      seed,
    }),
    names: new Map(selectedCards.map(({ name, stableId }) => [stableId, name])),
  };
}

function action(session: GameSession, predicate: (candidate: GameLegalAction) => boolean): GameLegalAction {
  const found = legalGameActions(session.state, session.state.decisionSeat).find(predicate);
  if (!found) throw new Error(`actual-card scenario has no expected action in ${session.state.phase}`);
  return found;
}

function accept(session: GameSession, candidate: GameLegalAction): GameSession {
  const result = stepGame(session, candidate);
  if (!result.accepted) throw new Error(`actual-card scenario action rejected: ${result.reason.code}`);
  return result.session;
}

function openingPair(
  session: GameSession,
  seat: GameSeat,
  preferredCardId?: string,
  maximumMana = 1,
  excludedSiteInstanceId?: string,
): Readonly<{
  minionInstanceId: string;
  siteInstanceId: string;
}> | null {
  const player = session.state.players[seat];
  for (const site of player.hand.atlas) {
    if (site.instanceId === excludedSiteInstanceId) continue;
    const siteDefinition = session.state.cards[site.cardId];
    if (siteDefinition?.cardType !== 'site') continue;
    const affinity = { air: 0, earth: 0, fire: 0, water: 0 };
    siteDefinition.elements.forEach((element) => { affinity[element] += 1; });
    for (const minion of player.hand.spellbook) {
      if (preferredCardId && minion.cardId !== preferredCardId) continue;
      const definition = session.state.cards[minion.cardId];
      if (definition?.cardType !== 'minion' || definition.manaCost > maximumMana) continue;
      if ((['air', 'earth', 'fire', 'water'] as const)
        .every((element) => affinity[element] >= definition.thresholds[element])) {
        return { minionInstanceId: minion.instanceId, siteInstanceId: site.instanceId };
      }
    }
  }
  return null;
}

function availableMinionInstance(
  session: GameSession,
  seat: GameSeat,
  cardId: string,
  draws: number,
): string | undefined {
  const player = session.state.players[seat];
  return [...player.hand.spellbook, ...player.spellbook.slice(0, draws)]
    .find((card) => card.cardId === cardId)?.instanceId;
}

function openingSiteForMinion(
  session: GameSession,
  seat: GameSeat,
  cardId: string,
  excludedSiteInstanceId: string,
): string | undefined {
  const definition = session.state.cards[cardId];
  if (definition?.cardType !== 'minion') return undefined;
  return session.state.players[seat].hand.atlas.find((site) => {
    if (site.instanceId === excludedSiteInstanceId) return false;
    const siteDefinition = session.state.cards[site.cardId];
    if (siteDefinition?.cardType !== 'site') return false;
    const affinity = { air: 0, earth: 0, fire: 0, water: 0 };
    siteDefinition.elements.forEach((element) => { affinity[element] += 1; });
    return (['air', 'earth', 'fire', 'water'] as const)
      .every((element) => affinity[element] >= definition.thresholds[element]);
  })?.instanceId;
}

function earthOpponentOpening(session: GameSession): Readonly<{
  firstSiteInstanceId: string;
  minionInstanceId: string;
  secondSiteInstanceId: string;
}> | null {
  const player = session.state.players.south;
  for (const first of player.hand.atlas) {
    for (const second of player.hand.atlas) {
      if (first.instanceId === second.instanceId) continue;
      const affinity = { air: 0, earth: 0, fire: 0, water: 0 };
      for (const site of [first, second]) {
        const definition = session.state.cards[site.cardId];
        if (definition?.cardType !== 'site') continue;
        definition.elements.forEach((element) => { affinity[element] += 1; });
      }
      const minion = [...player.hand.spellbook, ...player.spellbook.slice(0, 1)].find((card) => {
        const definition = session.state.cards[card.cardId];
        return definition?.cardType === 'minion'
          && definition.attack >= 2
          && definition.manaCost <= 2
          && (['air', 'earth', 'fire', 'water'] as const)
            .every((element) => affinity[element] >= definition.thresholds[element]);
      });
      if (minion) {
        return {
          firstSiteInstanceId: first.instanceId,
          minionInstanceId: minion.instanceId,
          secondSiteInstanceId: second.instanceId,
        };
      }
    }
  }
  return null;
}

function findOpening(
  input: Awaited<ReturnType<typeof readPrivateInputs>>,
): Readonly<{
  manifest: GameManifest;
  names: ReadonlyMap<string, string>;
  north: NonNullable<ReturnType<typeof openingPair>>;
  northChargeInstanceId: string;
  northChargeSiteInstanceId: string;
  northGenesisInstanceId: string;
  northProviderInstanceId: string;
  seed: number;
  session: GameSession;
  south: NonNullable<ReturnType<typeof openingPair>>;
}> {
  const seed = input.config.seed;
  const built = buildManifest(input, seed);
  const session = createGameSession(built.manifest);
  const north = openingPair(session, 'north', input.lethalMinion.stableId);
  const northChargeInstanceId = availableMinionInstance(
    session,
    'north',
    input.chargeMinion.stableId,
    1,
  );
  const northChargeSiteInstanceId = north && openingSiteForMinion(
    session,
    'north',
    input.chargeMinion.stableId,
    north.siteInstanceId,
  );
  const northProviderInstanceId = availableMinionInstance(
    session,
    'north',
    input.providerMinion.stableId,
    2,
  );
  const northGenesisInstanceId = availableMinionInstance(
    session,
    'north',
    input.genesisMinion.stableId,
    3,
  );
  const south = openingPair(session, 'south');
  if (north
    && northChargeInstanceId
    && northChargeSiteInstanceId
    && northGenesisInstanceId
    && northProviderInstanceId
    && northGenesisInstanceId !== northProviderInstanceId
    && south) {
    return {
      ...built,
      north,
      northChargeInstanceId,
      northChargeSiteInstanceId,
      northGenesisInstanceId,
      northProviderInstanceId,
      seed,
      session,
      south,
    };
  }
  throw new Error(`private scenario seed ${seed} no longer produces its supported opening`);
}

function findEarthOpening(
  input: Awaited<ReturnType<typeof readPrivateInputs>>,
): Readonly<{
  deathriteInstanceId: string;
  genesisInstanceId: string;
  ghostTownSiteInstanceId: string;
  manaInstanceId: string;
  manifest: GameManifest;
  names: ReadonlyMap<string, string>;
  northSiteInstanceIds: readonly [string, string, string];
  payoffInstanceId: string;
  providerInstanceId: string;
  seed: number;
  session: GameSession;
  southFirstSiteInstanceId: string;
  southMinionInstanceId: string;
  southSecondSiteInstanceId: string;
}> {
  const seed = input.config.earthSeed;
  const built = buildManifest(input, seed, 'earth');
  const session = createGameSession(built.manifest);
  const northSites = session.state.players.north.hand.atlas.filter((site) => {
    const definition = session.state.cards[site.cardId];
    return definition?.cardType === 'site' && definition.elements.includes('earth');
  });
  const ghostTownSiteInstanceId = session.state.players.north.hand.atlas
    .find(({ cardId }) => cardId === input.ghostTownSite.stableId)?.instanceId;
  const providerInstanceId = availableMinionInstance(
    session,
    'north',
    input.earthProviderMinion.stableId,
    1,
  );
  const manaInstanceId = availableMinionInstance(session, 'north', input.manaMinion.stableId, 2);
  const payoffInstanceId = availableMinionInstance(
    session,
    'north',
    input.cannotDefendMinion.stableId,
    3,
  );
  const genesisInstanceId = availableMinionInstance(session, 'north', input.genesisMinion.stableId, 4);
  const deathriteInstanceId = availableMinionInstance(
    session,
    'north',
    input.deathriteMinion.stableId,
    4,
  );
  const south = earthOpponentOpening(session);
  if (northSites.length >= 2
    && ghostTownSiteInstanceId
    && providerInstanceId
    && manaInstanceId
    && payoffInstanceId
    && genesisInstanceId
    && deathriteInstanceId
    && south) {
    return {
      ...built,
      deathriteInstanceId,
      genesisInstanceId,
      ghostTownSiteInstanceId,
      manaInstanceId,
      northSiteInstanceIds: [
        northSites[0]!.instanceId,
        northSites[1]!.instanceId,
        ghostTownSiteInstanceId,
      ],
      payoffInstanceId,
      providerInstanceId,
      seed,
      session,
      southFirstSiteInstanceId: south.firstSiteInstanceId,
      southMinionInstanceId: south.minionInstanceId,
      southSecondSiteInstanceId: south.secondSiteInstanceId,
    };
  }
  throw new Error(`private Earth scenario seed ${seed} no longer produces its supported opening`);
}

function findEarthDuelOpening(
  input: Awaited<ReturnType<typeof readPrivateInputs>>,
  mode: 'first-strike' | 'ranged' | 'ward' = 'ranged',
): Readonly<{
  attackerInstanceId: string;
  manifest: GameManifest;
  names: ReadonlyMap<string, string>;
  northSiteInstanceIds: readonly [string, string, string];
  seed: number;
  session: GameSession;
  southFirstSiteInstanceId: string;
  southSecondSiteInstanceId: string;
  targetInstanceId: string;
}> {
  const seed = mode === 'ward'
    ? input.config.earthWardSeed
    : mode === 'first-strike'
      ? input.config.earthFirstStrikeSeed
      : input.config.earthRangedSeed;
  const built = buildManifest(
    input,
    seed,
    mode === 'ward' ? 'earth-ward' : mode === 'first-strike' ? 'earth-first-strike' : 'earth',
  );
  const session = createGameSession(built.manifest);
  const northSites = session.state.players.north.hand.atlas.filter((site) => {
    const definition = session.state.cards[site.cardId];
    return definition?.cardType === 'site' && definition.elements.includes('earth');
  });
  const attackerCardId = mode === 'first-strike'
    ? input.firstStrikeMinion.stableId
    : input.rangedMinion.stableId;
  const attackerInstanceId = availableMinionInstance(
    session,
    'north',
    attackerCardId,
    2,
  );
  const south = session.state.players.south;
  const requiredTargetId = mode === 'ward'
    ? input.wardMinion.stableId
    : mode === 'first-strike'
      ? input.firstStrikeTargetMinion.stableId
      : undefined;
  for (const first of south.hand.atlas) {
    for (const second of south.hand.atlas) {
      if (first.instanceId === second.instanceId) continue;
      const affinity = { air: 0, earth: 0, fire: 0, water: 0 };
      for (const site of [first, second]) {
        const definition = session.state.cards[site.cardId];
        if (definition?.cardType !== 'site') continue;
        definition.elements.forEach((element) => { affinity[element] += 1; });
      }
      const target = [...south.hand.spellbook, ...south.spellbook.slice(0, 2)].find((card) => {
        if (requiredTargetId && card.cardId !== requiredTargetId) return false;
        const definition = session.state.cards[card.cardId];
        return definition?.cardType === 'minion'
          && definition.defense <= 3
          && definition.manaCost <= 2
          && (['air', 'earth', 'fire', 'water'] as const)
            .every((element) => affinity[element] >= definition.thresholds[element]);
      });
      if (northSites.length >= 3 && attackerInstanceId && target) {
        return {
          ...built,
          attackerInstanceId,
          northSiteInstanceIds: [
            northSites[0]!.instanceId,
            northSites[1]!.instanceId,
            northSites[2]!.instanceId,
          ],
          seed,
          session,
          southFirstSiteInstanceId: first.instanceId,
          southSecondSiteInstanceId: second.instanceId,
          targetInstanceId: target.instanceId,
        };
      }
    }
  }
  throw new Error(`private Earth ${mode} scenario seed ${seed} no longer produces its supported opening`);
}

function findAirOpening(
  input: Awaited<ReturnType<typeof readPrivateInputs>>,
): Readonly<{
  attackerInstanceId: string;
  manifest: GameManifest;
  movementInstanceId: string;
  names: ReadonlyMap<string, string>;
  northSiteInstanceIds: readonly [string, string, string];
  seed: number;
  session: GameSession;
  southSiteInstanceId: string;
}> {
  const seed = input.config.airSeed;
  const built = buildManifest(input, seed, 'air');
  const session = createGameSession(built.manifest);
  const northSites = session.state.players.north.hand.atlas.filter((site) => {
    const definition = session.state.cards[site.cardId];
    return definition?.cardType === 'site' && definition.elements.includes('air');
  });
  const movementInstanceId = availableMinionInstance(
    session,
    'north',
    input.movementMinion.stableId,
    2,
  );
  const south = openingPair(session, 'south');
  const southCardId = south && session.state.players.south.hand.spellbook
    .find(({ instanceId }) => instanceId === south.minionInstanceId)?.cardId;
  const southDefinition = southCardId ? session.state.cards[southCardId] : undefined;
  if (northSites.length >= 3
    && movementInstanceId
    && south
    && southDefinition?.cardType === 'minion'
    && southDefinition.attack <= 2) {
    return {
      ...built,
      attackerInstanceId: south.minionInstanceId,
      movementInstanceId,
      northSiteInstanceIds: [
        northSites[0]!.instanceId,
        northSites[1]!.instanceId,
        northSites[2]!.instanceId,
      ],
      seed,
      session,
      southSiteInstanceId: south.siteInstanceId,
    };
  }
  throw new Error(`private Air scenario seed ${seed} no longer produces its supported opening`);
}

function findAirborneOpening(
  input: Awaited<ReturnType<typeof readPrivateInputs>>,
  mode: 'airborne' | 'movement-two' = 'airborne',
): Readonly<{
  airborneInstanceId: string;
  groundInstanceId: string;
  manifest: GameManifest;
  names: ReadonlyMap<string, string>;
  northSiteInstanceIds: readonly [string, string, string];
  seed: number;
  session: GameSession;
  southSiteInstanceIds: readonly [string, string, string];
}> {
  const movementTwo = mode === 'movement-two';
  const seed = movementTwo ? input.config.movementTwoSeed : input.config.airborneSeed;
  const built = buildManifest(input, seed, mode);
  const session = createGameSession(built.manifest);
  const sites = (seat: GameSeat) => session.state.players[seat].hand.atlas;
  const northSites = sites('north');
    const southSites = sites('south');
    const northSiteInstanceIds = northSites.map(({ instanceId }) => instanceId);
    const southSiteInstanceIds = southSites.map(({ instanceId }) => instanceId);
    const airborneInstanceId = availableMinionInstance(
      session,
      'north',
      movementTwo ? input.movementTwoMinion.stableId : input.airborneMinion.stableId,
      2,
    );
    const groundInstanceId = availableMinionInstance(
      session,
      'south',
      input.airborneTargetMinion.stableId,
      3,
    );
    if (northSiteInstanceIds.length === 3
      && southSiteInstanceIds.length === 3
      && northSites.some(({ cardId }) => {
        const definition = session.state.cards[cardId];
        return definition?.cardType === 'site' && definition.elements.includes('air');
      })
      && (!movementTwo || northSites.filter(({ cardId }) => {
        const definition = session.state.cards[cardId];
        return definition?.cardType === 'site' && definition.elements.includes('air');
      }).length >= 2)
      && southSites.some(({ cardId }) => {
        const definition = session.state.cards[cardId];
        return definition?.cardType === 'site' && definition.elements.includes('air');
      })
      && airborneInstanceId
      && groundInstanceId) {
      return {
        ...built,
        airborneInstanceId,
        groundInstanceId,
        northSiteInstanceIds: northSiteInstanceIds as [string, string, string],
        seed,
        session,
        southSiteInstanceIds: southSiteInstanceIds as [string, string, string],
      };
  }
  throw new Error(`private ${movementTwo ? 'Movement +2' : 'Airborne'} scenario seed ${seed} no longer produces its supported opening`);
}

function findStealthOpening(
  input: Awaited<ReturnType<typeof readPrivateInputs>>,
): Readonly<{
  groundInstanceId: string;
  manifest: GameManifest;
  names: ReadonlyMap<string, string>;
  northSiteInstanceIds: readonly [string, string, string];
  seed: number;
  session: GameSession;
  southSiteInstanceIds: readonly [string, string, string];
  stealthInstanceId: string;
}> {
  const seed = input.config.stealthSeed;
  const built = buildManifest(input, seed, 'stealth');
  const session = createGameSession(built.manifest);
  const northSites = session.state.players.north.hand.atlas;
  const southSites = session.state.players.south.hand.atlas;
  const airAffinity = (sites: typeof northSites): number => sites.reduce((total, { cardId }) => {
    const definition = session.state.cards[cardId];
    return total + (definition?.cardType === 'site' && definition.elements.includes('air') ? 1 : 0);
  }, 0);
  const stealthInstanceId = availableMinionInstance(
    session,
    'north',
    input.stealthMinion.stableId,
    2,
  );
  const groundInstanceId = availableMinionInstance(
    session,
    'south',
    input.stealthTargetMinion.stableId,
    2,
  );
  if (northSites.length === 3
    && southSites.length === 3
    && airAffinity(northSites) >= 2
    && airAffinity(southSites.slice(0, 2)) >= 1
    && stealthInstanceId
    && groundInstanceId) {
    return {
      ...built,
      groundInstanceId,
      northSiteInstanceIds: northSites.map(({ instanceId }) => instanceId) as [string, string, string],
      seed,
      session,
      southSiteInstanceIds: southSites.map(({ instanceId }) => instanceId) as [string, string, string],
      stealthInstanceId,
    };
  }
  throw new Error(`private Stealth scenario seed ${seed} no longer produces its supported opening`);
}

function findAirSummoningOpening(
  input: Awaited<ReturnType<typeof readPrivateInputs>>,
): Readonly<{
  manifest: GameManifest;
  names: ReadonlyMap<string, string>;
  northSiteInstanceIds: readonly [string, string, string, string, string];
  ordinaryInstanceId: string;
  roamingInstanceId: string;
  seed: number;
  session: GameSession;
  southSiteInstanceId: string;
}> {
  const seed = input.config.roamingSeed;
  const built = buildManifest(input, seed, 'air');
  const session = createGameSession(built.manifest);
  const north = session.state.players.north;
  const sites = [...north.hand.atlas, ...north.atlas.slice(0, 2)];
  const spells = [...north.hand.spellbook, ...north.spellbook.slice(0, 2)];
  const affinity = { air: 0, earth: 0, fire: 0, water: 0 };
  sites.forEach(({ cardId }) => {
    const definition = session.state.cards[cardId];
    if (definition?.cardType === 'site') {
      definition.elements.forEach((element) => { affinity[element] += 1; });
    }
  });
  const roamingInstanceId = spells
    .find(({ cardId }) => cardId === input.roamingMinion.stableId)?.instanceId;
  const ordinaryInstanceId = spells.find(({ cardId }) => {
    const definition = session.state.cards[cardId];
    return cardId !== input.movementMinion.stableId
      && cardId !== input.roamingMinion.stableId
      && definition?.cardType === 'minion'
      && definition.manaCost <= 5
      && (['air', 'earth', 'fire', 'water'] as const)
        .every((element) => affinity[element] >= definition.thresholds[element]);
  })?.instanceId;
  const southSiteInstanceId = session.state.players.south.hand.atlas[0]?.instanceId;
  if (sites.length === 5
    && affinity.air >= 1
    && roamingInstanceId
    && ordinaryInstanceId
    && southSiteInstanceId) {
    return {
      ...built,
      northSiteInstanceIds: sites.map(({ instanceId }) => instanceId) as [string, string, string, string, string],
      ordinaryInstanceId,
      roamingInstanceId,
      seed,
      session,
      southSiteInstanceId,
    };
  }
  throw new Error(`private Air summoning seed ${input.config.roamingSeed} no longer produces its supported opening`);
}

function findFireOpening(
  input: Awaited<ReturnType<typeof readPrivateInputs>>,
): Readonly<{
  attackerInstanceId: string;
  lionInstanceId: string;
  lumberingInstanceId: string;
  manifest: GameManifest;
  names: ReadonlyMap<string, string>;
  northSiteInstanceIds: readonly [string, string, string, string];
  seed: number;
  session: GameSession;
  southSiteInstanceIds: readonly [string, string];
}> {
  const seed = input.config.fireSeed;
  const built = buildManifest(input, seed, 'fire');
  const session = createGameSession(built.manifest);
  const northSites = [
    ...session.state.players.north.hand.atlas,
    ...session.state.players.north.atlas.slice(0, 1),
  ];
  const fireAffinity = northSites.reduce((total, { cardId }) => {
    const definition = session.state.cards[cardId];
    return total + (definition?.cardType === 'site' && definition.elements.includes('fire') ? 1 : 0);
  }, 0);
  const lumberingInstanceId = availableMinionInstance(
    session,
    'north',
    input.lumberingMinion.stableId,
    3,
  );
  const lionInstanceId = availableMinionInstance(
    session,
    'north',
    input.monstrousLion.stableId,
    2,
  );
  for (const first of session.state.players.south.hand.atlas) {
    const siteDefinition = session.state.cards[first.cardId];
    if (siteDefinition?.cardType !== 'site') continue;
    const affinity = { air: 0, earth: 0, fire: 0, water: 0 };
    siteDefinition.elements.forEach((element) => { affinity[element] += 1; });
    const attacker = session.state.players.south.hand.spellbook.find(({ cardId }) => {
      const definition = session.state.cards[cardId];
      return definition?.cardType === 'minion'
        && definition.manaCost <= 1
        && (['air', 'earth', 'fire', 'water'] as const)
          .every((element) => affinity[element] >= definition.thresholds[element]);
    });
    const second = session.state.players.south.hand.atlas
      .find(({ instanceId }) => instanceId !== first.instanceId);
    if (northSites.length === 4
      && fireAffinity >= 2
      && lionInstanceId
      && lumberingInstanceId
      && attacker
      && second) {
      return {
        ...built,
        attackerInstanceId: attacker.instanceId,
        lionInstanceId,
        lumberingInstanceId,
        northSiteInstanceIds: northSites.map(({ instanceId }) => instanceId) as [string, string, string, string],
        seed,
        session,
        southSiteInstanceIds: [first.instanceId, second.instanceId],
      };
    }
  }
  throw new Error(`private Fire scenario seed ${input.config.fireSeed} no longer produces its supported opening`);
}

function findWaterOpening(
  input: Awaited<ReturnType<typeof readPrivateInputs>>,
  scenario: 'water' | 'water-stealth' = 'water',
): Readonly<{
  attackerInstanceId: string;
  featuredInstanceId: string;
  manifest: GameManifest;
  names: ReadonlyMap<string, string>;
  northSiteInstanceIds: readonly [string, string];
  seed: number;
  session: GameSession;
  southSiteInstanceIds: readonly [string, string];
}> {
  const endTurnStealth = scenario === 'water-stealth';
  const seed = endTurnStealth ? input.config.slyFoxSeed : input.config.waterSeed;
  const built = buildManifest(input, seed, scenario);
  const session = createGameSession(built.manifest);
  const northSites = session.state.players.north.hand.atlas.filter((site) => {
    const definition = session.state.cards[site.cardId];
    return definition?.cardType === 'site' && definition.elements.includes('water');
  });
  const featuredInstanceId = availableMinionInstance(
    session,
    'north',
    endTurnStealth ? input.slyFox.stableId : input.healingMinion.stableId,
    1,
  );
  for (const first of session.state.players.south.hand.atlas) {
    const siteDefinition = session.state.cards[first.cardId];
    if (siteDefinition?.cardType !== 'site') continue;
    const affinity = { air: 0, earth: 0, fire: 0, water: 0 };
    siteDefinition.elements.forEach((element) => { affinity[element] += 1; });
    const attacker = session.state.players.south.hand.spellbook.find((card) => {
      const definition = session.state.cards[card.cardId];
      return definition?.cardType === 'minion'
        && definition.manaCost <= 1
        && definition.attack >= 2
        && definition.defense <= 2
        && (['air', 'earth', 'fire', 'water'] as const)
          .every((element) => affinity[element] >= definition.thresholds[element]);
    });
    const second = session.state.players.south.hand.atlas
      .find(({ instanceId }) => instanceId !== first.instanceId);
    if (northSites.length >= 2 && featuredInstanceId && attacker && second) {
      return {
        ...built,
        attackerInstanceId: attacker.instanceId,
        featuredInstanceId,
        northSiteInstanceIds: [northSites[0]!.instanceId, northSites[1]!.instanceId],
        seed,
        session,
        southSiteInstanceIds: [first.instanceId, second.instanceId],
      };
    }
  }
  throw new Error(`private Water ${endTurnStealth ? 'end-turn Stealth' : 'healing'} scenario seed ${seed} no longer produces its supported opening`);
}

function keep(session: GameSession): GameSession {
  return accept(session, action(session, ({ descriptor }) =>
    descriptor.kind === 'mulligan'
      && descriptor.atlasOrder.length === 0
      && descriptor.spellbookOrder.length === 0));
}

function deckList(deck: GameDeckSpec, names: ReadonlyMap<string, string>): DeckList {
  const summarize = (cards: readonly string[]): readonly Readonly<{ copies: number; name: string }>[] =>
    [...new Set(cards)].map((cardId) => ({
      copies: cards.filter((value) => value === cardId).length,
      name: names.get(cardId) ?? cardId,
    }));
  return {
    atlas: summarize(deck.atlas),
    avatar: names.get(deck.avatar) ?? deck.avatar,
    spellbook: summarize(deck.spellbook),
  };
}

function runEarthRamp(
  input: Awaited<ReturnType<typeof readPrivateInputs>>,
): PrivateGameCheck['earthRamp'] {
  const opening = findEarthOpening(input);
  let session = keep(opening.session);
  session = keep(session);
  session = accept(session, action(session, ({ descriptor }) =>
    descriptor.kind === 'play-site'
      && descriptor.cardInstanceId === opening.northSiteInstanceIds[0]));
  session = accept(session, action(session, ({ descriptor }) => descriptor.kind === 'end-turn'));

  session = accept(session, action(session, ({ descriptor }) =>
    descriptor.kind === 'draw' && descriptor.zone === 'spellbook'));
  session = accept(session, action(session, ({ descriptor }) =>
    descriptor.kind === 'play-site'
      && descriptor.cardInstanceId === opening.southFirstSiteInstanceId));
  session = accept(session, action(session, ({ descriptor }) => descriptor.kind === 'end-turn'));

  session = accept(session, action(session, ({ descriptor }) =>
    descriptor.kind === 'draw' && descriptor.zone === 'spellbook'));
  session = accept(session, action(session, ({ descriptor }) =>
    descriptor.kind === 'play-site'
      && descriptor.cardInstanceId === opening.northSiteInstanceIds[1]
      && descriptor.cell === 'C3'));
  const affinityBeforeProvider = observeGame(session.state, 'north').players.north.affinity.earth;
  session = accept(session, action(session, ({ descriptor }) =>
    descriptor.kind === 'summon-minion'
      && descriptor.cardInstanceId === opening.providerInstanceId
      && descriptor.cell === 'C4'));
  const affinityAdded =
    observeGame(session.state, 'north').players.north.affinity.earth === affinityBeforeProvider + 1;
  session = accept(session, action(session, ({ descriptor }) => descriptor.kind === 'end-turn'));

  session = accept(session, action(session, ({ descriptor }) =>
    descriptor.kind === 'draw' && descriptor.zone === 'spellbook'));
  session = accept(session, action(session, ({ descriptor }) =>
    descriptor.kind === 'play-site'
      && descriptor.cardInstanceId === opening.southSecondSiteInstanceId
      && descriptor.cell === 'B1'));
  session = accept(session, action(session, ({ descriptor }) =>
    descriptor.kind === 'summon-minion'
      && descriptor.cardInstanceId === opening.southMinionInstanceId
      && descriptor.cell === 'C1'));
  session = accept(session, action(session, ({ descriptor }) => descriptor.kind === 'end-turn'));

  session = accept(session, action(session, ({ descriptor }) =>
    descriptor.kind === 'draw' && descriptor.zone === 'spellbook'));
  const manaBeforeGhostTown = session.state.players.north.mana;
  session = accept(session, action(session, ({ descriptor }) =>
    descriptor.kind === 'play-site'
      && descriptor.cardInstanceId === opening.northSiteInstanceIds[2]
      && descriptor.cell === 'C2'));
  const ghostTownBonusMana =
    session.state.players.north.mana - manaBeforeGhostTown - 1;
  session = accept(session, action(session, ({ descriptor }) =>
    descriptor.kind === 'summon-minion'
      && descriptor.cardInstanceId === opening.manaInstanceId
      && descriptor.cell === 'C3'));
  const manaUnavailableWhileSick = !legalGameActions(session.state, 'north').some(({ descriptor }) =>
    descriptor.kind === 'activate-mana' && descriptor.unitInstanceId === opening.manaInstanceId);
  session = accept(session, action(session, ({ descriptor }) => descriptor.kind === 'end-turn'));
  const ghostTownUnusedManaExpired = session.state.players.north.mana === 0;

  session = accept(session, action(session, ({ descriptor }) =>
    descriptor.kind === 'draw' && descriptor.zone === 'spellbook'));
  session = accept(session, action(session, ({ descriptor }) =>
    descriptor.kind === 'move-and-attack'
      && descriptor.unitInstanceId === opening.southMinionInstanceId
      && descriptor.to.cell === 'C2'));
  session = accept(session, action(session, ({ descriptor }) => descriptor.kind === 'decline-attack'));
  session = accept(session, action(session, ({ descriptor }) => descriptor.kind === 'end-turn'));

  session = accept(session, action(session, ({ descriptor }) =>
    descriptor.kind === 'draw' && descriptor.zone === 'spellbook'));
  const manaBefore = session.state.players.north.mana;
  session = accept(session, action(session, ({ descriptor }) =>
    descriptor.kind === 'activate-mana' && descriptor.unitInstanceId === opening.manaInstanceId));
  const manaGained = session.state.players.north.mana - manaBefore;
  const payoffManaBefore = session.state.players.north.mana;
  session = accept(session, action(session, ({ descriptor }) =>
    descriptor.kind === 'summon-minion'
      && descriptor.cardInstanceId === opening.payoffInstanceId
      && descriptor.cell === 'C3'));
  const rampPaidFive = payoffManaBefore === 5 && session.state.players.north.mana === 0;
  session = accept(session, action(session, ({ descriptor }) => descriptor.kind === 'end-turn'));

  session = accept(session, action(session, ({ descriptor }) =>
    descriptor.kind === 'draw' && descriptor.zone === 'spellbook'));
  session = accept(session, action(session, ({ descriptor }) => descriptor.kind === 'end-turn'));

  session = accept(session, action(session, ({ descriptor }) =>
    descriptor.kind === 'draw' && descriptor.zone === 'spellbook'));
  const payoffCanMoveAndAttack = legalGameActions(session.state, 'north').some(({ descriptor }) =>
    descriptor.kind === 'move-and-attack' && descriptor.unitInstanceId === opening.payoffInstanceId);
  session = accept(session, action(session, ({ descriptor }) =>
    descriptor.kind === 'activate-mana' && descriptor.unitInstanceId === opening.manaInstanceId));
  const beforeGenesis = session.state.players.north;
  session = accept(session, action(session, ({ descriptor }) =>
    descriptor.kind === 'summon-minion'
      && descriptor.cardInstanceId === opening.genesisInstanceId
      && descriptor.cell === 'C4'));
  const afterGenesis = session.state.players.north;
  session = accept(session, action(session, ({ descriptor }) =>
    descriptor.kind === 'summon-minion'
      && descriptor.cardInstanceId === opening.deathriteInstanceId
      && descriptor.cell === 'C2'));
  session = accept(session, action(session, ({ descriptor }) => descriptor.kind === 'end-turn'));

  session = accept(session, action(session, ({ descriptor }) =>
    descriptor.kind === 'draw' && descriptor.zone === 'spellbook'));
  session = accept(session, action(session, ({ descriptor }) =>
    descriptor.kind === 'move-and-attack'
      && descriptor.unitInstanceId === opening.southMinionInstanceId
      && descriptor.to.cell === 'C2'));
  session = accept(session, action(session, ({ descriptor }) =>
    descriptor.kind === 'declare-attack'
      && descriptor.target.kind === 'minion'
      && descriptor.target.instanceId === opening.deathriteInstanceId));
  const movingDefendUnavailable = !legalGameActions(session.state, 'north').some(({ descriptor }) =>
    descriptor.kind === 'defend' && descriptor.unitInstanceId === opening.payoffInstanceId);
  const beforeDeathrite = session.state.players.north;
  session = accept(session, action(session, ({ descriptor }) =>
    descriptor.kind === 'close-defend' && descriptor.originalTargetParticipates));
  const afterDeathrite = session.state.players.north;
  const finalEvents = session.transcript.at(-1)?.events ?? [];
  const siteDrawIndex = finalEvents.findIndex(({ type }) => type === 'site-drawn');
  const cemeteryIndex = finalEvents.findIndex(({ type }) => type === 'minion-died');
  const deathriteSiteDrawnBeforeCemetery =
    afterDeathrite.atlas.length === beforeDeathrite.atlas.length - 1
    && afterDeathrite.hand.atlas.length === beforeDeathrite.hand.atlas.length + 1
    && afterDeathrite.cemetery.some(({ instanceId }) => instanceId === opening.deathriteInstanceId)
    && siteDrawIndex >= 0
    && siteDrawIndex < cemeteryIndex;

  return Object.freeze({
    acceptedActionCount: session.transcript.length,
    affinityAdded,
    deck: deckList(opening.manifest.decks.north, opening.names),
    deathriteMinion:
      opening.names.get(input.deathriteMinion.stableId) ?? input.deathriteMinion.stableId,
    deathriteSiteDrawnBeforeCemetery,
    genesisSiteDrawn:
      afterGenesis.atlas.length === beforeGenesis.atlas.length - 1
      && afterGenesis.hand.atlas.length === beforeGenesis.hand.atlas.length + 1,
    ghostTown: opening.names.get(input.ghostTownSite.stableId) ?? input.ghostTownSite.stableId,
    ghostTownBonusMana,
    ghostTownUnusedManaExpired,
    manaGained,
    manaMinion: opening.names.get(input.manaMinion.stableId) ?? input.manaMinion.stableId,
    manaUnavailableWhileSick,
    movingDefendUnavailable,
    payoffCanMoveAndAttack,
    rampPaidFive,
    rampPayoffMinion:
      opening.names.get(input.cannotDefendMinion.stableId) ?? input.cannotDefendMinion.stableId,
    replayVerified: verifyGameReplay(session),
    seed: opening.seed,
  });
}

function stageEarthDuel(opening: ReturnType<typeof findEarthDuelOpening>): GameSession {
  let session = keep(opening.session);
  session = keep(session);
  const take = (predicate: (candidate: GameLegalAction) => boolean): void => {
    session = accept(session, action(session, predicate));
  };

  take(({ descriptor }) =>
    descriptor.kind === 'play-site'
      && descriptor.cardInstanceId === opening.northSiteInstanceIds[0]);
  take(({ descriptor }) => descriptor.kind === 'end-turn');
  take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
  take(({ descriptor }) =>
    descriptor.kind === 'play-site'
      && descriptor.cardInstanceId === opening.southFirstSiteInstanceId);
  take(({ descriptor }) => descriptor.kind === 'end-turn');

  take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
  take(({ descriptor }) =>
    descriptor.kind === 'play-site'
      && descriptor.cardInstanceId === opening.northSiteInstanceIds[1]
      && descriptor.cell === 'C3');
  take(({ descriptor }) => descriptor.kind === 'end-turn');
  take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
  take(({ descriptor }) =>
    descriptor.kind === 'play-site'
      && descriptor.cardInstanceId === opening.southSecondSiteInstanceId
      && descriptor.cell === 'C2');
  take(({ descriptor }) =>
    descriptor.kind === 'summon-minion'
      && descriptor.cardInstanceId === opening.targetInstanceId
      && descriptor.cell === 'C2');
  take(({ descriptor }) => descriptor.kind === 'end-turn');

  take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
  take(({ descriptor }) =>
    descriptor.kind === 'play-site'
      && descriptor.cardInstanceId === opening.northSiteInstanceIds[2]
      && descriptor.cell === 'B4');
  take(({ descriptor }) =>
    descriptor.kind === 'summon-minion'
      && descriptor.cardInstanceId === opening.attackerInstanceId
      && descriptor.cell === 'C3');
  take(({ descriptor }) => descriptor.kind === 'end-turn');
  take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
  take(({ descriptor }) => descriptor.kind === 'end-turn');
  take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');

  return session;
}

function runEarthRanged(
  input: Awaited<ReturnType<typeof readPrivateInputs>>,
): PrivateGameCheck['earthRanged'] {
  const opening = findEarthDuelOpening(input);
  let session = stageEarthDuel(opening);

  const shot = action(session, ({ descriptor }) =>
    descriptor.kind === 'shoot-projectile'
      && descriptor.shooterInstanceId === opening.attackerInstanceId
      && descriptor.direction === 'south'
      && descriptor.hit?.instanceId === opening.targetInstanceId);
  const rangedOneStep = shot.descriptor.kind === 'shoot-projectile'
    && shot.descriptor.path.map(({ cell }) => cell).join(',') === 'C3,C2';
  session = accept(session, shot);
  const shooter = session.state.realm.units
    .find(({ instanceId }) => instanceId === opening.attackerInstanceId);

  return Object.freeze({
    acceptedActionCount: session.transcript.length,
    deck: deckList(opening.manifest.decks.north, opening.names),
    rangedMinion: opening.names.get(input.rangedMinion.stableId) ?? input.rangedMinion.stableId,
    rangedOneStep,
    rangedShooterStayedSafe:
      shooter?.location === 'C3' && shooter.tapped && shooter.damage === 0,
    rangedTargetDied: session.state.players.south.cemetery
      .some(({ instanceId }) => instanceId === opening.targetInstanceId),
    replayVerified: verifyGameReplay(session),
    seed: opening.seed,
  });
}

function runEarthFirstStrike(
  input: Awaited<ReturnType<typeof readPrivateInputs>>,
): PrivateGameCheck['earthFirstStrike'] {
  const opening = findEarthDuelOpening(input, 'first-strike');
  let session = stageEarthDuel(opening);
  const take = (predicate: (candidate: GameLegalAction) => boolean): void => {
    session = accept(session, action(session, predicate));
  };
  take(({ descriptor }) =>
    descriptor.kind === 'move-and-attack'
      && descriptor.unitInstanceId === opening.attackerInstanceId
      && descriptor.from.cell === 'C3'
      && descriptor.to.cell === 'C2');
  take(({ descriptor }) =>
    descriptor.kind === 'declare-attack'
      && descriptor.target.kind === 'minion'
      && descriptor.target.instanceId === opening.targetInstanceId);
  take(({ descriptor }) => descriptor.kind === 'close-defend' && descriptor.originalTargetParticipates);
  const attacker = session.state.realm.units
    .find(({ instanceId }) => instanceId === opening.attackerInstanceId);
  const fightEvents = session.transcript.at(-1)?.events ?? [];
  const targetDied = session.state.players.south.cemetery
    .some(({ instanceId }) => instanceId === opening.targetInstanceId);
  const targetDiedBeforeReturn = targetDied && !fightEvents.some(({ payload, type }) =>
    type === 'damage-dealt'
      && isJsonRecord(payload)
      && payload.instanceId === opening.attackerInstanceId
      && payload.amount !== 0);

  return Object.freeze({
    acceptedActionCount: session.transcript.length,
    attackerSurvivedUndamaged: attacker?.damage === 0 && attacker.location === 'C2' && attacker.tapped,
    deck: deckList(opening.manifest.decks.north, opening.names),
    firstStrikeMinion:
      opening.names.get(input.firstStrikeMinion.stableId) ?? input.firstStrikeMinion.stableId,
    replayVerified: verifyGameReplay(session),
    seed: opening.seed,
    targetDiedBeforeReturn,
    targetMinion:
      opening.names.get(input.firstStrikeTargetMinion.stableId) ?? input.firstStrikeTargetMinion.stableId,
  });
}

function runEarthWard(
  input: Awaited<ReturnType<typeof readPrivateInputs>>,
): PrivateGameCheck['earthWard'] {
  const opening = findEarthDuelOpening(input, 'ward');
  let session = stageEarthDuel(opening);
  const targetBefore = session.state.realm.units
    .find(({ instanceId }) => instanceId === opening.targetInstanceId);
  const firstShot = action(session, ({ descriptor }) =>
    descriptor.kind === 'shoot-projectile'
      && descriptor.shooterInstanceId === opening.attackerInstanceId
      && descriptor.direction === 'south'
      && descriptor.hit?.instanceId === opening.targetInstanceId);
  session = accept(session, firstShot);
  const firstShotEvents = session.transcript.at(-1)?.events ?? [];
  const targetAfter = session.state.realm.units
    .find(({ instanceId }) => instanceId === opening.targetInstanceId);
  const wardBroke = targetBefore?.warded === true
    && targetAfter?.warded === false
    && firstShotEvents.some(({ payload, type }) =>
      type === 'ward-broken'
        && isJsonRecord(payload)
        && payload.instanceId === opening.targetInstanceId);
  const wardPreventedDamage = firstShotEvents.some(({ payload, type }) =>
    type === 'damage-dealt'
      && isJsonRecord(payload)
      && payload.instanceId === opening.targetInstanceId
      && payload.amount === 0
      && payload.prevented === true);
  const wardTargetSurvived = targetAfter?.damage === 0
    && !session.state.players.south.cemetery
      .some(({ instanceId }) => instanceId === opening.targetInstanceId);

  const take = (predicate: (candidate: GameLegalAction) => boolean): void => {
    session = accept(session, action(session, predicate));
  };
  take(({ descriptor }) => descriptor.kind === 'end-turn');
  take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
  take(({ descriptor }) => descriptor.kind === 'end-turn');
  take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
  take(({ descriptor }) =>
    descriptor.kind === 'shoot-projectile'
      && descriptor.shooterInstanceId === opening.attackerInstanceId
      && descriptor.hit?.instanceId === opening.targetInstanceId);

  return Object.freeze({
    acceptedActionCount: session.transcript.length,
    deck: deckList(opening.manifest.decks.north, opening.names),
    rangedMinion: opening.names.get(input.rangedMinion.stableId) ?? input.rangedMinion.stableId,
    replayVerified: verifyGameReplay(session),
    seed: opening.seed,
    wardBroke,
    wardMinion: opening.names.get(input.wardMinion.stableId) ?? input.wardMinion.stableId,
    wardPreventedDamage,
    wardTargetDiedAfterSecondShot: session.state.players.south.cemetery
      .some(({ instanceId }) => instanceId === opening.targetInstanceId),
    wardTargetSurvived,
  });
}

function runAirMovement(
  input: Awaited<ReturnType<typeof readPrivateInputs>>,
): PrivateGameCheck['airMovement'] {
  const opening = findAirOpening(input);
  let session = keep(opening.session);
  session = keep(session);
  session = accept(session, action(session, ({ descriptor }) =>
    descriptor.kind === 'play-site'
      && descriptor.cardInstanceId === opening.northSiteInstanceIds[0]));
  session = accept(session, action(session, ({ descriptor }) => descriptor.kind === 'end-turn'));

  session = accept(session, action(session, ({ descriptor }) =>
    descriptor.kind === 'draw' && descriptor.zone === 'spellbook'));
  session = accept(session, action(session, ({ descriptor }) =>
    descriptor.kind === 'play-site' && descriptor.cardInstanceId === opening.southSiteInstanceId));
  session = accept(session, action(session, ({ descriptor }) =>
    descriptor.kind === 'summon-minion' && descriptor.cardInstanceId === opening.attackerInstanceId));
  session = accept(session, action(session, ({ descriptor }) => descriptor.kind === 'end-turn'));

  session = accept(session, action(session, ({ descriptor }) =>
    descriptor.kind === 'draw' && descriptor.zone === 'spellbook'));
  session = accept(session, action(session, ({ descriptor }) =>
    descriptor.kind === 'play-site'
      && descriptor.cardInstanceId === opening.northSiteInstanceIds[1]
      && descriptor.cell === 'C3'));
  session = accept(session, action(session, ({ descriptor }) => descriptor.kind === 'end-turn'));
  session = accept(session, action(session, ({ descriptor }) =>
    descriptor.kind === 'draw' && descriptor.zone === 'spellbook'));
  session = accept(session, action(session, ({ descriptor }) => descriptor.kind === 'end-turn'));

  session = accept(session, action(session, ({ descriptor }) =>
    descriptor.kind === 'draw' && descriptor.zone === 'spellbook'));
  session = accept(session, action(session, ({ descriptor }) =>
    descriptor.kind === 'play-site'
      && descriptor.cardInstanceId === opening.northSiteInstanceIds[2]
      && descriptor.cell === 'C2'));
  session = accept(session, action(session, ({ descriptor }) =>
    descriptor.kind === 'summon-minion'
      && descriptor.cardInstanceId === opening.movementInstanceId
      && descriptor.cell === 'C4'));
  session = accept(session, action(session, ({ descriptor }) => descriptor.kind === 'end-turn'));

  session = accept(session, action(session, ({ descriptor }) =>
    descriptor.kind === 'draw' && descriptor.zone === 'spellbook'));
  session = accept(session, action(session, ({ descriptor }) =>
    descriptor.kind === 'move-and-attack'
      && descriptor.unitInstanceId === opening.attackerInstanceId
      && descriptor.path.map(({ cell }) => cell).join(',') === 'C1,C2'));
  session = accept(session, action(session, ({ descriptor }) =>
    descriptor.kind === 'declare-attack'
      && descriptor.target.kind === 'site'
      && descriptor.target.instanceId === session.state.realm.sites.C2?.instanceId));
  const defend = action(session, ({ descriptor }) =>
    descriptor.kind === 'defend'
      && descriptor.unitInstanceId === opening.movementInstanceId
      && descriptor.path.map(({ cell }) => cell).join(',') === 'C4,C3,C2');
  const twoStepDefend = defend.descriptor.kind === 'defend' && defend.descriptor.path.length === 3;
  session = accept(session, defend);
  session = accept(session, action(session, ({ descriptor }) =>
    descriptor.kind === 'close-defend' && !descriptor.originalTargetParticipates));
  session = accept(session, action(session, ({ descriptor }) => descriptor.kind === 'end-turn'));

  session = accept(session, action(session, ({ descriptor }) =>
    descriptor.kind === 'draw' && descriptor.zone === 'spellbook'));
  const move = action(session, ({ descriptor }) =>
    descriptor.kind === 'move-and-attack'
      && descriptor.unitInstanceId === opening.movementInstanceId
      && descriptor.path.map(({ cell }) => cell).join(',') === 'C2,C3,C4');
  const twoStepMoveAndAttack = move.descriptor.kind === 'move-and-attack'
    && move.descriptor.path.length === 3;
  session = accept(session, move);
  session = accept(session, action(session, ({ descriptor }) => descriptor.kind === 'decline-attack'));

  return Object.freeze({
    acceptedActionCount: session.transcript.length,
    deck: deckList(opening.manifest.decks.north, opening.names),
    movementMinion:
      opening.names.get(input.movementMinion.stableId) ?? input.movementMinion.stableId,
    replayVerified: verifyGameReplay(session),
    seed: opening.seed,
    twoStepDefend,
    twoStepMoveAndAttack,
  });
}

function runAirborne(
  input: Awaited<ReturnType<typeof readPrivateInputs>>,
): PrivateGameCheck['airborne'] {
  const opening = findAirborneOpening(input);
  let session = keep(opening.session);
  session = keep(session);
  const take = (predicate: (candidate: GameLegalAction) => boolean): void => {
    session = accept(session, action(session, predicate));
  };

  take(({ descriptor }) =>
    descriptor.kind === 'play-site'
      && descriptor.cardInstanceId === opening.northSiteInstanceIds[0]);
  take(({ descriptor }) => descriptor.kind === 'end-turn');
  take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
  take(({ descriptor }) =>
    descriptor.kind === 'play-site'
      && descriptor.cardInstanceId === opening.southSiteInstanceIds[0]);
  take(({ descriptor }) => descriptor.kind === 'end-turn');
  take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
  take(({ descriptor }) =>
    descriptor.kind === 'play-site'
      && descriptor.cardInstanceId === opening.northSiteInstanceIds[1]
      && descriptor.cell === 'C3');
  take(({ descriptor }) => descriptor.kind === 'end-turn');
  take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
  take(({ descriptor }) =>
    descriptor.kind === 'play-site'
      && descriptor.cardInstanceId === opening.southSiteInstanceIds[1]
      && descriptor.cell === 'C2');
  take(({ descriptor }) => descriptor.kind === 'end-turn');
  take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
  take(({ descriptor }) =>
    descriptor.kind === 'play-site'
      && descriptor.cardInstanceId === opening.northSiteInstanceIds[2]
      && descriptor.cell === 'B3');
  take(({ descriptor }) =>
    descriptor.kind === 'summon-minion'
      && descriptor.cardInstanceId === opening.airborneInstanceId
      && descriptor.cell === 'C3');
  take(({ descriptor }) => descriptor.kind === 'end-turn');
  take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
  take(({ descriptor }) =>
    descriptor.kind === 'play-site'
      && descriptor.cardInstanceId === opening.southSiteInstanceIds[2]
      && descriptor.cell === 'B2');
  take(({ descriptor }) =>
    descriptor.kind === 'summon-minion'
      && descriptor.cardInstanceId === opening.groundInstanceId
      && descriptor.cell === 'B2');
  take(({ descriptor }) => descriptor.kind === 'end-turn');
  take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');

  const move = action(session, ({ descriptor }) =>
    descriptor.kind === 'move-and-attack'
      && descriptor.unitInstanceId === opening.airborneInstanceId
      && descriptor.path.map(({ cell }) => cell).join(',') === 'C3,B2');
  const diagonalMove = move.descriptor.kind === 'move-and-attack'
    && move.descriptor.path.length === 2;
  session = accept(session, move);
  const airborneCanAttackGround = legalGameActions(session.state, 'north').some(({ descriptor }) =>
    descriptor.kind === 'declare-attack'
      && descriptor.target.kind === 'minion'
      && descriptor.target.instanceId === opening.groundInstanceId);
  take(({ descriptor }) => descriptor.kind === 'decline-attack');
  const groundCannotIntercept = session.state.phase === 'main';
  take(({ descriptor }) => descriptor.kind === 'end-turn');
  take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
  take(({ descriptor }) =>
    descriptor.kind === 'move-and-attack'
      && descriptor.unitInstanceId === opening.groundInstanceId
      && descriptor.path.map(({ cell }) => cell).join(',') === 'B2');
  const groundCannotAttackAirborne = legalGameActions(session.state, 'south').every(({ descriptor }) =>
    descriptor.kind !== 'declare-attack'
      || descriptor.target.kind !== 'minion'
      || descriptor.target.instanceId !== opening.airborneInstanceId);

  return Object.freeze({
    acceptedActionCount: session.transcript.length,
    airborneCanAttackGround,
    airborneMinion: opening.names.get(input.airborneMinion.stableId) ?? input.airborneMinion.stableId,
    deck: deckList(opening.manifest.decks.north, opening.names),
    diagonalMove,
    groundCannotAttackAirborne,
    groundCannotIntercept,
    groundMinion:
      opening.names.get(input.airborneTargetMinion.stableId) ?? input.airborneTargetMinion.stableId,
    replayVerified: verifyGameReplay(session),
    seed: opening.seed,
  });
}

function runAirMovementTwo(
  input: Awaited<ReturnType<typeof readPrivateInputs>>,
): PrivateGameCheck['airMovementTwo'] {
  const opening = findAirborneOpening(input, 'movement-two');
  let session = keep(opening.session);
  session = keep(session);
  const take = (predicate: (candidate: GameLegalAction) => boolean): void => {
    session = accept(session, action(session, predicate));
  };

  take(({ descriptor }) =>
    descriptor.kind === 'play-site'
      && descriptor.cardInstanceId === opening.northSiteInstanceIds[0]);
  take(({ descriptor }) => descriptor.kind === 'end-turn');
  take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
  take(({ descriptor }) =>
    descriptor.kind === 'play-site'
      && descriptor.cardInstanceId === opening.southSiteInstanceIds[0]);
  take(({ descriptor }) => descriptor.kind === 'end-turn');
  take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
  take(({ descriptor }) =>
    descriptor.kind === 'play-site'
      && descriptor.cardInstanceId === opening.northSiteInstanceIds[1]
      && descriptor.cell === 'C3');
  take(({ descriptor }) => descriptor.kind === 'end-turn');
  take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
  take(({ descriptor }) =>
    descriptor.kind === 'play-site'
      && descriptor.cardInstanceId === opening.southSiteInstanceIds[1]
      && descriptor.cell === 'C2');
  take(({ descriptor }) => descriptor.kind === 'end-turn');
  take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
  take(({ descriptor }) =>
    descriptor.kind === 'play-site'
      && descriptor.cardInstanceId === opening.northSiteInstanceIds[2]
      && descriptor.cell === 'B3');
  take(({ descriptor }) =>
    descriptor.kind === 'summon-minion'
      && descriptor.cardInstanceId === opening.airborneInstanceId
      && descriptor.cell === 'C3');
  take(({ descriptor }) => descriptor.kind === 'end-turn');
  take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
  take(({ descriptor }) =>
    descriptor.kind === 'play-site'
      && descriptor.cardInstanceId === opening.southSiteInstanceIds[2]
      && descriptor.cell === 'B2');
  take(({ descriptor }) =>
    descriptor.kind === 'summon-minion'
      && descriptor.cardInstanceId === opening.groundInstanceId
      && descriptor.cell === 'B2');
  take(({ descriptor }) => descriptor.kind === 'end-turn');
  take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');

  const move = action(session, ({ descriptor }) =>
    descriptor.kind === 'move-and-attack'
      && descriptor.unitInstanceId === opening.airborneInstanceId
      && descriptor.path.map(({ cell }) => cell).join(',') === 'C3,C4,B3,B2');
  const threeStepAirbornePath = move.descriptor.kind === 'move-and-attack'
    && move.descriptor.path.length === 4;
  session = accept(session, move);
  const attackAvailableAfterThreeSteps = legalGameActions(session.state, 'north')
    .some(({ descriptor }) =>
      descriptor.kind === 'declare-attack'
        && descriptor.target.kind === 'minion'
        && descriptor.target.instanceId === opening.groundInstanceId);
  take(({ descriptor }) => descriptor.kind === 'decline-attack');

  return Object.freeze({
    acceptedActionCount: session.transcript.length,
    attackAvailableAfterThreeSteps,
    deck: deckList(opening.manifest.decks.north, opening.names),
    movementMinion:
      opening.names.get(input.movementTwoMinion.stableId) ?? input.movementTwoMinion.stableId,
    replayVerified: verifyGameReplay(session),
    seed: opening.seed,
    threeStepAirbornePath,
  });
}

function runStealth(
  input: Awaited<ReturnType<typeof readPrivateInputs>>,
): PrivateGameCheck['stealth'] {
  const opening = findStealthOpening(input);
  let session = keep(opening.session);
  session = keep(session);
  const take = (predicate: (candidate: GameLegalAction) => boolean): void => {
    session = accept(session, action(session, predicate));
  };

  take(({ descriptor }) =>
    descriptor.kind === 'play-site'
      && descriptor.cardInstanceId === opening.northSiteInstanceIds[0]);
  take(({ descriptor }) => descriptor.kind === 'end-turn');
  take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
  take(({ descriptor }) =>
    descriptor.kind === 'play-site'
      && descriptor.cardInstanceId === opening.southSiteInstanceIds[0]);
  take(({ descriptor }) => descriptor.kind === 'end-turn');
  take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
  take(({ descriptor }) =>
    descriptor.kind === 'play-site'
      && descriptor.cardInstanceId === opening.northSiteInstanceIds[1]
      && descriptor.cell === 'C3');
  take(({ descriptor }) => descriptor.kind === 'end-turn');
  take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
  take(({ descriptor }) =>
    descriptor.kind === 'play-site'
      && descriptor.cardInstanceId === opening.southSiteInstanceIds[1]
      && descriptor.cell === 'C2');
  take(({ descriptor }) =>
    descriptor.kind === 'summon-minion'
      && descriptor.cardInstanceId === opening.groundInstanceId
      && descriptor.cell === 'C2');
  take(({ descriptor }) => descriptor.kind === 'end-turn');
  take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
  take(({ descriptor }) =>
    descriptor.kind === 'play-site'
      && descriptor.cardInstanceId === opening.northSiteInstanceIds[2]
      && descriptor.cell === 'B3');
  take(({ descriptor }) =>
    descriptor.kind === 'summon-minion'
      && descriptor.cardInstanceId === opening.stealthInstanceId
      && descriptor.cell === 'C3');
  const enteredStealthed = session.state.realm.units
    .some(({ instanceId, stealthed }) => instanceId === opening.stealthInstanceId && stealthed);
  take(({ descriptor }) => descriptor.kind === 'end-turn');
  take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
  take(({ descriptor }) =>
    descriptor.kind === 'move-and-attack'
      && descriptor.unitInstanceId === opening.groundInstanceId
      && descriptor.path.map(({ cell }) => cell).join(',') === 'C2,C3');
  const groundTargets = legalGameActions(session.state, 'south');
  const protectedUnit = session.state.realm.units
    .find(({ instanceId }) => instanceId === opening.stealthInstanceId);
  const groundCouldNotAttack = protectedUnit?.location === 'C3'
    && protectedUnit.stealthed
    && groundTargets.some(({ descriptor }) =>
      descriptor.kind === 'declare-attack' && descriptor.target.kind === 'site')
    && groundTargets.every(({ descriptor }) =>
      descriptor.kind !== 'declare-attack'
        || descriptor.target.kind !== 'minion'
        || descriptor.target.instanceId !== opening.stealthInstanceId);
  take(({ descriptor }) => descriptor.kind === 'decline-attack');
  take(({ descriptor }) => descriptor.kind === 'close-intercept');
  take(({ descriptor }) => descriptor.kind === 'end-turn');
  take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
  take(({ descriptor }) =>
    descriptor.kind === 'move-and-attack'
      && descriptor.unitInstanceId === opening.stealthInstanceId
      && descriptor.path.map(({ cell }) => cell).join(',') === 'C3');
  take(({ descriptor }) =>
    descriptor.kind === 'declare-attack'
      && descriptor.target.kind === 'minion'
      && descriptor.target.instanceId === opening.groundInstanceId);
  const finalEvents = session.transcript.at(-1)?.events ?? [];
  const attackSkippedDefend = session.state.phase === 'main'
    && session.state.decisionSeat === 'north'
    && finalEvents.some(({ type }) => type === 'attack-declared')
    && finalEvents.every(({ type }) => type !== 'defend-window-closed');
  const strikeIndex = finalEvents.findIndex(({ type }) => type === 'strike-damage-allocated');
  const stealthLostIndex = finalEvents.findIndex(({ type }) => type === 'stealth-lost');
  const deathIndex = finalEvents.findIndex(({ type }) => type === 'minion-died');
  const stealthLostAfterAttack = session.state.realm.units.some(({ instanceId, stealthed }) =>
    instanceId === opening.stealthInstanceId && !stealthed)
    && finalEvents.some(({ payload, type }) =>
      type === 'stealth-lost'
        && isJsonRecord(payload)
        && payload.instanceId === opening.stealthInstanceId)
    && strikeIndex >= 0
    && strikeIndex < stealthLostIndex
    && stealthLostIndex < deathIndex;

  return Object.freeze({
    acceptedActionCount: session.transcript.length,
    attackSkippedDefend,
    deck: deckList(opening.manifest.decks.north, opening.names),
    enteredStealthed,
    groundCouldNotAttack,
    groundMinion:
      opening.names.get(input.stealthTargetMinion.stableId) ?? input.stealthTargetMinion.stableId,
    groundMinionDied: session.state.players.south.cemetery
      .some(({ instanceId }) => instanceId === opening.groundInstanceId),
    replayVerified: verifyGameReplay(session),
    seed: opening.seed,
    stealthLostAfterAttack,
    stealthMinion: opening.names.get(input.stealthMinion.stableId) ?? input.stealthMinion.stableId,
  });
}

function runAirSummoning(
  input: Awaited<ReturnType<typeof readPrivateInputs>>,
): PrivateGameCheck['airSummoning'] {
  const opening = findAirSummoningOpening(input);
  let session = keep(opening.session);
  session = keep(session);
  const take = (predicate: (candidate: GameLegalAction) => boolean): void => {
    session = accept(session, action(session, predicate));
  };
  const cells = ['C4', 'C3', 'B4', 'D4', 'B3'] as const;

  take(({ descriptor }) =>
    descriptor.kind === 'play-site'
      && descriptor.cardInstanceId === opening.northSiteInstanceIds[0]);
  take(({ descriptor }) => descriptor.kind === 'end-turn');
  take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
  take(({ descriptor }) =>
    descriptor.kind === 'play-site'
      && descriptor.cardInstanceId === opening.southSiteInstanceId);
  take(({ descriptor }) => descriptor.kind === 'end-turn');

  for (let index = 1; index < cells.length; index += 1) {
    take(({ descriptor }) =>
      descriptor.kind === 'draw'
        && descriptor.zone === (index < 3 ? 'spellbook' : 'atlas'));
    take(({ descriptor }) =>
      descriptor.kind === 'play-site'
        && descriptor.cardInstanceId === opening.northSiteInstanceIds[index]
        && descriptor.cell === cells[index]);
    if (index === cells.length - 1) break;
    take(({ descriptor }) => descriptor.kind === 'end-turn');
    take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
    take(({ descriptor }) => descriptor.kind === 'end-turn');
  }

  const actions = legalGameActions(session.state, 'north');
  const ordinaryActions = actions.filter(({ descriptor }) =>
    descriptor.kind === 'summon-minion'
      && descriptor.cardInstanceId === opening.ordinaryInstanceId);
  const ordinaryRestricted = ordinaryActions.some(({ descriptor }) =>
    descriptor.kind === 'summon-minion' && descriptor.cell === 'C4')
    && ordinaryActions.every(({ descriptor }) =>
      descriptor.kind !== 'summon-minion' || descriptor.cell !== 'C1');
  take(({ descriptor }) =>
    descriptor.kind === 'summon-minion'
      && descriptor.cardInstanceId === opening.roamingInstanceId
      && descriptor.cell === 'C1');
  const roamingUnit = session.state.realm.units
    .find(({ instanceId }) => instanceId === opening.roamingInstanceId);

  return Object.freeze({
    acceptedActionCount: session.transcript.length,
    deck: deckList(opening.manifest.decks.north, opening.names),
    ordinaryRestricted,
    replayVerified: verifyGameReplay(session),
    roamingMinion: opening.names.get(input.roamingMinion.stableId) ?? input.roamingMinion.stableId,
    seed: opening.seed,
    summonedAtEnemySite:
      roamingUnit?.location === 'C1' && session.state.realm.sites.C1?.controller === 'south',
  });
}

function runFireResponse(
  input: Awaited<ReturnType<typeof readPrivateInputs>>,
): PrivateGameCheck['fireResponse'] {
  const opening = findFireOpening(input);
  let session = keep(opening.session);
  session = keep(session);
  const take = (predicate: (candidate: GameLegalAction) => boolean): void => {
    session = accept(session, action(session, predicate));
  };

  take(({ descriptor }) =>
    descriptor.kind === 'play-site'
      && descriptor.cardInstanceId === opening.northSiteInstanceIds[0]);
  take(({ descriptor }) => descriptor.kind === 'end-turn');
  take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
  take(({ descriptor }) =>
    descriptor.kind === 'play-site'
      && descriptor.cardInstanceId === opening.southSiteInstanceIds[0]);
  take(({ descriptor }) =>
    descriptor.kind === 'summon-minion'
      && descriptor.cardInstanceId === opening.attackerInstanceId);
  take(({ descriptor }) => descriptor.kind === 'end-turn');

  take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
  take(({ descriptor }) =>
    descriptor.kind === 'play-site'
      && descriptor.cardInstanceId === opening.northSiteInstanceIds[1]
      && descriptor.cell === 'C3');
  take(({ descriptor }) => descriptor.kind === 'end-turn');
  take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
  take(({ descriptor }) =>
    descriptor.kind === 'play-site'
      && descriptor.cardInstanceId === opening.southSiteInstanceIds[1]
      && descriptor.cell === 'C2');
  take(({ descriptor }) =>
    descriptor.kind === 'move-and-attack'
      && descriptor.unitInstanceId === opening.attackerInstanceId
      && descriptor.to.cell === 'C2');
  take(({ descriptor }) => descriptor.kind === 'decline-attack');
  take(({ descriptor }) => descriptor.kind === 'end-turn');

  take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
  take(({ descriptor }) =>
    descriptor.kind === 'play-site'
      && descriptor.cardInstanceId === opening.northSiteInstanceIds[2]
      && descriptor.cell === 'B4');
  take(({ descriptor }) => descriptor.kind === 'end-turn');
  take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
  take(({ descriptor }) => descriptor.kind === 'end-turn');

  take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'atlas');
  take(({ descriptor }) =>
    descriptor.kind === 'play-site'
      && descriptor.cardInstanceId === opening.northSiteInstanceIds[3]
      && descriptor.cell === 'D4');
  take(({ descriptor }) =>
    descriptor.kind === 'summon-minion'
      && descriptor.cardInstanceId === opening.lionInstanceId
      && descriptor.cell === 'C3');
  const chargeMoveAndAttack = legalGameActions(session.state, 'north').some(({ descriptor }) =>
    descriptor.kind === 'move-and-attack'
      && descriptor.unitInstanceId === opening.lionInstanceId
      && descriptor.to.cell === 'C2');
  take(({ descriptor }) =>
    descriptor.kind === 'move-and-attack'
      && descriptor.unitInstanceId === opening.lionInstanceId
      && descriptor.to.cell === 'C2');
  const lionTargets = legalGameActions(session.state, 'north');
  const unitTargetAvailable = lionTargets.some(({ descriptor }) =>
    descriptor.kind === 'declare-attack'
      && descriptor.target.kind === 'minion'
      && descriptor.target.instanceId === opening.attackerInstanceId);
  const siteTargetUnavailable = lionTargets.every(({ descriptor }) =>
    descriptor.kind !== 'declare-attack' || descriptor.target.kind !== 'site');
  take(({ descriptor }) => descriptor.kind === 'decline-attack');
  take(({ descriptor }) => descriptor.kind === 'close-intercept');
  take(({ descriptor }) => descriptor.kind === 'end-turn');
  take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
  take(({ descriptor }) => descriptor.kind === 'end-turn');
  take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
  take(({ descriptor }) =>
    descriptor.kind === 'summon-minion'
      && descriptor.cardInstanceId === opening.lumberingInstanceId
      && descriptor.cell === 'C3');
  take(({ descriptor }) => descriptor.kind === 'end-turn');

  take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
  take(({ descriptor }) =>
    descriptor.kind === 'move-and-attack'
      && descriptor.unitInstanceId === opening.attackerInstanceId
      && descriptor.to.cell === 'C3');
  take(({ descriptor }) => descriptor.kind === 'decline-attack');
  const interceptUnavailable = legalGameActions(session.state, 'north').every(({ descriptor }) =>
    descriptor.kind !== 'intercept' || descriptor.unitInstanceId !== opening.lumberingInstanceId);
  if (session.state.phase === 'intercept') {
    take(({ descriptor }) => descriptor.kind === 'close-intercept');
  }
  take(({ descriptor }) => descriptor.kind === 'end-turn');
  take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
  take(({ descriptor }) => descriptor.kind === 'end-turn');

  take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
  take(({ descriptor }) =>
    descriptor.kind === 'move-and-attack'
      && descriptor.unitInstanceId === opening.attackerInstanceId
      && descriptor.to.cell === 'C3');
  take(({ descriptor }) =>
    descriptor.kind === 'declare-attack'
      && descriptor.target.kind === 'site'
      && descriptor.target.instanceId === session.state.realm.sites.C3?.instanceId);
  const defendUnavailable = legalGameActions(session.state, 'north').every(({ descriptor }) =>
    descriptor.kind !== 'defend' || descriptor.unitInstanceId !== opening.lumberingInstanceId);
  take(({ descriptor }) =>
    descriptor.kind === 'close-defend' && !descriptor.originalTargetParticipates);

  return Object.freeze({
    acceptedActionCount: session.transcript.length,
    chargeMoveAndAttack,
    deck: deckList(opening.manifest.decks.north, opening.names),
    defendUnavailable,
    interceptUnavailable,
    lumberingGiant:
      opening.names.get(input.lumberingMinion.stableId) ?? input.lumberingMinion.stableId,
    monstrousLion:
      opening.names.get(input.monstrousLion.stableId) ?? input.monstrousLion.stableId,
    replayVerified: verifyGameReplay(session),
    seed: opening.seed,
    siteTargetUnavailable,
    unitTargetAvailable,
  });
}

function runWaterEndTurnStealth(
  input: Awaited<ReturnType<typeof readPrivateInputs>>,
): PrivateGameCheck['waterEndTurnStealth'] {
  const opening = findWaterOpening(input, 'water-stealth');
  let session = keep(opening.session);
  session = keep(session);
  const take = (predicate: (candidate: GameLegalAction) => boolean): void => {
    session = accept(session, action(session, predicate));
  };

  take(({ descriptor }) =>
    descriptor.kind === 'play-site'
      && descriptor.cardInstanceId === opening.northSiteInstanceIds[0]);
  take(({ descriptor }) => descriptor.kind === 'end-turn');
  take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
  take(({ descriptor }) =>
    descriptor.kind === 'play-site'
      && descriptor.cardInstanceId === opening.southSiteInstanceIds[0]);
  take(({ descriptor }) =>
    descriptor.kind === 'summon-minion'
      && descriptor.cardInstanceId === opening.attackerInstanceId);
  take(({ descriptor }) => descriptor.kind === 'end-turn');

  take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
  take(({ descriptor }) =>
    descriptor.kind === 'play-site'
      && descriptor.cardInstanceId === opening.northSiteInstanceIds[1]
      && descriptor.cell === 'C3');
  take(({ descriptor }) =>
    descriptor.kind === 'summon-minion'
      && descriptor.cardInstanceId === opening.featuredInstanceId
      && descriptor.cell === 'C3');
  const summonedUnstealthed = session.state.realm.units.some(({ instanceId, stealthed }) =>
    instanceId === opening.featuredInstanceId && !stealthed);
  take(({ descriptor }) => descriptor.kind === 'end-turn');
  const northEndEvents = session.transcript.at(-1)?.events ?? [];
  const stealthGainedIndex = northEndEvents.findIndex(({ payload, type }) =>
    type === 'stealth-gained'
      && isJsonRecord(payload)
      && payload.instanceId === opening.featuredInstanceId);
  const turnEndedIndex = northEndEvents.findIndex(({ type }) => type === 'turn-ended');
  const gainedStealthAtEndOfTurn = session.state.realm.units.some(({ instanceId, stealthed }) =>
    instanceId === opening.featuredInstanceId && stealthed)
    && stealthGainedIndex >= 0
    && stealthGainedIndex < turnEndedIndex;

  take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
  take(({ descriptor }) =>
    descriptor.kind === 'play-site'
      && descriptor.cardInstanceId === opening.southSiteInstanceIds[1]
      && descriptor.cell === 'C2');
  take(({ descriptor }) =>
    descriptor.kind === 'move-and-attack'
      && descriptor.unitInstanceId === opening.attackerInstanceId
      && descriptor.path.map(({ cell }) => cell).join(',') === 'C1,C2');
  take(({ descriptor }) => descriptor.kind === 'decline-attack');
  take(({ descriptor }) => descriptor.kind === 'end-turn');

  take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
  take(({ descriptor }) => descriptor.kind === 'end-turn');
  take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
  const readyAttacker = session.state.realm.units.find(({ instanceId }) =>
    instanceId === opening.attackerInstanceId);
  const attackerWasReady = readyAttacker?.location === 'C2'
    && !readyAttacker.tapped
    && !readyAttacker.summoningSickness;
  take(({ descriptor }) =>
    descriptor.kind === 'move-and-attack'
      && descriptor.unitInstanceId === opening.attackerInstanceId
      && descriptor.path.map(({ cell }) => cell).join(',') === 'C2,C3');
  const attackTargets = legalGameActions(session.state, 'south');
  const protectedUnit = session.state.realm.units.find(({ instanceId }) =>
    instanceId === opening.featuredInstanceId);
  const movedAttacker = session.state.realm.units.find(({ instanceId }) =>
    instanceId === opening.attackerInstanceId);
  const coLocatedReadyAttacker = attackerWasReady === true
    && protectedUnit?.location === 'C3'
    && movedAttacker?.location === 'C3';
  const attackSiteAvailable = attackTargets.some(({ descriptor }) =>
    descriptor.kind === 'declare-attack'
      && descriptor.target.kind === 'site'
      && descriptor.target.instanceId === session.state.realm.sites.C3?.instanceId);
  const slyFoxAttackUnavailable = protectedUnit?.stealthed === true
    && attackTargets.every(({ descriptor }) =>
      descriptor.kind !== 'declare-attack'
        || descriptor.target.kind !== 'minion'
        || descriptor.target.instanceId !== opening.featuredInstanceId);
  take(({ descriptor }) => descriptor.kind === 'decline-attack');
  take(({ descriptor }) => descriptor.kind === 'close-intercept');

  return Object.freeze({
    acceptedActionCount: session.transcript.length,
    attackSiteAvailable,
    coLocatedReadyAttacker,
    deck: deckList(opening.manifest.decks.north, opening.names),
    gainedStealthAtEndOfTurn,
    replayVerified: verifyGameReplay(session),
    seed: opening.seed,
    slyFox: opening.names.get(input.slyFox.stableId) ?? input.slyFox.stableId,
    slyFoxAttackUnavailable,
    summonedUnstealthed,
  });
}

function runWaterHealing(
  input: Awaited<ReturnType<typeof readPrivateInputs>>,
): PrivateGameCheck['waterHealing'] {
  const opening = findWaterOpening(input);
  let session = keep(opening.session);
  session = keep(session);
  const take = (predicate: (candidate: GameLegalAction) => boolean): void => {
    session = accept(session, action(session, predicate));
  };

  take(({ descriptor }) =>
    descriptor.kind === 'play-site'
      && descriptor.cardInstanceId === opening.northSiteInstanceIds[0]);
  take(({ descriptor }) => descriptor.kind === 'end-turn');
  take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
  take(({ descriptor }) =>
    descriptor.kind === 'play-site'
      && descriptor.cardInstanceId === opening.southSiteInstanceIds[0]);
  take(({ descriptor }) =>
    descriptor.kind === 'summon-minion'
      && descriptor.cardInstanceId === opening.attackerInstanceId);
  take(({ descriptor }) => descriptor.kind === 'end-turn');

  take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
  take(({ descriptor }) =>
    descriptor.kind === 'play-site'
      && descriptor.cardInstanceId === opening.northSiteInstanceIds[1]
      && descriptor.cell === 'C3');
  take(({ descriptor }) =>
    descriptor.kind === 'summon-minion'
      && descriptor.cardInstanceId === opening.featuredInstanceId
      && descriptor.cell === 'C3');
  take(({ descriptor }) => descriptor.kind === 'end-turn');
  take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
  take(({ descriptor }) =>
    descriptor.kind === 'play-site'
      && descriptor.cardInstanceId === opening.southSiteInstanceIds[1]
      && descriptor.cell === 'C2');
  take(({ descriptor }) =>
    descriptor.kind === 'move-and-attack'
      && descriptor.unitInstanceId === opening.attackerInstanceId
      && descriptor.to.cell === 'C2');
  take(({ descriptor }) => descriptor.kind === 'decline-attack');
  take(({ descriptor }) => descriptor.kind === 'end-turn');

  take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
  take(({ descriptor }) => descriptor.kind === 'end-turn');
  for (let strike = 0; strike < 2; strike += 1) {
    take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
    take(({ descriptor }) =>
      descriptor.kind === 'move-and-attack'
        && descriptor.unitInstanceId === opening.attackerInstanceId
        && descriptor.to.cell === 'C3');
    take(({ descriptor }) =>
      descriptor.kind === 'declare-attack'
        && descriptor.target.kind === 'site'
        && descriptor.target.instanceId === session.state.realm.sites.C3?.instanceId);
    take(({ descriptor }) =>
      descriptor.kind === 'close-defend' && !descriptor.originalTargetParticipates);
    take(({ descriptor }) => descriptor.kind === 'end-turn');
    take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
    if (strike === 0) take(({ descriptor }) => descriptor.kind === 'end-turn');
  }

  const lifeBeforeHealing = session.state.players.north.avatar.life;
  take(({ descriptor }) =>
    descriptor.kind === 'move-and-attack'
      && descriptor.unitInstanceId === opening.featuredInstanceId
      && descriptor.to.cell === 'C3');
  take(({ descriptor }) =>
    descriptor.kind === 'declare-attack'
      && descriptor.target.kind === 'minion'
      && descriptor.target.instanceId === opening.attackerInstanceId);
  take(({ descriptor }) =>
    descriptor.kind === 'close-defend' && descriptor.originalTargetParticipates);
  const finalEvents = session.transcript.at(-1)?.events ?? [];
  const healIndex = finalEvents.findIndex(({ type }) => type === 'avatar-healed');
  const cemeteryIndex = finalEvents.findIndex(({ payload, type }) =>
    type === 'minion-died'
      && isJsonRecord(payload)
      && payload.instanceId === opening.featuredInstanceId);

  return Object.freeze({
    acceptedActionCount: session.transcript.length,
    deck: deckList(opening.manifest.decks.north, opening.names),
    healed: session.state.players.north.avatar.life - lifeBeforeHealing,
    healedBeforeCemetery: healIndex >= 0 && healIndex < cemeteryIndex,
    healingMinionDied: session.state.players.north.cemetery
      .some(({ instanceId }) => instanceId === opening.featuredInstanceId),
    healingMinion:
      opening.names.get(input.healingMinion.stableId) ?? input.healingMinion.stableId,
    opponentMinionDied: session.state.players.south.cemetery
      .some(({ instanceId }) => instanceId === opening.attackerInstanceId),
    replayVerified: verifyGameReplay(session),
    seed: opening.seed,
  });
}

export async function runPrivateGameCheck(path = DEFAULT_SCENARIO): Promise<PrivateGameCheck> {
  const input = await readPrivateInputs(path);
  const airborne = runAirborne(input);
  const airMovement = runAirMovement(input);
  const airMovementTwo = runAirMovementTwo(input);
  const airSummoning = runAirSummoning(input);
  const earthFirstStrike = runEarthFirstStrike(input);
  const earthRamp = runEarthRamp(input);
  const earthRanged = runEarthRanged(input);
  const earthWard = runEarthWard(input);
  const fireResponse = runFireResponse(input);
  const stealth = runStealth(input);
  const waterEndTurnStealth = runWaterEndTurnStealth(input);
  const waterHealing = runWaterHealing(input);
  const opening = findOpening(input);
  let session = keep(opening.session);
  session = keep(session);
  session = accept(session, action(session, ({ descriptor }) =>
    descriptor.kind === 'play-site' && descriptor.cardInstanceId === opening.north.siteInstanceId));
  session = accept(session, action(session, ({ descriptor }) =>
    descriptor.kind === 'summon-minion' && descriptor.cardInstanceId === opening.north.minionInstanceId));
  session = accept(session, action(session, ({ descriptor }) => descriptor.kind === 'end-turn'));

  session = accept(session, action(session, ({ descriptor }) =>
    descriptor.kind === 'draw' && descriptor.zone === 'spellbook'));
  session = accept(session, action(session, ({ descriptor }) =>
    descriptor.kind === 'play-site' && descriptor.cardInstanceId === opening.south.siteInstanceId));
  session = accept(session, action(session, ({ descriptor }) =>
    descriptor.kind === 'summon-minion' && descriptor.cardInstanceId === opening.south.minionInstanceId));
  session = accept(session, action(session, ({ descriptor }) => descriptor.kind === 'end-turn'));

  session = accept(session, action(session, ({ descriptor }) =>
    descriptor.kind === 'draw' && descriptor.zone === 'spellbook'));
  session = accept(session, action(session, ({ descriptor }) =>
    descriptor.kind === 'play-site'
      && descriptor.cardInstanceId === opening.northChargeSiteInstanceId
      && descriptor.cell === 'C3'));
  session = accept(session, action(session, ({ descriptor }) =>
    descriptor.kind === 'summon-minion'
      && descriptor.cardInstanceId === opening.northChargeInstanceId
      && descriptor.cell === 'C3'));
  const chargeActivatedOnSummon = legalGameActions(session.state, 'north').some(({ descriptor }) =>
    descriptor.kind === 'move-and-attack'
      && descriptor.unitInstanceId === opening.northChargeInstanceId
      && descriptor.to.cell === 'C3');
  session = accept(session, action(session, ({ descriptor }) =>
    descriptor.kind === 'move-and-attack'
      && descriptor.unitInstanceId === opening.northChargeInstanceId
      && descriptor.to.cell === 'C3'));
  session = accept(session, action(session, ({ descriptor }) => descriptor.kind === 'decline-attack'));
  session = accept(session, action(session, ({ descriptor }) =>
    descriptor.kind === 'move-and-attack'
      && descriptor.unitInstanceId === opening.north.minionInstanceId
      && descriptor.to.cell === 'C3'));
  session = accept(session, action(session, ({ descriptor }) => descriptor.kind === 'decline-attack'));
  session = accept(session, action(session, ({ descriptor }) => descriptor.kind === 'end-turn'));

  session = accept(session, action(session, ({ descriptor }) =>
    descriptor.kind === 'draw' && descriptor.zone === 'spellbook'));
  session = accept(session, action(session, ({ descriptor }) =>
    descriptor.kind === 'play-site' && descriptor.cell === 'C2'));
  session = accept(session, action(session, ({ descriptor }) =>
    descriptor.kind === 'move-and-attack'
      && descriptor.unitInstanceId === opening.south.minionInstanceId
      && descriptor.to.cell === 'C2'));
  session = accept(session, action(session, ({ descriptor }) => descriptor.kind === 'decline-attack'));
  session = accept(session, action(session, ({ descriptor }) => descriptor.kind === 'end-turn'));

  session = accept(session, action(session, ({ descriptor }) =>
    descriptor.kind === 'draw' && descriptor.zone === 'spellbook'));
  const affinityBeforeProvider = observeGame(session.state, 'north').players.north.affinity.fire;
  session = accept(session, action(session, ({ descriptor }) =>
    descriptor.kind === 'summon-minion'
      && descriptor.cardInstanceId === opening.northProviderInstanceId
      && descriptor.cell === 'C3'));
  const providerAffinityAdded =
    observeGame(session.state, 'north').players.north.affinity.fire === affinityBeforeProvider + 1;
  const beforeAvatarDraw = session.state.players.north;
  session = accept(session, action(session, ({ descriptor }) => descriptor.kind === 'draw-spell'));
  const afterAvatarDraw = session.state.players.north;
  const avatarSpellDrawn = afterAvatarDraw.avatar.tapped
    && afterAvatarDraw.spellbook.length === beforeAvatarDraw.spellbook.length - 1
    && afterAvatarDraw.hand.spellbook.length === beforeAvatarDraw.hand.spellbook.length + 1;
  if (!avatarSpellDrawn) throw new Error('actual Avatar draw-spell ability did not resolve');
  session = accept(session, action(session, ({ descriptor }) =>
    descriptor.kind === 'move-and-attack'
      && descriptor.unitInstanceId === opening.north.minionInstanceId
      && descriptor.to.cell === 'C2'));
  session = accept(session, action(session, ({ descriptor }) =>
    descriptor.kind === 'declare-attack'
      && descriptor.target.kind === 'minion'
      && descriptor.target.instanceId === opening.south.minionInstanceId));
  session = accept(session, action(session, ({ descriptor }) =>
    descriptor.kind === 'close-defend' && descriptor.originalTargetParticipates));

  session = accept(session, action(session, ({ descriptor }) => descriptor.kind === 'end-turn'));
  session = accept(session, action(session, ({ descriptor }) =>
    descriptor.kind === 'draw' && descriptor.zone === 'spellbook'));
  session = accept(session, action(session, ({ descriptor }) => descriptor.kind === 'end-turn'));
  session = accept(session, action(session, ({ descriptor }) =>
    descriptor.kind === 'draw' && descriptor.zone === 'spellbook'));
  session = accept(session, action(session, ({ descriptor }) => descriptor.kind === 'play-site'));
  const beforeGenesis = session.state.players.north;
  session = accept(session, action(session, ({ descriptor }) =>
    descriptor.kind === 'summon-minion'
      && descriptor.cardInstanceId === opening.northGenesisInstanceId));
  const afterGenesis = session.state.players.north;
  const genesisSiteDrawn =
    afterGenesis.atlas.length === beforeGenesis.atlas.length - 1
    && afterGenesis.hand.atlas.length === beforeGenesis.hand.atlas.length + 1;

  const northCardId = opening.session.state.players.north.hand.spellbook
    .find(({ instanceId }) => instanceId === opening.north.minionInstanceId)!.cardId;
  const southCardId = opening.session.state.players.south.hand.spellbook
    .find(({ instanceId }) => instanceId === opening.south.minionInstanceId)!.cardId;
  const northDefinition = opening.session.state.cards[northCardId];
  const southDefinition = opening.session.state.cards[southCardId];
  return Object.freeze({
    acceptedActionCount: session.transcript.length,
    airborne,
    airMovement,
    airMovementTwo,
    airSummoning,
    authorityHash: input.authorityHash,
    avatarSpellDrawn,
    charge: {
      activatedOnSummon: chargeActivatedOnSummon,
      minion: opening.names.get(input.chargeMinion.stableId) ?? input.chargeMinion.stableId,
    },
    classification: 'private-local_actual-cards_unranked-partial-rules',
    combat: {
      northMinion: opening.names.get(northCardId) ?? northCardId,
      northMinionDied: session.state.players.north.cemetery
        .some(({ instanceId }) => instanceId === opening.north.minionInstanceId),
      southMinion: opening.names.get(southCardId) ?? southCardId,
      southMinionDied: session.state.players.south.cemetery
        .some(({ instanceId }) => instanceId === opening.south.minionInstanceId),
    },
    decks: {
      north: deckList(opening.manifest.decks.north, opening.names),
      south: deckList(opening.manifest.decks.south, opening.names),
    },
    earthRamp,
    earthFirstStrike,
    earthRanged,
    earthWard,
    fireResponse,
    finalStateHash: hashGameState(session.state),
    formatStableId: input.formatStableId,
    genesis: {
      minion: opening.names.get(input.genesisMinion.stableId) ?? input.genesisMinion.stableId,
      siteDrawn: genesisSiteDrawn,
    },
    lethal: {
      minion: opening.names.get(input.lethalMinion.stableId) ?? input.lethalMinion.stableId,
      tougherMinionKilled:
        northDefinition?.cardType === 'minion'
        && northDefinition.lethal === true
        && southDefinition?.cardType === 'minion'
        && northDefinition.attack < southDefinition.defense
        && session.state.players.south.cemetery
          .some(({ instanceId }) => instanceId === opening.south.minionInstanceId),
    },
    provider: {
      affinityAdded: providerAffinityAdded,
      minion: opening.names.get(input.providerMinion.stableId) ?? input.providerMinion.stableId,
    },
    replayVerified: verifyGameReplay(session),
    revisionId: input.config.revisionId,
    seed: opening.seed,
    stealth,
    waterEndTurnStealth,
    waterHealing,
  });
}

if (process.argv[1] && resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  const result = await runPrivateGameCheck(process.argv[2]);
  process.stdout.write(`${canonicalJson(result as unknown as JsonValue)}\n`);
}
