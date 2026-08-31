import { relative, resolve } from 'node:path';
import { pathToFileURL } from 'node:url';
import { parseArgs } from 'node:util';

import { canonicalJson, type JsonValue } from '../authority/canonical-json.ts';
import {
  createTopDeckCandidateSnapshot,
  publishTopDeckCandidateSnapshot,
} from '../ingestion/topdeck-candidates.ts';
import { ingestTopDeckTournaments, TopDeckIngestionError } from '../ingestion/topdeck.ts';

type CommandIo = Readonly<{
  stderr: (line: string) => void;
  stdout: (line: string) => void;
}>;

type CommandDependencies = Readonly<{
  apiKey?: string | undefined;
  fetchImpl?: typeof fetch;
  now?: () => Date;
  repositoryRoot?: string;
}>;

function lastDays(argv: readonly string[]): number {
  const { tokens, values } = parseArgs({
    args: [...argv],
    allowPositionals: false,
    options: { 'last-days': { type: 'string', default: '30' } },
    strict: true,
    tokens: true,
  });
  if (tokens.length > 1) throw new Error('Provide --last-days at most once.');
  const value = Number(values['last-days']);
  if (!Number.isInteger(value) || value < 1 || value > 90) {
    throw new Error('--last-days must be an integer from 1 through 90.');
  }
  return value;
}

function errorMessage(error: unknown): string {
  if (error instanceof TopDeckIngestionError && error.code === 'http_error') {
    if (error.message.endsWith(' 401')) return 'TopDeck authentication failed (HTTP 401); request was not retried.';
    if (error.message.endsWith(' 403')) return 'TopDeck access was forbidden (HTTP 403); request was not retried.';
    if (error.message.endsWith(' 429')) return 'TopDeck rate-limited the request (HTTP 429); request was not retried.';
    return 'TopDeck request failed; request was not retried.';
  }
  if (error instanceof TopDeckIngestionError) return error.message;
  if (error instanceof Error && error.message.startsWith('TOPDECK_API_KEY')) return error.message;
  if (error instanceof Error && error.message.startsWith('--')) return error.message;
  if (error instanceof Error && error.message.startsWith('Provide ')) return error.message;
  return 'TopDeck import failed; request was not retried.';
}

const defaultIo: CommandIo = {
  stderr: (line) => process.stderr.write(line + '\n'),
  stdout: (line) => process.stdout.write(line + '\n'),
};

export async function runImportTopDeckCandidatesCommand(
  argv: readonly string[],
  dependencies: CommandDependencies = {},
  io: CommandIo = defaultIo,
): Promise<number> {
  try {
    const apiKey = dependencies.apiKey ?? process.env.TOPDECK_API_KEY;
    if (apiKey === undefined || apiKey === '') {
      throw new Error('TOPDECK_API_KEY is required for the manual TopDeck import.');
    }
    const repositoryRoot = resolve(dependencies.repositoryRoot ?? process.cwd());
    const ingestion = await ingestTopDeckTournaments({
      apiKey,
      ...(dependencies.fetchImpl === undefined ? {} : { fetchImpl: dependencies.fetchImpl }),
      lastDays: lastDays(argv),
      retrievedAt: (dependencies.now ?? (() => new Date()))().toISOString(),
    });
    const snapshot = createTopDeckCandidateSnapshot(ingestion);
    const outputPath = await publishTopDeckCandidateSnapshot(snapshot, repositoryRoot);
    io.stdout(canonicalJson({
      contentHash: snapshot.contentHash,
      outputPath: relative(repositoryRoot, outputPath).replaceAll('\\', '/'),
      status: 'created',
    } as JsonValue));
    return 0;
  } catch (error: unknown) {
    io.stderr(errorMessage(error));
    return 1;
  }
}

if (process.argv[1] !== undefined && pathToFileURL(resolve(process.argv[1])).href === import.meta.url) {
  process.exitCode = await runImportTopDeckCandidatesCommand(process.argv.slice(2));
}
