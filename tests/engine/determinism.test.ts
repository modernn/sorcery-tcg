import assert from 'node:assert/strict';
import { spawnSync } from 'node:child_process';
import { resolve } from 'node:path';
import test from 'node:test';
import { pathToFileURL } from 'node:url';

import { canonicalJson } from '../../src/authority/canonical-json.ts';
import {
  createEngineState,
  drawUint32,
  hashEngineState,
  type EngineState,
} from '../../src/engine/determinism.ts';

const REPOSITORY_ROOT = resolve(import.meta.dirname, '..', '..');
const INITIAL_JSON =
  '{"prng":{"algorithm":"mulberry32-v1","draws":0,"word":0},"schemaVersion":1,"stateVersion":0}';
const INITIAL_HASH = 'sha256:9ec0a8e3f08e46c6c6e2a44542de0a7c20a50a1fe6af9ea5b195dd0379d4e4b2';
const FIVE_DRAW_HASH = 'sha256:7a06a448ee0b11752a630f6a16d51640c37c03d13d73fa328e420e018bafc5dc';
const GOLDEN_VALUES = [1_144_304_738, 1_416_247, 958_946_056, 627_933_444, 2_007_157_716];

test('ENG-01/02 canonical state and seeded PRNG match fixed vectors without mutation', () => {
  const initial = createEngineState(0);
  assert.deepEqual(initial, {
    schemaVersion: 1,
    stateVersion: 0,
    prng: { algorithm: 'mulberry32-v1', word: 0, draws: 0 },
  });
  assert.equal(Object.isFrozen(initial), true);
  assert.equal(Object.isFrozen(initial.prng), true);
  assert.equal(canonicalJson(initial), INITIAL_JSON);
  assert.equal(hashEngineState(initial), INITIAL_HASH);

  const values: number[] = [];
  let state = initial;
  for (let index = 0; index < GOLDEN_VALUES.length; index += 1) {
    const draw = drawUint32(state);
    values.push(draw.value);
    state = draw.nextState;
  }

  assert.deepEqual(values, GOLDEN_VALUES);
  assert.deepEqual(initial.prng, { algorithm: 'mulberry32-v1', word: 0, draws: 0 });
  assert.equal(state.stateVersion, 0);
  assert.equal(state.prng.word, 567_894_473);
  assert.equal(state.prng.draws, 5);
  assert.equal(hashEngineState(state), FIVE_DRAW_HASH);
  assert.equal(
    canonicalJson(state),
    '{"prng":{"algorithm":"mulberry32-v1","draws":5,"word":567894473},"schemaVersion":1,"stateVersion":0}',
  );

  for (const invalidSeed of [-1, 0.5, 0x1_0000_0000]) {
    assert.throws(() => createEngineState(invalidSeed), RangeError);
  }
});

test('ENG-02 canonical JSON checkpoint resumes the exact random stream', () => {
  let uninterrupted = createEngineState(123_456_789);
  let checkpoint = uninterrupted;
  for (let index = 0; index < 2; index += 1) {
    uninterrupted = drawUint32(uninterrupted).nextState;
    checkpoint = drawUint32(checkpoint).nextState;
  }

  let resumed = JSON.parse(canonicalJson(checkpoint)) as EngineState;
  const uninterruptedValues: number[] = [];
  const resumedValues: number[] = [];
  for (let index = 0; index < 5; index += 1) {
    const uninterruptedDraw = drawUint32(uninterrupted);
    const resumedDraw = drawUint32(resumed);
    uninterruptedValues.push(uninterruptedDraw.value);
    resumedValues.push(resumedDraw.value);
    uninterrupted = uninterruptedDraw.nextState;
    resumed = resumedDraw.nextState;
  }

  assert.deepEqual(resumedValues, uninterruptedValues);
  assert.equal(canonicalJson(resumed), canonicalJson(uninterrupted));
  assert.equal(hashEngineState(resumed), hashEngineState(uninterrupted));
});

test('TEST-02 fresh processes emit byte-identical deterministic state evidence', () => {
  const determinismUrl = pathToFileURL(
    resolve(REPOSITORY_ROOT, 'src', 'engine', 'determinism.ts'),
  ).href;
  const canonicalJsonUrl = pathToFileURL(
    resolve(REPOSITORY_ROOT, 'src', 'authority', 'canonical-json.ts'),
  ).href;
  const program = [
    `import { canonicalJson } from ${JSON.stringify(canonicalJsonUrl)};`,
    `import { createEngineState, drawUint32, hashEngineState } from ${JSON.stringify(determinismUrl)};`,
    'let state = createEngineState(0);',
    'const values = [];',
    'for (let index = 0; index < 5; index += 1) { const draw = drawUint32(state); values.push(draw.value); state = draw.nextState; }',
    'process.stdout.write(canonicalJson({ hash: hashEngineState(state), state, values }));',
  ].join('');

  function run(): Buffer {
    const result = spawnSync(process.execPath, ['--input-type=module', '--eval', program], {
      cwd: REPOSITORY_ROOT,
      maxBuffer: 1_048_576,
    });
    assert.equal(result.status, 0, result.stderr.toString('utf8'));
    assert.equal(result.stderr.length, 0);
    return result.stdout;
  }

  const first = run();
  const second = run();
  assert.deepEqual(first, second);
  assert.equal(
    first.toString('utf8'),
    `{"hash":"${FIVE_DRAW_HASH}","state":{"prng":{"algorithm":"mulberry32-v1","draws":5,"word":567894473},"schemaVersion":1,"stateVersion":0},"values":[1144304738,1416247,958946056,627933444,2007157716]}`,
  );
});
