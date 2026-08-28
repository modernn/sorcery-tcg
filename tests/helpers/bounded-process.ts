import { execFile, type ExecFileException } from 'node:child_process';
import { promisify } from 'node:util';

const execFileAsync = promisify(execFile);

export type ProcessResult = Readonly<{
  code: number | null;
  stdout: string;
  stderr: string;
}>;

export async function runBounded(
  command: string,
  arguments_: readonly string[],
  cwd: string,
  options: Readonly<{
    environment?: NodeJS.ProcessEnv;
    maxBuffer?: number;
    timeout?: number;
  }> = {},
): Promise<ProcessResult> {
  try {
    const { stdout, stderr } = await execFileAsync(command, [...arguments_], {
      cwd,
      encoding: 'utf8',
      env: options.environment ?? process.env,
      killSignal: 'SIGKILL',
      maxBuffer: options.maxBuffer ?? 1_048_576,
      timeout: options.timeout ?? 120_000,
      windowsHide: true,
    });
    return { code: 0, stdout, stderr };
  } catch (error) {
    const failure = error as ExecFileException & { stdout?: string; stderr?: string };
    return {
      code: typeof failure.code === 'number' ? failure.code : null,
      stdout: failure.stdout ?? '',
      stderr: failure.stderr ?? '',
    };
  }
}
