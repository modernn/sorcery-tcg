import { readFileSync, writeFileSync } from 'node:fs';
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
  hashGameState,
  type GameLegalAction,
  type GameManifest,
  type GameSession,
} from '../src/engine/game.ts';
import { RustSessionClient } from '../src/engine/rust-engine.ts';

const FIXTURE_PATH = fileURLToPath(
  new URL('../tests/engine/fixtures/typescript-parity-v1.json', import.meta.url),
);
const SEEDS = [0, 31, 0xffff_ffff] as const;
const PRNG_DRAWS = 5;
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

function parseExportedSession(exported: JsonValue, manifest: GameManifest): GameSession {
  if (typeof exported !== 'object' || exported === null || Array.isArray(exported)) {
    throw new Error('exportSession result was malformed');
  }
  const value = exported as Record<string, unknown>;
  return {
    attempts: value.attempts as GameSession['attempts'],
    initialRandomDraws: value.initialRandomDraws as GameSession['initialRandomDraws'],
    manifest,
    state: value.state as GameSession['state'],
    transcript: value.transcript as GameSession['transcript'],
  };
}

async function replayRustGame(
  client: RustSessionClient,
  manifest: GameManifest,
  actionIds: readonly string[],
): Promise<GameSession> {
  await client.newSession(canonicalJson(manifest as unknown as JsonValue));
  let snapshot = parseExportedSession(await client.exportSession(), manifest);
  for (const actionId of actionIds) {
    const actions = await client.legalActions(snapshot.state.decisionSeat);
    const action = actions.find((candidate) => candidate.actionId === actionId);
    if (!action) throw new Error(`missing replay action ${actionId}`);
    const result = await client.step(action);
    if (!result.accepted) throw new Error(`replay rejected ${actionId}`);
    snapshot = parseExportedSession(await client.exportSession(), manifest);
  }
  return snapshot;
}

async function captureGame(client: RustSessionClient, seed: number): Promise<JsonValue> {
  const manifest = createSyntheticDemoManifest(seed);
  await client.newSession(canonicalJson(manifest as unknown as JsonValue));
  let snapshot = parseExportedSession(await client.exportSession(), manifest);
  const initialActions = await client.legalActions(snapshot.state.decisionSeat);
  const initial = {
    legalActionIds: initialActions.map(({ actionId }) => actionId),
    randomDrawsHash: hash(snapshot.initialRandomDraws),
    stateHash: hashGameState(snapshot.state),
  };
  const actionIds: string[] = [];
  const steps: JsonValue[] = [];

  while (snapshot.state.terminal.status === 'active' && actionIds.length < MAX_ACTIONS) {
    const legalActions = await client.legalActions(snapshot.state.decisionSeat);
    const action = selectDeterministicGameAction(
      snapshot,
      legalActions as unknown as GameLegalAction[],
    );
    const preStateHash = hashGameState(snapshot.state);
    const result = await client.step(action);
    if (!result.accepted) throw new Error(`issued action was rejected`);
    const receipt = result.receipt as GameSession['transcript'][number];
    actionIds.push(action.actionId);
    steps.push({
      events: receipt.events,
      eventIds: receipt.events.map(({ eventId }) => eventId),
      eventTypes: receipt.events.map(({ type }) => type),
      legalActionIds: legalActions.map(({ actionId }) => actionId),
      postStateHash: receipt.postStateHash,
      preStateHash,
      randomDrawsHash: hash(receipt.randomDraws),
      receiptId: receipt.receiptId,
      selectedAction: action,
      selectedActionId: action.actionId,
      stateVersion: snapshot.state.stateVersion,
    });
    snapshot = parseExportedSession(await client.exportSession(), manifest);
  }

  if (snapshot.state.terminal.status !== 'finished') {
    throw new Error(`seed ${seed} exceeded ${MAX_ACTIONS} actions`);
  }
  const verified = await client.verifyReplay();
  const replayed = await replayRustGame(client, manifest, actionIds);
  return {
    actionIds,
    finalStateHash: hashGameState(snapshot.state),
    initial,
    ...(seed === 31 ? { manifestRecipe: 'synthetic-demo-v1' } : {}),
    manifestId: manifest.manifestId,
    receiptIdsHash: hash(snapshot.transcript.map(({ receiptId }) => receiptId)),
    replay: {
      finalStateHash: hashGameState(replayed.state),
      transcriptHash: hash(replayed.transcript),
      verified,
    },
    seed,
    steps,
    terminal: snapshot.state.terminal,
    transcriptHash: hash(snapshot.transcript),
  };
}

export async function captureEngineParityFixture(): Promise<JsonValue> {
  const client = await RustSessionClient.start();
  try {
    const games: JsonValue[] = [];
    for (const seed of SEEDS) {
      games.push(await captureGame(client, seed));
    }
    return {
      fixtureVersion: 1,
      games,
      prng: SEEDS.map(capturePrng),
      source: 'rust-legality-engine',
    };
  } finally {
    await client.close();
  }
}

export async function serializeEngineParityFixture(): Promise<string> {
  return `${canonicalJson(await captureEngineParityFixture())}\n`;
}

async function main(): Promise<void> {
  const serialized = await serializeEngineParityFixture();
  if (process.argv[2] === '--check') {
    if (readFileSync(FIXTURE_PATH, 'utf8') !== serialized) {
      throw new Error(`parity fixture is stale: ${FIXTURE_PATH}`);
    }
  } else if (process.argv[2] === '--write') {
    writeFileSync(FIXTURE_PATH, serialized);
  } else {
    process.stdout.write(serialized);
  }
}

if (process.argv[1] && resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  void main();
}

export type { GameLegalAction, GameManifest };
