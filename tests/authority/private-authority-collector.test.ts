import assert from 'node:assert/strict';
import { spawn } from 'node:child_process';
import { createHash, randomUUID } from 'node:crypto';
import { createServer as createTcpServer } from 'node:net';
import {
  mkdir,
  mkdtemp,
  readFile,
  readdir,
  rm,
  stat,
  symlink,
  writeFile,
} from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { basename, dirname, join, resolve } from 'node:path';
import test from 'node:test';

import {
  PRIVATE_AUTHORITY_SOURCE_PATHS,
  verifyPrivateSourceSet,
  type PrivateAuthoritySourceEntry,
} from '../../src/authority/private-source-set.ts';

const REPOSITORY_ROOT = resolve(import.meta.dirname, '..', '..');
const SCRIPT_PATH = join(REPOSITORY_ROOT, 'scripts', 'collect-private-authority.ps1');
const REVISION_ID = 'official-2026-08-27-v3';
const MANUAL_METHOD = 'user-provided-manual-download';
const MANUAL_REFERENCE = 'phase-01-20260827-manual-provision-1';
const RETRIEVED_AT = '2026-08-27T20:00:00.000Z';
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
});

type SourcePath = (typeof PRIVATE_AUTHORITY_SOURCE_PATHS)[number];
type ProcessResult = Readonly<{ code: number | null; stdout: string; stderr: string }>;
type Fixture = Readonly<{
  sandbox: string;
  repositoryRoot: string;
  inboxRoot: string;
  primaryRoot: string;
  backupRoot: string;
  lockPath: string;
}>;

function syntheticOfficialCard(index: number): Record<string, unknown> {
  const metadata = {
    attack: 1,
    cost: 1,
    defence: 1,
    life: null,
    rarity: 'Ordinary',
    rulesText: 'Synthetic rules text.',
    thresholds: { air: 0, earth: 0, fire: 1, water: 0 },
    type: 'Minion',
  };
  return {
    elements: 'Fire',
    guardian: metadata,
    name: 'Synthetic Card ' + String(index),
    sets: [
      {
        metadata,
        name: 'Synthetic Set',
        releasedAt: '2026-08-20T00:00:00Z',
        variants: [
          {
            artist: 'Synthetic Artist',
            finish: 'Standard',
            flavorText: '',
            product: 'Synthetic Product',
            slug: 'synthetic-card-' + String(index),
            typeText: 'Minion',
          },
        ],
      },
    ],
    subTypes: 'Synthetic',
  };
}

function syntheticPdf(): Buffer {
  const content = 'BT /F1 12 Tf ET\n%' + 'synthetic-padding'.repeat(2_048) + '\n';
  const objects = [
    '<< /Type /Catalog /Pages 2 0 R >>',
    '<< /Type /Pages /Kids [3 0 R] /Count 1 >>',
    '<< /Type /Page /Parent 2 0 R /MediaBox [0 0 100 100] /Contents 4 0 R >>',
    '<< /Length ' + String(Buffer.byteLength(content)) + ' >>\nstream\n' + content + 'endstream',
  ];
  let document = '%PDF-1.7\n';
  const offsets = [0];
  for (const [index, object] of objects.entries()) {
    offsets.push(Buffer.byteLength(document));
    document += String(index + 1) + ' 0 obj\n' + object + '\nendobj\n';
  }
  const xrefOffset = Buffer.byteLength(document);
  document +=
    'xref\n0 5\n0000000000 65535 f \n' +
    offsets
      .slice(1)
      .map((offset) => String(offset).padStart(10, '0') + ' 00000 n \n')
      .join('') +
    'trailer\n<< /Size 5 /Root 1 0 R >>\nstartxref\n' +
    String(xrefOffset) +
    '\n%%EOF';
  return Buffer.from(document);
}

function syntheticHtml(body: string): Buffer {
  const detail =
    '<p>' +
    'Synthetic independently authored authority material for offline validation only. '.repeat(12) +
    '</p>';
  return Buffer.from('<!doctype html><html><body><main>' + body + detail + '</main></body></html>');
}

const SOURCE_BYTES: Readonly<Record<SourcePath, Buffer>> = Object.freeze({
  'rulebook/rulebook-current.pdf': syntheticPdf(),
  'formats/constructed-current.html': syntheticHtml(
    '<h1>Constructed Format</h1><h2>Deck Construction</h2>',
  ),
  'codex/codex-current.html': syntheticHtml(
    '<h1>Welcome to the Codex</h1><p>Card Rulings</p>',
  ),
  'codex/faqs-current.html': syntheticHtml(
    '<h1>FAQs</h1><h2>Frequently Asked Questions</h2>',
  ),
  'codex/changelog-current.html': syntheticHtml(
    '<h1>Codex Changelog</h1><article><h2>20 August 2026</h2><p>Rules update</p></article>',
  ),
  'updates/card-updates-2025.html': syntheticHtml(
    '<h1>Sorcery: Contested Realm Card Updates 2025</h1><h2>Card Updates</h2>',
  ),
  'cards/cards.raw.json': Buffer.from(
    JSON.stringify(Array.from({ length: 1_100 }, (_, index) => syntheticOfficialCard(index))) +
      '\n',
  ),
});

function descriptors(): readonly Record<string, unknown>[] {
  const markers: Partial<Record<SourcePath, readonly string[]>> = {
    'formats/constructed-current.html': ['Constructed Format', 'Deck Construction'],
    'codex/codex-current.html': ['Welcome to the Codex', 'Card Rulings'],
    'codex/faqs-current.html': ['FAQs', 'Frequently Asked Questions'],
    'codex/changelog-current.html': ['Codex Changelog'],
    'updates/card-updates-2025.html': [
      'Sorcery: Contested Realm Card Updates 2025',
      'Card Updates',
    ],
  };
  return PRIVATE_AUTHORITY_SOURCE_PATHS.map((relativePath) => ({
    relativePath,
    provenanceUrl: OFFICIAL_URLS[relativePath],
    mediaType: relativePath.endsWith('.pdf')
      ? 'application/pdf'
      : relativePath.endsWith('.json')
        ? 'application/json'
        : 'text/html',
    sourceMarker:
      relativePath === 'rulebook/rulebook-current.pdf'
        ? 'Sorcery: Contested Realm December 2025 Rulebook Update'
        : (markers[relativePath]?.[0] ?? null),
    visibleBodyMarkers: markers[relativePath] ?? null,
    expectedCardCount: relativePath === 'cards/cards.raw.json' ? 1_100 : null,
    effectiveDatePolicy: relativePath.includes('changelog')
      ? 'changelog'
      : relativePath === 'rulebook/rulebook-current.pdf' ||
          relativePath === 'updates/card-updates-2025.html'
        ? 'fixed'
        : 'none',
    effectiveDate:
      relativePath === 'rulebook/rulebook-current.pdf'
        ? '2025-12-19'
        : relativePath === 'updates/card-updates-2025.html'
          ? '2025-11-25'
          : null,
    minBytes: 1,
    maxBytes: 10_000_000,
  }));
}

function runPwsh(
  arguments_: readonly string[],
  environment: Readonly<Record<string, string>> = {},
  timeoutMilliseconds = 30_000,
): Promise<ProcessResult> {
  return new Promise((resolveProcess, reject) => {
    const child = spawn('pwsh', ['-NoProfile', '-NonInteractive', ...arguments_], {
      cwd: REPOSITORY_ROOT,
      env: { ...process.env, ...environment },
      windowsHide: true,
    });
    let stdout = '';
    let stderr = '';
    let timedOut = false;
    child.stdout.setEncoding('utf8').on('data', (chunk: string) => {
      stdout = (stdout + chunk).slice(0, 1_048_576);
    });
    child.stderr.setEncoding('utf8').on('data', (chunk: string) => {
      stderr = (stderr + chunk).slice(0, 1_048_576);
    });
    const timeout = setTimeout(() => {
      timedOut = true;
      if (child.pid === undefined) return;
      if (process.platform === 'win32') {
        const killer = spawn('taskkill.exe', ['/pid', String(child.pid), '/t', '/f'], {
          stdio: 'ignore',
          windowsHide: true,
        });
        killer.once('error', () => child.kill('SIGKILL'));
      } else {
        child.kill('SIGKILL');
      }
    }, timeoutMilliseconds);
    child.once('error', (error) => {
      clearTimeout(timeout);
      reject(error);
    });
    child.once('close', (code) => {
      clearTimeout(timeout);
      resolveProcess({
        code: timedOut ? null : code,
        stdout,
        stderr: timedOut ? stderr + '\nPowerShell test process timed out.' : stderr,
      });
    });
  });
}

async function createFixture(): Promise<Fixture> {
  const sandbox = await mkdtemp(join(tmpdir(), 'sorcery-manual-intake-'));
  const repositoryRoot = join(sandbox, 'repository');
  return {
    sandbox,
    repositoryRoot,
    inboxRoot: join(
      repositoryRoot,
      '.local',
      'authority',
      'manual-inbox',
      REVISION_ID,
    ),
    primaryRoot: join(repositoryRoot, '.local', 'authority', 'inputs', REVISION_ID, 'primary'),
    backupRoot: join(sandbox, 'sorcery-tcg-authority-backup-official-2026-08-27-v3'),
    lockPath: join(
      repositoryRoot,
      '.local',
      'authority',
      'locks',
      REVISION_ID,
      'source-set-lock.json',
    ),
  };
}

async function cleanupFixture(fixture: Fixture): Promise<void> {
  const sandbox = resolve(fixture.sandbox);
  assert.equal(dirname(sandbox), resolve(tmpdir()));
  assert.match(basename(sandbox), /^sorcery-manual-intake-/);
  await rm(sandbox, { recursive: true, force: true });
}

async function writeSourceRoot(
  root: string,
  overrides: Readonly<Partial<Record<SourcePath, Buffer>>> = {},
): Promise<void> {
  for (const relativePath of PRIVATE_AUTHORITY_SOURCE_PATHS) {
    const destination = join(root, ...relativePath.split('/'));
    await mkdir(dirname(destination), { recursive: true });
    await writeFile(destination, overrides[relativePath] ?? SOURCE_BYTES[relativePath]);
  }
}

async function writeInbox(
  fixture: Fixture,
  overrides: Readonly<Partial<Record<SourcePath, Buffer>>> = {},
): Promise<void> {
  await writeSourceRoot(fixture.inboxRoot, overrides);
}

async function invokeManualIntake(
  fixture: Fixture,
  options: Readonly<{
    acknowledge?: boolean;
    faultPoint?: string | null;
    descriptorSet?: readonly Record<string, unknown>[];
  }> = {},
): Promise<ProcessResult> {
  const configurationPath = join(fixture.sandbox, 'configuration-' + randomUUID() + '.json');
  await writeFile(
    configurationPath,
    JSON.stringify({
      ...fixture,
      descriptors: options.descriptorSet ?? descriptors(),
      retrievedAt: RETRIEVED_AT,
      acknowledge: options.acknowledge ?? true,
      faultPoint: options.faultPoint ?? null,
    }),
  );
  const command = [
    '. $env:SORCERY_COLLECTOR_SCRIPT',
    '$c = Get-Content -Raw -LiteralPath $env:SORCERY_COLLECTOR_CONFIG | ConvertFrom-Json -Depth 32 -DateKind String',
    'try { Invoke-PrivateAuthorityManualIntakeForTest -RepositoryRoot $c.repositoryRoot -InboxRoot $c.inboxRoot -PrimaryRoot $c.primaryRoot -BackupRoot $c.backupRoot -LockPath $c.lockPath -Descriptors $c.descriptors -AcknowledgePrivateUseRisk:$c.acknowledge -RetrievedAt $c.retrievedAt -FaultPoint $c.faultPoint | ConvertTo-Json -Depth 32 -Compress } catch { [Console]::Error.WriteLine("Synthetic manual intake failed."); exit 1 }',
  ].join('; ');
  return runPwsh(
    ['-Command', command],
    {
      SORCERY_COLLECTOR_SCRIPT: SCRIPT_PATH,
      SORCERY_COLLECTOR_CONFIG: configurationPath,
    },
    60_000,
  );
}

async function exists(path: string): Promise<boolean> {
  return stat(path)
    .then(() => true)
    .catch((error: unknown) => {
      if (typeof error === 'object' && error !== null && 'code' in error && error.code === 'ENOENT') {
        return false;
      }
      throw error;
    });
}

async function inboxHashes(fixture: Fixture): Promise<Readonly<Record<string, string>>> {
  const hashes: Record<string, string> = {};
  for (const relativePath of PRIVATE_AUTHORITY_SOURCE_PATHS) {
    hashes[relativePath] = createHash('sha256')
      .update(await readFile(join(fixture.inboxRoot, ...relativePath.split('/'))))
      .digest('hex');
  }
  return hashes;
}

test('manual intake publishes exact independent roots and a lock last without transport', async () => {
  const fixture = await createFixture();
  try {
    await writeInbox(fixture);
    const before = await inboxHashes(fixture);
    const result = await invokeManualIntake(fixture);
    assert.equal(result.code, 0, result.stderr);
    assert.equal(result.stderr, '');
    assert.equal(result.stdout, '');
    const lock = JSON.parse(await readFile(fixture.lockPath, 'utf8')) as {
      acquisitionMethod: string;
      authorizationReference: string;
      primaryRoot: string;
      backupRoot: string;
      entries: PrivateAuthoritySourceEntry[];
      sourceSetRootHash: string;
      rulebookAcquisitionEvidence: {
        byteHash: string;
        retrievedAt: string;
      };
    };
    assert.equal(lock.acquisitionMethod, MANUAL_METHOD);
    assert.equal(lock.authorizationReference, MANUAL_REFERENCE);
    assert.equal(lock.entries.length, 7);
    assert.equal(await exists(join(fixture.repositoryRoot, '.local', 'authority', 'authorizations')), false);
    assert.deepEqual(await inboxHashes(fixture), before);

    const verified = await verifyPrivateSourceSet({
      repositoryRoot: fixture.repositoryRoot,
      primaryRoot: fixture.primaryRoot,
      backupRoot: fixture.backupRoot,
      entries: lock.entries,
    });
    assert.equal(verified.sourceSetRootHash, lock.sourceSetRootHash);
    for (const relativePath of PRIVATE_AUTHORITY_SOURCE_PATHS) {
      const original = await readFile(join(fixture.inboxRoot, ...relativePath.split('/')));
      assert.deepEqual(
        await readFile(join(fixture.primaryRoot, ...relativePath.split('/'))),
        original,
      );
      assert.deepEqual(
        await readFile(join(fixture.backupRoot, ...relativePath.split('/'))),
        original,
      );
    }

    const offline = await runPwsh([
      '-File',
      SCRIPT_PATH,
      '-VerifyExistingRoots',
      '-LockPath',
      fixture.lockPath,
    ]);
    assert.equal(offline.code, 0, offline.stderr);
    assert.equal(offline.stdout.trimEnd(), 'Private authority existing roots verified.');
    assert.equal(offline.stderr, '');

    const historicalPrimary = join(
      fixture.repositoryRoot,
      '.local',
      'authority',
      'inputs',
      'official-2026-08-20',
      'primary',
    );
    const historicalBackup = join(fixture.sandbox, 'historical-independent-backup');
    const historicalLockPath = join(
      fixture.repositoryRoot,
      '.local',
      'authority',
      'locks',
      'official-2026-08-20',
      'source-set-lock.json',
    );
    await writeSourceRoot(historicalPrimary);
    await writeSourceRoot(historicalBackup);
    await mkdir(dirname(historicalLockPath), { recursive: true });
    const historicalBase = {
      ...lock,
      primaryRoot: historicalPrimary,
      backupRoot: historicalBackup,
    };
    const legacyLock: Partial<typeof lock> = {
      ...historicalBase,
      acquisitionMethod: 'user-run-one-shot-powershell',
    };
    delete legacyLock.authorizationReference;
    await writeFile(historicalLockPath, JSON.stringify(legacyLock));
    const legacy = await runPwsh([
      '-File',
      SCRIPT_PATH,
      '-VerifyExistingRoots',
      '-LockPath',
      historicalLockPath,
    ]);
    assert.equal(legacy.code, 0, legacy.stderr);

    const agentLock = {
      ...historicalBase,
      acquisitionMethod: 'user-authorized-agent-run-one-shot-powershell',
      authorizationReference: 'quick-260825-mhh-retry-1',
    };
    await writeFile(fixture.lockPath, JSON.stringify(agentLock));
    const wrongRevision = await runPwsh([
      '-File',
      SCRIPT_PATH,
      '-VerifyExistingRoots',
      '-LockPath',
      fixture.lockPath,
    ]);
    assert.notEqual(wrongRevision.code, 0);

    await writeFile(historicalLockPath, JSON.stringify(agentLock));
    const unboundAgent = await runPwsh([
      '-File',
      SCRIPT_PATH,
      '-VerifyExistingRoots',
      '-LockPath',
      historicalLockPath,
    ]);
    assert.notEqual(unboundAgent.code, 0);

    const authorizationPath = join(
      fixture.repositoryRoot,
      '.local',
      'authority',
      'authorizations',
      'quick-260825-mhh-retry-1.consumed.json',
    );
    await mkdir(dirname(authorizationPath), { recursive: true });
    const authorizationRecord = {
      schemaVersion: 1,
      revisionId: 'official-2026-08-20',
      acquisitionMethod: 'user-authorized-agent-run-one-shot-powershell',
      authorizationReference: 'quick-260825-mhh-retry-1',
      consumedAt: RETRIEVED_AT,
    };
    await writeFile(
      authorizationPath,
      JSON.stringify({ ...authorizationRecord, consumedAt: '2026-99-99T99:99:99.999Z' }),
    );
    const impossibleTimestamp = await runPwsh([
      '-File',
      SCRIPT_PATH,
      '-VerifyExistingRoots',
      '-LockPath',
      historicalLockPath,
    ]);
    assert.notEqual(impossibleTimestamp.code, 0);

    await writeFile(authorizationPath, JSON.stringify(authorizationRecord));
    const boundAgent = await runPwsh([
      '-File',
      SCRIPT_PATH,
      '-VerifyExistingRoots',
      '-LockPath',
      historicalLockPath,
    ]);
    assert.equal(boundAgent.code, 0, boundAgent.stderr);

    await writeFile(
      fixture.lockPath,
      JSON.stringify({
        ...lock,
        acquisitionMethod: 'user-authorized-user-run-one-shot-powershell',
        authorizationReference: 'phase-01-20260827-private-reacquisition-1',
      }),
    );
    const retired = await runPwsh([
      '-File',
      SCRIPT_PATH,
      '-VerifyExistingRoots',
      '-LockPath',
      fixture.lockPath,
    ]);
    assert.notEqual(retired.code, 0);

    await writeFile(
      fixture.lockPath,
      JSON.stringify({
        ...lock,
        rulebookAcquisitionEvidence: {
          ...lock.rulebookAcquisitionEvidence,
          byteHash: 'sha256:' + '0'.repeat(64),
        },
      }),
    );
    const evidenceMismatch = await runPwsh([
      '-File',
      SCRIPT_PATH,
      '-VerifyExistingRoots',
      '-LockPath',
      fixture.lockPath,
    ]);
    assert.notEqual(evidenceMismatch.code, 0);
  } finally {
    await cleanupFixture(fixture);
  }
});

test('manual intake rejects missing extra linked and malformed inboxes before publication', async (context) => {
  const cases: readonly Readonly<{
    name: string;
    mutate: (fixture: Fixture) => Promise<void>;
    mutatedPath?: SourcePath;
    descriptorSet?: readonly Record<string, unknown>[];
  }>[] = [
    {
      name: 'missing file',
      mutate: async (fixture) =>
        rm(join(fixture.inboxRoot, 'codex', 'faqs-current.html')),
    },
    {
      name: 'extra file',
      mutate: async (fixture) =>
        writeFile(join(fixture.inboxRoot, 'codex', 'extra.html'), '<!doctype html>extra'),
    },
    {
      name: 'file above declared maximum',
      mutate: () => Promise.resolve(),
      descriptorSet: descriptors().map((descriptor) =>
        descriptor.relativePath === 'formats/constructed-current.html'
          ? {
              ...descriptor,
              maxBytes: SOURCE_BYTES['formats/constructed-current.html'].byteLength - 1,
            }
          : descriptor,
      ),
    },
    {
      name: 'header and navigation only HTML shell',
      mutatedPath: 'formats/constructed-current.html',
      mutate: async (fixture) =>
        writeFile(
          join(fixture.inboxRoot, 'formats', 'constructed-current.html'),
          '<!doctype html><header><h1>Constructed Format</h1></header>' +
            '<nav>Deck Construction</nav><script>' +
            'x'.repeat(600) +
            '</script>',
        ),
    },    {
      name: 'HTML truncated after valid visible content',
      mutatedPath: 'formats/constructed-current.html',
      mutate: async (fixture) =>
        writeFile(
          join(fixture.inboxRoot, 'formats', 'constructed-current.html'),
          '<!doctype html><html><body><main><h1>Constructed Format</h1>' +
            '<h2>Deck Construction</h2><p>' +
            'visible content '.repeat(50),
        ),
    },
    {
      name: 'structurally incomplete PDF',
      mutatedPath: 'rulebook/rulebook-current.pdf',
      mutate: async (fixture) =>
        writeFile(
          join(fixture.inboxRoot, 'rulebook', 'rulebook-current.pdf'),
          '%PDF-1.7\n' + 'x'.repeat(32_768) + '\n%%EOF',
        ),
    },    {
      name: 'PDF with invalid startxref target',
      mutatedPath: 'rulebook/rulebook-current.pdf',
      mutate: async (fixture) =>
        writeFile(
          join(fixture.inboxRoot, 'rulebook', 'rulebook-current.pdf'),
          SOURCE_BYTES['rulebook/rulebook-current.pdf']
            .toString('utf8')
            .replace(/startxref\n\d+/, 'startxref\n0'),
        ),
    },
    {
      name: 'partial card array',
      mutatedPath: 'cards/cards.raw.json',
      mutate: async (fixture) =>
        writeFile(
          join(fixture.inboxRoot, 'cards', 'cards.raw.json'),
          JSON.stringify([syntheticOfficialCard(0)]),
        ),
    },
  ];
  for (const scenario of cases) {
    await context.test(scenario.name, async () => {
      const fixture = await createFixture();
      try {
        await writeInbox(fixture);
        const before = await inboxHashes(fixture).catch(() => null);
        await scenario.mutate(fixture);
        const result = await invokeManualIntake(
          fixture,
          scenario.descriptorSet === undefined
            ? {}
            : { descriptorSet: scenario.descriptorSet },
        );
        assert.notEqual(result.code, 0);
        assert.equal(result.stdout, '');
        assert.equal(result.stderr.trimEnd(), 'Synthetic manual intake failed.');
        assert.equal(await exists(fixture.primaryRoot), false);
        assert.equal(await exists(fixture.backupRoot), false);
        assert.equal(await exists(fixture.lockPath), false);
        if (before !== null && scenario.name !== 'missing file') {
          const after = await inboxHashes(fixture);
          for (const relativePath of PRIVATE_AUTHORITY_SOURCE_PATHS) {
            if (relativePath === scenario.mutatedPath) continue;
            assert.equal(after[relativePath], before[relativePath]);
          }
        }
      } finally {
        await cleanupFixture(fixture);
      }
    });
  }

  await context.test('linked file', async (nested) => {
    const fixture = await createFixture();
    try {
      await writeInbox(fixture);
      const linkedPath = join(fixture.inboxRoot, 'formats', 'constructed-current.html');
      const targetPath = join(fixture.sandbox, 'linked-source.html');
      await writeFile(targetPath, SOURCE_BYTES['formats/constructed-current.html']);
      await rm(linkedPath);
      try {
        await symlink(targetPath, linkedPath, 'file');
      } catch (error: unknown) {
        if (
          typeof error === 'object' &&
          error !== null &&
          'code' in error &&
          (error.code === 'EPERM' || error.code === 'EACCES')
        ) {
          nested.skip('file symlink creation is unavailable');
          return;
        }
        throw error;
      }
      const result = await invokeManualIntake(fixture);
      assert.notEqual(result.code, 0);
      assert.equal(result.stderr.trimEnd(), 'Synthetic manual intake failed.');
      assert.equal(await exists(fixture.lockPath), false);
    } finally {
      await cleanupFixture(fixture);
    }
  });
});

test('manual intake never overwrites destinations and preserves inbox on publication failure', async (context) => {
  await context.test('preexisting destination', async () => {
    const fixture = await createFixture();
    try {
      await writeInbox(fixture);
      await mkdir(fixture.primaryRoot, { recursive: true });
      await writeFile(join(fixture.primaryRoot, 'sentinel'), 'unchanged');
      const result = await invokeManualIntake(fixture);
      assert.notEqual(result.code, 0);
      assert.equal(await readFile(join(fixture.primaryRoot, 'sentinel'), 'utf8'), 'unchanged');
      assert.equal(await exists(fixture.backupRoot), false);
      assert.equal(await exists(fixture.lockPath), false);
    } finally {
      await cleanupFixture(fixture);
    }
  });

  await context.test('publication destination overlapping inbox', async () => {
    const fixture = await createFixture();
    try {
      await writeInbox(fixture);
      const before = await inboxHashes(fixture);
      const overlapping = {
        ...fixture,
        primaryRoot: join(fixture.inboxRoot, 'published'),
      };
      const result = await invokeManualIntake(overlapping);
      assert.notEqual(result.code, 0);
      assert.deepEqual(await inboxHashes(fixture), before);
      assert.equal(await exists(overlapping.primaryRoot), false);
      assert.equal(await exists(fixture.backupRoot), false);
      assert.equal(await exists(fixture.lockPath), false);
    } finally {
      await cleanupFixture(fixture);
    }
  });

  await context.test('failure before lock publication', async () => {
    const fixture = await createFixture();
    try {
      await writeInbox(fixture);
      const before = await inboxHashes(fixture);
      const result = await invokeManualIntake(fixture, { faultPoint: 'before-lock-move' });
      assert.notEqual(result.code, 0);
      assert.equal(result.stderr.trimEnd(), 'Synthetic manual intake failed.');
      assert.deepEqual(await inboxHashes(fixture), before);
      assert.equal(await exists(fixture.primaryRoot), false);
      assert.equal(await exists(fixture.backupRoot), false);
      assert.equal(await exists(fixture.lockPath), false);
      const backupParent = dirname(fixture.backupRoot);
      assert.ok(
        (await readdir(backupParent)).some((name) =>
          name.startsWith(basename(fixture.backupRoot) + '.failed-'),
        ),
      );
      assert.ok(
        (await readdir(dirname(fixture.primaryRoot))).some((name) =>
          name.startsWith(basename(fixture.primaryRoot) + '.failed-'),
        ),
      );
    } finally {
      await cleanupFixture(fixture);
    }
  });
});

test('manual intake requires acknowledgment before reading the inbox', async () => {
  const fixture = await createFixture();
  try {
    const result = await invokeManualIntake(fixture, { acknowledge: false });
    assert.notEqual(result.code, 0);
    assert.equal(result.stdout, '');
    assert.equal(result.stderr.trimEnd(), 'Synthetic manual intake failed.');
    assert.equal(await exists(fixture.primaryRoot), false);
    assert.equal(await exists(fixture.backupRoot), false);
    assert.equal(await exists(fixture.lockPath), false);
  } finally {
    await cleanupFixture(fixture);
  }
});

test('manual intake performs no runtime network requests', async () => {
  const fixture = await createFixture();
  let requestCount = 0;
  const server = createTcpServer((socket) => {
    requestCount += 1;
    socket.destroy();
  });
  try {
    await writeInbox(fixture);
    await new Promise<void>((resolveListen, rejectListen) => {
      server.once('error', rejectListen);
      server.listen(0, '127.0.0.1', resolveListen);
    });
    const address = server.address();
    if (address === null || typeof address === 'string') throw new Error('Loopback canary failed');
    const canaryDescriptors = descriptors().map((descriptor, index) => ({
      ...descriptor,
      provenanceUrl:
        'https://127.0.0.1:' + String(address.port) + '/unexpected-' + String(index),
    }));
    const result = await invokeManualIntake(fixture, { descriptorSet: canaryDescriptors });
    assert.notEqual(result.code, 0);
    assert.equal(result.stdout, '');
    assert.equal(requestCount, 0);
    assert.equal(await exists(fixture.primaryRoot), false);
    assert.equal(await exists(fixture.backupRoot), false);
    assert.equal(await exists(fixture.lockPath), false);
  } finally {
    if (server.listening) {
      await new Promise<void>((resolveClose, rejectClose) => {
        server.close((error) => {
          if (error) rejectClose(error);
          else resolveClose();
        });
      });
    }
    await cleanupFixture(fixture);
  }
});

test('production manual intake has one fixed offline source and no transport surface', async () => {
  const source = await readFile(SCRIPT_PATH, 'utf8');
  for (const required of [
    '.local/authority/manual-inbox/official-2026-08-27-v3',
    '.local/authority/inputs/official-2026-08-27-v3/primary',
    '.local/authority/locks/official-2026-08-27-v3/source-set-lock.json',
    'sorcery-tcg-authority-backup-official-2026-08-27-v3',
    MANUAL_METHOD,
    MANUAL_REFERENCE,
    'Private authority manual intake completed.',
    'Private authority manual intake failed.',
    'Private authority existing roots verified.',
  ]) {
    assert.ok(source.includes(required), 'missing fixed manual-intake contract');
  }
  for (const relativePath of PRIVATE_AUTHORITY_SOURCE_PATHS) {
    assert.ok(source.includes(relativePath));
  }
  assert.doesNotMatch(
    source,
    /\b(?:HttpClient|HttpWebRequest|Invoke-WebRequest|Invoke-RestMethod|WebClient|TcpClient|Start-BitsTransfer|Start-Process|curl(?:\.exe)?|fetch|Socket|Invoke-PrivateAuthorityTransport|Invoke-BoundedHttpToFile|New-PrivateAuthorityAcquisitionContext)\b|node:(?:http|https|net|tls)/i,
  );
  assert.doesNotMatch(source, /Start-Sleep|while\s*\(\s*\$true|\.png|\.jpe?g/i);


  const ast = await runPwsh([
    '-Command',
    [
      '$tokens=$null; $errors=$null',
      '$ast=[Management.Automation.Language.Parser]::ParseFile($env:SORCERY_COLLECTOR_SCRIPT,[ref]$tokens,[ref]$errors)',
      'if($errors.Count){exit 1}',
      '$params=@($ast.ParamBlock.Parameters.Name.VariablePath.UserPath)',
      '$exports=@($ast.FindAll({param($n) $n -is [Management.Automation.Language.CommandAst] -and $n.GetCommandName() -eq "Export-ModuleMember"},$true) | ForEach-Object {$_.Extent.Text})',
      '[pscustomobject]@{params=$params;exports=$exports} | ConvertTo-Json -Depth 8 -Compress',
    ].join('; '),
  ], { SORCERY_COLLECTOR_SCRIPT: SCRIPT_PATH });
  assert.equal(ast.code, 0, ast.stderr);
  const shape = JSON.parse(ast.stdout) as { params: string[]; exports: string[] };
  assert.deepEqual(shape.params, [
    'ImportManualInbox',
    'AcknowledgePrivateUseRisk',
    'VerifyExistingRoots',
    'LockPath',
  ]);
  assert.equal(shape.exports.length, 1);
  assert.match(shape.exports[0] ?? '', /Invoke-PrivateAuthorityManualIntakeForTest/);
  assert.doesNotMatch(shape.exports[0] ?? '', /Collection|Transport|Http/i);
});
