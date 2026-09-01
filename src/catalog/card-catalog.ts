import { randomUUID } from 'node:crypto';
import { chmod, link, lstat, mkdir, readdir, realpath, unlink } from 'node:fs/promises';
import { dirname, join, relative, resolve } from 'node:path';
import { DatabaseSync } from 'node:sqlite';
import { z } from 'zod';

import { canonicalJson, type JsonValue } from '../authority/canonical-json.ts';
import { identityHash, sha256 } from '../authority/hash.ts';
import {
  DEFAULT_AUTHORITY_JSON_LIMITS,
  jsonValueSchema,
  normalizedCardSnapshotSchema,
  parseAuthorityJson,
  type CanonicalArtifact,
  type Hash,
  type NormalizedCardSnapshot,
} from '../authority/schemas.ts';
import {
  readBoundedWithinAuthorityRoot,
  validateAuthorityBundle,
} from '../authority/validate-bundle.ts';

const HASH_PATTERN = /^sha256:[0-9a-f]{64}$/u;
const REVISION_PATTERN = /^[a-z0-9][a-z0-9._-]{0,99}$/u;
const MAX_RECEIPT_BYTES = 256_000;
const MAX_REVISION_BYTES = 32_000_000;
const MAX_REVISION_FILES = 512;

const receiptSchema = z.strictObject({
  acquisitionMethod: z.string().min(1).max(200),
  attestation: z.strictObject({
    artwork: z.boolean(),
    commercialUse: z.boolean(),
    hosting: z.boolean(),
    publicApi: z.boolean(),
    publisherPermission: z.boolean(),
    recurringAcquisition: z.boolean(),
    redistribution: z.boolean(),
    release: z.boolean(),
    scope: z.string().min(1).max(200),
    thirdPartyUpload: z.boolean(),
  }),
  authorizationReference: z.string().min(1).max(300),
  bundleRootHash: z.string().regex(HASH_PATTERN),
  gitContainsCorpus: z.literal(false),
  inputRootHash: z.string().regex(HASH_PATTERN),
  revisionFileMapHash: z.string().regex(HASH_PATTERN),
  revisionId: z.string().regex(REVISION_PATTERN),
  schemaVersion: z.literal(1),
  sourceSetRootHash: z.string().regex(HASH_PATTERN),
  sourceSets: z.strictObject({
    backup: z.string().min(1).max(300),
    primary: z.string().min(1).max(300),
  }),
  sources: z.array(jsonValueSchema).max(512),
  stableId: z.string().min(1).max(200),
});

const SCHEMA = `
PRAGMA foreign_keys = ON;
CREATE TABLE catalog_revision (
  revision_id TEXT PRIMARY KEY CHECK (length(trim(revision_id)) > 0),
  bundle_content_hash TEXT NOT NULL UNIQUE,
  artifact_kind TEXT NOT NULL CHECK (artifact_kind = 'card-snapshot'),
  artifact_stable_id TEXT NOT NULL,
  artifact_schema_version INTEGER NOT NULL CHECK (artifact_schema_version = 1),
  artifact_content_hash TEXT NOT NULL UNIQUE,
  card_count INTEGER NOT NULL CHECK (card_count >= 0)
) STRICT;
CREATE TABLE card (
  stable_id TEXT PRIMARY KEY,
  official_source_id TEXT,
  name TEXT NOT NULL CHECK (length(name) > 0),
  card_type TEXT NOT NULL CHECK (card_type IN ('avatar', 'site', 'minion', 'aura', 'artifact', 'magic')),
  rarity TEXT CHECK (rarity IS NULL OR rarity IN ('ordinary', 'exceptional', 'elite', 'unique')),
  mana_cost INTEGER CHECK (mana_cost IS NULL OR mana_cost >= 0),
  attack INTEGER CHECK (attack IS NULL OR attack >= 0),
  defense INTEGER CHECK (defense IS NULL OR defense >= 0),
  life INTEGER CHECK (life IS NULL OR life >= 0),
  threshold_air INTEGER NOT NULL CHECK (threshold_air >= 0),
  threshold_earth INTEGER NOT NULL CHECK (threshold_earth >= 0),
  threshold_fire INTEGER NOT NULL CHECK (threshold_fire >= 0),
  threshold_water INTEGER NOT NULL CHECK (threshold_water >= 0),
  rules_text TEXT NOT NULL
) STRICT;
CREATE TABLE card_source_mapping (
  card_stable_id TEXT NOT NULL REFERENCES card(stable_id) ON DELETE CASCADE,
  source TEXT NOT NULL CHECK (length(trim(source)) > 0),
  source_card_id TEXT NOT NULL CHECK (length(trim(source_card_id)) > 0),
  PRIMARY KEY (card_stable_id, source, source_card_id),
  UNIQUE (source, source_card_id)
) STRICT;
CREATE TABLE card_element (
  card_stable_id TEXT NOT NULL REFERENCES card(stable_id) ON DELETE CASCADE,
  ordinal INTEGER NOT NULL CHECK (ordinal >= 0),
  element TEXT NOT NULL CHECK (element IN ('earth', 'fire', 'water', 'air')),
  PRIMARY KEY (card_stable_id, ordinal),
  UNIQUE (card_stable_id, element)
) STRICT;
CREATE TABLE card_subtype (
  card_stable_id TEXT NOT NULL REFERENCES card(stable_id) ON DELETE CASCADE,
  ordinal INTEGER NOT NULL CHECK (ordinal >= 0),
  subtype TEXT NOT NULL CHECK (length(subtype) > 0),
  PRIMARY KEY (card_stable_id, ordinal),
  UNIQUE (card_stable_id, subtype)
) STRICT;
CREATE TABLE printing_alias (
  alias TEXT PRIMARY KEY,
  card_stable_id TEXT NOT NULL REFERENCES card(stable_id) ON DELETE CASCADE,
  ordinal INTEGER NOT NULL CHECK (ordinal >= 0),
  UNIQUE (card_stable_id, ordinal)
) STRICT;
CREATE TABLE price_snapshot (
  snapshot_id TEXT PRIMARY KEY CHECK (length(trim(snapshot_id)) > 0),
  catalog_revision_id TEXT NOT NULL REFERENCES catalog_revision(revision_id),
  observed_at_unix_ms INTEGER NOT NULL CHECK (observed_at_unix_ms >= 0)
) STRICT;
CREATE TABLE price_quote (
  snapshot_id TEXT NOT NULL REFERENCES price_snapshot(snapshot_id) ON DELETE CASCADE,
  source TEXT NOT NULL,
  source_card_id TEXT NOT NULL,
  printing_id TEXT NOT NULL CHECK (length(trim(printing_id)) > 0),
  variant TEXT NOT NULL CHECK (length(trim(variant)) > 0),
  condition TEXT NOT NULL CHECK (length(trim(condition)) > 0),
  currency TEXT NOT NULL CHECK (length(trim(currency)) > 0),
  unit_price_minor INTEGER NOT NULL CHECK (unit_price_minor >= 0),
  PRIMARY KEY (
    snapshot_id, source, source_card_id, printing_id, variant, condition, currency
  ),
  FOREIGN KEY (source, source_card_id) REFERENCES card_source_mapping(source, source_card_id)
) STRICT;
CREATE INDEX card_name_lookup ON card(name COLLATE NOCASE);
CREATE INDEX card_type_lookup ON card(card_type);
CREATE INDEX card_source_lookup ON card_source_mapping(card_stable_id, source, source_card_id);
CREATE INDEX printing_card_lookup ON printing_alias(card_stable_id, ordinal);
CREATE INDEX price_quote_lookup ON price_quote(
  source, source_card_id, currency, condition, variant, unit_price_minor, printing_id
);
`;

export type CardCatalogBuild = Readonly<{
  artifactContentHash: Hash;
  bundleContentHash: Hash;
  cardCount: number;
  outputPath: string;
  revisionId: string;
}>;

async function rejectLink(path: string, label: string): Promise<void> {
  try {
    if ((await lstat(path)).isSymbolicLink()) throw new Error(`${label} may not be a symbolic link or junction`);
  } catch (error) {
    if ((error as NodeJS.ErrnoException).code !== 'ENOENT') throw error;
  }
}

async function confinedPrivateRoots(root: string): Promise<Readonly<{
  authorityRoot: string;
  resolvedAuthorityRoot: string;
  revisionsRoot: string;
}>> {
  const localRoot = resolve(root, '.local');
  const authorityRoot = resolve(localRoot, 'authority');
  const revisionsRoot = resolve(authorityRoot, 'revisions');
  for (const [path, label] of [
    [root, 'repository root'],
    [localRoot, 'private local root'],
    [authorityRoot, 'private authority root'],
    [revisionsRoot, 'private revisions root'],
  ] as const) await rejectLink(path, label);
  const [resolvedRoot, resolvedLocalRoot, resolvedAuthorityRoot, resolvedRevisionsRoot] =
    await Promise.all([
      realpath(root),
      realpath(localRoot),
      realpath(authorityRoot),
      realpath(revisionsRoot),
    ]);
  if (dirname(resolvedLocalRoot) !== resolvedRoot
    || dirname(resolvedAuthorityRoot) !== resolvedLocalRoot
    || dirname(resolvedRevisionsRoot) !== resolvedAuthorityRoot) {
    throw new Error('private authority roots must be direct non-aliased repository descendants');
  }
  return { authorityRoot, resolvedAuthorityRoot, revisionsRoot: resolvedRevisionsRoot };
}

async function revisionFileMapHash(revisionRoot: string): Promise<Hash> {
  const entries = await readdir(revisionRoot, { recursive: true, withFileTypes: true });
  if (entries.some((entry) => !entry.isFile() && !entry.isDirectory())) {
    throw new Error('authority revision may contain only ordinary files and directories');
  }
  const paths = entries
    .filter((entry) => entry.isFile())
    .map((entry) => relative(revisionRoot, join(entry.parentPath, entry.name)).replaceAll('\\', '/'))
    .sort();
  if (paths.length > MAX_REVISION_FILES) throw new Error('authority revision exceeds the fixed file limit');

  let remainingBytes = MAX_REVISION_BYTES;
  const mapped: JsonValue[] = [];
  for (const path of paths) {
    const read = await readBoundedWithinAuthorityRoot(revisionRoot, path, remainingBytes);
    if (read.status !== 'ok') throw new Error(`authority revision file is ${read.status}`);
    remainingBytes -= read.bytes.byteLength;
    mapped.push({
      byteHash: sha256(read.bytes),
      byteLength: read.bytes.byteLength,
      relativePath: path,
    });
  }
  return identityHash(mapped);
}

function populate(
  database: DatabaseSync,
  revisionId: string,
  bundleContentHash: Hash,
  artifact: CanonicalArtifact<JsonValue>,
  snapshot: NormalizedCardSnapshot,
): void {
  database.exec(SCHEMA);
  const revision = database.prepare(`
    INSERT INTO catalog_revision VALUES (?, ?, ?, ?, ?, ?, ?)
  `);
  const card = database.prepare(`
    INSERT INTO card VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
  `);
  const sourceMapping = database.prepare('INSERT INTO card_source_mapping VALUES (?, ?, ?)');
  const element = database.prepare('INSERT INTO card_element VALUES (?, ?, ?)');
  const subtype = database.prepare('INSERT INTO card_subtype VALUES (?, ?, ?)');
  const printing = database.prepare('INSERT INTO printing_alias VALUES (?, ?, ?)');

  database.exec('BEGIN IMMEDIATE');
  try {
    revision.run(
      revisionId,
      bundleContentHash,
      artifact.identity.artifactKind,
      artifact.identity.stableId,
      artifact.identity.schemaVersion,
      artifact.contentHash,
      snapshot.cards.length,
    );
    for (const value of snapshot.cards) {
      card.run(
        value.stableId,
        value.officialSourceId,
        value.name,
        value.cardType,
        value.rarity,
        value.manaCost,
        value.attack,
        value.defense,
        value.life,
        value.thresholds.air,
        value.thresholds.earth,
        value.thresholds.fire,
        value.thresholds.water,
        value.rulesText,
      );
      value.elements.forEach((valueElement, ordinal) => element.run(value.stableId, ordinal, valueElement));
      value.subtypes.forEach((valueSubtype, ordinal) => subtype.run(value.stableId, ordinal, valueSubtype));
      value.printingSlugs.forEach((alias, ordinal) => printing.run(alias, value.stableId, ordinal));
      if (value.officialSourceId !== null) {
        for (const reference of artifact.identity.sourceRefs) {
          sourceMapping.run(value.stableId, reference.sourceId, value.officialSourceId);
        }
      }
    }
    database.exec('COMMIT');
  } catch (error) {
    database.exec('ROLLBACK');
    throw error;
  }
}

function isErrno(error: unknown, code: string): boolean {
  return error instanceof Error
    && 'code' in error
    && (error as NodeJS.ErrnoException).code === code;
}

async function validatedCards(
  root: string,
  revisionsRoot: string,
  revisionId: string,
): Promise<Readonly<{
  artifact: CanonicalArtifact<JsonValue>;
  bundleContentHash: Hash;
  snapshot: NormalizedCardSnapshot;
}>> {
  const receiptRead = await readBoundedWithinAuthorityRoot(
    root,
    `data/authority/receipts/${revisionId}.json`,
    MAX_RECEIPT_BYTES,
  );
  if (receiptRead.status !== 'ok') {
    throw new Error(`authority selection receipt is ${receiptRead.status}`);
  }
  const parsed = parseAuthorityJson(receiptRead.bytes, {
    ...DEFAULT_AUTHORITY_JSON_LIMITS,
    maxBytes: MAX_RECEIPT_BYTES,
  });
  if (!Buffer.from(canonicalJson(parsed), 'utf8').equals(Buffer.from(receiptRead.bytes))) {
    throw new Error('authority selection receipt must use canonical JSON bytes');
  }
  const receipt = receiptSchema.parse(parsed);
  if (receipt.revisionId !== revisionId || receipt.stableId !== `bundle:${revisionId}`) {
    throw new Error('authority selection receipt does not select the requested revision');
  }

  const validated = await validateAuthorityBundle(
    revisionsRoot,
    `${revisionId}/bundle.json`,
    { stableId: receipt.stableId, contentHash: receipt.bundleRootHash },
  );
  if (validated.bundle.identity.payload.inputRootHash !== receipt.inputRootHash) {
    throw new Error('authority selection receipt input root does not match the validated bundle');
  }
  if (await revisionFileMapHash(dirname(validated.resolvedBundlePath)) !== receipt.revisionFileMapHash) {
    throw new Error('authority selection receipt file map does not match the validated revision');
  }
  const cardArtifacts = validated.bundle.identity.payload.artifacts.filter(
    (artifact) => artifact.identity.artifactKind === 'card-snapshot',
  );
  if (cardArtifacts.length !== 1) {
    throw new Error('validated authority bundle must contain exactly one card snapshot');
  }
  const artifact = cardArtifacts[0]!;
  if (artifact.identity.sourceRefs.length !== 1) {
    throw new Error('validated card snapshot must identify exactly one source namespace');
  }
  return {
    artifact,
    bundleContentHash: validated.bundle.contentHash,
    snapshot: normalizedCardSnapshotSchema.parse(artifact.identity.payload),
  };
}

export async function buildCardCatalog(
  revisionId: string,
  repositoryRoot = process.cwd(),
): Promise<CardCatalogBuild> {
  if (!REVISION_PATTERN.test(revisionId)) {
    throw new TypeError('catalog revision ID must be one confined path segment');
  }
  const root = resolve(repositoryRoot);
  const { authorityRoot, resolvedAuthorityRoot, revisionsRoot } = await confinedPrivateRoots(root);
  const { artifact, bundleContentHash, snapshot } = await validatedCards(root, revisionsRoot, revisionId);

  const catalogRoot = resolve(authorityRoot, 'catalog');
  await rejectLink(catalogRoot, 'private catalog root');
  await mkdir(catalogRoot, { recursive: true });
  const resolvedCatalogRoot = await realpath(catalogRoot);
  if (dirname(resolvedCatalogRoot) !== resolvedAuthorityRoot) {
    throw new Error('catalog output resolves outside the private authority boundary');
  }
  const destination = join(resolvedCatalogRoot, `${revisionId}.sqlite3`);
  const temporary = join(resolvedCatalogRoot, `.${revisionId}.${randomUUID()}.tmp`);
  let database: DatabaseSync | undefined;
  let primaryError: unknown;
  try {
    database = new DatabaseSync(temporary, { enableForeignKeyConstraints: true });
    populate(database, revisionId, bundleContentHash, artifact, snapshot);
    database.close();
    database = undefined;
    await chmod(temporary, 0o600);
    await link(temporary, destination);
  } catch (error) {
    primaryError = error;
    throw error;
  } finally {
    let cleanupError: unknown;
    if (database !== undefined) {
      try {
        database.close();
      } catch (error) {
        cleanupError = error;
      }
    }
    try {
      await unlink(temporary);
    } catch (error) {
      if (!isErrno(error, 'ENOENT')) cleanupError ??= error;
    }
    if (primaryError === undefined && cleanupError !== undefined) throw cleanupError;
  }
  return {
    artifactContentHash: artifact.contentHash,
    bundleContentHash,
    cardCount: snapshot.cards.length,
    outputPath: destination,
    revisionId,
  };
}
