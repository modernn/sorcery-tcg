import { mkdir, writeFile } from 'node:fs/promises';
import { relative, resolve } from 'node:path';
import { pathToFileURL } from 'node:url';
import { parseArgs } from 'node:util';

import { canonicalJson, type JsonValue } from '../authority/canonical-json.ts';
import { resolveWithinAuthorityRoot } from '../authority/validate-bundle.ts';
import { loadPrivateStarterCatalog } from './run-private-game-check.ts';

/** Writes an experiment from existing, explicitly bound private preset facts. */
export async function createPrivateExperiment(argv: readonly string[]): Promise<string> {
  const { values } = parseArgs({
    args: [...argv],
    allowPositionals: false,
    strict: true,
    options: {
      preset: { type: 'string', default: 'air-vs-earth-lesson' },
      'output-id': { type: 'string', default: 'first-experiment' },
    },
  });
  const outputId = values['output-id'];
  if (!/^[a-zA-Z0-9][a-zA-Z0-9._-]{0,127}$/u.test(outputId)) {
    throw new TypeError('--output-id must be a single path segment starting with a letter or digit');
  }
  const presets = await loadPrivateStarterCatalog();
  const preset = presets.find(({ id }) => id === values.preset);
  if (!preset) throw new RangeError(`--preset must be one of: ${presets.map(({ id }) => id).join(', ')}`);
  const root = resolve(import.meta.dirname, '../..');
  const authorityRoot = resolve(root, '.local/authority');
  const experimentRoot = resolve(authorityRoot, 'experiments');
  await mkdir(experimentRoot, { recursive: true });
  const confinedRoot = await resolveWithinAuthorityRoot(authorityRoot, 'experiments');
  const outputPath = resolve(confinedRoot, `${outputId}.json`);
  const request = {
    schemaVersion: 1,
    baseManifest: preset.manifest,
    candidate: preset.manifest.decks.north,
    opponent: preset.manifest.decks.south,
    seeds: [preset.manifest.seed],
    workers: 1,
  };
  await writeFile(outputPath, canonicalJson(request as unknown as JsonValue) + '\n', {
    flag: 'wx', mode: 0o600,
  });
  return relative(root, outputPath).replaceAll('\\', '/');
}

if (process.argv[1] && pathToFileURL(resolve(process.argv[1])).href === import.meta.url) {
  createPrivateExperiment(process.argv.slice(2)).then((requestPath) => {
    process.stdout.write(canonicalJson({ requestPath, status: 'created' }) + '\n');
  }).catch((error: unknown) => {
    process.stderr.write(`${error instanceof Error ? error.message : 'private experiment creation failed'}\n`);
    process.exitCode = 1;
  });
}
