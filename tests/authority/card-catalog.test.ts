import assert from 'node:assert/strict';
import { cp, mkdir, mkdtemp, readFile, readdir, rm, symlink, writeFile } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join, relative } from 'node:path';
import { DatabaseSync } from 'node:sqlite';
import test from 'node:test';

import { canonicalJson, type JsonValue } from '../../src/authority/canonical-json.ts';
import { identityHash, sha256 } from '../../src/authority/hash.ts';
import { AuthorityValidationError, type Hash } from '../../src/authority/schemas.ts';
import { buildCardCatalog } from '../../src/catalog/card-catalog.ts';
import { importAuthority, type ImportAuthorityResult } from '../../src/commands/import-authority.ts';

const FIXTURE_INPUT = join(import.meta.dirname, 'fixtures', 'bundle-input');
const REVISION_ID = 'synthetic-v1';
const ZERO_HASH = `sha256:${'0'.repeat(64)}` as Hash;

async function fileMapHash(root: string): Promise<Hash> {
  const entries = await readdir(root, { recursive: true, withFileTypes: true });
  const files = await Promise.all(entries.filter((entry) => entry.isFile()).map(async (entry) => {
    const path = join(entry.parentPath, entry.name);
    const bytes = await readFile(path);
    return {
      byteHash: sha256(bytes),
      byteLength: bytes.byteLength,
      relativePath: relative(root, path).replaceAll('\\', '/'),
    };
  }));
  files.sort((left, right) => left.relativePath.localeCompare(right.relativePath));
  return identityHash(files);
}

async function writeReceipt(
  root: string,
  revisionId: string,
  imported: ImportAuthorityResult,
): Promise<void> {
  const receiptRoot = join(root, 'data', 'authority', 'receipts');
  await mkdir(receiptRoot, { recursive: true });
  const revisionFileMapHash = await fileMapHash(
    join(root, '.local', 'authority', 'revisions', revisionId),
  );
  await writeFile(join(receiptRoot, `${revisionId}.json`), canonicalJson({
    acquisitionMethod: 'synthetic-test-fixture',
    attestation: {
      artwork: false,
      commercialUse: false,
      hosting: false,
      publicApi: false,
      publisherPermission: false,
      recurringAcquisition: false,
      redistribution: false,
      release: false,
      scope: 'test-only',
      thirdPartyUpload: false,
    },
    authorizationReference: 'synthetic-test',
    bundleRootHash: imported.bundleRootHash,
    gitContainsCorpus: false,
    inputRootHash: imported.verifiedInputRootHash,
    revisionFileMapHash,
    revisionId,
    schemaVersion: 1,
    sourceSetRootHash: imported.bundleRootHash,
    sourceSets: { backup: 'synthetic-backup', primary: 'synthetic-primary' },
    sources: [],
    stableId: imported.bundleId,
  }));
}

async function importFixture(root: string): Promise<ImportAuthorityResult> {
  const inputRoot = join(root, 'input');
  await cp(FIXTURE_INPUT, inputRoot, { recursive: true });
  const lock = JSON.parse(await readFile(join(inputRoot, 'input-lock.json'), 'utf8')) as {
    inputRootHash: Hash;
  };
  const imported = await importAuthority({
    expectedInputRootHash: lock.inputRootHash,
    inputLockPath: 'input-lock.json',
    inputRoot,
    outputRoot: join(root, '.local', 'authority', 'revisions'),
    revisionId: REVISION_ID,
  });
  await writeReceipt(root, REVISION_ID, imported);
  return imported;
}

function plainRows(rows: readonly Record<string, unknown>[]): readonly Record<string, unknown>[] {
  return rows.map((row) => ({ ...row }));
}

test('builds a receipt-bound strict catalog with source-qualified price constraints', async () => {
  const root = await mkdtemp(join(tmpdir(), 'sorcery-card-catalog-'));
  try {
    const imported = await importFixture(root);
    const built = await buildCardCatalog(REVISION_ID, root);
    assert.equal(built.bundleContentHash, imported.bundleRootHash);
    assert.equal(built.cardCount, 1);
    assert.equal(built.outputPath, join(root, '.local', 'authority', 'catalog', `${REVISION_ID}.sqlite3`));

    const database = new DatabaseSync(built.outputPath, { enableForeignKeyConstraints: true });
    try {
      const revision = { ...database.prepare('SELECT * FROM catalog_revision').get() };
      assert.deepEqual(revision, {
        artifact_content_hash: built.artifactContentHash,
        artifact_kind: 'card-snapshot',
        artifact_schema_version: 1,
        artifact_stable_id: revision.artifact_stable_id,
        bundle_content_hash: imported.bundleRootHash,
        card_count: 1,
        revision_id: REVISION_ID,
      });
      assert.deepEqual(plainRows(database.prepare(
        'SELECT source, source_card_id FROM card_source_mapping',
      ).all()), [{ source: 'source:cards-synthetic', source_card_id: 'synthetic-apprentice-alpha' }]);
      assert.deepEqual(plainRows(database.prepare(
        'SELECT alias, ordinal FROM printing_alias',
      ).all()), [{ alias: 'synthetic-apprentice-alpha', ordinal: 0 }]);
      assert.equal(database.prepare(
        "SELECT count(*) AS count FROM pragma_table_list WHERE name IN ('catalog_revision', 'card', 'card_source_mapping', 'card_element', 'card_subtype', 'printing_alias', 'price_snapshot', 'price_quote') AND strict = 1",
      ).get()!.count, 8);
      assert.deepEqual(plainRows(database.prepare(
        "SELECT name FROM sqlite_schema WHERE type = 'index' AND name NOT LIKE 'sqlite_%' ORDER BY name",
      ).all()), [
        { name: 'card_name_lookup' },
        { name: 'card_source_lookup' },
        { name: 'card_type_lookup' },
        { name: 'price_quote_lookup' },
        { name: 'printing_card_lookup' },
      ]);

      database.prepare('INSERT INTO price_snapshot VALUES (?, ?, ?)').run(
        'prices:synthetic',
        REVISION_ID,
        1_788_225_300_000,
      );
      assert.throws(
        () => database.prepare('INSERT INTO price_snapshot VALUES (?, ?, ?)').run(
          'prices:bad-time',
          REVISION_ID,
          -1,
        ),
        /CHECK constraint failed/u,
      );
      const quote = database.prepare('INSERT INTO price_quote VALUES (?, ?, ?, ?, ?, ?, ?, ?)');
      quote.run(
        'prices:synthetic',
        'source:cards-synthetic',
        'synthetic-apprentice-alpha',
        'official-printing:alpha',
        'standard',
        'near-mint',
        'USD',
        125,
      );
      assert.throws(() => quote.run(
        'prices:synthetic',
        'source:cards-synthetic',
        'missing-card',
        'official-printing:missing',
        'standard',
        'near-mint',
        'USD',
        1,
      ), /FOREIGN KEY constraint failed/u);
      assert.throws(() => quote.run(
        'prices:synthetic',
        'source:cards-synthetic',
        'synthetic-apprentice-alpha',
        'official-printing:negative',
        'standard',
        'near-mint',
        'USD',
        -1,
      ), /CHECK constraint failed/u);
      assert.equal(database.prepare('PRAGMA integrity_check').get()!.integrity_check, 'ok');
      assert.deepEqual(database.prepare('PRAGMA foreign_key_check').all(), []);
    } finally {
      database.close();
    }

    await assert.rejects(() => buildCardCatalog(REVISION_ID, root), { code: 'EEXIST' });
    assert.deepEqual(
      await readdir(join(root, '.local', 'authority', 'catalog')),
      [`${REVISION_ID}.sqlite3`],
    );

    const receiptPath = join(root, 'data', 'authority', 'receipts', `${REVISION_ID}.json`);
    const mismatchedReceipt = JSON.parse(await readFile(receiptPath, 'utf8')) as Record<string, JsonValue>;
    mismatchedReceipt.inputRootHash = ZERO_HASH;
    await writeFile(receiptPath, canonicalJson(mismatchedReceipt));
    await assert.rejects(
      () => buildCardCatalog(REVISION_ID, root),
      /receipt input root does not match the validated bundle/u,
    );
    await writeReceipt(root, REVISION_ID, imported);
    const staleMapReceipt = JSON.parse(await readFile(receiptPath, 'utf8')) as Record<string, JsonValue>;
    staleMapReceipt.revisionFileMapHash = ZERO_HASH;
    await writeFile(receiptPath, canonicalJson(staleMapReceipt));
    await assert.rejects(
      () => buildCardCatalog(REVISION_ID, root),
      /receipt file map does not match the validated revision/u,
    );
    await writeReceipt(root, REVISION_ID, imported);

    const copiedRevision = 'synthetic-copy';
    await cp(
      join(root, '.local', 'authority', 'revisions', REVISION_ID),
      join(root, '.local', 'authority', 'revisions', copiedRevision),
      { recursive: true },
    );
    await writeReceipt(root, copiedRevision, imported);
    await assert.rejects(
      () => buildCardCatalog(copiedRevision, root),
      /receipt does not select the requested revision/u,
    );

    const cardsPath = join(root, '.local', 'authority', 'revisions', REVISION_ID, 'cards.normalized.json');
    const artifact = JSON.parse(await readFile(cardsPath, 'utf8')) as {
      contentHash: Hash;
      identity: { payload: { cards: Array<{ name: string }> } };
    };
    artifact.identity.payload.cards[0]!.name = 'Rehashed tampering';
    artifact.contentHash = identityHash(artifact.identity as unknown as JsonValue);
    await writeFile(cardsPath, canonicalJson(artifact as unknown as JsonValue));
    await assert.rejects(
      () => buildCardCatalog(REVISION_ID, root),
      (error) => error instanceof AuthorityValidationError
        && error.diagnostics.some(({ code }) => code === 'companion_content_mismatch'),
    );

    for (const revisionId of ['../escape', '..', 'a/b', 'A']) {
      await assert.rejects(() => buildCardCatalog(revisionId, root), /one confined path segment/u);
    }
  } finally {
    await rm(root, { force: true, recursive: true });
  }
});

test('bounds receipt reads and rejects an aliased catalog output', async () => {
  const root = await mkdtemp(join(tmpdir(), 'sorcery-card-alias-'));
  const outside = await mkdtemp(join(tmpdir(), 'sorcery-card-outside-'));
  const aliasedRoot = await mkdtemp(join(tmpdir(), 'sorcery-card-ancestor-alias-'));
  const externalRoot = await mkdtemp(join(tmpdir(), 'sorcery-card-external-authority-'));
  try {
    const imported = await importFixture(root);
    const receiptPath = join(root, 'data', 'authority', 'receipts', `${REVISION_ID}.json`);
    await writeFile(receiptPath, Buffer.alloc(256_001, 120));
    await assert.rejects(() => buildCardCatalog(REVISION_ID, root), /too-large/u);
    await writeReceipt(root, REVISION_ID, imported);

    const catalogRoot = join(root, '.local', 'authority', 'catalog');
    await symlink(outside, catalogRoot, process.platform === 'win32' ? 'junction' : 'dir');
    await assert.rejects(
      () => buildCardCatalog(REVISION_ID, root),
      /may not be a symbolic link or junction/u,
    );
    assert.deepEqual(await readdir(outside), []);

    const externalImported = await importFixture(externalRoot);
    await mkdir(join(aliasedRoot, '.local'), { recursive: true });
    await symlink(
      join(externalRoot, '.local', 'authority'),
      join(aliasedRoot, '.local', 'authority'),
      process.platform === 'win32' ? 'junction' : 'dir',
    );
    await writeReceipt(aliasedRoot, REVISION_ID, externalImported);
    await assert.rejects(
      () => buildCardCatalog(REVISION_ID, aliasedRoot),
      /private authority root may not be a symbolic link or junction/u,
    );
  } finally {
    await rm(root, { force: true, recursive: true });
    await rm(outside, { force: true, recursive: true });
    await rm(aliasedRoot, { force: true, recursive: true });
    await rm(externalRoot, { force: true, recursive: true });
  }
});
