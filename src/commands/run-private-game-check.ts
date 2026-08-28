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
  avatar: Readonly<{ drawSpell: boolean; stableId: string }>;
  chargeMinionStableId: string;
  providerMinionStableId: string;
  revisionId: string;
  seed: number;
}>;

type DeckList = Readonly<{
  atlas: readonly Readonly<{ copies: number; name: string }>[];
  avatar: string;
  spellbook: readonly Readonly<{ copies: number; name: string }>[];
}>;

export type PrivateGameCheck = Readonly<{
  acceptedActionCount: number;
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
  provider: Readonly<{ affinityAdded: boolean; minion: string }>;
  replayVerified: boolean;
  revisionId: string;
  seed: number;
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
    || typeof avatar.stableId !== 'string'
    || typeof avatar.drawSpell !== 'boolean'
    || typeof value.chargeMinionStableId !== 'string'
    || typeof value.providerMinionStableId !== 'string'
    || typeof value.revisionId !== 'string'
    || !Number.isSafeInteger(value.seed)
    || typeof value.seed !== 'number'
    || value.seed < 0
    || Object.keys(value).sort().join(',') !== 'avatar,chargeMinionStableId,providerMinionStableId,revisionId,seed'
    || Object.keys(avatar).sort().join(',') !== 'drawSpell,stableId') {
    throw new Error('private game scenario has an unsupported shape');
  }
  return {
    avatar: { drawSpell: avatar.drawSpell, stableId: avatar.stableId },
    chargeMinionStableId: value.chargeMinionStableId,
    providerMinionStableId: value.providerMinionStableId,
    revisionId: value.revisionId,
    seed: value.seed,
  };
}

async function readPrivateInputs(path: string): Promise<Readonly<{
  authorityHash: Hash;
  cards: readonly NormalizedCard[];
  chargeMinion: NormalizedCard;
  config: ScenarioConfig;
  format: FormatDefinition;
  formatStableId: string;
  providerMinion: NormalizedCard;
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
    chargeMinion,
    config,
    format: selected.identity.payload,
    formatStableId: selected.identity.stableId,
    providerMinion,
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
  provides?: GameElement,
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
  if (card.cardType === 'site') return { cardType: 'site', elements: card.elements };
  if (card.cardType === 'minion'
    && card.attack !== null
    && card.defense !== null
    && card.manaCost !== null) {
    return {
      attack: card.attack,
      cardType: 'minion',
      charge,
      defense: card.defense,
      manaCost: card.manaCost,
      ...(provides ? { provides } : {}),
      thresholds: card.thresholds,
    };
  }
  throw new Error(`actual card ${card.stableId} lacks required supported facts`);
}

function buildManifest(
  input: Awaited<ReturnType<typeof readPrivateInputs>>,
  seed: number,
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
    return {
    atlas: fillZone(sites, input.format.atlasMinimum, input.format, reverse),
    avatar: avatar.stableId,
    spellbook: [
      ...Array.from({ length: chargeCopies }, () => input.chargeMinion.stableId),
      ...Array.from({ length: providerCopies }, () => input.providerMinion.stableId),
      ...fillZone(
        minions,
        input.format.spellbookMinimum - chargeCopies - providerCopies,
        input.format,
        reverse,
      ),
    ],
    };
  };
  const decks = { north: deck(false, true), south: deck(true, false) };
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
      card.stableId === input.chargeMinion.stableId,
      card.stableId === input.providerMinion.stableId ? 'fire' : undefined,
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

function findOpening(
  input: Awaited<ReturnType<typeof readPrivateInputs>>,
): Readonly<{
  manifest: GameManifest;
  names: ReadonlyMap<string, string>;
  north: NonNullable<ReturnType<typeof openingPair>>;
  northCharge: NonNullable<ReturnType<typeof openingPair>>;
  northProvider: NonNullable<ReturnType<typeof openingPair>>;
  seed: number;
  session: GameSession;
  south: NonNullable<ReturnType<typeof openingPair>>;
}> {
  const seed = input.config.seed;
  const built = buildManifest(input, seed);
  const session = createGameSession(built.manifest);
  const north = openingPair(session, 'north');
  const northCharge = north && openingPair(
    session,
    'north',
    input.chargeMinion.stableId,
    2,
    north.siteInstanceId,
  );
  const northProvider = openingPair(session, 'north', input.providerMinion.stableId, 2);
  const south = openingPair(session, 'south');
  if (north && northCharge && northProvider && south) {
    return { ...built, north, northCharge, northProvider, seed, session, south };
  }
  throw new Error(`private scenario seed ${seed} no longer produces its supported opening`);
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

export async function runPrivateGameCheck(path = DEFAULT_SCENARIO): Promise<PrivateGameCheck> {
  const input = await readPrivateInputs(path);
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
      && descriptor.cardInstanceId === opening.northCharge.siteInstanceId
      && descriptor.cell === 'C3'));
  session = accept(session, action(session, ({ descriptor }) =>
    descriptor.kind === 'summon-minion'
      && descriptor.cardInstanceId === opening.northCharge.minionInstanceId
      && descriptor.cell === 'C3'));
  const chargeActivatedOnSummon = legalGameActions(session.state, 'north').some(({ descriptor }) =>
    descriptor.kind === 'move-and-attack'
      && descriptor.unitInstanceId === opening.northCharge.minionInstanceId
      && descriptor.to.cell === 'C3');
  session = accept(session, action(session, ({ descriptor }) =>
    descriptor.kind === 'move-and-attack'
      && descriptor.unitInstanceId === opening.northCharge.minionInstanceId
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
      && descriptor.cardInstanceId === opening.northProvider.minionInstanceId
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

  const northCardId = opening.session.state.players.north.hand.spellbook
    .find(({ instanceId }) => instanceId === opening.north.minionInstanceId)!.cardId;
  const southCardId = opening.session.state.players.south.hand.spellbook
    .find(({ instanceId }) => instanceId === opening.south.minionInstanceId)!.cardId;
  return Object.freeze({
    acceptedActionCount: session.transcript.length,
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
    finalStateHash: hashGameState(session.state),
    formatStableId: input.formatStableId,
    provider: {
      affinityAdded: providerAffinityAdded,
      minion: opening.names.get(input.providerMinion.stableId) ?? input.providerMinion.stableId,
    },
    replayVerified: verifyGameReplay(session),
    revisionId: input.config.revisionId,
    seed: opening.seed,
  });
}

if (process.argv[1] && resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  const result = await runPrivateGameCheck(process.argv[2]);
  process.stdout.write(`${canonicalJson(result as unknown as JsonValue)}\n`);
}
