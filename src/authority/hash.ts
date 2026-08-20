import { createHash } from 'node:crypto';

import { canonicalJson, type JsonValue } from './canonical-json.ts';

export function sha256(bytes: Uint8Array): `sha256:${string}` {
  return `sha256:${createHash('sha256').update(bytes).digest('hex')}`;
}

export function identityHash(value: JsonValue): `sha256:${string}` {
  return sha256(new TextEncoder().encode(canonicalJson(value)));
}
