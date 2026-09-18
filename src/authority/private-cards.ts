import { readFile } from 'node:fs/promises';
import { basename, resolve } from 'node:path';

import { parseJsonWithDuplicateKeyCheck, type JsonValue } from './canonical-json.ts';
import { identityHash } from './hash.ts';
import {
  canonicalArtifactSchema,
  formatArtifactSchema,
  normalizedCardSnapshotSchema,
  type FormatDefinition,
  type Hash,
  type NormalizedCard,
} from './schemas.ts';

const DEFAULT_SCENARIO = resolve(
  process.cwd(),
  '.local',
  'authority',
  'scenarios',
  'vanilla-constructed.json',
);

function isJsonRecord(value: JsonValue): value is Record<string, JsonValue> {
  return typeof value === 'object' && value !== null && !Array.isArray(value);
}

function revisionIdFromScenario(value: JsonValue): string {
  if (!isJsonRecord(value) || typeof value.revisionId !== 'string' || value.revisionId.trim() === '') {
    throw new Error('private game scenario must include revisionId');
  }
  return value.revisionId;
}

export type PrivateCardSnapshot = Readonly<{
  authorityHash: Hash;
  cards: readonly NormalizedCard[];
  format: FormatDefinition;
  revisionId: string;
}>;

export async function loadPrivateCardSnapshot(
  scenarioPath = DEFAULT_SCENARIO,
  repositoryRoot = process.cwd(),
): Promise<PrivateCardSnapshot> {
  const config = parseJsonWithDuplicateKeyCheck(await readFile(scenarioPath, 'utf8'));
  const revisionId = revisionIdFromScenario(config);
  const revisionRoot = resolve(repositoryRoot, '.local', 'authority', 'revisions', revisionId);
  if (basename(revisionRoot) !== revisionId) {
    throw new Error('private revision ID must be one path segment');
  }

  const artifact = canonicalArtifactSchema.parse(parseJsonWithDuplicateKeyCheck(
    await readFile(resolve(revisionRoot, 'cards.normalized.json'), 'utf8'),
  ));
  if (artifact.identity.artifactKind !== 'card-snapshot'
    || identityHash(artifact.identity as unknown as JsonValue) !== artifact.contentHash) {
    throw new Error('private normalized card artifact identity is invalid');
  }
  const snapshot = normalizedCardSnapshotSchema.parse(artifact.identity.payload);

  const formatsValue = parseJsonWithDuplicateKeyCheck(
    await readFile(resolve(revisionRoot, 'formats.json'), 'utf8'),
  );
  if (!isJsonRecord(formatsValue) || !Array.isArray(formatsValue.formats)) {
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
    .sort((left, right) =>
      right.identity.payload.effectiveDate.localeCompare(left.identity.payload.effectiveDate),
    )[0];
  if (!selected) throw new Error('private authority has no Constructed format');

  return {
    authorityHash: artifact.contentHash,
    cards: snapshot.cards,
    format: selected.identity.payload,
    revisionId,
  };
}
