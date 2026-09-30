import { chmod, lstat, mkdir, readFile, realpath, writeFile } from 'node:fs/promises';
import { dirname, isAbsolute, join, relative, resolve } from 'node:path';
import { DatabaseSync } from 'node:sqlite';

import { parseJsonWithDuplicateKeyCheck, type JsonValue } from '../authority/canonical-json.ts';
import { loadPrivateCardSnapshot } from '../authority/private-cards.ts';
import { identityHash, sha256 } from '../authority/hash.ts';
import type { NormalizedCard } from '../authority/schemas.ts';

const MAX_INPUT = 16 * 1024 * 1024;
const DB_NAME = 'card-tracker.sqlite3';
const FEED = 'binding-cycles/direct-maps-20260929/spreadsheet-feed.json';
const CODEX = 'codex/official-codex.snapshot.json';
const CATALOG = 'catalog/current-bindings.catalog.json';
const REVIEWED = 'bindings/reviewed.json';

export const CARD_TRACKER_SCHEMA = `
PRAGMA foreign_keys = ON;
CREATE TABLE IF NOT EXISTS tracker_meta (
  key TEXT PRIMARY KEY,
  value TEXT NOT NULL
) STRICT;
CREATE TABLE IF NOT EXISTS cards (
  card_id TEXT PRIMARY KEY,
  official_source_id TEXT,
  name TEXT NOT NULL,
  card_type TEXT NOT NULL,
  elements_json TEXT NOT NULL,
  stats_json TEXT NOT NULL,
  printed_text TEXT NOT NULL,
  source_hash TEXT NOT NULL,
  authority_hash TEXT NOT NULL,
  authority_revision TEXT NOT NULL,
  binding_status TEXT NOT NULL CHECK (binding_status IN ('admitted-current', 'unbound')),
  facts_hash TEXT,
  facts_json TEXT,
  is_current INTEGER NOT NULL CHECK (is_current IN (0, 1))
) STRICT;
CREATE TABLE IF NOT EXISTS card_status (
  card_id TEXT PRIMARY KEY REFERENCES cards(card_id),
  feed_status TEXT NOT NULL,
  review_status TEXT NOT NULL,
  validation_status TEXT NOT NULL,
  validation_ready_annotation INTEGER NOT NULL CHECK (validation_ready_annotation IN (0, 1)),
  source_review_status TEXT NOT NULL CHECK (source_review_status IN ('source-reviewed', 'source-review-required', 'pending')),
  source_review_hash TEXT,
  dependency_groups_json TEXT NOT NULL,
  blockers_json TEXT NOT NULL,
  selected_deck_frequency INTEGER CHECK (selected_deck_frequency IS NULL OR selected_deck_frequency >= 0),
  payload_json TEXT NOT NULL
) STRICT;
CREATE TABLE IF NOT EXISTS manual_card_status (
  card_id TEXT PRIMARY KEY REFERENCES cards(card_id),
  status TEXT NOT NULL DEFAULT 'pending',
  note TEXT NOT NULL DEFAULT '',
  updated_at TEXT
) STRICT;
CREATE TABLE IF NOT EXISTS source_reviews (
  card_id TEXT NOT NULL REFERENCES cards(card_id),
  source_hash TEXT NOT NULL,
  proofs_json TEXT NOT NULL,
  PRIMARY KEY (card_id, source_hash)
) STRICT;
CREATE TABLE IF NOT EXISTS codex_entries (
  codex_id TEXT PRIMARY KEY,
  title TEXT NOT NULL,
  url TEXT NOT NULL,
  parent_id TEXT REFERENCES codex_entries(codex_id),
  article_text TEXT NOT NULL,
  entry_text TEXT NOT NULL,
  source_hash TEXT NOT NULL,
  source_snapshot_hash TEXT NOT NULL,
  is_current INTEGER NOT NULL CHECK (is_current IN (0, 1))
) STRICT;
CREATE TABLE IF NOT EXISTS capability_groups (
  group_id TEXT PRIMARY KEY,
  title TEXT NOT NULL,
  rank INTEGER NOT NULL,
  scope TEXT NOT NULL,
  dependent_remaining_count INTEGER NOT NULL,
  conditional_family_reach_count INTEGER NOT NULL,
  payload_json TEXT NOT NULL,
  source_hash TEXT NOT NULL,
  is_current INTEGER NOT NULL CHECK (is_current IN (0, 1))
) STRICT;
CREATE TABLE IF NOT EXISTS rule_slices (
  slice_id TEXT PRIMARY KEY,
  requirement_id TEXT NOT NULL,
  card_id TEXT NOT NULL REFERENCES cards(card_id),
  group_id TEXT NOT NULL REFERENCES capability_groups(group_id),
  implementation_status TEXT NOT NULL,
  authored_family TEXT NOT NULL,
  payload_json TEXT NOT NULL,
  source_hash TEXT NOT NULL,
  is_current INTEGER NOT NULL CHECK (is_current IN (0, 1))
) STRICT;
CREATE TABLE IF NOT EXISTS card_requirements (
  card_id TEXT NOT NULL REFERENCES cards(card_id),
  slice_id TEXT NOT NULL REFERENCES rule_slices(slice_id),
  source_card_hash TEXT NOT NULL,
  is_current INTEGER NOT NULL CHECK (is_current IN (0, 1)),
  PRIMARY KEY (card_id, slice_id)
) STRICT;
CREATE TABLE IF NOT EXISTS codex_refs (
  scope TEXT NOT NULL CHECK (scope IN ('group', 'slice', 'card')),
  scope_id TEXT NOT NULL,
  codex_id TEXT NOT NULL REFERENCES codex_entries(codex_id),
  relationship TEXT NOT NULL,
  applicability TEXT NOT NULL,
  PRIMARY KEY (scope, scope_id, codex_id, relationship, applicability)
) STRICT;
CREATE TABLE IF NOT EXISTS proof_metadata (
  proof_key TEXT PRIMARY KEY,
  card_id TEXT NOT NULL REFERENCES cards(card_id),
  slice_id TEXT NOT NULL REFERENCES rule_slices(slice_id),
  proof_reference TEXT NOT NULL,
  status TEXT NOT NULL CHECK (status IN ('referenced-unverified', 'verified', 'stale', 'failed')),
  source_hash TEXT NOT NULL,
  slice_hash TEXT NOT NULL,
  engine_hash TEXT NOT NULL,
  evidence_hash TEXT
) STRICT;
CREATE TABLE IF NOT EXISTS proof_references (
  card_id TEXT NOT NULL REFERENCES cards(card_id),
  source_hash TEXT NOT NULL,
  reference TEXT NOT NULL,
  status TEXT NOT NULL CHECK (status = 'referenced-unverified'),
  PRIMARY KEY (card_id, source_hash, reference)
) STRICT;
CREATE TABLE IF NOT EXISTS validation_jobs (
  dependency_fingerprint TEXT PRIMARY KEY,
  card_id TEXT NOT NULL REFERENCES cards(card_id),
  slice_id TEXT NOT NULL REFERENCES rule_slices(slice_id),
  source_hash TEXT NOT NULL,
  slice_hash TEXT NOT NULL,
  engine_hash TEXT NOT NULL,
  status TEXT NOT NULL CHECK (status IN ('pending', 'blocked-binding', 'blocked-source-review', 'leased', 'complete', 'failed')),
  created_at TEXT NOT NULL,
  result_hash TEXT
) STRICT;
CREATE INDEX IF NOT EXISTS card_current_status ON cards(is_current, binding_status, card_type);
CREATE INDEX IF NOT EXISTS requirement_card_lookup ON card_requirements(card_id, is_current);
CREATE INDEX IF NOT EXISTS requirement_slice_lookup ON card_requirements(slice_id, is_current);
CREATE INDEX IF NOT EXISTS validation_job_status ON validation_jobs(status, created_at);
CREATE INDEX IF NOT EXISTS codex_parent_lookup ON codex_entries(parent_id, codex_id);
`;

type Row = Record<string, JsonValue>;

function record(value: JsonValue, label: string): Row {
  if (typeof value !== 'object' || value === null || Array.isArray(value)) {
    throw new Error(`${label} must be an object`);
  }
  return value as Row;
}

function array(value: JsonValue, label: string): JsonValue[] {
  if (!Array.isArray(value)) throw new Error(`${label} must be an array`);
  return value;
}

function string(value: JsonValue, label: string): string {
  if (typeof value !== 'string' || value.length === 0) throw new Error(`${label} must be a nonempty string`);
  return value;
}

function text(value: JsonValue, label: string): string {
  if (typeof value !== 'string') throw new Error(`${label} must be text`);
  return value;
}

function number(value: JsonValue, label: string): number {
  if (typeof value !== 'number' || !Number.isSafeInteger(value)) throw new Error(`${label} must be an integer`);
  return value;
}

function bool(value: JsonValue, label: string): boolean {
  if (typeof value !== 'boolean') throw new Error(`${label} must be boolean`);
  return value;
}

function json(value: JsonValue): string {
  const serialized = JSON.stringify(value);
  if (serialized === undefined) throw new Error('tracker values must be JSON-serializable');
  return serialized;
}

function requireHash(value: JsonValue, label: string): string {
  const text = string(value, label);
  if (!/^sha256:[a-f0-9]{64}$/u.test(text)) throw new Error(`${label} must be a SHA-256 identity`);
  return text;
}

export function assertExactSourceHash(expected: JsonValue, actual: JsonValue, label: string): void {
  if (requireHash(expected, `${label} expected hash`) !== requireHash(actual, `${label} actual hash`)) {
    throw new Error(`${label} source hash mismatch`);
  }
}

function entryRows(snapshot: Row, snapshotHash: string): Row[] {
  const entries = array(snapshot.entries!, 'Codex entries');
  const result: Row[] = [];
  for (const raw of entries) {
    const entry = record(raw, 'Codex article');
    const id = string(entry.id!, 'Codex article ID');
    result.push({
      id: id as JsonValue,
      title: entry.title!,
      url: entry.url!,
      parentId: null,
      articleText: JSON.stringify(entry.content),
      entryText: text(entry.text!, 'Codex article text'),
      sourceHash: sha256(Buffer.from(JSON.stringify(entry))),
      snapshotHash,
    } as Row);
    for (const childRaw of array(entry.subentries!, 'Codex subentries')) {
      const child = record(childRaw, 'Codex subentry');
      result.push({
        id: child.id!,
        title: child.title!,
        url: child.url!,
        parentId: id,
        articleText: JSON.stringify(entry.content),
        entryText: text(child.text!, 'Codex subentry text'),
        sourceHash: sha256(Buffer.from(JSON.stringify(child))),
        snapshotHash,
      } as Row);
    }
  }
  const actualTop = entries.length;
  const expectedTop = number(snapshot.primaryEntryCount!, 'Codex primary count');
  const expectedChildren = number(snapshot.subentryCount!, 'Codex subentry count');
  if (actualTop !== expectedTop || result.length - actualTop !== expectedChildren
    || result.length !== number(snapshot.totalEntryCount!, 'Codex total count')) {
    throw new Error('Codex snapshot entry counts do not match its pinned summary');
  }
  if (new Set(result.map((row) => row.id)).size !== result.length) {
    throw new Error('Codex snapshot contains duplicate exact IDs');
  }
  return result;
}

function validateFeedReferences(feed: Row, codex: readonly Row[]): void {
  const codexById = new Map(codex.map((entry) => [entry.id, entry]));
  const check = (raw: JsonValue, label: string): void => {
    const ref = record(raw, label);
    const found = codexById.get(ref.id!);
    if (!found) throw new Error(`${label} has an unknown exact Codex ID`);
    if (ref.title !== found.title) throw new Error(`${label} title differs from the exact Codex entry`);
    if (ref.url !== found.url) throw new Error(`${label} URL differs from the exact Codex entry`);
    if (ref.relationship !== undefined && typeof ref.relationship !== 'string') throw new Error(`${label} relationship must be text when provided`);
    if (ref.applicability !== undefined && typeof ref.applicability !== 'string') throw new Error(`${label} applicability must be text when provided`);
  };
  for (const groupRaw of array(feed.groups!, 'Capability groups')) {
    const group = record(groupRaw, 'Capability group');
    for (const ref of array(group.codexReferences!, 'Group Codex references')) check(ref, 'group Codex reference');
  }
  for (const cardRaw of array(feed.cards!, 'Feed cards')) {
    const card = record(cardRaw, 'Feed card');
    for (const ref of array(card.directCodexReferences!, 'Direct card Codex references')) check(ref, 'card Codex reference');
    for (const requirementRaw of array(card.requirements!, 'Card requirements')) {
      const requirement = record(requirementRaw, 'Card requirement');
      for (const ref of array(requirement.codexReferences!, 'Requirement Codex references')) check(ref, 'requirement Codex reference');
    }
  }
}

export type TrackerSyncResult = Readonly<{
  cardCount: number;
  codexEntryCount: number;
  groupCount: number;
  requirementCount: number;
  sourceReviewCandidateCount: number;
  sourceReviewedCount: number;
  boundCount: number;
  databasePath: string;
  authorityHash: string;
  revisionId: string;
  feedHash: string;
  engineHash: string;
}>;

async function engineSourceHash(repositoryRoot: string): Promise<string> {
  const { readdir } = await import('node:fs/promises');
  const paths = [
    join(repositoryRoot, 'Cargo.lock'),
    join(repositoryRoot, 'crates/sorcery-engine/Cargo.toml'),
  ];
  async function walk(dir: string): Promise<void> {
    for (const item of await readdir(dir, { withFileTypes: true })) {
      const path = join(dir, item.name);
      if (item.isDirectory()) await walk(path);
      else if (item.isFile() && path.endsWith('.rs')) paths.push(path);
    }
  }
  await walk(join(repositoryRoot, 'crates/sorcery-engine/src'));
  await walk(join(repositoryRoot, 'crates/sorcery-engine/tests'));
  paths.sort();
  const inputs: JsonValue[] = [];
  for (const path of paths) {
    inputs.push({ path: relative(repositoryRoot, path).replaceAll('\\', '/'), hash: sha256(await readFile(path)) });
  }
  return identityHash(inputs);
}

function ensurePrivateDatabaseRoot(repositoryRoot: string): string {
  const root = resolve(repositoryRoot);
  const authorityRoot = join(root, '.local', 'authority');
  const catalogRoot = join(authorityRoot, 'catalog');
  if (isAbsolute(relative(root, catalogRoot)) || relative(root, catalogRoot).startsWith('..')) {
    throw new Error('private catalog path escapes repository root');
  }
  return catalogRoot;
}

function upsertRows(database: DatabaseSync, rows: Readonly<{
  authorityHash: string;
  revisionId: string;
  cards: readonly NormalizedCard[];
  catalog: Row;
  reviews: Row;
  codex: readonly Row[];
  codexSnapshotHash: string;
  feed: Row;
  feedHash: string;
  engineHash: string;
}>): Readonly<{ boundCount: number; sourceReviewedCount: number; sourceReviewCandidateCount: number; requirementCount: number }> {
  database.exec(CARD_TRACKER_SCHEMA);
  database.exec('BEGIN IMMEDIATE');
  try {
    const putMeta = database.prepare('INSERT INTO tracker_meta(key,value) VALUES (?,?) ON CONFLICT(key) DO UPDATE SET value=excluded.value');
    const cardMap = new Map(array(rows.catalog.cards!, 'Catalog cards').map((value) => {
      const card = record(value, 'Catalog card');
      return [string(card.cardId!, 'Catalog card ID'), card] as const;
    }));
    const reviewMap = new Map(array(rows.reviews.cards!, 'Source review cards').map((value) => {
      const card = record(value, 'Source review card');
      return [string(card.cardId!, 'Reviewed card ID'), card] as const;
    }));
    const feedMap = new Map(array(rows.feed.cards!, 'Astra feed cards').map((value) => {
      const card = record(value, 'Astra feed card');
      return [string(card.cardId!, 'Feed card ID'), card] as const;
    }));
    if (cardMap.size !== rows.cards.length || feedMap.size !== rows.cards.length
      || reviewMap.size !== array(rows.reviews.cards!, 'Source review cards').length) {
      throw new Error('catalog, reviewed-binding, and feed card partitions do not match expected cardinalities');
    }
    database.exec('UPDATE cards SET is_current=0; UPDATE codex_entries SET is_current=0; UPDATE capability_groups SET is_current=0; UPDATE rule_slices SET is_current=0; UPDATE card_requirements SET is_current=0;');
    const insertGroup = database.prepare(`INSERT INTO capability_groups VALUES (?,?,?,?,?,?,?,?,1)
      ON CONFLICT(group_id) DO UPDATE SET title=excluded.title,rank=excluded.rank,scope=excluded.scope,
      dependent_remaining_count=excluded.dependent_remaining_count,
      conditional_family_reach_count=excluded.conditional_family_reach_count,payload_json=excluded.payload_json,
      source_hash=excluded.source_hash,is_current=1`);
    for (const raw of array(rows.feed.groups!, 'Capability groups')) {
      const group = record(raw, 'Capability group');
      insertGroup.run(string(group.id!, 'Capability group ID'), string(group.title!, 'Group title'),
        number(group.rank!, 'Group rank'), string(group.scope!, 'Group scope'),
        number(group.dependentRemainingCardCount!, 'Remaining dependent count'),
        number(group.conditionalFamilyReachCount!, 'Conditional reach count'), json(group), sha256(Buffer.from(json(group))));
    }
    const insertCodex = database.prepare(`INSERT INTO codex_entries VALUES (?,?,?,?,?,?,?,?,1)
      ON CONFLICT(codex_id) DO UPDATE SET title=excluded.title,url=excluded.url,parent_id=excluded.parent_id,
      article_text=excluded.article_text,entry_text=excluded.entry_text,source_hash=excluded.source_hash,
      source_snapshot_hash=excluded.source_snapshot_hash,is_current=1`);
    for (const entry of rows.codex) {
      insertCodex.run(string(entry.id!, 'Codex ID'), string(entry.title!, 'Codex title'),
        string(entry.url!, 'Codex URL'), typeof entry.parentId === 'string' ? entry.parentId : null,
        text(entry.articleText!, 'Codex article text'),
        text(entry.entryText!, 'Codex entry text'), string(entry.sourceHash!, 'Codex entry hash'),
        rows.codexSnapshotHash);
    }
    const insertCard = database.prepare(`INSERT INTO cards VALUES (?,?,?,?,?,?,?,?,?,?,?,?,?,1)
      ON CONFLICT(card_id) DO UPDATE SET official_source_id=excluded.official_source_id,
      name=excluded.name,card_type=excluded.card_type,elements_json=excluded.elements_json,
      stats_json=excluded.stats_json,printed_text=excluded.printed_text,source_hash=excluded.source_hash,
      authority_hash=excluded.authority_hash,authority_revision=excluded.authority_revision,
      binding_status=excluded.binding_status,facts_hash=excluded.facts_hash,facts_json=excluded.facts_json,is_current=1`);
    const insertStatus = database.prepare(`INSERT INTO card_status VALUES (?,?,?,?,?,?,?,?,?,?,?)
      ON CONFLICT(card_id) DO UPDATE SET feed_status=excluded.feed_status,review_status=excluded.review_status,
      validation_status=excluded.validation_status,validation_ready_annotation=excluded.validation_ready_annotation,
      source_review_status=excluded.source_review_status,source_review_hash=excluded.source_review_hash,
      dependency_groups_json=excluded.dependency_groups_json,blockers_json=excluded.blockers_json,
      selected_deck_frequency=excluded.selected_deck_frequency,payload_json=excluded.payload_json`);
    const insertManual = database.prepare("INSERT INTO manual_card_status(card_id) VALUES (?) ON CONFLICT(card_id) DO NOTHING");
    const insertReview = database.prepare('INSERT INTO source_reviews VALUES (?,?,?) ON CONFLICT(card_id,source_hash) DO UPDATE SET proofs_json=excluded.proofs_json');
    let boundCount = 0;
    let sourceReviewedCount = 0;
    const sourceIds = new Set<string>();
    for (const source of rows.cards) {
      const cardId = source.stableId;
      const catalog = cardMap.get(cardId);
      const feed = feedMap.get(cardId);
      if (!catalog || !feed) throw new Error('feed/catalog is missing a normalized source card');
      const sourceHash = requireHash(catalog.sourceCardHash!, 'Catalog source card hash');
      assertExactSourceHash(sourceHash, identityHash(source as unknown as JsonValue), 'Normalized card');
      assertExactSourceHash(catalog.sourceCardHash!, record(feed.source!, 'Feed source').sourceCardHash!, 'Card');
      if (feed.name !== source.name || feed.cardType !== source.cardType) {
        throw new Error('feed card identity differs from exact normalized card');
      }
      sourceIds.add(cardId);
      const isBound = bool(catalog.engineSupported!, 'catalog engineSupported');
      if ((feed.status === 'admitted') !== isBound) throw new Error('feed admission and current catalog binding partition differ');
      if (isBound) boundCount += 1;
      const review = reviewMap.get(cardId);
      const sourceReviewed = review !== undefined;
      if (review) {
        if (review.sourceCardHash !== sourceHash) throw new Error('reviewed binding source hash is stale');
        const attest = record(review.review!, 'source review attestation');
        if (attest.entireRulesText !== true) throw new Error('source review must cover all printed rules text');
        sourceReviewedCount += 1;
      }
      const stats = {
        manaCost: source.manaCost,
        attack: source.attack,
        defense: source.defense,
        life: source.life,
        thresholds: source.thresholds as unknown as JsonValue,
        rarity: source.rarity,
        subtypes: source.subtypes as unknown as JsonValue,
      };
      insertCard.run(
        cardId, source.officialSourceId, source.name, source.cardType, json(source.elements as unknown as JsonValue),
        json(stats), source.rulesText, sourceHash, rows.authorityHash, rows.revisionId,
        isBound ? 'admitted-current' : 'unbound',
        typeof catalog.factsHash === 'string' ? catalog.factsHash : null,
        catalog.facts === null || catalog.facts === undefined ? null : json(catalog.facts),
      );
      if (review) insertReview.run(cardId, sourceHash, json(record(review.review!, 'source review').proofs!));
      const reqs = array(feed.requirements!, 'Card requirements');
      const feedStatus = string(feed.status!, 'Feed status');
      insertStatus.run(
        cardId, feedStatus, string(feed.reviewStatus!, 'Feed review status'),
        string(feed.validationStatus!, 'Feed validation status'), bool(feed.validationReady!, 'Feed validation readiness') ? 1 : 0,
        sourceReviewed ? 'source-reviewed' : feedStatus === 'remaining' ? 'source-review-required' : 'pending',
        sourceReviewed ? sourceHash : null,
        json(feed.dependencyGroups!), json(feed.explicitBlockers!),
        feed.selectedDeckFrequency === null ? null : number(feed.selectedDeckFrequency!, 'Selected deck frequency'),
        json(feed),
      );
      insertManual.run(cardId);
      const insertSlice = database.prepare(`INSERT INTO rule_slices VALUES (?,?,?,?,?,?,?,?,1)
        ON CONFLICT(slice_id) DO UPDATE SET requirement_id=excluded.requirement_id,card_id=excluded.card_id,
        group_id=excluded.group_id,
        implementation_status=excluded.implementation_status,authored_family=excluded.authored_family,
        payload_json=excluded.payload_json,source_hash=excluded.source_hash,is_current=1`);
      const insertRequirement = database.prepare(`INSERT INTO card_requirements VALUES (?,?,?,1)
        ON CONFLICT(card_id,slice_id) DO UPDATE SET source_card_hash=excluded.source_card_hash,is_current=1`);
      for (const reqRaw of reqs) {
        const req = record(reqRaw, 'Card requirement');
        const requirementId = string(req.id!, 'Requirement ID');
        const sliceId = identityHash({ cardId, requirementId } as unknown as JsonValue);
        const groupId = string(req.groupId!, 'Requirement group ID');
        const sliceHash = sha256(Buffer.from(json(req)));
        insertSlice.run(sliceId, requirementId, cardId, groupId, string(req.implementationStatus!, 'Requirement implementation status'),
          string(req.authoredFamily!, 'Requirement authored family'), json(req), sliceHash);
        insertRequirement.run(cardId, sliceId, sourceHash);
      }
    }
    if (sourceIds.size !== rows.cards.length || boundCount !== number(rows.catalog.boundCardCount!, 'Bound-card count')
      || sourceReviewedCount !== reviewMap.size) {
      throw new Error('private source cardinalities changed; tracker sync requires explicit review');
    }
    database.exec('DELETE FROM codex_refs');
    const insertRef = database.prepare('INSERT INTO codex_refs VALUES (?,?,?,?,?)');
    database.exec('DELETE FROM proof_references');
    const insertProofReference = database.prepare("INSERT INTO proof_references VALUES (?,?,?,'referenced-unverified')");
    for (const raw of array(rows.feed.cards!, 'Feed cards')) {
      const card = record(raw, 'Feed card');
      const cardId = string(card.cardId!, 'Feed card ID');
      const sourceHash = string(record(card.source!, 'Feed source').sourceCardHash!, 'Feed source hash');
      for (const proofRaw of array(card.nativeProofReferences!, 'Native proof references')) {
        const proofReference = string(proofRaw, 'Native proof reference');
        insertProofReference.run(cardId, sourceHash, proofReference);
      }
    }
    for (const raw of array(rows.feed.groups!, 'Capability groups')) {
      const group = record(raw, 'Capability group');
      const groupId = string(group.id!, 'Capability group ID');
      for (const refRaw of array(group.codexReferences!, 'Group Codex references')) {
        const ref = record(refRaw, 'Codex ref');
        insertRef.run('group', groupId, string(ref.id!, 'Codex ID'),
          text(ref.relationship ?? group.codexReferenceRelationship ?? '', 'Codex relationship'),
          text(ref.applicability ?? group.codexApplicability ?? '', 'Codex applicability'));
      }
    }
    for (const raw of array(rows.feed.cards!, 'Feed cards')) {
      const card = record(raw, 'Feed card');
      const cardId = string(card.cardId!, 'Feed card ID');
      for (const refRaw of array(card.directCodexReferences!, 'Direct Codex references')) {
        const ref = record(refRaw, 'Codex ref');
        insertRef.run('card', cardId, string(ref.id!, 'Codex ID'), typeof ref.relationship === 'string' ? ref.relationship : '', typeof ref.applicability === 'string' ? ref.applicability : '');
      }
      for (const reqRaw of array(card.requirements!, 'Card requirements')) {
        const req = record(reqRaw, 'Card requirement');
        for (const refRaw of array(req.codexReferences!, 'Requirement Codex references')) {
          const ref = record(refRaw, 'Codex ref');
          const sliceId = identityHash({ cardId, requirementId: req.id! } as unknown as JsonValue);
          insertRef.run('slice', sliceId, string(ref.id!, 'Codex ID'),
            text(ref.relationship ?? req.codexReferenceRelationship ?? '', 'Codex relationship'),
            text(ref.applicability ?? req.codexApplicability ?? '', 'Codex applicability'));
        }
      }
    }
    const requirements = database.prepare('SELECT count(*) AS n FROM card_requirements WHERE is_current=1').get()!.n as number;
    const sourceReviewCandidateCount = database.prepare("SELECT count(*) AS n FROM card_status WHERE feed_status='remaining'").get()!.n as number;
    const expectedRequirements = array(rows.feed.cards!, 'Feed cards')
      .reduce<number>((total, raw) => total + array(record(raw, 'Feed card').requirements!, 'Card requirements').length, 0);
    if (requirements !== expectedRequirements) throw new Error('card requirement links differ from the exact imported feed');
    for (const [key, value] of Object.entries({
      authorityHash: rows.authorityHash, revisionId: rows.revisionId, feedHash: rows.feedHash,
      engineHash: rows.engineHash, codexSnapshotHash: rows.codexSnapshotHash,
    })) putMeta.run(key, value);
    database.exec('COMMIT');
    return { boundCount, sourceReviewedCount, sourceReviewCandidateCount, requirementCount: requirements };
  } catch (error) {
    database.exec('ROLLBACK');
    throw error;
  }
}

export async function syncPrivateCardTracker(repositoryRoot = process.cwd()): Promise<TrackerSyncResult> {
  const root = resolve(repositoryRoot);
  const authorityRoot = join(root, '.local', 'authority');
  const rootReal = await realpath(root);
  const localInfo = await lstat(join(root, '.local'));
  const authorityInfo = await lstat(authorityRoot);
  if (localInfo.isSymbolicLink() || authorityInfo.isSymbolicLink()
    || await realpath(authorityRoot) !== join(rootReal, '.local', 'authority')) {
    throw new Error('private authority root must be a real directory inside the repository');
  }
  const scenarioPath = join(authorityRoot, 'scenarios', 'vanilla-constructed.json');
  if ((await lstat(scenarioPath)).isSymbolicLink()) throw new Error('private authority scenario may not be linked');
  const authority = await loadPrivateCardSnapshot(scenarioPath, root);
  const read = async (path: string): Promise<Readonly<{ bytes: Buffer; value: JsonValue }>> => {
    const sourcePath = join(authorityRoot, path);
    const sourceInfo = await lstat(sourcePath);
    if (!sourceInfo.isFile() || sourceInfo.isSymbolicLink()) throw new Error('private tracker source must be an ordinary file');
    const resolvedSource = await realpath(sourcePath);
    if (dirname(resolvedSource) === '' || isAbsolute(relative(await realpath(authorityRoot), resolvedSource))
      || relative(await realpath(authorityRoot), resolvedSource).startsWith('..')) {
      throw new Error('private tracker source escapes authority root');
    }
    const bytes = await readFile(resolvedSource);
    if (bytes.byteLength > MAX_INPUT) throw new Error('private tracker source exceeds fixed byte limit');
    return {
      bytes,
      value: parseJsonWithDuplicateKeyCheck(bytes.toString('utf8'), {
        validateCanonical: !path.endsWith('spreadsheet-feed.json'),
        ...(path.endsWith('spreadsheet-feed.json') ? { maxStringBytes: MAX_INPUT, maxNodes: 500_000 } : {}),
      }),
    };
  };
  const [catalogRead, codexRead, reviewRead, feedRead] = await Promise.all([
    read(CATALOG), read(CODEX), read(REVIEWED), read(FEED),
  ]);
  const catalog = record(catalogRead.value, 'Current private catalog');
  const codex = record(codexRead.value, 'Private Codex snapshot');
  const reviews = record(reviewRead.value, 'Reviewed source bindings');
  const feed = record(feedRead.value, 'Astra tracker feed');
  const catalogHash = sha256(catalogRead.bytes);
  const feedHash = sha256(feedRead.bytes);
  const codexSnapshotHash = sha256(codexRead.bytes);
  if (catalog.revisionId !== authority.revisionId || feed.revisionId !== authority.revisionId
    || reviews.revisionId !== authority.revisionId) {
    throw new Error('private tracker sources do not identify the exact current authority revision');
  }
  assertExactSourceHash(catalog.authorityHash!, authority.authorityHash, 'Catalog authority');
  assertExactSourceHash(feed.authorityHash!, authority.authorityHash, 'Feed authority');
  assertExactSourceHash(reviews.authorityHash!, authority.authorityHash, 'Reviewed bindings authority');
  assertExactSourceHash(feed.baselineCatalogHash!, catalogHash, 'Tracker feed catalog baseline');
  if (array(catalog.cards!, 'Current catalog cards').length !== authority.cards.length
    || array(feed.cards!, 'Tracker feed cards').length !== authority.cards.length) {
    throw new Error('private tracker card snapshot is not the exact 1,100-card partition');
  }
  const codexRows = entryRows(codex, codexSnapshotHash);
  if (codexRows.length !== 281) throw new Error('private Codex snapshot is not the pinned 281-entry partition');
  validateFeedReferences(feed, codexRows);
  const engineHash = await engineSourceHash(root);
  const catalogRoot = ensurePrivateDatabaseRoot(root);
  await mkdir(catalogRoot, { recursive: true, mode: 0o700 });
  if ((await lstat(catalogRoot)).isSymbolicLink()) throw new Error('private tracker database directory may not be linked');
  const resolvedRoot = await realpath(root);
  const resolvedCatalog = await realpath(catalogRoot);
  if (dirname(resolvedCatalog) !== join(resolvedRoot, '.local', 'authority')) {
    throw new Error('private tracker database directory escapes its authority root');
  }
  const databasePath = join(resolvedCatalog, DB_NAME);
  if ((await lstat(databasePath).catch((error: unknown) => {
    if (error instanceof Error && 'code' in error && (error as NodeJS.ErrnoException).code === 'ENOENT') return null;
    throw error;
  }))?.isSymbolicLink()) throw new Error('private tracker database file may not be linked');
  const db = new DatabaseSync(databasePath, { enableForeignKeyConstraints: true });
  db.exec('PRAGMA journal_mode=DELETE; PRAGMA synchronous=FULL; PRAGMA foreign_keys=ON;');
  let counts: ReturnType<typeof upsertRows>;
  try {
    counts = upsertRows(db, {
      authorityHash: authority.authorityHash,
      revisionId: authority.revisionId,
      cards: authority.cards,
      catalog,
      reviews,
      codex: codexRows,
      codexSnapshotHash,
      feed,
      feedHash,
      engineHash,
    });
  } finally {
    db.close();
  }
  await chmod(databasePath, 0o600);
  return {
    cardCount: authority.cards.length,
    codexEntryCount: codexRows.length,
    groupCount: array(feed.groups!, 'Capability groups').length,
    requirementCount: counts.requirementCount,
    sourceReviewCandidateCount: counts.sourceReviewCandidateCount,
    sourceReviewedCount: counts.sourceReviewedCount,
    boundCount: counts.boundCount,
    databasePath,
    authorityHash: authority.authorityHash,
    revisionId: authority.revisionId,
    feedHash,
    engineHash,
  };
}

export function exportCardTracker(databasePath: string, format: 'json' | 'csv' = 'json'): string {
  const db = new DatabaseSync(databasePath, { readOnly: true, enableForeignKeyConstraints: true });
  try {
    const meta = Object.fromEntries(db.prepare('SELECT key,value FROM tracker_meta').all().map((row) => [row.key, row.value]));
    const groups = db.prepare('SELECT group_id,payload_json FROM capability_groups WHERE is_current=1 ORDER BY rank,group_id').all();
    const cards = db.prepare(`
      SELECT c.card_id AS cardId,c.name,c.card_type AS cardType,c.binding_status AS bindingStatus,
        c.source_hash AS sourceCardHash,c.authority_hash AS authorityHash,c.authority_revision AS authorityRevision,
        s.feed_status AS status,s.review_status AS reviewStatus,s.validation_status AS validationStatus,
        s.validation_ready_annotation AS validationReady,s.dependency_groups_json AS dependencyGroups,
        s.blockers_json AS explicitBlockers,s.selected_deck_frequency AS selectedDeckFrequency,
        s.source_review_status AS sourceReviewStatus,s.payload_json AS feedPayload,m.status AS manualStatus
      FROM cards c JOIN card_status s USING(card_id)
      JOIN manual_card_status m USING(card_id)
      WHERE c.is_current=1 ORDER BY c.card_id
    `).all();
    const refs = db.prepare(`
      SELECT scope,scope_id AS scopeId,c.codex_id AS codexId,c.title,c.url,relationship,applicability
      FROM codex_refs r JOIN codex_entries c ON c.codex_id=r.codex_id AND c.is_current=1
      ORDER BY scope,scope_id,c.codex_id,relationship,applicability
    `).all();
    const reqs = db.prepare(`
      SELECT r.card_id AS cardId,r.slice_id AS sliceId,s.requirement_id AS id,s.group_id AS groupId,
        s.implementation_status AS implementationStatus,s.authored_family AS authoredFamily
      FROM card_requirements r JOIN rule_slices s USING(slice_id)
      WHERE r.is_current=1 AND s.is_current=1 ORDER BY r.card_id,r.slice_id
    `).all();
    const proofReferences = db.prepare(`SELECT card_id AS cardId,source_hash AS sourceHash,reference,status
      FROM proof_references ORDER BY card_id,reference`).all();
    const codexRefs = refs.map((row) => ({ scope: row.scope, scopeId: row.scopeId, id: row.codexId, title: row.title,
      url: row.url, relationship: row.relationship || null, applicability: row.applicability || null }));
    const publicRef = (ref: (typeof codexRefs)[number]) => ({ id: ref.id, title: ref.title, url: ref.url,
      relationship: ref.relationship, applicability: ref.applicability });
    const requirements = reqs.map((row) => ({ cardId: row.cardId, sliceId: row.sliceId, id: row.id, groupId: row.groupId,
      implementationStatus: row.implementationStatus, authoredFamily: row.authoredFamily }));
    const counts = {
      cards: cards.length,
      groups: groups.length,
      requirements: requirements.length,
      currentlyBound: cards.filter((row) => row.bindingStatus === 'admitted-current').length,
      sourceReviewCandidates: cards.filter((row) => row.sourceReviewStatus === 'source-review-required').length,
      explicitSourceReviews: cards.filter((row) => row.sourceReviewStatus === 'source-reviewed').length,
    };
    const exportCard = (row: (typeof cards)[number]) => {
      const feed = JSON.parse(String(row.feedPayload)) as Record<string, JsonValue>;
      const source = record(feed.source!, 'Feed source');
      return {
        cardId: row.cardId,
        name: row.name,
        cardType: row.cardType,
        bindingStatus: row.bindingStatus,
        sourceCardHash: row.sourceCardHash,
        authorityHash: row.authorityHash,
        authorityRevision: row.authorityRevision,
        status: row.status,
        reviewStatus: row.reviewStatus,
        validationStatus: row.validationStatus,
        feedValidationReadyAnnotation: row.validationReady === 1,
        sourceReviewStatus: row.sourceReviewStatus,
        manualStatus: row.manualStatus,
        dependencyGroups: JSON.parse(String(row.dependencyGroups)),
        explicitBlockers: JSON.parse(String(row.explicitBlockers)),
        selectedDeckFrequency: row.selectedDeckFrequency,
        source: {
          authorityHash: source.authorityHash,
          sourceCardHash: source.sourceCardHash,
          semanticIndex: source.semanticIndex,
          normalizedLocator: source.normalizedLocator,
          sourceReviewLocator: source.sourceReviewLocator,
        },
        requirements: requirements.filter((req) => req.cardId === row.cardId).map((req) => ({
          id: req.id,
          groupId: req.groupId,
          implementationStatus: req.implementationStatus,
          authoredFamily: req.authoredFamily,
          codexReferences: codexRefs.filter((ref) => ref.scope === 'slice' && ref.scopeId === req.sliceId)
            .map(publicRef),
        })),
        codexReferences: codexRefs.filter((ref) => ref.scope === 'card' && ref.scopeId === row.cardId)
          .map(publicRef),
      };
    };
    const exportedCards = cards.map(exportCard);
    if (format === 'csv') {
      const columns = ['cardId','name','cardType','status','reviewStatus','bindingStatus','sourceReviewStatus',
        'manualStatus','validationStatus','feedValidationReadyAnnotation','sourceCardHash','authorityHash',
        'authorityRevision','dependencyGroups','explicitBlockers','selectedDeckFrequency','source','requirements','codexReferences'];
      const quote = (value: unknown): string => `"${String(value ?? '').replaceAll('"', '""')}"`;
      return [columns.join(','), ...exportedCards.map((row) => columns.map((column) => {
        const value = row[column as keyof typeof row];
        return quote(typeof value === 'object' && value !== null ? JSON.stringify(value) : value);
      }).join(','))].join('\n') + '\n';
    }
    const exported = {
      schemaVersion: 1,
      authorityHash: meta.authorityHash,
      revisionId: meta.revisionId,
      feedHash: meta.feedHash,
      engineHash: meta.engineHash,
      generatedAt: new Date().toISOString(),
      counts,
      cards: exportedCards,
      groups: groups.map((group) => {
        const payload = JSON.parse(String(group.payload_json)) as Record<string, JsonValue>;
        return {
        ...payload,
        codexReferences: codexRefs.filter((ref) => ref.scope === 'group' && ref.scopeId === group.group_id)
          .map(publicRef),
      }; }),
      proofReferences,
    };
    return JSON.stringify(exported) + '\n';
  } finally {
    db.close();
  }
}

export async function writeTrackerExport(repositoryRoot: string, format: 'json' | 'csv'): Promise<string> {
  const root = resolve(repositoryRoot);
  const catalogRoot = ensurePrivateDatabaseRoot(root);
  const localInfo = await lstat(join(root, '.local'));
  const authorityInfo = await lstat(join(root, '.local', 'authority'));
  if ((await lstat(catalogRoot)).isSymbolicLink()) throw new Error('private catalog directory may not be linked');
  if (localInfo.isSymbolicLink() || authorityInfo.isSymbolicLink()) throw new Error('private catalog parents may not be linked');
  const realRoot = await realpath(root);
  const realCatalogRoot = await realpath(catalogRoot);
  if (dirname(realCatalogRoot) !== join(realRoot, '.local', 'authority')) {
    throw new Error('private catalog export directory escapes authority root');
  }
  const dbPath = join(realCatalogRoot, DB_NAME);
  const dbInfo = await lstat(dbPath);
  if (!dbInfo.isFile() || dbInfo.isSymbolicLink()) throw new Error('private tracker database must be an ordinary file');
  const bytes = Buffer.from(exportCardTracker(dbPath, format), 'utf8');
  const path = join(realCatalogRoot, format === 'json' ? 'card-tracker-workbook.json' : 'card-tracker-workbook.csv');
  const outputInfo = await lstat(path).catch((error: unknown) => {
    if (error instanceof Error && 'code' in error && (error as NodeJS.ErrnoException).code === 'ENOENT') return null;
    throw error;
  });
  if (outputInfo?.isSymbolicLink()) throw new Error('private tracker export may not be linked');
  await writeFile(path, bytes, { mode: 0o600 });
  await chmod(path, 0o600);
  return path;
}

export function privateTrackerPath(repositoryRoot: string): string {
  return join(ensurePrivateDatabaseRoot(repositoryRoot), DB_NAME);
}
