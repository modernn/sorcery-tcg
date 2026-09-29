// Node supplies HTTPS and process transport; all scenarios, search, and replay run in Rust.
import assert from 'node:assert/strict';
import { spawnSync } from 'node:child_process';
import { fileURLToPath } from 'node:url';
import { callJev, MODEL } from './jev-semantic-review.mjs';

function native(binary, operation, input) {
  const env = { ...process.env };
  delete env.TYPESAFE_API_KEY;
  delete env.JEV_API_KEY;
  const result = spawnSync('cargo', [
    'run', '--quiet', '--locked', '-p', 'sorcery-engine', '--bin', binary, '--', operation,
  ], {
    cwd: fileURLToPath(new URL('..', import.meta.url)),
    env, encoding: 'utf8', input: input === undefined ? undefined : JSON.stringify(input),
    timeout: 120_000, maxBuffer: 1_048_576,
  });
  if (result.status !== 0) throw new Error(`Fixed Rust ${binary} ${operation} failed`);
  return JSON.parse(result.stdout);
}

async function main() {
  const args = process.argv.slice(2);
  if (args.length !== 2 || new Set(args).size !== 2
      || !args.every(arg => ['--live', '--self-check', '--play', '--development'].includes(arg))
      || Number(args.includes('--live')) + Number(args.includes('--self-check')) !== 1
      || Number(args.includes('--play')) + Number(args.includes('--development')) !== 1) {
    throw new Error('Use (--live | --self-check) (--play | --development)');
  }
  const live = args.includes('--live');
  const evaluate = request => callJev(request, { live: true, apiKey: process.env.TYPESAFE_API_KEY });
  if (args.includes('--play')) {
    const binding = native('agent-play', 'binding');
    const request = native('agent-play', 'packet');
    const response = live ? await evaluate(request) : {
      model: MODEL, answers: { preferred_action: { type: 'choice', choice: 'a3' } },
    };
    const report = native('agent-play', 'evaluate', { binding, response });
    assert.equal(report.rootUnchanged, true);
    assert.ok([...report.baseline, ...report.ordered].every(branch => branch.replayVerified));
    return { externalModel: live ? response.model : 'not-called', response, report };
  }
  const evidence = native('agent-probe', 'receipt');
  assert.equal(evidence.clean.replayVerified, true);
  assert.equal(evidence.altered.resumeRejected, true);
  if (!live) return { externalModel: 'not-called', evidence };
  const initial = await evaluate(native('agent-probe', 'packet'));
  const followup = initial.answers.next_probe.choice === 'compare_stored_history'
    ? await evaluate(native('agent-probe', 'packet-after-history')) : null;
  return { externalModel: initial.model, initial, followup, evidence };
}

try {
  process.stdout.write(JSON.stringify(await main(), null, 2) + '\n');
} catch (error) {
  process.stderr.write((error instanceof Error ? error.message : 'Fixed Jev pilot failed') + '\n');
  process.exitCode = 1;
}
