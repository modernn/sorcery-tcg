import assert from 'node:assert/strict';
import { spawn } from 'node:child_process';
import { createHash } from 'node:crypto';
import { mkdir, mkdtemp, readFile, readdir, rm, stat, writeFile } from 'node:fs/promises';
import { createServer, type Server } from 'node:http';
import { tmpdir } from 'node:os';
import { basename, dirname, join, resolve } from 'node:path';
import test from 'node:test';

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
const SOURCE_BYTES = Object.freeze({
  'rulebook/rulebook-current.pdf': Buffer.from('%PDF-1.7\nsynthetic rulebook\n%%EOF'),
  'formats/constructed-current.html': Buffer.from('<!doctype html><title>Constructed Format</title>'),
  'codex/codex-current.html': Buffer.from('<!doctype html><title>Welcome to the Codex</title>'),
  'codex/faqs-current.html': Buffer.from('<!doctype html><title>FAQs</title>'),
  'codex/changelog-current.html': Buffer.from(
    '<!doctype html><h1>Codex Changelog</h1><time>August 20, 2026</time>',
  ),
  'updates/card-updates-2025.html': Buffer.from(
    '<!doctype html><title>Sorcery: Contested Realm Card Updates 2025</title>',
  ),
  'cards/cards.raw.json': Buffer.from('[{"name":"Synthetic Adept","power":1}]\n'),
});

type SourcePath = keyof typeof SOURCE_BYTES;
type Fixture = Readonly<{
  sandbox: string;
  repositoryRoot: string;
  primaryRoot: string;
  backupRoot: string;
  lockPath: string;
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
  await mkdir(repositoryRoot, { recursive: true });
  return { sandbox, repositoryRoot, primaryRoot, backupRoot, lockPath };
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
  limits: Readonly<{ headerTimeoutSeconds?: number; bodyTimeoutSeconds?: number }> = {},
): Promise<ProcessResult> {
  const configurationPath = join(fixture.sandbox, 'configuration.json');
  await writeFile(
    configurationPath,
    JSON.stringify({
      ...fixture,
      descriptors,
      headerTimeoutSeconds: limits.headerTimeoutSeconds ?? 3,
      bodyTimeoutSeconds: limits.bodyTimeoutSeconds ?? 3,
      maxTotalBytes: 131_072,
    }),
  );
  const command = [
    '. $env:SORCERY_COLLECTOR_SCRIPT',
    '$c = Get-Content -Raw -LiteralPath $env:SORCERY_COLLECTOR_CONFIG | ConvertFrom-Json -Depth 32',
    'Invoke-PrivateAuthorityCollectionForLoopbackTest -RepositoryRoot $c.repositoryRoot -PrimaryRoot $c.primaryRoot -BackupRoot $c.backupRoot -LockPath $c.lockPath -Descriptors $c.descriptors -HeaderTimeoutSeconds $c.headerTimeoutSeconds -BodyTimeoutSeconds $c.bodyTimeoutSeconds -MaxTotalBytes $c.maxTotalBytes | ConvertTo-Json -Depth 32 -Compress',
  ].join('; ');
  return runPwsh(['-Command', command], {
    SORCERY_COLLECTOR_SCRIPT: SCRIPT_PATH,
    SORCERY_COLLECTOR_CONFIG: configurationPath,
  });
}

test('production wrapper is inert when dot-sourced and exposes only two direct parameters', async () => {
  const fixture = await createFixture();
  try {
    const command = [
      '. $env:SORCERY_COLLECTOR_SCRIPT',
      '$tokens = $null; $errors = $null',
      '$ast = [Management.Automation.Language.Parser]::ParseFile($env:SORCERY_COLLECTOR_SCRIPT, [ref]$tokens, [ref]$errors)',
      '[pscustomobject]@{ parameters = @($ast.ParamBlock.Parameters.Name.VariablePath.UserPath); functions = @((Get-Command Get-ProductionSourceDescriptors).Name, (Get-Command Invoke-PrivateAuthorityCollectionForLoopbackTest).Name) } | ConvertTo-Json -Compress',
    ].join('; ');
    const result = await runPwsh(['-Command', command], { SORCERY_COLLECTOR_SCRIPT: SCRIPT_PATH });
    assert.equal(result.code, 0, result.stderr);
    assert.deepEqual(JSON.parse(result.stdout.trim()), {
      parameters: ['BackupRoot', 'AcknowledgePrivateUseRisk'],
      functions: ['Get-ProductionSourceDescriptors', 'Invoke-PrivateAuthorityCollectionForLoopbackTest'],
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

test('loopback collection selects the standard rulebook and preserves exact bounded bytes', async () => {
  const fixture = await createFixture();
  const requests: string[] = [];
  let port = 0;
  const server = createServer((request, response) => {
    const path = request.url ?? '';
    requests.push(path);
    if (path === '/release') {
      response.setHeader('content-type', 'text/html; charset=utf-8');
      response.end(
        '<!doctype html><title>Sorcery: Contested Realm December 2025 Rulebook Update</title>' +
          '<time>2025-12-19</time>' +
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
      response.end(SOURCE_BYTES['rulebook/rulebook-current.pdf']);
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
    response.end(SOURCE_BYTES[relativePath]);
  });
  try {
    port = await listen(server);
    const result = await invokeLoopback(fixture, testDescriptors(`http://127.0.0.1:${port}`));
    assert.equal(result.code, 0, result.stderr);
    for (const [relativePath, bytes] of Object.entries(SOURCE_BYTES)) {
      assert.deepEqual(await readFile(join(fixture.primaryRoot, ...relativePath.split('/'))), bytes);
    }
    assert.equal(requests.includes('/artwork-must-not-be-requested'), false);
    assert.equal(requests.includes('/file/d/annotated/view'), false);
    assert.equal(requests.filter((path) => path === '/cards-cards.raw.json').length, 1);
  } finally {
    await close(server);
    await cleanupFixture(fixture);
  }
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
              '<time>2025-12-19</time>' +
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
              '<time>2025-12-19</time>' +
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

test('hash helper fixture remains synthetic and deterministic', () => {
  assert.equal(
    createHash('sha256').update(SOURCE_BYTES['cards/cards.raw.json']).digest('hex'),
    'd8d2bd6cffb31825441021dec513cdb3403f29fc4b458af9093113365d909d4f',
  );
});
