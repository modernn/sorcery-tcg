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
  airSeed: number;
  avatar: Readonly<{ drawSpell: boolean; stableId: string }>;
  cannotDefendMinionStableId: string;
  chargeMinionStableId: string;
  deathriteMinionStableId: string;
  earthProviderMinionStableId: string;
  earthSeed: number;
  fireSeed: number;
  genesisMinionStableId: string;
  ghostTownSiteStableId: string;
  healingMinionStableId: string;
  lethalMinionStableId: string;
  lumberingMinionStableId: string;
  manaMinionStableId: string;
  monstrousLionStableId: string;
  movementMinionStableId: string;
  providerMinionStableId: string;
  revisionId: string;
  roamingMinionStableId: string;
  roamingSeed: number;
  seed: number;
  waterSeed: number;
}>;

type DeckList = Readonly<{
  atlas: readonly Readonly<{ copies: number; name: string }>[];
  avatar: string;
  spellbook: readonly Readonly<{ copies: number; name: string }>[];
}>;

export type PrivateGameCheck = Readonly<{
  acceptedActionCount: number;
  airMovement: Readonly<{
    acceptedActionCount: number;
    deck: DeckList;
    movementMinion: string;
    replayVerified: boolean;
    seed: number;
    twoStepDefend: boolean;
    twoStepMoveAndAttack: boolean;
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
    || !Number.isSafeInteger(value.airSeed)
    || typeof value.airSeed !== 'number'
    || value.airSeed < 0
    || typeof avatar.stableId !== 'string'
    || typeof avatar.drawSpell !== 'boolean'
    || typeof value.cannotDefendMinionStableId !== 'string'
    || typeof value.chargeMinionStableId !== 'string'
    || typeof value.deathriteMinionStableId !== 'string'
    || typeof value.earthProviderMinionStableId !== 'string'
    || !Number.isSafeInteger(value.earthSeed)
    || typeof value.earthSeed !== 'number'
    || value.earthSeed < 0
    || !Number.isSafeInteger(value.fireSeed)
    || typeof value.fireSeed !== 'number'
    || value.fireSeed < 0
    || typeof value.genesisMinionStableId !== 'string'
    || typeof value.ghostTownSiteStableId !== 'string'
    || typeof value.healingMinionStableId !== 'string'
    || typeof value.lethalMinionStableId !== 'string'
    || typeof value.lumberingMinionStableId !== 'string'
    || typeof value.manaMinionStableId !== 'string'
    || typeof value.monstrousLionStableId !== 'string'
    || typeof value.movementMinionStableId !== 'string'
    || typeof value.providerMinionStableId !== 'string'
    || typeof value.revisionId !== 'string'
    || typeof value.roamingMinionStableId !== 'string'
    || !Number.isSafeInteger(value.roamingSeed)
    || typeof value.roamingSeed !== 'number'
    || value.roamingSeed < 0
    || !Number.isSafeInteger(value.seed)
    || typeof value.seed !== 'number'
    || value.seed < 0
    || !Number.isSafeInteger(value.waterSeed)
    || typeof value.waterSeed !== 'number'
    || value.waterSeed < 0
    || Object.keys(value).sort().join(',') !== 'airSeed,avatar,cannotDefendMinionStableId,chargeMinionStableId,deathriteMinionStableId,earthProviderMinionStableId,earthSeed,fireSeed,genesisMinionStableId,ghostTownSiteStableId,healingMinionStableId,lethalMinionStableId,lumberingMinionStableId,manaMinionStableId,monstrousLionStableId,movementMinionStableId,providerMinionStableId,revisionId,roamingMinionStableId,roamingSeed,seed,waterSeed'
    || Object.keys(avatar).sort().join(',') !== 'drawSpell,stableId') {
    throw new Error('private game scenario has an unsupported shape');
  }
  return {
    airSeed: value.airSeed,
    avatar: { drawSpell: avatar.drawSpell, stableId: avatar.stableId },
    cannotDefendMinionStableId: value.cannotDefendMinionStableId,
    chargeMinionStableId: value.chargeMinionStableId,
    deathriteMinionStableId: value.deathriteMinionStableId,
    earthProviderMinionStableId: value.earthProviderMinionStableId,
    earthSeed: value.earthSeed,
    fireSeed: value.fireSeed,
    genesisMinionStableId: value.genesisMinionStableId,
    ghostTownSiteStableId: value.ghostTownSiteStableId,
    healingMinionStableId: value.healingMinionStableId,
    lethalMinionStableId: value.lethalMinionStableId,
    lumberingMinionStableId: value.lumberingMinionStableId,
    manaMinionStableId: value.manaMinionStableId,
    monstrousLionStableId: value.monstrousLionStableId,
    movementMinionStableId: value.movementMinionStableId,
    providerMinionStableId: value.providerMinionStableId,
    revisionId: value.revisionId,
    roamingMinionStableId: value.roamingMinionStableId,
    roamingSeed: value.roamingSeed,
    seed: value.seed,
    waterSeed: value.waterSeed,
  };
}

async function readPrivateInputs(path: string): Promise<Readonly<{
  authorityHash: Hash;
  cards: readonly NormalizedCard[];
  cannotDefendMinion: NormalizedCard;
  chargeMinion: NormalizedCard;
  config: ScenarioConfig;
  deathriteMinion: NormalizedCard;
  earthProviderMinion: NormalizedCard;
  format: FormatDefinition;
  formatStableId: string;
  genesisMinion: NormalizedCard;
  ghostTownSite: NormalizedCard;
  healingMinion: NormalizedCard;
  lethalMinion: NormalizedCard;
  lumberingMinion: NormalizedCard;
  manaMinion: NormalizedCard;
  monstrousLion: NormalizedCard;
  movementMinion: NormalizedCard;
  providerMinion: NormalizedCard;
  roamingMinion: NormalizedCard;
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
    authorityHash: artifact.contentHash,
    cards: snapshot.cards,
    cannotDefendMinion,
    chargeMinion,
    config,
    deathriteMinion,
    earthProviderMinion,
    format: selected.identity.payload,
    formatStableId: selected.identity.stableId,
    genesisMinion,
    ghostTownSite,
    healingMinion,
    lethalMinion,
    lumberingMinion,
    manaMinion,
    monstrousLion,
    movementMinion,
    providerMinion,
    roamingMinion,
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
  movementPlusOne = false,
  deathriteHeal = 0,
  siteGenesisGainMana = 0,
  summonToAnySite = false,
  cannotDefendOrIntercept = false,
  cannotAttackSites = false,
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
      lethal,
      manaCost: card.manaCost,
      movementPlusOne,
      ...(provides ? { provides } : {}),
      summonToAnySite,
      ...(tapForMana ? { tapForMana } : {}),
      thresholds: card.thresholds,
    };
  }
  throw new Error(`actual card ${card.stableId} lacks required supported facts`);
}

function buildManifest(
  input: Awaited<ReturnType<typeof readPrivateInputs>>,
  seed: number,
  scenario: 'air' | 'combat' | 'earth' | 'fire' | 'water' = 'combat',
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
          minions,
          input.format.spellbookMinimum - featuredMinionCards.length,
          input.format,
          false,
        ),
      ],
    };
  };
  const decks = {
    north: scenario === 'earth'
      ? elementalDeck('earth', [
        input.earthProviderMinion,
        input.manaMinion,
        input.genesisMinion,
        input.deathriteMinion,
        input.cannotDefendMinion,
      ], [input.ghostTownSite])
      : scenario === 'air'
        ? elementalDeck('air', [input.movementMinion, input.roamingMinion])
        : scenario === 'fire'
          ? elementalDeck('fire', [input.lumberingMinion, input.monstrousLion])
        : scenario === 'water'
          ? elementalDeck('water', [input.healingMinion])
          : deck(false, true),
    south: deck(true, false),
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
      card.stableId === input.movementMinion.stableId,
      card.stableId === input.healingMinion.stableId ? 3 : 0,
      card.stableId === input.ghostTownSite.stableId ? 1 : 0,
      card.stableId === input.roamingMinion.stableId,
      card.stableId === input.lumberingMinion.stableId,
      card.stableId === input.monstrousLion.stableId,
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
): Readonly<{
  attackerInstanceId: string;
  healingInstanceId: string;
  manifest: GameManifest;
  names: ReadonlyMap<string, string>;
  northSiteInstanceIds: readonly [string, string];
  seed: number;
  session: GameSession;
  southSiteInstanceIds: readonly [string, string];
}> {
  const seed = input.config.waterSeed;
  const built = buildManifest(input, seed, 'water');
  const session = createGameSession(built.manifest);
  const northSites = session.state.players.north.hand.atlas.filter((site) => {
    const definition = session.state.cards[site.cardId];
    return definition?.cardType === 'site' && definition.elements.includes('water');
  });
  const healingInstanceId = availableMinionInstance(
    session,
    'north',
    input.healingMinion.stableId,
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
    if (northSites.length >= 2 && healingInstanceId && attacker && second) {
      return {
        ...built,
        attackerInstanceId: attacker.instanceId,
        healingInstanceId,
        northSiteInstanceIds: [northSites[0]!.instanceId, northSites[1]!.instanceId],
        seed,
        session,
        southSiteInstanceIds: [first.instanceId, second.instanceId],
      };
    }
  }
  throw new Error(`private Water scenario seed ${input.config.waterSeed} no longer produces its supported opening`);
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
      && descriptor.cardInstanceId === opening.healingInstanceId
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
      && descriptor.unitInstanceId === opening.healingInstanceId
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
      && payload.instanceId === opening.healingInstanceId);

  return Object.freeze({
    acceptedActionCount: session.transcript.length,
    deck: deckList(opening.manifest.decks.north, opening.names),
    healed: session.state.players.north.avatar.life - lifeBeforeHealing,
    healedBeforeCemetery: healIndex >= 0 && healIndex < cemeteryIndex,
    healingMinionDied: session.state.players.north.cemetery
      .some(({ instanceId }) => instanceId === opening.healingInstanceId),
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
  const airMovement = runAirMovement(input);
  const airSummoning = runAirSummoning(input);
  const earthRamp = runEarthRamp(input);
  const fireResponse = runFireResponse(input);
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
    airMovement,
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
    waterHealing,
  });
}

if (process.argv[1] && resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  const result = await runPrivateGameCheck(process.argv[2]);
  process.stdout.write(`${canonicalJson(result as unknown as JsonValue)}\n`);
}
