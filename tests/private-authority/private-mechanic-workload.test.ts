import assert from 'node:assert/strict';
import { mkdir, mkdtemp, readFile, rm, symlink } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join, relative, resolve } from 'node:path';
import test from 'node:test';

import { parseJsonWithDuplicateKeyCheck, type JsonValue } from '../../src/authority/canonical-json.ts';
import { identityHash } from '../../src/authority/hash.ts';
import { canonicalArtifactSchema, normalizedCardSnapshotSchema } from '../../src/authority/schemas.ts';
import {
  reportPrivateMechanicWorkload,
  resolveMechanicWorkloadOutputPath,
} from '../../src/commands/report-private-mechanic-workload.ts';
import { loadPrivateStarterCatalog } from '../../src/commands/run-private-game-check.ts';
import { EFFECT_FAMILY_IDS, FACET_IDS } from '../../src/mechanics/mechanic-workload.ts';
import { runBounded } from '../helpers/bounded-process.ts';

const REPOSITORY_ROOT = resolve(import.meta.dirname, '..', '..');
const SCENARIO_PATH = resolve(
  REPOSITORY_ROOT,
  '.local',
  'authority',
  'scenarios',
  'vanilla-constructed.json',
);

test('private mechanic workload rejects report-directory junction escapes and output symlinks', async () => {
  const sandbox = await mkdtemp(join(tmpdir(), 'sorcery-mechanic-workload-'));
  const authorityRoot = join(sandbox, 'authority');
  const reportsRoot = join(authorityRoot, 'reports');
  const outside = join(sandbox, 'outside');
  try {
    await mkdir(reportsRoot, { recursive: true });
    await mkdir(outside);
    await symlink(outside, join(reportsRoot, 'escaped'), process.platform === 'win32' ? 'junction' : 'dir');
    await assert.rejects(
      () => resolveMechanicWorkloadOutputPath(authorityRoot, 'escaped'),
      /resolved path escapes the configured authority root/,
    );

    await mkdir(join(reportsRoot, 'linked-output'));
    const outsideFile = join(outside, 'outside.json');
    await symlink(outsideFile, join(reportsRoot, 'linked-output', 'mechanic-workload.json'), 'file');
    await assert.rejects(
      () => resolveMechanicWorkloadOutputPath(authorityRoot, 'linked-output'),
      /output path may not be a symbolic link or junction/,
    );
  } finally {
    await rm(sandbox, { recursive: true, force: true });
  }
});

test('private mechanic workload covers the pinned catalog and six presets without leaking authority prose', async () => {
  let networkCalls = 0;
  const originalFetch = globalThis.fetch;
  globalThis.fetch = (() => {
    networkCalls += 1;
    throw new Error('network access is forbidden in private mechanic workload reporting');
  }) as typeof fetch;
  try {
    const result = await reportPrivateMechanicWorkload(SCENARIO_PATH);
    assert.equal(networkCalls, 0);
    assert.equal(result.report.cards.length, 1_100);
    assert.equal(result.report.totals.cards, 1_100);
    assert.deepEqual(result.report.totals, {
      blank: 31,
      cards: 1_100,
      classified: 1_046,
      presetDemandAbsent: 1_005,
      presetDemandPresent: 95,
      unclassified: 23,
    });
    assert.equal(
      result.report.totals.blank
        + result.report.totals.classified
        + result.report.totals.unclassified,
      1_100,
    );
    assert.deepEqual(result.report.families.map(({ id }) => id), EFFECT_FAMILY_IDS);
    assert.deepEqual(result.report.facets.map(({ id }) => id), FACET_IDS);
    assert.equal(
      result.report.scopeDisclaimer,
      'Lexical workload labels and preset demand only; this report proves neither engine implementation nor scenario verification.',
    );

    const scenario = parseJsonWithDuplicateKeyCheck(await readFile(SCENARIO_PATH, 'utf8')) as {
      revisionId: string;
    };
    const artifactPath = resolve(
      REPOSITORY_ROOT,
      '.local',
      'authority',
      'revisions',
      scenario.revisionId,
      'cards.normalized.json',
    );
    const artifact = canonicalArtifactSchema.parse(parseJsonWithDuplicateKeyCheck(
      await readFile(artifactPath, 'utf8'),
    ));
    const snapshot = normalizedCardSnapshotSchema.parse(artifact.identity.payload);
    const cardsById = new Map(snapshot.cards.map((card) => [card.stableId, card]));
    for (const card of result.report.cards) {
      const normalized = cardsById.get(card.stableId);
      assert.ok(normalized);
      assert.equal(card.ruleDigest, identityHash(normalized.rulesText as JsonValue));
      assert.equal(card.name, normalized.name);
      assert.equal(card.cardType, normalized.cardType);
      assert.ok(card.presetDemandStatus === 'present' || card.presetDemandStatus === 'absent');
      assert.equal(
        card.classification === 'classified',
        card.familyIds.length > 0,
      );
      if (card.classification === 'blank') {
        assert.deepEqual(card.familyIds, []);
        assert.deepEqual(card.facetIds, []);
      }
      if (card.classification === 'unclassified') assert.deepEqual(card.familyIds, []);
    }

    const presets = await loadPrivateStarterCatalog(SCENARIO_PATH);
    assert.equal(presets.length, 6);
    const expectedCopies = presets.reduce((total, preset) => total
      + preset.manifest.decks.north.atlas.length
      + preset.manifest.decks.north.spellbook.length
      + 1
      + preset.manifest.decks.south.atlas.length
      + preset.manifest.decks.south.spellbook.length
      + 1, 0);
    const reportedCopies = result.report.cards.flatMap(({ presetDemand }) => presetDemand)
      .reduce((total, impact) => total + impact.totalCopies, 0);
    assert.equal(reportedCopies, expectedCopies);
    assert.deepEqual(
      [...new Set(result.report.cards.flatMap(({ presetDemand }) =>
        presetDemand.map(({ presetId }) => presetId)))].sort(),
      presets.map(({ id }) => id).sort(),
    );

    const serialized = await readFile(result.outputPath, 'utf8');
    const parsed = JSON.parse(serialized) as Record<string, unknown>;
    assert.equal(Number((parsed.totals as Record<string, unknown>).cards), 1_100);
    assert.doesNotMatch(
      serialized,
      /bindingStatus|fallback|officialSourceId|presetBound|presetImpact|printingSlugs|relativePath|rulesText|sourceId|sourceRefs/,
    );
    const relativeOutput = relative(REPOSITORY_ROOT, result.outputPath).replaceAll('\\', '/');
    assert.equal(
      relativeOutput,
      `.local/authority/reports/${scenario.revisionId}/mechanic-workload.json`,
    );
    const ignored = await runBounded('git', ['check-ignore', '--quiet', relativeOutput], REPOSITORY_ROOT);
    assert.equal(ignored.code, 0, ignored.stderr);
  } finally {
    globalThis.fetch = originalFetch;
  }
});
