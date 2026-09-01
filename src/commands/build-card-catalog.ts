import { relative, resolve } from 'node:path';
import { pathToFileURL } from 'node:url';
import { parseArgs } from 'node:util';

import { canonicalJson, type JsonValue } from '../authority/canonical-json.ts';
import { buildCardCatalog } from '../catalog/card-catalog.ts';

type CommandIo = Readonly<{
  stderr: (line: string) => void;
  stdout: (line: string) => void;
}>;

const defaultIo: CommandIo = {
  stderr: (line) => process.stderr.write(line + '\n'),
  stdout: (line) => process.stdout.write(line + '\n'),
};

export async function runBuildCardCatalogCommand(
  argv: readonly string[],
  repositoryRoot = process.cwd(),
  io: CommandIo = defaultIo,
): Promise<number> {
  try {
    const { tokens, values } = parseArgs({
      args: [...argv],
      allowPositionals: false,
      options: { revision: { type: 'string' } },
      strict: true,
      tokens: true,
    });
    if (tokens.length !== 1 || values.revision === undefined) {
      throw new Error('Provide exactly one --revision <immutable-revision-id>.');
    }
    const root = resolve(repositoryRoot);
    const built = await buildCardCatalog(values.revision, root);
    io.stdout(canonicalJson({
      artifactContentHash: built.artifactContentHash,
      bundleContentHash: built.bundleContentHash,
      cardCount: built.cardCount,
      outputPath: relative(root, built.outputPath).replaceAll('\\', '/'),
      revisionId: built.revisionId,
      status: 'created',
    } as JsonValue));
    return 0;
  } catch (error) {
    io.stderr(error instanceof Error ? error.message : 'Card catalog build failed.');
    return 1;
  }
}

if (process.argv[1] !== undefined && pathToFileURL(resolve(process.argv[1])).href === import.meta.url) {
  process.exitCode = await runBuildCardCatalogCommand(process.argv.slice(2));
}
