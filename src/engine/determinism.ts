import { identityHash } from '../authority/hash.ts';

const UINT32_MAX = 0xffff_ffff;
const MULBERRY32_INCREMENT = 0x6d2b_79f5;

export type PrngState = Readonly<{
  algorithm: 'mulberry32-v1';
  word: number;
  draws: number;
}>;

export type EngineState = Readonly<{
  schemaVersion: 1;
  stateVersion: number;
  prng: PrngState;
}>;

export type RandomDraw = Readonly<{
  value: number;
  nextState: EngineState;
}>;

function frozenState(stateVersion: number, word: number, draws: number): EngineState {
  return Object.freeze({
    schemaVersion: 1,
    stateVersion,
    prng: Object.freeze({ algorithm: 'mulberry32-v1', word, draws }),
  });
}

export function createEngineState(seed: number): EngineState {
  if (!Number.isInteger(seed) || seed < 0 || seed > UINT32_MAX) {
    throw new RangeError('seed must be an unsigned 32-bit integer');
  }
  return frozenState(0, seed, 0);
}

export function drawUint32(state: EngineState): RandomDraw {
  if (!Number.isSafeInteger(state.prng.draws) || state.prng.draws < 0) {
    throw new RangeError('PRNG draw count must be a non-negative safe integer');
  }
  if (state.prng.draws === Number.MAX_SAFE_INTEGER) {
    throw new RangeError('PRNG draw count exhausted');
  }

  const word = (state.prng.word + MULBERRY32_INCREMENT) >>> 0;
  let value = word;
  value = Math.imul(value ^ (value >>> 15), value | 1);
  value ^= value + Math.imul(value ^ (value >>> 7), value | 61);
  value = (value ^ (value >>> 14)) >>> 0;

  return Object.freeze({
    value,
    nextState: frozenState(state.stateVersion, word, state.prng.draws + 1),
  });
}

export function hashEngineState(state: EngineState): ReturnType<typeof identityHash> {
  return identityHash(state);
}
