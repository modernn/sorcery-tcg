import assert from 'node:assert/strict';
import test from 'node:test';

import {
  CanonicalJsonError,
  MAX_CANONICAL_DEPTH,
  MAX_CANONICAL_NODES,
  MAX_CANONICAL_STRING_BYTES,
  canonicalJson,
  parseJsonWithDuplicateKeyCheck,
  type JsonValue,
} from '../../src/authority/canonical-json.ts';
import { identityHash, sha256 } from '../../src/authority/hash.ts';

function asJson(value: unknown): JsonValue {
  return value as JsonValue;
}

function assertCanonicalError(value: unknown, code: CanonicalJsonError['code']): void {
  assert.throws(
    () => canonicalJson(asJson(value)),
    (error: unknown) => error instanceof CanonicalJsonError && error.code === code,
  );
}

test('DATA-03 canonical JSON matches RFC 8785 example vectors', () => {
  const value = {
    numbers: [333333333.33333329, 1e30, 4.5, 2e-3, 1e-27],
    string: "€$\u000f\nA'B\"\\\"/",
    literals: [null, true, false],
  };

  assert.equal(
    canonicalJson(value),
    String.raw`{"literals":[null,true,false],"numbers":[333333333.3333333,1e+30,4.5,0.002,1e-27],"string":"€$\u000f\nA'B\"\\\"/"}`,
  );
});

test('DATA-03 canonical JSON orders object keys by UTF-16 code units recursively', () => {
  const value = {
    '€': 'Euro Sign',
    '\r': 'Carriage Return',
    'דּ': 'Hebrew Letter Dalet With Dagesh',
    1: 'One',
    '😀': 'Emoji: Grinning Face',
    '\u0080': 'Control',
    'ö': { z: 1, a: 2 },
  };

  assert.equal(
    canonicalJson(value),
    String.raw`{"\r":"Carriage Return","1":"One","":"Control","ö":{"a":2,"z":1},"€":"Euro Sign","😀":"Emoji: Grinning Face","דּ":"Hebrew Letter Dalet With Dagesh"}`,
  );
});

test('DATA-03 canonical JSON uses ECMAScript number serialization', () => {
  assert.equal(canonicalJson([-0, 0.000001, 1e-7, 1e21, 1.2345]), '[0,0.000001,1e-7,1e+21,1.2345]');
});

test('DATA-03 canonical JSON preserves Unicode without normalization', () => {
  assert.equal(canonicalJson('\u00e9'), '"é"');
  assert.equal(canonicalJson('e\u0301'), '"é"');
  assert.notEqual(canonicalJson('\u00e9'), canonicalJson('e\u0301'));
  assertCanonicalError('\ud800', 'invalid_unicode');
});

test('DATA-03 canonical JSON preserves array order', () => {
  assert.equal(canonicalJson(['third', 'first', 'second']), '["third","first","second"]');
});

test('DATA-03 canonical JSON rejects non-finite and unsafe numbers', () => {
  for (const value of [Number.NaN, Number.POSITIVE_INFINITY, Number.NEGATIVE_INFINITY]) {
    assertCanonicalError(value, 'non_finite_number');
  }
  assertCanonicalError(9_007_199_254_740_992, 'unsafe_number');
});

test('DATA-03 canonical JSON rejects undefined and unsupported JavaScript values', () => {
  assertCanonicalError(undefined, 'unsupported_type');
  assertCanonicalError(Symbol('value'), 'unsupported_type');
  assertCanonicalError(() => undefined, 'unsupported_type');
  assertCanonicalError(1n, 'unsupported_type');
  assertCanonicalError({ value: undefined }, 'unsupported_type');
});

test('DATA-03 canonical JSON rejects sparse arrays and unsupported prototypes', () => {
  assertCanonicalError(new Array(1), 'sparse_array');
  assertCanonicalError(new Date(0), 'unsupported_prototype');
  assertCanonicalError(Object.create(null), 'unsupported_prototype');
});

test('DATA-03 canonical JSON rejects cycles and bounded-work overflows', () => {
  const cyclic: { self?: unknown } = {};
  cyclic.self = cyclic;
  assertCanonicalError(cyclic, 'cyclic_value');

  let nested: unknown = null;
  for (let index = 0; index <= MAX_CANONICAL_DEPTH; index += 1) nested = [nested];
  assertCanonicalError(nested, 'max_depth');
  assertCanonicalError(Array.from({ length: MAX_CANONICAL_NODES }, () => null), 'max_nodes');
  assertCanonicalError('a'.repeat(MAX_CANONICAL_STRING_BYTES + 1), 'max_string_bytes');
});

test('DATA-03 canonical JSON rejects duplicate-key JSON before canonicalization', () => {
  assert.throws(
    () => parseJsonWithDuplicateKeyCheck('{"safe":1,"safe":2}'),
    (error: unknown) => error instanceof CanonicalJsonError && error.code === 'duplicate_key',
  );
});

test('DATA-03 formatting and property insertion order do not change canonical identity', () => {
  const first = { alpha: 1, nested: { left: true, right: null } };
  const second = JSON.parse('{\n  "nested": { "right": null, "left": true },\n  "alpha": 1\n}') as JsonValue;

  assert.equal(canonicalJson(first), canonicalJson(second));
  assert.equal(identityHash(first), identityHash(second));
  assert.match(identityHash(first), /^sha256:[0-9a-f]{64}$/);
});

test('DATA-03 a one-byte source change changes its raw SHA-256', () => {
  const encoder = new TextEncoder();
  assert.equal(
    sha256(encoder.encode('a')),
    'sha256:ca978112ca1bbdcafac231b39a23dc4da786eff8147c4e72b9807785afee48bb',
  );
  assert.notEqual(sha256(encoder.encode('a')), sha256(encoder.encode('b')));
});

test('DATA-03 a payload change changes the artifact hash without self-hashing contentHash', () => {
  const identity = {
    artifactKind: 'card',
    stableId: 'card:synthetic-one',
    schemaVersion: 1,
    parentRefs: [],
    sourceRefs: [],
    payload: { cost: 1 },
  };

  assert.notEqual(identityHash(identity), identityHash({ ...identity, payload: { cost: 2 } }));
});
