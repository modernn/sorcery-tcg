import assert from 'node:assert/strict';
import { spawn } from 'node:child_process';
import { createHash, randomUUID } from 'node:crypto';
import { mkdir, mkdtemp, readFile, readdir, rm, stat, writeFile } from 'node:fs/promises';
import { createServer, type Server } from 'node:http';
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

function syntheticOfficialCard(
  name = 'Synthetic Adept',
  slug = 'synthetic-adept',
): Record<string, unknown> {
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
    name,
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
            slug,
            typeText: 'Minion',
          },
        ],
      },
    ],
    subTypes: 'Synthetic',
  };
}

const SOURCE_BYTES = Object.freeze({
  'rulebook/rulebook-current.pdf': Buffer.from('%PDF-1.7\nsynthetic rulebook\n%%EOF'),
  'formats/constructed-current.html': Buffer.from(
    '<!doctype html><title>Constructed Format</title><main><h1>Constructed Format</h1><h2>Deck Construction</h2><p>minimum deck size</p></main>',
  ),
  'codex/codex-current.html': Buffer.from(
    '<!doctype html><title>Welcome to the Codex</title><main><h1>Welcome to the Codex</h1><nav>Card Rulings</nav><article>Rules Questions</article></main>',
  ),
  'codex/faqs-current.html': Buffer.from(
    '<!doctype html><title>FAQs</title><main><h1>FAQs</h1><h2>Frequently Asked Questions</h2><article>Gameplay Questions</article></main>',
  ),
  'codex/changelog-current.html': Buffer.from(
    '<!doctype html><title>Codex Changelog</title><main><h1>Codex Changelog</h1><article><h2>20 August 2026</h2><p>Rules update</p></article></main>',
  ),
  'updates/card-updates-2025.html': Buffer.from(
    '<!doctype html><title>Sorcery: Contested Realm Card Updates 2025</title><main><h1>Sorcery: Contested Realm Card Updates 2025</h1><article><h2>Card Updates</h2><p>Effective November 25</p></article></main>',
  ),
  'cards/cards.raw.json': Buffer.from(`${JSON.stringify([syntheticOfficialCard()])}\n`),
});

type SourcePath = keyof typeof SOURCE_BYTES;
type Fixture = Readonly<{
  sandbox: string;
  repositoryRoot: string;
  primaryRoot: string;
  backupRoot: string;
  lockPath: string;
  authorizationPath: string;
  retryAuthorizationPath: string;
}>;
type ProcessResult = Readonly<{ code: number | null; stdout: string; stderr: string }>;

function runPwsh(
  arguments_: readonly string[],
  environment: Readonly<Record<string, string>> = {},
): Promise<ProcessResult> {
  return new Promise((resolveProcess, reject) => {
    const child = spawn('pwsh', ['-NoProfile', '-NonInteractive', ...arguments_], {
      cwd: REPOSITORY_ROOT,
      env: { ...process.env, ...environment },
      windowsHide: true,
    });
    let stdout = '';
    let stderr = '';
    child.stdout.setEncoding('utf8').on('data', (chunk: string) => {
      stdout += chunk;
    });
    child.stderr.setEncoding('utf8').on('data', (chunk: string) => {
      stderr += chunk;
    });
    child.once('error', reject);
    child.once('close', (code) => resolveProcess({ code, stdout, stderr }));
  });
}

async function createFixture(): Promise<Fixture> {
  const sandbox = await mkdtemp(join(tmpdir(), 'sorcery-private-collector-'));
  const repositoryRoot = join(sandbox, 'repository');
  const primaryRoot = join(repositoryRoot, '.local', 'authority', 'inputs', 'test', 'primary');
  const backupRoot = join(sandbox, "backup with spaces & apostrophe's ($) [safe]");
  const lockPath = join(repositoryRoot, '.local', 'authority', 'locks', 'test', 'source-set-lock.json');
  const authorizationPath = join(
    repositoryRoot,
    '.local',
    'authority',
    'authorizations',
    'quick-260825-mhh.consumed.json',
  );
  const retryAuthorizationPath = join(
    repositoryRoot,
    '.local',
    'authority',
    'authorizations',
    'quick-260825-mhh-retry-1.consumed.json',
  );
  await mkdir(repositoryRoot, { recursive: true });
  return {
    sandbox,
    repositoryRoot,
    primaryRoot,
    backupRoot,
    lockPath,
    authorizationPath,
    retryAuthorizationPath,
  };
}

async function cleanupFixture(fixture: Fixture): Promise<void> {
  const sandbox = resolve(fixture.sandbox);
  assert.equal(dirname(sandbox), resolve(tmpdir()));
  assert.match(basename(sandbox), /^sorcery-private-collector-/);
  await rm(sandbox, { recursive: true, force: true });
}

function descriptor(relativePath: SourcePath, requestUrl: string): Record<string, unknown> {
  const htmlMarkers: Partial<Record<SourcePath, string>> = {
    'formats/constructed-current.html': 'Constructed Format',
    'codex/codex-current.html': 'Welcome to the Codex',
    'codex/faqs-current.html': 'FAQs',
    'codex/changelog-current.html': 'Codex Changelog',
    'updates/card-updates-2025.html': 'Sorcery: Contested Realm Card Updates 2025',
  };
  const visibleBodyMarkers: Partial<Record<SourcePath, readonly string[]>> = {
    'formats/constructed-current.html': ['Constructed Format', 'Deck Construction', 'minimum deck size'],
    'codex/codex-current.html': ['Welcome to the Codex', 'Card Rulings', 'Rules Questions'],
    'codex/faqs-current.html': ['FAQs', 'Frequently Asked Questions', 'Gameplay Questions'],
    'codex/changelog-current.html': ['Codex Changelog', 'Rules update'],
    'updates/card-updates-2025.html': [
      'Sorcery: Contested Realm Card Updates 2025',
      'Card Updates',
      'Effective November 25',
    ],
  };
  return {
    relativePath,
    provenanceUrl: OFFICIAL_URLS[relativePath],
    requestUrl,
    mediaType: relativePath.endsWith('.pdf')
      ? 'application/pdf'
      : relativePath.endsWith('.json')
        ? 'application/json'
        : 'text/html',
    sourceMarker:
      relativePath === 'rulebook/rulebook-current.pdf'
        ? 'Sorcery: Contested Realm December 2025 Rulebook Update'
        : (htmlMarkers[relativePath] ?? null),
    visibleBodyMarkers: visibleBodyMarkers[relativePath] ?? null,
    expectedCardCount: relativePath === 'cards/cards.raw.json' ? 1 : null,
    effectiveDatePolicy: relativePath.includes('changelog')
      ? 'changelog'
      : relativePath === 'rulebook/rulebook-current.pdf' || relativePath.includes('card-updates')
        ? 'fixed'
        : 'none',
    effectiveDate:
      relativePath === 'rulebook/rulebook-current.pdf'
        ? '2025-12-19'
        : relativePath.includes('card-updates')
          ? '2025-11-25'
          : null,
    maxBytes: 16_384,
    allowedHosts: ['127.0.0.1'],
    allowedRedirectHosts: ['127.0.0.1'],
  };
}

async function listen(server: Server): Promise<number> {
  await new Promise<void>((resolveListen, reject) => {
    server.once('error', reject);
    server.listen(0, '127.0.0.1', () => resolveListen());
  });
  const address = server.address();
  assert.ok(address && typeof address === 'object');
  return address.port;
}

async function close(server: Server): Promise<void> {
  await new Promise<void>((resolveClose, reject) =>
    server.close((error) => (error === undefined ? resolveClose() : reject(error))),
  );
}

function testDescriptors(baseUrl: string): readonly Record<string, unknown>[] {
  return (Object.keys(SOURCE_BYTES) as SourcePath[]).map((relativePath) => {
    const route =
      relativePath === 'rulebook/rulebook-current.pdf'
        ? '/release'
        : `/${relativePath.replaceAll('/', '-')}`;
    const value = descriptor(relativePath, `${baseUrl}${route}`);
    if (relativePath === 'rulebook/rulebook-current.pdf') {
      value.rulebookDownloadBaseUrl = `${baseUrl}/drive-download`;
      value.rulebookLocatorHosts = ['127.0.0.1'];
      value.allowedRedirectHosts = ['127.0.0.1'];
    }
    return value;
  });
}

async function invokeLoopback(
  fixture: Fixture,
  descriptors: readonly Record<string, unknown>[],
  options: Readonly<{
    headerTimeoutSeconds?: number;
    bodyTimeoutSeconds?: number;
    maxTotalBytes?: number;
    faultPoint?: string;
    userAuthorizationReference?: string;
    acknowledgePrivateUseRisk?: boolean;
  }> = {},
): Promise<ProcessResult> {
  const configurationPath = join(fixture.sandbox, `configuration-${randomUUID()}.json`);
  await writeFile(
    configurationPath,
    JSON.stringify({
      ...fixture,
      descriptors,
      headerTimeoutSeconds: options.headerTimeoutSeconds ?? 3,
      bodyTimeoutSeconds: options.bodyTimeoutSeconds ?? 3,
      maxTotalBytes: options.maxTotalBytes ?? 131_072,
      faultPoint: options.faultPoint ?? null,
      userAuthorizationReference: options.userAuthorizationReference ?? null,
      acknowledgePrivateUseRisk: options.acknowledgePrivateUseRisk ?? true,
    }),
  );
  const command = [
    '. $env:SORCERY_COLLECTOR_SCRIPT',
    '$c = Get-Content -Raw -LiteralPath $env:SORCERY_COLLECTOR_CONFIG | ConvertFrom-Json -Depth 32',
    'Invoke-PrivateAuthorityCollectionForLoopbackTest -RepositoryRoot $c.repositoryRoot -PrimaryRoot $c.primaryRoot -BackupRoot $c.backupRoot -LockPath $c.lockPath -Descriptors $c.descriptors -HeaderTimeoutSeconds $c.headerTimeoutSeconds -BodyTimeoutSeconds $c.bodyTimeoutSeconds -MaxTotalBytes $c.maxTotalBytes -AcknowledgePrivateUseRisk:$c.acknowledgePrivateUseRisk -FaultPoint $c.faultPoint -UserAuthorizationReference $c.userAuthorizationReference | ConvertTo-Json -Depth 32 -Compress',
  ].join('; ');
  return runPwsh(['-Command', command], {
    SORCERY_COLLECTOR_SCRIPT: SCRIPT_PATH,
    SORCERY_COLLECTOR_CONFIG: configurationPath,
  });
}

function createHappyAuthorityServer(
  rulebookBytes: Buffer = SOURCE_BYTES['rulebook/rulebook-current.pdf'],
  changelogBytes: Buffer = SOURCE_BYTES['codex/changelog-current.html'],
  overrides: Readonly<Partial<Record<SourcePath, Buffer>>> = {},
): Readonly<{
  server: Server;
  requests: string[];
  setPort: (value: number) => void;
}> {
  const requests: string[] = [];
  let port = 0;
  const server = createServer((request, response) => {
    const path = request.url ?? '';
    requests.push(path);
    if (path === '/release') {
      response.setHeader('content-type', 'text/html; charset=utf-8');
      response.end(
        '<!doctype html><title>Sorcery: Contested Realm December 2025 Rulebook Update</title>' +
          '<time>19 Dec 2025</time>' +
          `<a href="http://127.0.0.1:${port}/file/d/annotated/view">Sorcery: Contested Realm Rulebook (December 2025) Annotated</a>` +
          `<a href="http://127.0.0.1:${port}/file/d/standard/view">Sorcery: Contested Realm Rulebook (December 2025)</a>` +
          '<img src="/artwork-must-not-be-requested">',
      );
      return;
    }
    if (path === '/drive-download?export=download&id=standard') {
      response.statusCode = 303;
      response.setHeader('location', '/pdf');
      response.end();
      return;
    }
    if (path === '/pdf') {
      response.setHeader('content-type', 'application/octet-stream');
      response.setHeader('content-disposition', 'attachment; filename=SorceryRulebook.pdf');
      response.end(rulebookBytes);
      return;
    }
    const relativePath = (Object.keys(SOURCE_BYTES) as SourcePath[]).find(
      (candidate) => path === `/${candidate.replaceAll('/', '-')}`,
    );
    if (relativePath === undefined || relativePath === 'rulebook/rulebook-current.pdf') {
      response.statusCode = 500;
      response.end('unexpected route');
      return;
    }
    response.setHeader(
      'content-type',
      relativePath.endsWith('.json') ? 'application/json; charset=utf-8' : 'text/html; charset=utf-8',
    );
    response.end(
      overrides[relativePath] ??
        (relativePath === 'codex/changelog-current.html' ? changelogBytes : SOURCE_BYTES[relativePath]),
    );
  });
  return { server, requests, setPort: (value) => (port = value) };
}

async function relativeFiles(root: string): Promise<readonly string[]> {
  const found: string[] = [];
  async function visit(directory: string, prefix: string): Promise<void> {
    for (const entry of await readdir(directory, { withFileTypes: true })) {
      const relativePath = prefix === '' ? entry.name : `${prefix}/${entry.name}`;
      if (entry.isDirectory()) await visit(join(directory, entry.name), relativePath);
      else found.push(relativePath);
    }
  }
  await visit(root, '');
  return found.sort();
}

async function safeReaddir(root: string): Promise<readonly string[]> {
  return readdir(root).catch((error: unknown) => {
    if (typeof error === 'object' && error !== null && 'code' in error && error.code === 'ENOENT') return [];
    throw error;
  });
}

test('dot-sourcing exposes only the closed collector surface and forwards authorization context', async () => {
  const fixture = await createFixture();
  try {
    const command = [
      '. $env:SORCERY_COLLECTOR_SCRIPT',
      '$tokens = $null; $errors = $null',
      '$ast = [Management.Automation.Language.Parser]::ParseFile($env:SORCERY_COLLECTOR_SCRIPT, [ref]$tokens, [ref]$errors)',
      '$definedFunctions = @($ast.FindAll({ param($node) $node -is [Management.Automation.Language.FunctionDefinitionAst] }, $true) | ForEach-Object Name)',
      '$resolvedFunctions = @($definedFunctions | Where-Object { $null -ne (Get-Command -Name $_ -ErrorAction SilentlyContinue) } | Sort-Object -Unique)',
      '$signatures = [ordered]@{}',
      "foreach ($name in @('Invoke-PrivateAuthorityCollection', 'Invoke-PrivateAuthorityCollectionForLoopbackTest', 'Invoke-PrivateAuthorityCollectionCore')) { $functionAst = @($ast.FindAll({ param($node) $node -is [Management.Automation.Language.FunctionDefinitionAst] -and $node.Name -eq $name }, $true))[0]; $signatures[$name] = @($functionAst.Body.ParamBlock.Parameters.Name.VariablePath.UserPath) }",
      '[pscustomobject]@{ parameters = @($ast.ParamBlock.Parameters.Name.VariablePath.UserPath); functions = $resolvedFunctions; signatures = $signatures; moduleVariableLeaked = [bool](Get-Variable collectorModule -ErrorAction SilentlyContinue) } | ConvertTo-Json -Depth 8 -Compress',
    ].join('; ');
    const result = await runPwsh(['-Command', command], { SORCERY_COLLECTOR_SCRIPT: SCRIPT_PATH });
    assert.equal(result.code, 0, result.stderr);
    assert.deepEqual(JSON.parse(result.stdout.trim()), {
      parameters: ['BackupRoot', 'AcknowledgePrivateUseRisk', 'UserAuthorizationReference'],
      functions: [
        'Get-ProductionSourceDescriptors',
        'Invoke-PrivateAuthorityCollection',
        'Invoke-PrivateAuthorityCollectionForLoopbackTest',
      ],
      signatures: {
        'Invoke-PrivateAuthorityCollection': [
          'BackupRoot',
          'AcknowledgePrivateUseRisk',
          'UserAuthorizationReference',
        ],
        'Invoke-PrivateAuthorityCollectionForLoopbackTest': [
          'RepositoryRoot',
          'PrimaryRoot',
          'BackupRoot',
          'LockPath',
          'Descriptors',
          'HeaderTimeoutSeconds',
          'BodyTimeoutSeconds',
          'MaxTotalBytes',
          'AcknowledgePrivateUseRisk',
          'FaultPoint',
          'UserAuthorizationReference',
        ],
        'Invoke-PrivateAuthorityCollectionCore': [
          'RepositoryRoot',
          'PrimaryRoot',
          'BackupRoot',
          'LockPath',
          'Descriptors',
          'HeaderTimeoutSeconds',
          'BodyTimeoutSeconds',
          'MaxTotalBytes',
          'AcknowledgePrivateUseRisk',
          'LoopbackOnly',
          'FaultPoint',
          'UserAuthorizationReference',
        ],
      },
      moduleVariableLeaked: false,
    });
    assert.deepEqual(await readdir(fixture.sandbox), ['repository']);
  } finally {
    await cleanupFixture(fixture);
  }
});

test('direct wrapper rejects missing backup or acknowledgment before filesystem work', async () => {
  const fixture = await createFixture();
  try {
    const noBackup = await runPwsh(['-File', SCRIPT_PATH, '-AcknowledgePrivateUseRisk']);
    assert.notEqual(noBackup.code, 0);
    assert.match(noBackup.stderr, /BackupRoot/);

    const noAcknowledgment = await runPwsh(['-File', SCRIPT_PATH, '-BackupRoot', fixture.backupRoot]);
    assert.notEqual(noAcknowledgment.code, 0);
    assert.match(noAcknowledgment.stderr, /AcknowledgePrivateUseRisk/);
    assert.deepEqual(await readdir(fixture.sandbox), ['repository']);
  } finally {
    await cleanupFixture(fixture);
  }
});

test('every exported collection path requires acknowledgment before authorization or transport', async () => {
  const fixture = await createFixture();
  const authority = createHappyAuthorityServer();
  try {
    const exported = await runPwsh(
      [
        '-Command',
        '. $env:SORCERY_COLLECTOR_SCRIPT; Invoke-PrivateAuthorityCollection -BackupRoot relative -UserAuthorizationReference unknown',
      ],
      { SORCERY_COLLECTOR_SCRIPT: SCRIPT_PATH },
    );
    assert.notEqual(exported.code, 0);
    assert.match(exported.stderr, /AcknowledgePrivateUseRisk/);

    const port = await listen(authority.server);
    authority.setPort(port);
    const descriptors = testDescriptors(`http://127.0.0.1:${port}`);
    const missing = await invokeLoopback(fixture, descriptors, {
      acknowledgePrivateUseRisk: false,
      userAuthorizationReference: 'quick-260825-mhh-retry-1',
    });
    assert.notEqual(missing.code, 0);
    assert.match(missing.stderr, /AcknowledgePrivateUseRisk/);
    assert.equal(authority.requests.length, 0);
    assert.equal(await stat(fixture.retryAuthorizationPath).then(() => true, () => false), false);
    assert.equal(await stat(fixture.primaryRoot).then(() => true, () => false), false);
    assert.equal(await stat(fixture.backupRoot).then(() => true, () => false), false);
    assert.equal(await stat(fixture.lockPath).then(() => true, () => false), false);

    const acknowledged = await invokeLoopback(fixture, descriptors, {
      userAuthorizationReference: 'quick-260825-mhh-retry-1',
    });
    assert.equal(acknowledged.code, 0, acknowledged.stderr);
    assert.ok(authority.requests.length > 0);
    const lock = JSON.parse(await readFile(fixture.lockPath, 'utf8')) as Record<string, unknown>;
    assert.equal(lock.authorizationReference, 'quick-260825-mhh-retry-1');
    assert.deepEqual(lock.operatingAcknowledgment, {
      scope: 'private-local-noncommercial',
      noRedistributionReleaseHostingUploadOrArtwork: true,
      apiTermsRobotsConflictAndPrivateUseRiskAccepted: true,
      establishesLegalPermission: false,
      stopOnBlockedStatusCaptchaOrPublisherObjection: true,
      retryOrEvasion: false,
    });
  } finally {
    await close(authority.server);
    await cleanupFixture(fixture);
  }
});

test('only the two exact agent authorizations are consumed independently before transport', async () => {
  const fixture = await createFixture();
  const authority = createHappyAuthorityServer();
  try {
    const port = await listen(authority.server);
    authority.setPort(port);
    const descriptors = testDescriptors(`http://127.0.0.1:${port}`);

    for (const reference of [
      'unknown',
      ' quick-260825-mhh',
      'quick-260825-mhh ',
      'QUICK-260825-MHH',
      ' quick-260825-mhh-retry-1',
      'quick-260825-mhh-retry-1 ',
      'QUICK-260825-MHH-RETRY-1',
      'quick-260825-mhh-retry-2',
    ]) {
      const rejected = await invokeLoopback(fixture, descriptors, {
        userAuthorizationReference: reference,
      });
      assert.notEqual(rejected.code, 0);
      assert.match(rejected.stderr, /authorization reference/i);
    }
    assert.equal(authority.requests.length, 0);
    assert.equal(await stat(fixture.authorizationPath).then(() => true, () => false), false);

    const consumed = await invokeLoopback(fixture, descriptors, {
      faultPoint: 'after-authorization-consumption',
      userAuthorizationReference: 'quick-260825-mhh',
    });
    assert.notEqual(consumed.code, 0);
    assert.match(consumed.stderr, /after-authorization-consumption/i);
    assert.equal(authority.requests.length, 0);

    const recordBytes = await readFile(fixture.authorizationPath, 'utf8');
    const record = JSON.parse(recordBytes) as Record<string, unknown>;
    assert.deepEqual(Object.keys(record).sort(), [
      'acquisitionMethod',
      'authorizationReference',
      'consumedAt',
      'revisionId',
      'schemaVersion',
    ]);
    assert.equal(record.schemaVersion, 1);
    assert.equal(record.revisionId, 'official-2026-08-20');
    assert.equal(record.acquisitionMethod, 'user-authorized-agent-run-one-shot-powershell');
    assert.equal(record.authorizationReference, 'quick-260825-mhh');
    assert.match(String(record.consumedAt), /^\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}\.\d{3}Z$/);
    assert.doesNotMatch(recordBytes, /https?:|primaryRoot|backupRoot|locator/i);

    const retried = await invokeLoopback(fixture, descriptors, {
      userAuthorizationReference: 'quick-260825-mhh',
    });
    assert.notEqual(retried.code, 0);
    assert.match(retried.stderr, /authorization.*consumed|already exists/i);
    assert.equal(authority.requests.length, 0);
    assert.equal(await readFile(fixture.authorizationPath, 'utf8'), recordBytes);

    const retryConsumed = await invokeLoopback(fixture, descriptors, {
      faultPoint: 'after-authorization-consumption',
      userAuthorizationReference: 'quick-260825-mhh-retry-1',
    });
    assert.notEqual(retryConsumed.code, 0);
    assert.match(retryConsumed.stderr, /after-authorization-consumption/i);
    assert.equal(authority.requests.length, 0);

    const retryRecordBytes = await readFile(fixture.retryAuthorizationPath, 'utf8');
    const retryRecord = JSON.parse(retryRecordBytes) as Record<string, unknown>;
    assert.deepEqual(Object.keys(retryRecord).sort(), Object.keys(record).sort());
    assert.equal(retryRecord.schemaVersion, 1);
    assert.equal(retryRecord.revisionId, 'official-2026-08-20');
    assert.equal(retryRecord.acquisitionMethod, 'user-authorized-agent-run-one-shot-powershell');
    assert.equal(retryRecord.authorizationReference, 'quick-260825-mhh-retry-1');
    assert.match(String(retryRecord.consumedAt), /^\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}\.\d{3}Z$/);
    assert.doesNotMatch(retryRecordBytes, /https?:|primaryRoot|backupRoot|locator/i);
    assert.equal(await readFile(fixture.authorizationPath, 'utf8'), recordBytes);

    const retryBlocked = await invokeLoopback(fixture, descriptors, {
      userAuthorizationReference: 'quick-260825-mhh-retry-1',
    });
    assert.notEqual(retryBlocked.code, 0);
    assert.match(retryBlocked.stderr, /authorization.*consumed|already exists/i);
    assert.equal(authority.requests.length, 0);
    assert.equal(await readFile(fixture.retryAuthorizationPath, 'utf8'), retryRecordBytes);
    assert.equal(await readFile(fixture.authorizationPath, 'utf8'), recordBytes);
  } finally {
    await close(authority.server);
    await cleanupFixture(fixture);
  }
});

test('authorized success and transport failure remain consumed after cleanup and output deletion', async (context) => {
  for (const scenario of ['success', 'failure after transport'] as const) {
    await context.test(scenario, async () => {
      const fixture = await createFixture();
      const authority = createHappyAuthorityServer();
      try {
        const port = await listen(authority.server);
        authority.setPort(port);
        const descriptors = testDescriptors(`http://127.0.0.1:${port}`);
        const first = await invokeLoopback(fixture, descriptors, {
          ...(scenario === 'success' ? {} : { faultPoint: 'after-staged-verification' }),
          userAuthorizationReference: 'quick-260825-mhh',
        });
        if (scenario === 'success') {
          assert.equal(first.code, 0, first.stderr);
          const lock = JSON.parse(await readFile(fixture.lockPath, 'utf8')) as Record<string, unknown>;
          assert.equal(lock.acquisitionMethod, 'user-authorized-agent-run-one-shot-powershell');
          assert.equal(lock.authorizationReference, 'quick-260825-mhh');
        } else {
          assert.notEqual(first.code, 0);
          assert.match(first.stderr, /after-staged-verification/i);
          assert.equal(await stat(fixture.lockPath).then(() => true, () => false), false);
        }
        assert.ok(authority.requests.length > 0);
        const requestCount = authority.requests.length;
        const recordBytes = await readFile(fixture.authorizationPath, 'utf8');

        await rm(fixture.primaryRoot, { recursive: true, force: true });
        await rm(fixture.backupRoot, { recursive: true, force: true });
        await rm(fixture.lockPath, { force: true });
        const retried = await invokeLoopback(fixture, descriptors, {
          userAuthorizationReference: 'quick-260825-mhh',
        });
        assert.notEqual(retried.code, 0);
        assert.match(retried.stderr, /authorization.*consumed|already exists/i);
        assert.equal(authority.requests.length, requestCount);
        assert.equal(await readFile(fixture.authorizationPath, 'utf8'), recordBytes);
      } finally {
        await close(authority.server);
        await cleanupFixture(fixture);
      }
    });
  }
});

test('concurrent authorized invocations let at most one cross the atomic consumption guard', async () => {
  const fixture = await createFixture();
  const authority = createHappyAuthorityServer();
  try {
    const port = await listen(authority.server);
    authority.setPort(port);
    const descriptors = testDescriptors(`http://127.0.0.1:${port}`);
    const options = {
      faultPoint: 'after-authorization-consumption',
      userAuthorizationReference: 'quick-260825-mhh',
    } as const;
    const results = await Promise.all([
      invokeLoopback(fixture, descriptors, options),
      invokeLoopback(fixture, descriptors, options),
    ]);
    assert.equal(results.filter(({ stderr }) => /after-authorization-consumption/i.test(stderr)).length, 1);
    assert.equal(results.filter(({ stderr }) => /authorization.*consumed|already exists/i.test(stderr)).length, 1);
    assert.equal(authority.requests.length, 0);
    assert.equal(await stat(fixture.authorizationPath).then(() => true, () => false), true);
  } finally {
    await close(authority.server);
    await cleanupFixture(fixture);
  }
});

test('offline production manifest locks the seven non-artwork sources', async () => {
  const result = await runPwsh([
    '-Command',
    '. $env:SORCERY_COLLECTOR_SCRIPT; Get-ProductionSourceDescriptors | ConvertTo-Json -Depth 16 -Compress',
  ], { SORCERY_COLLECTOR_SCRIPT: SCRIPT_PATH });
  assert.equal(result.code, 0, result.stderr);
  const manifest = JSON.parse(result.stdout.trim()) as readonly Record<string, unknown>[];
  assert.equal(manifest.length, 7);
  assert.deepEqual(
    manifest.map(({ relativePath, provenanceUrl }) => [relativePath, provenanceUrl]),
    Object.entries(OFFICIAL_URLS),
  );
  assert.equal(JSON.stringify(manifest).toLowerCase().includes('artwork'), false);
  assert.equal(JSON.stringify(manifest).toLowerCase().includes('image'), false);
});

test('caller-supplied non-loopback configuration is rejected before transport', async (context) => {
  let requestCount = 0;
  const server = createServer((_request, response) => {
    requestCount += 1;
    response.statusCode = 500;
    response.end('production configuration validation should prevent this request');
  });
  try {
    const port = await listen(server);
    await context.test('production wrapper rejects descriptor overrides', async () => {
      const fixture = await createFixture();
      try {
        const configurationPath = join(fixture.sandbox, 'production-override.json');
        await writeFile(
          configurationPath,
          JSON.stringify({ backupRoot: fixture.backupRoot, descriptors: testDescriptors(`http://127.0.0.1:${port}`) }),
        );
        const command = [
          '. $env:SORCERY_COLLECTOR_SCRIPT',
          '$c = Get-Content -Raw -LiteralPath $env:SORCERY_COLLECTOR_CONFIG | ConvertFrom-Json -Depth 32',
          'Invoke-PrivateAuthorityCollection -BackupRoot $c.backupRoot -Descriptors $c.descriptors',
        ].join('; ');
        const result = await runPwsh(['-Command', command], {
          SORCERY_COLLECTOR_SCRIPT: SCRIPT_PATH,
          SORCERY_COLLECTOR_CONFIG: configurationPath,
        });
        assert.notEqual(result.code, 0);
        assert.match(result.stderr, /Descriptors|parameter/i);
        assert.equal(requestCount, 0);
        assert.deepEqual(await readdir(fixture.sandbox), ['production-override.json', 'repository']);
      } finally {
        await cleanupFixture(fixture);
      }
    });

    await context.test('lower transport and publication helpers are absent', async () => {
      const fixture = await createFixture();
      try {
        const command = [
          '. $env:SORCERY_COLLECTOR_SCRIPT',
          "$names = @('Invoke-PrivateAuthorityCollectionCore', 'Invoke-PrivateAuthorityTransport', 'Invoke-BoundedHttpToFile', 'Assert-RequestUri', 'Resolve-CollectionPaths', 'Invoke-PrivateSourceVerifier')",
          '$resolved = @($names | Where-Object { $null -ne (Get-Command -Name $_ -ErrorAction SilentlyContinue) })',
          'if ($resolved.Count -ne 0) { throw "Private collector helpers leaked: $($resolved -join \", \")" }',
          'ConvertTo-Json -InputObject @($resolved) -Compress',
        ].join('; ');
        const result = await runPwsh(['-Command', command], {
          SORCERY_COLLECTOR_SCRIPT: SCRIPT_PATH,
        });
        assert.equal(result.code, 0, result.stderr);
        assert.deepEqual(JSON.parse(result.stdout.trim()), []);
        assert.equal(requestCount, 0);
        assert.deepEqual(await readdir(fixture.sandbox), ['repository']);
      } finally {
        await cleanupFixture(fixture);
      }
    });
  } finally {
    await close(server);
  }
});

test('loopback collection selects the standard rulebook and preserves exact bounded bytes', async () => {
  const fixture = await createFixture();
  const authority = createHappyAuthorityServer();
  try {
    const port = await listen(authority.server);
    authority.setPort(port);
    const result = await invokeLoopback(fixture, testDescriptors(`http://127.0.0.1:${port}`));
    assert.equal(result.code, 0, result.stderr);
    for (const [relativePath, bytes] of Object.entries(SOURCE_BYTES)) {
      assert.deepEqual(await readFile(join(fixture.primaryRoot, ...relativePath.split('/'))), bytes);
    }
    assert.equal(authority.requests.includes('/artwork-must-not-be-requested'), false);
    assert.equal(authority.requests.includes('/file/d/annotated/view'), false);
    assert.equal(authority.requests.filter((path) => path === '/cards-cards.raw.json').length, 1);
  } finally {
    await close(authority.server);
    await cleanupFixture(fixture);
  }
});

test('changelog accepts the official day-first date and rejects impossible dates', async (context) => {
  for (const scenario of [
    { name: 'official day-first date', text: '19 May 2026', expected: '2026-05-19' },
    { name: 'impossible day-first date', text: '31 February 2026', expected: null },
  ] as const) {
    await context.test(scenario.name, async () => {
      const fixture = await createFixture();
      const authority = createHappyAuthorityServer(
        SOURCE_BYTES['rulebook/rulebook-current.pdf'],
        Buffer.from(`<!doctype html><h1>Codex Changelog</h1><time>${scenario.text}</time>`),
      );
      try {
        const port = await listen(authority.server);
        authority.setPort(port);
        const result = await invokeLoopback(fixture, testDescriptors(`http://127.0.0.1:${port}`));
        if (scenario.expected === null) {
          assert.notEqual(result.code, 0);
          assert.match(result.stderr, /changelog date is invalid/i);
          assert.equal(await stat(fixture.lockPath).then(() => true, () => false), false);
        } else {
          assert.equal(result.code, 0, result.stderr);
          const lock = JSON.parse(await readFile(fixture.lockPath, 'utf8')) as {
            entries: readonly PrivateAuthoritySourceEntry[];
          };
          assert.equal(
            lock.entries.find(({ relativePath }) => relativePath === 'codex/changelog-current.html')
              ?.effectiveDate,
            scenario.expected,
          );
        }
      } finally {
        await close(authority.server);
        await cleanupFixture(fixture);
      }
    });
  }
});

test('changelog ignores hidden dates and requires one semantic date in the first visible entry', async (context) => {
  const cases = [
    {
      name: 'hidden earlier dates do not win over a unique visible day-first date',
      html:
        '<!doctype html><head><title>Codex Changelog</title>' +
        '<script>August 19, 2026</script><style>.date::after{content:"August 18, 2026"}</style>' +
        '<template>August 17, 2026</template><noscript>August 16, 2026</noscript>' +
        '<!-- August 15, 2026 --></head><body><main><h1>Codex Changelog</h1>' +
        '<article><h2>20 August 2026</h2><p>Rules update</p></article></main></body>',
      effectiveDate: '2026-08-20',
    },
    {
      name: 'ambiguous first visible entry dates fail closed',
      html:
        '<!doctype html><title>Codex Changelog</title><main><h1>Codex Changelog</h1>' +
        '<article><h2>20 August 2026 / August 21, 2026</h2><p>Rules update</p></article></main>',
      effectiveDate: null,
    },
    {
      name: 'a date in only the second visible entry fails closed',
      html:
        '<!doctype html><title>Codex Changelog</title><main><h1>Codex Changelog</h1>' +
        '<article><h2>Rules update</h2><p>No effective date</p></article>' +
        '<article><h2>20 August 2026</h2><p>Older entry</p></article></main>',
      effectiveDate: null,
    },
  ] as const;

  for (const scenario of cases) {
    await context.test(scenario.name, async () => {
      const fixture = await createFixture();
      const authority = createHappyAuthorityServer(
        SOURCE_BYTES['rulebook/rulebook-current.pdf'],
        Buffer.from(scenario.html),
      );
      try {
        const port = await listen(authority.server);
        authority.setPort(port);
        const result = await invokeLoopback(fixture, testDescriptors(`http://127.0.0.1:${port}`));
        if (scenario.effectiveDate === null) {
          assert.notEqual(result.code, 0);
          assert.equal(await stat(fixture.lockPath).then(() => true, () => false), false);
          assert.equal(await stat(fixture.primaryRoot).then(() => true, () => false), false);
          assert.equal(await stat(fixture.backupRoot).then(() => true, () => false), false);
        } else {
          assert.equal(result.code, 0, result.stderr);
          const lock = JSON.parse(await readFile(fixture.lockPath, 'utf8')) as {
            entries: readonly PrivateAuthoritySourceEntry[];
          };
          assert.equal(
            lock.entries.find(({ relativePath }) => relativePath === 'codex/changelog-current.html')
              ?.effectiveDate,
            scenario.effectiveDate,
          );
        }
      } finally {
        await close(authority.server);
        await cleanupFixture(fixture);
      }
    });
  }
});

test('complete HTML sources require their source-specific visible body structure', async (context) => {
  const shells: Readonly<Partial<Record<SourcePath, Buffer>>>[] = [
    {
      'formats/constructed-current.html': Buffer.from(
        '<!doctype html><title>Constructed Format</title><main><h1>Constructed Format</h1></main>',
      ),
    },
    {
      'codex/codex-current.html': Buffer.from(
        '<!doctype html><title>Welcome to the Codex</title><main><h1>Welcome to the Codex</h1></main>',
      ),
    },
    {
      'codex/faqs-current.html': Buffer.from(
        '<!doctype html><title>FAQs</title><main><h1>FAQs</h1></main>',
      ),
    },
    {
      'codex/changelog-current.html': Buffer.from(
        '<!doctype html><title>Codex Changelog</title><main><h1>Codex Changelog</h1><article><h2>20 August 2026</h2></article></main>',
      ),
    },
    {
      'updates/card-updates-2025.html': Buffer.from(
        '<!doctype html><title>Sorcery: Contested Realm Card Updates 2025</title><main><h1>Sorcery: Contested Realm Card Updates 2025</h1></main>',
      ),
    },
  ];

  for (const sourceOverride of shells) {
    const relativePath = Object.keys(sourceOverride)[0] as SourcePath;
    await context.test(relativePath, async () => {
      const fixture = await createFixture();
      const authority = createHappyAuthorityServer(
        SOURCE_BYTES['rulebook/rulebook-current.pdf'],
        SOURCE_BYTES['codex/changelog-current.html'],
        sourceOverride,
      );
      try {
        const port = await listen(authority.server);
        authority.setPort(port);
        const result = await invokeLoopback(fixture, testDescriptors(`http://127.0.0.1:${port}`));
        assert.notEqual(result.code, 0);
        assert.equal(await stat(fixture.lockPath).then(() => true, () => false), false);
        assert.equal(await stat(fixture.primaryRoot).then(() => true, () => false), false);
        assert.equal(await stat(fixture.backupRoot).then(() => true, () => false), false);
      } finally {
        await close(authority.server);
        await cleanupFixture(fixture);
      }
    });
  }
});

test('partial and structurally invalid card arrays fail before publication', async (context) => {
  const cases = [
    {
      name: 'one-card truncated array',
      cards: [syntheticOfficialCard()],
      expectedCardCount: 2,
    },
    {
      name: 'missing guardian and set fields',
      cards: [{ name: 'Synthetic Adept', power: 1 }],
      expectedCardCount: 1,
    },
    {
      name: 'unknown card field',
      cards: [{ ...syntheticOfficialCard(), unknownField: true }],
      expectedCardCount: 1,
    },
    {
      name: 'duplicate printing slugs',
      cards: [
        syntheticOfficialCard('Synthetic Adept', 'duplicate-printing'),
        syntheticOfficialCard('Synthetic Avatar', 'duplicate-printing'),
      ],
      expectedCardCount: 2,
    },
  ] as const;

  for (const scenario of cases) {
    await context.test(scenario.name, async () => {
      const fixture = await createFixture();
      const authority = createHappyAuthorityServer(
        SOURCE_BYTES['rulebook/rulebook-current.pdf'],
        SOURCE_BYTES['codex/changelog-current.html'],
        { 'cards/cards.raw.json': Buffer.from(JSON.stringify(scenario.cards)) },
      );
      try {
        const port = await listen(authority.server);
        authority.setPort(port);
        const descriptors = testDescriptors(`http://127.0.0.1:${port}`).map((value) =>
          value.relativePath === 'cards/cards.raw.json'
            ? { ...value, expectedCardCount: scenario.expectedCardCount }
            : value,
        );
        const result = await invokeLoopback(fixture, descriptors);
        assert.notEqual(result.code, 0);
        assert.equal(await stat(fixture.lockPath).then(() => true, () => false), false);
        assert.equal(await stat(fixture.primaryRoot).then(() => true, () => false), false);
        assert.equal(await stat(fixture.backupRoot).then(() => true, () => false), false);
      } finally {
        await close(authority.server);
        await cleanupFixture(fixture);
      }
    });
  }
});

test('rulebook accepts only EOF with no bytes LF CR or CRLF after it', async (context) => {
  for (const ending of ['\n', '\r', '\r\n'] as const) {
    await context.test(JSON.stringify(ending), async () => {
      const fixture = await createFixture();
      const bytes = Buffer.concat([SOURCE_BYTES['rulebook/rulebook-current.pdf'], Buffer.from(ending)]);
      const authority = createHappyAuthorityServer(bytes);
      try {
        const port = await listen(authority.server);
        authority.setPort(port);
        const result = await invokeLoopback(fixture, testDescriptors(`http://127.0.0.1:${port}`));
        assert.equal(result.code, 0, result.stderr);
        assert.deepEqual(await readFile(join(fixture.primaryRoot, 'rulebook', 'rulebook-current.pdf')), bytes);
      } finally {
        await close(authority.server);
        await cleanupFixture(fixture);
      }
    });
  }

  await context.test('rejects arbitrary trailing bytes', async () => {
    const fixture = await createFixture();
    const authority = createHappyAuthorityServer(
      Buffer.concat([SOURCE_BYTES['rulebook/rulebook-current.pdf'], Buffer.from('junk')]),
    );
    try {
      const port = await listen(authority.server);
      authority.setPort(port);
      const result = await invokeLoopback(fixture, testDescriptors(`http://127.0.0.1:${port}`));
      assert.notEqual(result.code, 0);
      assert.match(result.stderr, /EOF marker or trailing bytes/i);
      assert.equal(await stat(fixture.lockPath).then(() => true, () => false), false);
    } finally {
      await close(authority.server);
      await cleanupFixture(fixture);
    }
  });
});

test('loopback seam rejects non-loopback request targets before filesystem mutation', async () => {
  const fixture = await createFixture();
  try {
    const descriptors = testDescriptors('http://127.0.0.1:1').map((value, index) =>
      index === 0 ? { ...value, requestUrl: 'https://example.com/not-loopback' } : value,
    );
    const result = await invokeLoopback(fixture, descriptors);
    assert.notEqual(result.code, 0);
    assert.match(result.stderr, /loopback/i);
    assert.equal(await stat(fixture.primaryRoot).then(() => true, () => false), false);
    assert.equal(await stat(fixture.backupRoot).then(() => true, () => false), false);
  } finally {
    await cleanupFixture(fixture);
  }
});

test('blocked status challenge malformed content and streamed size stop without retry', async (context) => {
  const cases = [
    { name: '401', route: '/formats-constructed-current.html', status: 401, body: 'unauthorized' },
    { name: '403', route: '/formats-constructed-current.html', status: 403, body: 'forbidden' },
    { name: '429', route: '/formats-constructed-current.html', status: 429, body: 'slow down' },
    { name: 'other non-success', route: '/formats-constructed-current.html', status: 500, body: 'failed' },
    {
      name: 'captcha',
      route: '/formats-constructed-current.html',
      status: 200,
      body: '<!doctype html><title>Attention Required</title><div>captcha</div>',
    },
    { name: 'malformed card json', route: '/cards-cards.raw.json', status: 200, body: '[}' },
    { name: 'empty card json', route: '/cards-cards.raw.json', status: 200, body: '[]' },
    { name: 'wrong card json root', route: '/cards-cards.raw.json', status: 200, body: '{}' },
    { name: 'empty card name', route: '/cards-cards.raw.json', status: 200, body: '[{"name":""}]' },
    {
      name: 'wrong html media type',
      route: '/formats-constructed-current.html',
      status: 200,
      body: '<!doctype html><title>Constructed Format</title>',
      contentType: 'application/json',
    },
    {
      name: 'missing html marker',
      route: '/formats-constructed-current.html',
      status: 200,
      body: '<!doctype html><title>Wrong source</title>',
    },
    {
      name: 'declared size',
      route: '/formats-constructed-current.html',
      status: 200,
      body: '<!doctype html><title>Constructed Format</title>',
      contentLength: 20_000,
    },
    {
      name: 'streamed size',
      route: '/formats-constructed-current.html',
      status: 200,
      body: '<!doctype html><title>Constructed Format</title>' + 'x'.repeat(20_000),
    },
  ] as const;
  for (const scenario of cases) {
    await context.test(scenario.name, async () => {
      const fixture = await createFixture();
      const counts = new Map<string, number>();
      let serverPort = 0;
      const server = createServer((request, response) => {
        const path = request.url ?? '';
        counts.set(path, (counts.get(path) ?? 0) + 1);
        if (path === scenario.route) {
          response.statusCode = scenario.status;
          response.setHeader(
            'content-type',
            'contentType' in scenario
              ? scenario.contentType
              : path.endsWith('.json')
                ? 'application/json'
                : 'text/html',
          );
          if ('contentLength' in scenario) response.setHeader('content-length', scenario.contentLength);
          response.end(scenario.body);
          return;
        }
        if (path === '/release') {
          response.setHeader('content-type', 'text/html');
          response.end(
            '<!doctype html><title>Sorcery: Contested Realm December 2025 Rulebook Update</title>' +
              '<time>19 Dec 2025</time>' +
              `<a href="http://127.0.0.1:${String(serverPort)}/file/d/standard/view">Sorcery: Contested Realm Rulebook (December 2025)</a>`,
          );
          return;
        }
        if (path === '/drive-download?export=download&id=standard') {
          response.statusCode = 303;
          response.setHeader('location', '/pdf');
          response.end();
          return;
        }
        if (path === '/pdf') {
          response.setHeader('content-type', 'application/pdf');
          response.setHeader('content-disposition', 'attachment; filename=SorceryRulebook.pdf');
          response.end(SOURCE_BYTES['rulebook/rulebook-current.pdf']);
          return;
        }
        const relativePath = (Object.keys(SOURCE_BYTES) as SourcePath[]).find(
          (candidate) => path === `/${candidate.replaceAll('/', '-')}`,
        );
        assert.ok(relativePath);
        response.setHeader('content-type', relativePath.endsWith('.json') ? 'application/json' : 'text/html');
        response.end(SOURCE_BYTES[relativePath]);
      });
      try {
        serverPort = await listen(server);
        const result = await invokeLoopback(fixture, testDescriptors(`http://127.0.0.1:${serverPort}`));
        assert.notEqual(result.code, 0);
        assert.equal(counts.get(scenario.route), 1);
        assert.equal(await stat(fixture.lockPath).then(() => true, () => false), false);
      } finally {
        await close(server);
        await cleanupFixture(fixture);
      }
    });
  }
});

test('rulebook anchor redirect and PDF controls fail closed', async (context) => {
  const cases = [
    'wrong release date',
    'missing anchor',
    'duplicate anchor',
    'unexpected locator host',
    'missing redirect location',
    'redirect loop',
    'redirect limit',
    'wrong PDF filename',
    'wrong PDF media',
    'wrong PDF signature',
  ] as const;
  for (const scenario of cases) {
    await context.test(scenario, async () => {
      const fixture = await createFixture();
      let port = 0;
      const requests: string[] = [];
      const server = createServer((request, response) => {
        const path = request.url ?? '';
        requests.push(path);
        if (path === '/release') {
          const standard =
            scenario === 'unexpected locator host'
              ? 'https://example.com/file/d/standard/view'
              : `http://127.0.0.1:${String(port)}/file/d/standard/view`;
          const anchors =
            scenario === 'missing anchor'
              ? ''
              : `<a href="${standard}">Sorcery: Contested Realm Rulebook (December 2025)</a>` +
                (scenario === 'duplicate anchor'
                  ? `<a href="${standard}">Sorcery: Contested Realm Rulebook (December 2025)</a>`
                  : '');
          response.setHeader('content-type', 'text/html');
          response.end(
            '<!doctype html><title>Sorcery: Contested Realm December 2025 Rulebook Update</title>' +
              (scenario === 'wrong release date' ? '<time>18 Dec 2025</time>' : '<time>19 Dec 2025</time>') +
              anchors,
          );
          return;
        }
        if (path === '/drive-download?export=download&id=standard') {
          response.statusCode = 303;
          if (scenario !== 'missing redirect location') {
            response.setHeader(
              'location',
              scenario === 'redirect loop'
                ? '/drive-download?export=download&id=standard'
                : scenario === 'redirect limit'
                  ? '/redirect-1'
                  : '/pdf',
            );
          }
          response.end();
          return;
        }
        const redirectMatch = /^\/redirect-(\d+)$/.exec(path);
        if (redirectMatch) {
          response.statusCode = 302;
          response.setHeader('location', `/redirect-${Number(redirectMatch[1]) + 1}`);
          response.end();
          return;
        }
        if (path === '/pdf') {
          response.setHeader(
            'content-type',
            scenario === 'wrong PDF media' ? 'text/html' : 'application/octet-stream',
          );
          response.setHeader(
            'content-disposition',
            `attachment; filename=${scenario === 'wrong PDF filename' ? 'Wrong.pdf' : 'SorceryRulebook.pdf'}`,
          );
          response.end(
            scenario === 'wrong PDF signature'
              ? Buffer.from('not a pdf at all')
              : SOURCE_BYTES['rulebook/rulebook-current.pdf'],
          );
          return;
        }
        response.statusCode = 500;
        response.end('unexpected route');
      });
      try {
        port = await listen(server);
        const result = await invokeLoopback(fixture, testDescriptors(`http://127.0.0.1:${port}`));
        assert.notEqual(result.code, 0);
        if (scenario === 'wrong PDF signature') assert.match(result.stderr, /PDF prefix/i);
        assert.equal(await stat(fixture.lockPath).then(() => true, () => false), false);
        assert.equal(requests.filter((path) => path === '/release').length, 1);
      } finally {
        await close(server);
        await cleanupFixture(fixture);
      }
    });
  }
});

test('header timeout body timeout and disconnect are terminal', async (context) => {
  for (const scenario of ['header timeout', 'body timeout', 'disconnect'] as const) {
    await context.test(scenario, async () => {
      const fixture = await createFixture();
      let requestCount = 0;
      const server = createServer((_request, response) => {
        requestCount += 1;
        if (scenario === 'header timeout') return;
        response.writeHead(200, { 'content-type': 'text/html' });
        response.flushHeaders();
        response.write('<!doctype html>');
        if (scenario === 'body timeout') return;
        setTimeout(() => response.destroy(), 10);
      });
      try {
        const port = await listen(server);
        const result = await invokeLoopback(
          fixture,
          testDescriptors(`http://127.0.0.1:${port}`),
          { headerTimeoutSeconds: 1, bodyTimeoutSeconds: 1 },
        );
        assert.notEqual(result.code, 0);
        assert.equal(requestCount, 1);
        assert.equal(await stat(fixture.lockPath).then(() => true, () => false), false);
      } finally {
        await close(server);
        await cleanupFixture(fixture);
      }
    });
  }
});

test('aggregate byte overflow stops before the next source and publishes no receipt', async () => {
  const fixture = await createFixture();
  const authority = createHappyAuthorityServer();
  try {
    const port = await listen(authority.server);
    authority.setPort(port);
    const maxTotalBytes =
      SOURCE_BYTES['rulebook/rulebook-current.pdf'].byteLength +
      SOURCE_BYTES['formats/constructed-current.html'].byteLength -
      1;
    const result = await invokeLoopback(
      fixture,
      testDescriptors(`http://127.0.0.1:${port}`),
      { maxTotalBytes },
    );
    assert.notEqual(result.code, 0);
    assert.match(result.stderr, /aggregate byte limit/i);
    assert.equal(authority.requests.filter((path) => path === '/formats-constructed-current.html').length, 1);
    assert.equal(authority.requests.includes('/codex-codex-current.html'), false);
    assert.equal(await stat(fixture.primaryRoot).then(() => true, () => false), false);
    assert.equal(await stat(fixture.backupRoot).then(() => true, () => false), false);
    assert.equal(await stat(fixture.lockPath).then(() => true, () => false), false);
  } finally {
    await close(authority.server);
    await cleanupFixture(fixture);
  }
});

test('publication creates independent exact trees and a verifier-approved private lock', async () => {
  const fixture = await createFixture();
  const authority = createHappyAuthorityServer();
  try {
    const port = await listen(authority.server);
    authority.setPort(port);
    const result = await invokeLoopback(fixture, testDescriptors(`http://127.0.0.1:${port}`));
    assert.equal(result.code, 0, result.stderr);

    const expectedFiles = [...PRIVATE_AUTHORITY_SOURCE_PATHS].sort();
    assert.deepEqual(await relativeFiles(fixture.primaryRoot), expectedFiles);
    assert.deepEqual(await relativeFiles(fixture.backupRoot), expectedFiles);
    for (const relativePath of PRIVATE_AUTHORITY_SOURCE_PATHS) {
      const primaryPath = join(fixture.primaryRoot, ...relativePath.split('/'));
      const backupPath = join(fixture.backupRoot, ...relativePath.split('/'));
      assert.deepEqual(await readFile(primaryPath), SOURCE_BYTES[relativePath]);
      assert.deepEqual(await readFile(backupPath), SOURCE_BYTES[relativePath]);
      const primaryIdentity = await stat(primaryPath);
      const backupIdentity = await stat(backupPath);
      assert.notDeepEqual(
        [primaryIdentity.dev, primaryIdentity.ino],
        [backupIdentity.dev, backupIdentity.ino],
      );
    }

    const lockBytes = await readFile(fixture.lockPath);
    assert.notEqual(lockBytes[0], 0xef);
    const lock = JSON.parse(lockBytes.toString('utf8')) as {
      acquisitionMethod: string;
      authorizationReference?: string;
      primaryRoot: string;
      backupRoot: string;
      entries: readonly PrivateAuthoritySourceEntry[];
      sourceSetRootHash: string;
      operatingAcknowledgment: Record<string, unknown>;
      rulebookAcquisitionEvidence: Record<string, unknown>;
    };
    assert.equal(lock.acquisitionMethod, 'user-run-one-shot-powershell');
    assert.equal('authorizationReference' in lock, false);
    assert.equal(lock.primaryRoot, resolve(fixture.primaryRoot));
    assert.equal(lock.backupRoot, resolve(fixture.backupRoot));
    assert.deepEqual(
      lock.entries.map(({ relativePath }) => relativePath),
      expectedFiles,
    );
    assert.deepEqual(
      lock.entries.map(({ effectiveDate }) => effectiveDate),
      [null, '2026-08-20', null, null, null, '2025-12-19', '2025-11-25'],
    );
    assert.deepEqual(lock.operatingAcknowledgment, {
      scope: 'private-local-noncommercial',
      noRedistributionReleaseHostingUploadOrArtwork: true,
      apiTermsRobotsConflictAndPrivateUseRiskAccepted: true,
      establishesLegalPermission: false,
      stopOnBlockedStatusCaptchaOrPublisherObjection: true,
      retryOrEvasion: false,
    });
    assert.equal(lock.rulebookAcquisitionEvidence.privateLocatorIsNormative, false);
    assert.equal(lock.rulebookAcquisitionEvidence.observedFilename, 'SorceryRulebook.pdf');
    assert.equal(lock.rulebookAcquisitionEvidence.sourceUrl, OFFICIAL_URLS['rulebook/rulebook-current.pdf']);
    assert.equal(JSON.stringify(lock).includes('Synthetic Adept'), false);

    const verified = await verifyPrivateSourceSet({
      primaryRoot: lock.primaryRoot,
      backupRoot: lock.backupRoot,
      repositoryRoot: fixture.repositoryRoot,
      entries: lock.entries,
    });
    assert.equal(lock.sourceSetRootHash, verified.sourceSetRootHash);
    assert.deepEqual(lock.entries, verified.entries);
  } finally {
    await close(authority.server);
    await cleanupFixture(fixture);
  }
});

test('preflight rejects existing or overlapping destinations before any request', async (context) => {
  const cases = [
    {
      name: 'existing primary',
      expected: /already exists/i,
      change: async (fixture: Fixture) => {
        await mkdir(fixture.primaryRoot, { recursive: true });
        await writeFile(join(fixture.primaryRoot, 'sentinel'), 'unchanged');
        return fixture;
      },
    },
    {
      name: 'existing backup',
      expected: /already exists/i,
      change: async (fixture: Fixture) => {
        await mkdir(fixture.backupRoot, { recursive: true });
        await writeFile(join(fixture.backupRoot, 'sentinel'), 'unchanged');
        return fixture;
      },
    },
    {
      name: 'existing lock',
      expected: /already exists/i,
      change: async (fixture: Fixture) => {
        await mkdir(dirname(fixture.lockPath), { recursive: true });
        await writeFile(fixture.lockPath, 'unchanged');
        return fixture;
      },
    },
    {
      name: 'backup equals primary',
      expected: /outside|overlap|equal/i,
      change: async (fixture: Fixture) => ({ ...fixture, backupRoot: fixture.primaryRoot }),
    },
    {
      name: 'backup nested in repository',
      expected: /outside|repository/i,
      change: async (fixture: Fixture) => ({ ...fixture, backupRoot: join(fixture.repositoryRoot, 'backup') }),
    },
    {
      name: 'backup contains repository',
      expected: /outside|contain/i,
      change: async (fixture: Fixture) => ({ ...fixture, backupRoot: fixture.sandbox }),
    },
    {
      name: 'relative backup',
      expected: /absolute/i,
      change: async (fixture: Fixture) => ({ ...fixture, backupRoot: 'relative-backup' }),
    },
  ] as const;
  let requestCount = 0;
  const server = createServer((_request, response) => {
    requestCount += 1;
    response.statusCode = 500;
    response.end('preflight should prevent this request');
  });
  try {
    const port = await listen(server);
    for (const scenario of cases) {
      await context.test(scenario.name, async () => {
        const original = await createFixture();
        try {
          const fixture = await scenario.change(original);
          const before = requestCount;
          const result = await invokeLoopback(fixture, testDescriptors(`http://127.0.0.1:${port}`));
          assert.notEqual(result.code, 0);
          assert.match(result.stderr, scenario.expected);
          assert.equal(requestCount, before);
          if (scenario.name.startsWith('existing')) {
            const sentinel =
              scenario.name === 'existing primary'
                ? join(fixture.primaryRoot, 'sentinel')
                : scenario.name === 'existing backup'
                  ? join(fixture.backupRoot, 'sentinel')
                  : fixture.lockPath;
            assert.equal(await readFile(sentinel, 'utf8'), 'unchanged');
          }
        } finally {
          await cleanupFixture(original);
        }
      });
    }
  } finally {
    await close(server);
  }
});

test('controlled publication faults leave no canonical receipt and quarantine only owned outputs', async (context) => {
  const faultPoints = [
    'after-staged-verification',
    'after-backup-move',
    'after-primary-move',
    'during-final-verifier',
    'before-lock-move',
  ] as const;
  const authority = createHappyAuthorityServer();
  try {
    const port = await listen(authority.server);
    authority.setPort(port);
    for (const faultPoint of faultPoints) {
      await context.test(faultPoint, async () => {
        const fixture = await createFixture();
        try {
          const result = await invokeLoopback(
            fixture,
            testDescriptors(`http://127.0.0.1:${port}`),
            { faultPoint },
          );
          assert.notEqual(result.code, 0);
          if (faultPoint === 'during-final-verifier') {
            assert.match(result.stderr, /Private source verifier failed/i);
            assert.match(result.stderr, /ENOENT|no such file/i);
          }
          assert.equal(await stat(fixture.lockPath).then(() => true, () => false), false);
          assert.equal(await stat(fixture.primaryRoot).then(() => true, () => false), false);
          assert.equal(await stat(fixture.backupRoot).then(() => true, () => false), false);
          const primarySiblings = await safeReaddir(dirname(fixture.primaryRoot));
          const backupSiblings = await safeReaddir(dirname(fixture.backupRoot));
          assert.equal(primarySiblings.some((name) => name.includes('.collecting-')), false);
          assert.equal(backupSiblings.some((name) => name.includes('.collecting-')), false);
          if (faultPoint === 'after-primary-move' || faultPoint === 'during-final-verifier' || faultPoint === 'before-lock-move') {
            assert.equal(primarySiblings.some((name) => name.includes('.failed-')), true);
          }
          if (faultPoint !== 'after-staged-verification') {
            assert.equal(backupSiblings.some((name) => name.includes('.failed-')), true);
          }
        } finally {
          await cleanupFixture(fixture);
        }
      });
    }
  } finally {
    await close(authority.server);
  }
});

test('hash helper fixture remains synthetic and deterministic', () => {
  assert.equal(
    createHash('sha256').update(SOURCE_BYTES['cards/cards.raw.json']).digest('hex'),
    'c073ef9097c6a1593b5d87b24fee13886577966648c3f59edef1a91e278bca64',
  );
});
