import { mkdir, writeFile } from 'node:fs/promises';
import { relative, resolve } from 'node:path';
import { pathToFileURL } from 'node:url';
import { parseArgs } from 'node:util';

import { canonicalJson, parseJsonWithDuplicateKeyCheck, type JsonValue } from '../authority/canonical-json.ts';
import { loadPrivateCardSnapshot } from '../authority/private-cards.ts';
import { readBoundedWithinAuthorityRoot, resolveWithinAuthorityRoot } from '../authority/validate-bundle.ts';
import { RustSessionClient } from '../engine/rust-engine.ts';
import { buildPresetCardPool, prepareBoundExperiment, presetCardCatalog } from '../ingestion/preset-card-pool.ts';
import { loadPrivateStarterCatalog } from './run-private-game-check.ts';

/** Writes an experiment from existing, explicitly bound private preset facts. */
export async function createPrivateExperiment(argv: readonly string[]): Promise<string> {
  const { values } = parseArgs({
    args: [...argv],
    allowPositionals: false,
    strict: true,
    options: {
      preset: { type: 'string' },
      catalog: { type: 'boolean', default: false },
      decks: { type: 'string' },
      'output-id': { type: 'string', default: 'first-experiment' },
    },
  });
  const outputId = values['output-id'];
  if (!/^[a-zA-Z0-9][a-zA-Z0-9._-]{0,127}$/u.test(outputId)) {
    throw new TypeError('--output-id must be a single path segment starting with a letter or digit');
  }
  if (Number(values.catalog) + Number(values.decks !== undefined) + Number(values.preset !== undefined) > 1) {
    throw new TypeError('choose only one of --preset, --catalog, or --decks');
  }
  const root = resolve(import.meta.dirname, '../..');
  const authorityRoot = resolve(root, '.local/authority');
  let deckInput: JsonValue | undefined;
  if (values.decks !== undefined) {
    const read = await readBoundedWithinAuthorityRoot(authorityRoot, values.decks, 1_048_576);
    if (read.status !== 'ok') throw new Error('deck input must be a private JSON file of at most 1 MiB');
    deckInput = parseJsonWithDuplicateKeyCheck(Buffer.from(read.bytes).toString('utf8'));
  }
  const presets = await loadPrivateStarterCatalog();
  const preset = presets.find(({ id }) => id === (values.preset ?? 'air-vs-earth-lesson'));
  if (!preset) throw new RangeError(`--preset must be one of: ${presets.map(({ id }) => id).join(', ')}`);
  let document: unknown = {
    schemaVersion: 1,
    baseManifest: preset.manifest,
    candidate: preset.manifest.decks.north,
    opponent: preset.manifest.decks.south,
    seeds: [preset.manifest.seed],
    workers: 1,
  };
  if (values.catalog || deckInput !== undefined) {
    const authority = await loadPrivateCardSnapshot(resolve(authorityRoot, 'scenarios/vanilla-constructed.json'), root);
    const pool = buildPresetCardPool(authority, presets);
    if (values.catalog) {
      document = presetCardCatalog(authority, pool);
    } else {
      const request = prepareBoundExperiment(deckInput, authority, pool);
      const client = await RustSessionClient.start();
      try {
        await client.newSession(canonicalJson(request.baseManifest as unknown as JsonValue));
      } finally {
        await client.close();
      }
      document = request;
    }
  }
  const experimentRoot = resolve(authorityRoot, 'experiments');
  await mkdir(experimentRoot, { recursive: true });
  const confinedRoot = await resolveWithinAuthorityRoot(authorityRoot, 'experiments');
  const outputPath = resolve(confinedRoot, `${outputId}${values.catalog ? '.catalog' : ''}.json`);
  await writeFile(outputPath, canonicalJson(document as JsonValue) + '\n', {
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
