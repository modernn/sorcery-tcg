import { readFileSync } from 'node:fs';
import { resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

import { canonicalJson, type JsonValue } from '../src/authority/canonical-json.ts';
import { identityHash } from '../src/authority/hash.ts';
import {
  createSyntheticDemoManifest,
  selectDeterministicGameAction,
} from '../src/commands/run-game-demo.ts';
import { createEngineState, drawUint32, hashEngineState } from '../src/engine/determinism.ts';
import {
  createGameSession,
  hashGameState,
  legalGameActions,
  replayGame,
  stepGame,
  verifyGameReplay,
} from '../src/engine/game.ts';

const FIXTURE_PATH = fileURLToPath(
  new URL('../tests/engine/fixtures/typescript-parity-v1.json', import.meta.url),
);
const SEEDS = [0, 31, 0xffff_ffff] as const;
const PRNG_DRAWS = 5;
const CAPTURED_STEPS = 8;
const MAX_ACTIONS = 500;

function hash(value: unknown): ReturnType<typeof identityHash> {
  return identityHash(value as JsonValue);
}

function capturePrng(seed: number): JsonValue {
  let state = createEngineState(seed);
  const draws: JsonValue[] = [];
  for (let index = 0; index < PRNG_DRAWS; index += 1) {
    const draw = drawUint32(state);
    state = draw.nextState;
    draws.push({
      draws: state.prng.draws,
      stateHash: hashEngineState(state),
      value: draw.value,
      word: state.prng.word,
    });
  }
  return { draws, initialStateHash: hashEngineState(createEngineState(seed)), seed };
}

function captureGame(seed: number): JsonValue {
  const manifest = createSyntheticDemoManifest(seed);
  let session = createGameSession(manifest);
  const initialActions = legalGameActions(session.state, session.state.decisionSeat);
  const initial = {
    legalActionIds: initialActions.map(({ actionId }) => actionId),
    randomDrawsHash: hash(session.initialRandomDraws),
    stateHash: hashGameState(session.state),
  };
  const actionIds: string[] = [];
  const steps: JsonValue[] = [];

  while (session.state.terminal.status === 'active' && actionIds.length < MAX_ACTIONS) {
    const legalActions = legalGameActions(session.state, session.state.decisionSeat);
    const action = selectDeterministicGameAction(session);
    const preStateHash = hashGameState(session.state);
    const result = stepGame(session, action);
    if (!result.accepted) throw new Error(`issued action was rejected: ${result.reason.code}`);
    actionIds.push(action.actionId);
    if (steps.length < CAPTURED_STEPS) {
      steps.push({
        eventIds: result.receipt.events.map(({ eventId }) => eventId),
        eventTypes: result.receipt.events.map(({ type }) => type),
        legalActionIds: legalActions.map(({ actionId }) => actionId),
        postStateHash: result.receipt.postStateHash,
        preStateHash,
        randomDrawsHash: hash(result.receipt.randomDraws),
        receiptId: result.receipt.receiptId,
        selectedActionId: action.actionId,
        stateVersion: session.state.stateVersion,
      });
    }
    session = result.session;
  }

  if (session.state.terminal.status !== 'finished') {
    throw new Error(`seed ${seed} exceeded ${MAX_ACTIONS} actions`);
  }
  const replayed = replayGame(manifest, actionIds);
  return {
    actionIds,
    finalStateHash: hashGameState(session.state),
    initial,
    ...(seed === 31 ? { manifestJson: canonicalJson(manifest as unknown as JsonValue) } : {}),
    manifestId: manifest.manifestId,
    receiptIdsHash: hash(session.transcript.map(({ receiptId }) => receiptId)),
    replay: {
      finalStateHash: hashGameState(replayed.state),
      transcriptHash: hash(replayed.transcript),
      verified: verifyGameReplay(session),
    },
    seed,
    steps,
    terminal: session.state.terminal,
    transcriptHash: hash(session.transcript),
  };
}

export function captureEngineParityFixture(): JsonValue {
  return {
    fixtureVersion: 1,
    games: SEEDS.map(captureGame),
    prng: SEEDS.map(capturePrng),
    source: 'typescript-legality-engine',
  };
}

export function serializeEngineParityFixture(): string {
  return `${canonicalJson(captureEngineParityFixture())}\n`;
}

if (process.argv[1] && resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  const serialized = serializeEngineParityFixture();
  if (process.argv[2] === '--check') {
    if (readFileSync(FIXTURE_PATH, 'utf8') !== serialized) {
      throw new Error(`parity fixture is stale: ${FIXTURE_PATH}`);
    }
  } else {
    process.stdout.write(serialized);
  }
}
