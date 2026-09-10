import { resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

import { canonicalJson, type JsonValue } from '../authority/canonical-json.ts';
import { identityHash } from '../authority/hash.ts';
import {
  createGameManifest,
  hashGameState,
  type GameCardDefinition,
  type GameDeckSpec,
  type GameLegalAction,
  type GameManifest,
  type GameSession,
  type GameTerminal,
} from '../engine/game.ts';
import { runRustSyntheticDemo, type Sha256Hash } from '../engine/rust-engine.ts';
import { withRustSession } from '../engine/rust-session-helpers.ts';

const SYNTHETIC_AUTHORITY_HASH =
  'sha256:1111111111111111111111111111111111111111111111111111111111111111' as const;
const MAX_ACTIONS = 500;

function demoDeck(prefix: string): GameDeckSpec {
  return {
    atlas: Array.from({ length: 30 }, (_, index) => `${prefix}-site-${index + 1}`),
    avatar: `${prefix}-avatar`,
    spellbook: Array.from({ length: 50 }, (_, index) => `${prefix}-spell-${index + 1}`),
  };
}

function demoCards(decks: Readonly<Record<'north' | 'south', GameDeckSpec>>): Record<string, GameCardDefinition> {
  const cards: Record<string, GameCardDefinition> = {};
  for (const deck of Object.values(decks)) {
    cards[deck.avatar] = { attack: 1, cardType: 'avatar', defense: 1, drawSpell: false, life: 20 };
    deck.atlas.forEach((cardId) => {
      cards[cardId] = { cardType: 'site', elements: ['earth'] };
    });
    deck.spellbook.forEach((cardId) => {
      cards[cardId] = {
        attack: 1,
        cardType: 'minion',
        defense: 1,
        manaCost: 1,
        thresholds: { air: 0, earth: 1, fire: 0, water: 0 },
      };
    });
  }
  return cards;
}

export function createSyntheticDemoManifest(seed = 1): GameManifest {
  const decks = { north: demoDeck('north'), south: demoDeck('south') };
  return createGameManifest({
    authority: {
      contentHash: SYNTHETIC_AUTHORITY_HASH,
      mode: 'synthetic',
      revisionId: 'synthetic-setup-fixture-v1',
    },
    cards: demoCards(decks),
    decks,
    firstSeat: 'north',
    seed,
  });
}

export function selectDeterministicGameAction(
  session: GameSession,
  issuedActions: readonly GameLegalAction[],
): GameLegalAction {
  const actions = issuedActions;
  const seat = session.state.decisionSeat;
  const player = session.state.players[seat];
  const enemySeat = seat === 'north' ? 'south' : 'north';
  // ponytail: preserve one opening-hand-sized Atlas reserve; replace when opponent strategy exists.
  const drawZone = player.atlas.length > 3 || player.spellbook.length <= player.atlas.length
    ? 'atlas'
    : 'spellbook';
  const enemyAvatar = session.state.players[enemySeat].avatar.location;
  const movement = actions
    .map((action) => {
      const inPlaceAvatarAttack = action.descriptor.kind === 'move-and-attack'
        && action.descriptor.path.length === 1
        && action.descriptor.to.cell === enemyAvatar
        && action.descriptor.to.region === 'surface';
      return {
        action,
        distance: inPlaceAvatarAttack
          ? -1
          : action.descriptor.kind === 'move-and-attack' && action.descriptor.path.length > 1
            ? Math.abs(action.descriptor.to.cell.charCodeAt(0) - enemyAvatar.charCodeAt(0))
              + Math.abs(Number(action.descriptor.to.cell[1]) - Number(enemyAvatar[1]))
            : Number.POSITIVE_INFINITY,
      };
    })
    .sort((left, right) => left.distance - right.distance)[0];
  // ponytail: exercise obviously beneficial supported tactics; add evaluation when the opponent needs strategy.
  const tactic = actions.find(({ descriptor }) => {
    if (descriptor.kind === 'cast-magic') {
      const definition = session.state.cards[descriptor.cardId];
      return definition?.cardType === 'magic'
        && ((definition.damageTargetUnit !== undefined && descriptor.target?.seat === enemySeat)
          || (definition.grantPowerToAllyThisTurn !== undefined
            && descriptor.ally?.seat === seat
            && movement?.action.descriptor.kind === 'move-and-attack'
            && movement.action.descriptor.unitInstanceId === descriptor.ally.instanceId
            && movement.action.descriptor.to.cell === enemyAvatar
            && movement.action.descriptor.to.region === 'surface'));
    }
    if (descriptor.kind === 'shoot-projectile') return descriptor.hit?.seat === enemySeat;
    if (descriptor.kind === 'shoot-drag-projectile') {
      return descriptor.hit?.seat === enemySeat && !descriptor.fightOnArrival;
    }
    if (descriptor.kind !== 'activate-sparkmage'
      || (player.airThresholdsCastThisTurn ?? 0) === 0) return false;
    const targetControllers = [
      ...(['north', 'south'] as const).flatMap((targetSeat) => {
        const avatar = session.state.players[targetSeat].avatar;
        return avatar.card.instanceId !== descriptor.sourceInstanceId
          && avatar.location === descriptor.targetLocation.cell
          && avatar.region === descriptor.targetLocation.region
          ? [targetSeat]
          : [];
      }),
      ...session.state.realm.units
        .filter(({ instanceId, location, region }) =>
          instanceId !== descriptor.sourceInstanceId
            && location === descriptor.targetLocation.cell
            && region === descriptor.targetLocation.region)
        .map(({ controller }) => controller),
    ];
    return targetControllers.length > 0
      && targetControllers.every((controller) => controller === enemySeat);
  });
  const movingUnitInstanceId = movement?.action.descriptor.kind === 'move-and-attack'
    ? movement.action.descriptor.unitInstanceId
    : undefined;
  const poweredMovement = movingUnitInstanceId
    && (player.avatar.card.instanceId === movingUnitInstanceId
      ? (player.avatar.temporaryPowerSources?.length ?? 0) > 0
      : session.state.realm.units.some(({ instanceId, temporaryPowerSources }) =>
        instanceId === movingUnitInstanceId
          && (temporaryPowerSources?.length ?? 0) > 0))
    ? movement?.action
    : undefined;
  const selected = actions.find(({ descriptor }) =>
    descriptor.kind === 'mulligan'
      && descriptor.atlasOrder.length === 0
      && descriptor.spellbookOrder.length === 0)
    ?? actions.find(({ descriptor }) => descriptor.kind === 'play-site')
    ?? actions.find(({ descriptor }) => descriptor.kind === 'summon-minion')
    ?? actions.find(({ descriptor }) =>
      descriptor.kind === 'draw' && descriptor.zone === drawZone)
    ?? poweredMovement
    ?? tactic
    ?? (movement && Number.isFinite(movement.distance) ? movement.action : undefined)
    ?? actions.find(({ descriptor }) => descriptor.kind === 'end-turn')
    ?? actions[0];
  if (!selected) throw new Error('deterministic demo agent has no supported legal action');
  return selected;
}

export type DeterministicGameReport = Readonly<{
  acceptedActionCount: number;
  classification: 'unranked_partial_rules';
  finalStateHash: Sha256Hash;
  fightCount: number;
  replayVerified: boolean;
  terminal: Extract<GameTerminal, { status: 'finished' }>;
  transcriptHash: Sha256Hash;
  turnCount: number;
}>;

export async function runDeterministicGame(manifest: GameManifest): Promise<DeterministicGameReport> {
  return withRustSession(manifest, async (handle) => {
    while (handle.snapshot.state.terminal.status === 'active'
      && handle.snapshot.transcript.length < MAX_ACTIONS) {
      const issued = await handle.legalActions();
      const result = await handle.stepAction(
        selectDeterministicGameAction(handle.snapshot, issued),
      );
      if (!result.accepted) throw new Error(`deterministic demo action rejected: ${result.reason.code}`);
    }
    const session = handle.snapshot;
    if (session.state.terminal.status !== 'finished') {
      throw new Error('deterministic demo exceeded action limit');
    }
    return Object.freeze({
      acceptedActionCount: session.transcript.length,
      classification: 'unranked_partial_rules',
      finalStateHash: hashGameState(session.state),
      fightCount: session.transcript.flatMap(({ events }) => events)
        .filter(({ type }) => type === 'fight-started').length,
      replayVerified: await handle.verifyReplay(),
      terminal: session.state.terminal,
      transcriptHash: identityHash(session.transcript as unknown as JsonValue),
      turnCount: session.state.turnNumber,
    });
  });
}

export function runGameDemo(seed = 1): DeterministicGameReport {
  const report = runRustSyntheticDemo(seed);
  return Object.freeze({
    acceptedActionCount: report.acceptedActionCount,
    classification: 'unranked_partial_rules',
    fightCount: report.fightCount,
    finalStateHash: report.finalStateHash,
    replayVerified: report.replayVerified,
    terminal: report.terminal as DeterministicGameReport['terminal'],
    transcriptHash: report.transcriptHash,
    turnCount: report.turnCount,
  });
}

if (process.argv[1] && resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  const seed = process.argv[2] === undefined ? 1 : Number(process.argv[2]);
  process.stdout.write(`${canonicalJson(runGameDemo(seed))}\n`);
}
