import assert from 'node:assert/strict';
import { link, mkdir, mkdtemp, rm, symlink, unlink, writeFile } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { basename, dirname, join, resolve } from 'node:path';
import test from 'node:test';

import { sha256 } from '../../src/authority/hash.ts';
import {
  PRIVATE_AUTHORITY_SOURCE_PATHS,
  PrivateSourceSetVerificationError,
  verifyPrivateSourceSet,
  type PrivateAuthoritySourceEntry,
} from '../../src/authority/private-source-set.ts';

const OFFICIAL_URLS = Object.freeze({
  'rulebook/rulebook-current.pdf':
    'https://sorcerytcg.com/news/sorcery-contested-realm-december-2025-rulebook-update',
  'formats/constructed-current.html': 'https://sorcerytcg.com/constructed',
  'codex/codex-current.html': 'https://curiosa.io/codex',
  'codex/faqs-current.html': 'https://curiosa.io/faqs',
  'codex/changelog-current.html': 'https://curiosa.io/codex/changelog',
  'updates/card-updates-2025.html':
    'https://sorcerytcg.com/news/sorcery-contested-realm-card-updates-2025',
  'cards/cards.raw.json': 'https://api.sorcerytcg.com/api/cards',
} satisfies Record<(typeof PRIVATE_AUTHORITY_SOURCE_PATHS)[number], string>);

const SOURCE_BYTES = Object.freeze({
  'rulebook/rulebook-current.pdf': Buffer.from('%PDF-1.7\nsynthetic rulebook\n%%EOF\n'),
  'formats/constructed-current.html': Buffer.from('<!doctype html><title>Constructed</title>\n'),
  'codex/codex-current.html': Buffer.from('<!doctype html><title>Codex</title>\n'),
  'codex/faqs-current.html': Buffer.from('<!doctype html><title>FAQs</title>\n'),
  'codex/changelog-current.html': Buffer.from('<!doctype html><title>Changelog</title>\n'),
  'updates/card-updates-2025.html': Buffer.from('<!doctype html><title>Card updates</title>\n'),
  'cards/cards.raw.json': Buffer.from('{"cards":[]}\n'),
} satisfies Record<(typeof PRIVATE_AUTHORITY_SOURCE_PATHS)[number], Buffer>);

type Fixture = Readonly<{
  sandbox: string;
  repositoryRoot: string;
  primaryRoot: string;
  backupRoot: string;
  entries: readonly PrivateAuthoritySourceEntry[];
}>;

function mediaType(relativePath: string): PrivateAuthoritySourceEntry['mediaType'] {
  if (relativePath.endsWith('.pdf')) return 'application/pdf';
  if (relativePath.endsWith('.json')) return 'application/json';
  return 'text/html';
}

async function writeSourceTree(root: string): Promise<void> {
  for (const relativePath of PRIVATE_AUTHORITY_SOURCE_PATHS) {
    const target = join(root, ...relativePath.split('/'));
    await mkdir(dirname(target), { recursive: true });
    await writeFile(target, SOURCE_BYTES[relativePath]);
  }
}

async function createFixture(): Promise<Fixture> {
  const sandbox = await mkdtemp(join(tmpdir(), 'sorcery-private-source-set-'));
  const repositoryRoot = join(sandbox, 'repository');
  const primaryRoot = join(repositoryRoot, '.local', 'authority', 'inputs', 'primary');
  const backupRoot = join(sandbox, 'backup');
  await mkdir(repositoryRoot, { recursive: true });
  await writeSourceTree(primaryRoot);
  await writeSourceTree(backupRoot);
  const entries = PRIVATE_AUTHORITY_SOURCE_PATHS.map((relativePath) => {
    const bytes = SOURCE_BYTES[relativePath];
    return Object.freeze({
      relativePath,
      url: OFFICIAL_URLS[relativePath],
      retrievedAt: '2026-08-20T19:00:00Z',
      effectiveDate: relativePath.includes('changelog') ? null : '2026-08-20',
      mediaType: mediaType(relativePath),
      byteLength: bytes.byteLength,
      byteHash: sha256(bytes),
    });
  });
  return { sandbox, repositoryRoot, primaryRoot, backupRoot, entries };
}

async function cleanupFixture(fixture: Fixture): Promise<void> {
  const resolved = resolve(fixture.sandbox);
  assert.equal(dirname(resolved), resolve(tmpdir()));
  assert.match(basename(resolved), /^sorcery-private-source-set-/);
  await rm(resolved, { recursive: true, force: true });
}

async function withFixture(run: (fixture: Fixture) => Promise<void>): Promise<void> {
  const fixture = await createFixture();
  try {
    await run(fixture);
  } finally {
    await cleanupFixture(fixture);
  }
}

function replaceEntry(
  entries: readonly PrivateAuthoritySourceEntry[],
  relativePath: PrivateAuthoritySourceEntry['relativePath'],
  changes: Record<string, unknown>,
): readonly PrivateAuthoritySourceEntry[] {
  return entries.map((entry) =>
    entry.relativePath === relativePath
      ? ({ ...entry, ...changes } as unknown as PrivateAuthoritySourceEntry)
      : entry,
  );
}

async function expectVerificationError(
  run: () => Promise<unknown>,
  expectedPath: string,
  expectedCode: string,
): Promise<void> {
  await assert.rejects(run, (error: unknown) => {
    assert.ok(error instanceof PrivateSourceSetVerificationError);
    assert.equal(error.path, expectedPath);
    assert.equal(error.code, expectedCode);
    return true;
  });
}

test('DATA-01 locks exactly the seven canonical private authority paths', () => {
  assert.deepEqual(PRIVATE_AUTHORITY_SOURCE_PATHS, [
    'rulebook/rulebook-current.pdf',
    'formats/constructed-current.html',
    'codex/codex-current.html',
    'codex/faqs-current.html',
    'codex/changelog-current.html',
    'updates/card-updates-2025.html',
    'cards/cards.raw.json',
  ]);
});

test('DATA-01 verifies independent matching sets and returns deterministic sorted evidence', async () => {
  await withFixture(async (fixture) => {
    const options = {
      primaryRoot: fixture.primaryRoot,
      backupRoot: fixture.backupRoot,
      repositoryRoot: fixture.repositoryRoot,
      entries: fixture.entries,
    };
    const first = await verifyPrivateSourceSet(options);
    const reordered = await verifyPrivateSourceSet({ ...options, entries: [...fixture.entries].reverse() });
    assert.deepEqual(first, reordered);
    assert.deepEqual(
      first.entries.map(({ relativePath }) => relativePath),
      [...PRIVATE_AUTHORITY_SOURCE_PATHS].sort(),
    );
    assert.match(first.sourceSetRootHash, /^sha256:[0-9a-f]{64}$/);
    assert.equal(JSON.stringify(first).includes(fixture.sandbox), false);
  });
});

test('DATA-01 rejects incomplete and extra source trees with exact paths', async () => {
  await withFixture(async (fixture) => {
    const missing = 'codex/faqs-current.html';
    await unlink(join(fixture.primaryRoot, ...missing.split('/')));
    await expectVerificationError(
      () => verifyPrivateSourceSet(fixture),
      `/primary/${missing}`,
      'missing_file',
    );
  });

  await withFixture(async (fixture) => {
    await writeFile(join(fixture.backupRoot, 'extra.txt'), 'extra');
    await expectVerificationError(
      () => verifyPrivateSourceSet(fixture),
      '/backup/extra.txt',
      'unexpected_path',
    );
  });

  await withFixture(async (fixture) => {
    await mkdir(join(fixture.primaryRoot, 'empty-extra'));
    await expectVerificationError(
      () => verifyPrivateSourceSet(fixture),
      '/primary/empty-extra',
      'unexpected_path',
    );
  });
});

test('DATA-01 rejects missing unknown and malformed metadata before filesystem access', async () => {
  const missingRoots = {
    primaryRoot: join(tmpdir(), 'does-not-exist-primary'),
    backupRoot: join(tmpdir(), 'does-not-exist-backup'),
    repositoryRoot: join(tmpdir(), 'does-not-exist-repository'),
  };
  const fixture = await createFixture();
  try {
    const target = fixture.entries[0]!;
    const missingUrl = { ...target } as Record<string, unknown>;
    delete missingUrl.url;
    const invalidCases: readonly [unknown, string, string][] = [
      [missingUrl, '/entries/0/url', 'missing_field'],
      [{ ...target, unexpected: true }, '/entries/0/unexpected', 'unknown_field'],
      [{ ...target, url: 'http://sorcerytcg.com/rules' }, '/entries/0/url', 'invalid_url'],
      [{ ...target, url: 'https://example.com/rules' }, '/entries/0/url', 'unapproved_host'],
      [{ ...target, url: 'https://user@sorcerytcg.com/rules' }, '/entries/0/url', 'invalid_url'],
      [{ ...target, retrievedAt: '2026-02-30T00:00:00Z' }, '/entries/0/retrievedAt', 'invalid_timestamp'],
      [{ ...target, effectiveDate: '2026-02-30' }, '/entries/0/effectiveDate', 'invalid_date'],
      [{ ...target, mediaType: 'text/html' }, '/entries/0/mediaType', 'invalid_media_type'],
      [{ ...target, byteLength: 0 }, '/entries/0/byteLength', 'invalid_byte_length'],
      [{ ...target, byteHash: `sha256:${'A'.repeat(64)}` }, '/entries/0/byteHash', 'invalid_hash'],
    ];
    for (const [invalid, path, code] of invalidCases) {
      const entries = [invalid, ...fixture.entries.slice(1)] as readonly PrivateAuthoritySourceEntry[];
      await expectVerificationError(
        () => verifyPrivateSourceSet({ ...missingRoots, entries }),
        path,
        code,
      );
    }
  } finally {
    await cleanupFixture(fixture);
  }
});

test('DATA-01 rejects non-canonical relative paths before filesystem access', async () => {
  const fixture = await createFixture();
  try {
    const invalidPaths = [
      '',
      '.',
      '..',
      '/rulebook/rulebook-current.pdf',
      'C:/rulebook/rulebook-current.pdf',
      'rulebook\\rulebook-current.pdf',
      'rulebook/../rulebook-current.pdf',
      'rulebook//rulebook-current.pdf',
      'CON/file.html',
    ];
    for (const relativePath of invalidPaths) {
      const entries = replaceEntry(fixture.entries, PRIVATE_AUTHORITY_SOURCE_PATHS[0], { relativePath });
      await expectVerificationError(
        () =>
          verifyPrivateSourceSet({
            primaryRoot: join(fixture.sandbox, 'missing-primary'),
            backupRoot: join(fixture.sandbox, 'missing-backup'),
            repositoryRoot: join(fixture.sandbox, 'missing-repository'),
            entries,
          }),
        '/entries/0/relativePath',
        'invalid_relative_path',
      );
    }
  } finally {
    await cleanupFixture(fixture);
  }
});

test('DATA-01 rejects individual and aggregate byte limits before trusting metadata', async () => {
  await withFixture(async (fixture) => {
    const entries = replaceEntry(fixture.entries, 'cards/cards.raw.json', { byteLength: 10_000_001 });
    await expectVerificationError(
      () => verifyPrivateSourceSet({ ...fixture, entries }),
      '/entries/6/byteLength',
      'max_file_bytes',
    );
  });

  await withFixture(async (fixture) => {
    const entries = fixture.entries.map((entry) =>
      entry.mediaType === 'text/html' || entry.mediaType === 'application/pdf'
        ? { ...entry, byteLength: 134_217_728 }
        : entry,
    );
    await expectVerificationError(
      () => verifyPrivateSourceSet({ ...fixture, entries }),
      '/entries',
      'max_total_bytes',
    );
  });

  await withFixture(async (fixture) => {
    const relativePath = 'cards/cards.raw.json';
    await writeFile(
      join(fixture.primaryRoot, ...relativePath.split('/')),
      Buffer.alloc(10_000_001, 32),
    );
    await expectVerificationError(
      () => verifyPrivateSourceSet(fixture),
      `/primary/${relativePath}`,
      'max_file_bytes',
    );
  });
});

test('DATA-02 rejects malformed bounded card JSON and changed backup bytes', async () => {
  await withFixture(async (fixture) => {
    const relativePath = 'cards/cards.raw.json';
    const bytes = Buffer.from('{"cards":[}\n');
    await writeFile(join(fixture.primaryRoot, ...relativePath.split('/')), bytes);
    await writeFile(join(fixture.backupRoot, ...relativePath.split('/')), bytes);
    const entries = replaceEntry(fixture.entries, relativePath, {
      byteLength: bytes.byteLength,
      byteHash: sha256(bytes),
    });
    await expectVerificationError(
      () => verifyPrivateSourceSet({ ...fixture, entries }),
      `/primary/${relativePath}`,
      'invalid_json',
    );
  });

  await withFixture(async (fixture) => {
    const relativePath = 'codex/codex-current.html';
    const original = SOURCE_BYTES[relativePath];
    const changed = Buffer.from(original);
    changed[0] = changed[0] === 60 ? 61 : 60;
    await writeFile(join(fixture.backupRoot, ...relativePath.split('/')), changed);
    await expectVerificationError(
      () => verifyPrivateSourceSet(fixture),
      `/backup/${relativePath}/byteHash`,
      'byte_hash_mismatch',
    );
  });
});

test('DATA-01 rejects same nested and repository-contained backup roots', async () => {
  await withFixture(async (fixture) => {
    await expectVerificationError(
      () => verifyPrivateSourceSet({ ...fixture, backupRoot: fixture.primaryRoot }),
      '/backupRoot',
      'root_overlap',
    );
  });

  await withFixture(async (fixture) => {
    const nested = join(fixture.primaryRoot, 'nested-backup');
    await mkdir(nested);
    await expectVerificationError(
      () => verifyPrivateSourceSet({ ...fixture, backupRoot: nested }),
      '/backupRoot',
      'root_overlap',
    );
  });

  await withFixture(async (fixture) => {
    const repositoryBackup = join(fixture.repositoryRoot, 'independent-looking-backup');
    await writeSourceTree(repositoryBackup);
    await expectVerificationError(
      () => verifyPrivateSourceSet({ ...fixture, backupRoot: repositoryBackup }),
      '/backupRoot',
      'backup_inside_repository',
    );
  });
});

test('DATA-01 rejects symlinked files and symlink or junction root aliases', async () => {
  await withFixture(async (fixture) => {
    const relativePath = 'formats/constructed-current.html';
    const target = join(fixture.primaryRoot, ...relativePath.split('/'));
    const outside = join(fixture.sandbox, 'outside-file.html');
    await writeFile(outside, SOURCE_BYTES[relativePath]);
    await unlink(target);
    await symlink(outside, target, 'file');
    await expectVerificationError(
      () => verifyPrivateSourceSet(fixture),
      `/primary/${relativePath}`,
      'filesystem_alias',
    );
  });

  await withFixture(async (fixture) => {
    const alias = join(fixture.sandbox, 'primary-alias');
    await symlink(fixture.primaryRoot, alias, process.platform === 'win32' ? 'junction' : 'dir');
    await expectVerificationError(
      () => verifyPrivateSourceSet({ ...fixture, primaryRoot: alias }),
      '/primaryRoot',
      'filesystem_alias',
    );
  });
});

test('DATA-01 rejects non-ordinary entries and primary-backup hardlinks', async () => {
  await withFixture(async (fixture) => {
    const relativePath = 'updates/card-updates-2025.html';
    const target = join(fixture.primaryRoot, ...relativePath.split('/'));
    await unlink(target);
    await mkdir(target);
    await expectVerificationError(
      () => verifyPrivateSourceSet(fixture),
      `/primary/${relativePath}`,
      'non_ordinary_file',
    );
  });

  await withFixture(async (fixture) => {
    const relativePath = 'rulebook/rulebook-current.pdf';
    const primary = join(fixture.primaryRoot, ...relativePath.split('/'));
    const backup = join(fixture.backupRoot, ...relativePath.split('/'));
    await unlink(backup);
    await link(primary, backup);
    await expectVerificationError(
      () => verifyPrivateSourceSet(fixture),
      `/backup/${relativePath}`,
      'linked_file',
    );
  });
});

test('DATA-01 applies platform-aware root casing after realpath normalization', async () => {
  await withFixture(async (fixture) => {
    const changedCase = join(dirname(fixture.primaryRoot), basename(fixture.primaryRoot).toUpperCase());
    if (process.platform === 'win32') {
      const result = await verifyPrivateSourceSet({ ...fixture, primaryRoot: changedCase });
      assert.equal(result.entries.length, 7);
    } else {
      await expectVerificationError(
        () => verifyPrivateSourceSet({ ...fixture, primaryRoot: changedCase }),
        '/primaryRoot',
        'path_not_found',
      );
    }
  });
});
