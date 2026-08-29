import assert from 'node:assert/strict';
import { spawnSync } from 'node:child_process';
import { mkdir, mkdtemp, readFile, rm, writeFile } from 'node:fs/promises';
import { join, resolve } from 'node:path';
import test from 'node:test';

import { canonicalJson, type JsonValue } from '../../src/authority/canonical-json.ts';
import { createCanonicalArtifact, type NormalizedCardSnapshot } from '../../src/authority/schemas.ts';
import { runCollectionPhotoScanCommand } from '../../src/commands/scan-collection-photos.ts';

const REPOSITORY_ROOT = resolve(import.meta.dirname, '..', '..');
const AUTHORITY_ROOT = join(REPOSITORY_ROOT, '.local', 'authority');
const COLLECTION_ROOT = join(REPOSITORY_ROOT, '.local', 'collection');
const HASH = `sha256:${'1'.repeat(64)}` as const;
const HAS_OPENCV = spawnSync('python', ['-c', 'import cv2'], { windowsHide: true }).status === 0;

function syntheticCard(width: number, height: number): Uint8Array {
  const pixels = new Uint8Array(width * height * 3);
  for (let y = 0; y < height; y += 1) {
    for (let x = 0; x < width; x += 1) {
      const index = (y * width + x) * 3;
      const border = x < 8 || y < 8 || x >= width - 8 || y >= height - 8;
      const checker = (Math.floor(x / 16) + Math.floor(y / 16)) % 2 === 0;
      const stripe = Math.abs(x - Math.floor(width * 0.63)) < 5 || Math.abs(y - Math.floor(height * 0.31)) < 5;
      pixels[index] = border ? 15 : stripe ? 230 : checker ? 180 : 40;
      pixels[index + 1] = border ? 20 : stripe ? 40 : checker ? 60 : 190;
      pixels[index + 2] = border ? 25 : stripe ? 80 : checker ? 160 : 70;
    }
  }
  return pixels;
}

function ppm(width: number, height: number, pixels: Uint8Array): Buffer {
  return Buffer.concat([Buffer.from(`P6\n${width} ${height}\n255\n`, 'ascii'), Buffer.from(pixels)]);
}

function duplicatePage(card: Uint8Array, width: number, height: number): Uint8Array {
  const page = new Uint8Array(width * 2 * height * 3);
  for (let y = 0; y < height; y += 1) {
    const sourceStart = y * width * 3;
    const targetStart = y * width * 2 * 3;
    page.set(card.subarray(sourceStart, sourceStart + width * 3), targetStart);
    page.set(card.subarray(sourceStart, sourceStart + width * 3), targetStart + width * 3);
  }
  return page;
}

function tabletopPage(card: Uint8Array, cardWidth: number, cardHeight: number): Readonly<{
  width: number;
  height: number;
  pixels: Uint8Array;
}> {
  const width = 720;
  const height = 520;
  const pixels = new Uint8Array(width * height * 3).fill(235);
  for (const [left, top] of [[40, 40], [410, 100]] as const) {
    for (let y = 0; y < cardHeight; y += 1) {
      const sourceStart = y * cardWidth * 3;
      const targetStart = ((top + y) * width + left) * 3;
      pixels.set(card.subarray(sourceStart, sourceStart + cardWidth * 3), targetStart);
    }
  }
  return { width, height, pixels };
}

async function removeFixture(path: string, expectedRoot: string): Promise<void> {
  assert.ok(path.startsWith(expectedRoot + '\\') || path.startsWith(expectedRoot + '/'));
  await rm(path, { recursive: true, force: true });
}

function runPrivateDetector(request: JsonValue): Readonly<{
  photos: readonly Readonly<{
    detections: readonly Readonly<{
      detectionId: string;
      quad: readonly (readonly [number, number])[];
      status: string;
    }>[];
  }>[];
}> {
  const result = spawnSync('python', [join(REPOSITORY_ROOT, 'scripts', 'scan-card-photos.py')], {
    cwd: REPOSITORY_ROOT,
    input: canonicalJson(request),
    encoding: 'utf8',
    maxBuffer: 1_000_000,
    timeout: 30_000,
    windowsHide: true,
  });
  assert.equal(result.status, 0, result.stderr);
  return JSON.parse(result.stdout) as ReturnType<typeof runPrivateDetector>;
}

test('private-local ignore boundary covers collection photos and outputs', async () => {
  const ignore = await readFile(join(REPOSITORY_ROOT, '.gitignore'), 'utf8');
  assert.match(ignore, /^\.local\/collection\/$/m);
});

test('tabletop contour detection finds separated cards in stable reading order', {
  skip: HAS_OPENCV ? false : 'Python OpenCV is not installed',
}, async () => {
  await mkdir(COLLECTION_ROOT, { recursive: true });
  const fixtureRoot = await mkdtemp(join(COLLECTION_ROOT, 'photo-contour-test-'));
  try {
    const cardWidth = 180;
    const cardHeight = 252;
    const cardPixels = syntheticCard(cardWidth, cardHeight);
    const tabletop = tabletopPage(cardPixels, cardWidth, cardHeight);
    const referencePath = join(fixtureRoot, 'reference.ppm');
    const photoPath = join(fixtureRoot, 'tabletop.ppm');
    await Promise.all([
      writeFile(referencePath, ppm(cardWidth, cardHeight, cardPixels)),
      writeFile(photoPath, ppm(tabletop.width, tabletop.height, tabletop.pixels)),
    ]);
    const output = runPrivateDetector({
      schemaVersion: 1,
      photos: [{ photoId: 'tabletop', path: photoPath }],
      references: [{ labelId: 'alpha', path: referencePath }],
      options: {
        confidenceThreshold: 0.05,
        marginThreshold: 0,
        minimumCardAreaRatio: 0.05,
        maximumCardAreaRatio: 0.5,
        grid: null,
      },
    });
    assert.deepEqual(output.photos[0]?.detections.map(({ detectionId }) => detectionId), [
      'tabletop:0',
      'tabletop:1',
    ]);
    assert.ok((output.photos[0]?.detections[0]?.quad[0]?.[0] ?? 1_000) < 100);
    assert.ok((output.photos[0]?.detections[1]?.quad[0]?.[0] ?? 0) > 350);
    assert.ok(output.photos[0]?.detections.every(({ status }) => status === 'confirmed'));

    const blankPath = join(fixtureRoot, 'blank.ppm');
    await writeFile(blankPath, ppm(cardWidth, cardHeight, new Uint8Array(cardWidth * cardHeight * 3).fill(128)));
    const zeroEvidence = runPrivateDetector({
      schemaVersion: 1,
      photos: [{ photoId: 'blank', path: blankPath }],
      references: [{ labelId: 'blank-reference', path: blankPath }],
      options: {
        confidenceThreshold: 0,
        marginThreshold: 0,
        minimumCardAreaRatio: 0.01,
        maximumCardAreaRatio: 0.7,
        grid: { columns: 1, rows: 1 },
      },
    });
    assert.equal(zeroEvidence.photos[0]?.detections[0]?.status, 'review');

    const tied = runPrivateDetector({
      schemaVersion: 1,
      photos: [{ photoId: 'tie', path: referencePath }],
      references: [
        { labelId: 'alpha', path: referencePath },
        { labelId: 'beta', path: referencePath },
      ],
      options: {
        confidenceThreshold: 0,
        marginThreshold: 0,
        minimumCardAreaRatio: 0.01,
        maximumCardAreaRatio: 0.7,
        grid: { columns: 1, rows: 1 },
      },
    });
    assert.equal(tied.photos[0]?.detections[0]?.status, 'review');
  } finally {
    await removeFixture(fixtureRoot, COLLECTION_ROOT);
  }
});

test('grid scan identifies duplicate private references and writes an additive reconciliation', {
  skip: HAS_OPENCV ? false : 'Python OpenCV is not installed',
}, async () => {
  await Promise.all([
    mkdir(AUTHORITY_ROOT, { recursive: true }),
    mkdir(COLLECTION_ROOT, { recursive: true }),
  ]);
  const authorityRoot = await mkdtemp(join(AUTHORITY_ROOT, 'photo-scan-test-'));
  const collectionRoot = await mkdtemp(join(COLLECTION_ROOT, 'photo-scan-test-'));
  try {
    const photosRoot = join(collectionRoot, 'photos');
    const referencesRoot = join(collectionRoot, 'references');
    const reportsRoot = join(collectionRoot, 'reports');
    await Promise.all([
      mkdir(photosRoot),
      mkdir(referencesRoot),
      mkdir(reportsRoot),
    ]);
    const width = 240;
    const height = 336;
    const cardPixels = syntheticCard(width, height);
    const pageBytes = ppm(width * 2, height, duplicatePage(cardPixels, width, height));
    await Promise.all([
      writeFile(join(referencesRoot, 'alpha.ppm'), ppm(width, height, cardPixels)),
      writeFile(join(photosRoot, 'page.ppm'), pageBytes),
    ]);

    const snapshot: NormalizedCardSnapshot = { cards: [{
      stableId: 'card:alpha',
      officialSourceId: null,
      name: 'Alpha',
      cardType: 'minion',
      elements: ['earth'],
      rarity: 'ordinary',
      manaCost: 1,
      attack: 1,
      defense: 1,
      life: null,
      thresholds: { air: 0, earth: 1, fire: 0, water: 0 },
      rulesText: '',
      printingSlugs: ['alpha-base'],
    }] };
    const artifact = createCanonicalArtifact({
      artifactKind: 'card-snapshot',
      stableId: 'card-snapshot:synthetic-collection-test',
      schemaVersion: 1,
      parentRefs: [],
      sourceRefs: [{ sourceId: 'source:synthetic-collection-test', byteHash: HASH }],
      payload: snapshot as unknown as JsonValue,
    });
    const authorityPath = join(authorityRoot, 'cards.normalized.json');
    const catalogPath = join(collectionRoot, 'catalog.json');
    const savedPath = join(collectionRoot, 'saved.json');
    const reportPath = join(reportsRoot, 'scan.json');
    const collectionOutPath = join(collectionRoot, 'collection-next.json');
    await Promise.all([
      writeFile(authorityPath, canonicalJson(artifact)),
      writeFile(catalogPath, canonicalJson({
        schemaVersion: 1,
        entries: [{
          cardId: 'card:alpha',
          printingSlug: 'alpha-base',
          finish: 'standard',
          referenceImages: ['references/alpha.ppm'],
        }],
      })),
      writeFile(savedPath, canonicalJson({
        schemaVersion: 1,
        counts: [{ cardId: 'card:alpha', printingSlug: 'alpha-base', finish: 'standard', count: 1 }],
      })),
    ]);

    const commandArguments = [
      '--authority', authorityPath,
      '--catalog', catalogPath,
      '--photos', photosRoot,
      '--saved', savedPath,
      '--report', reportPath,
      '--collection-out', collectionOutPath,
      '--grid', '2x1',
      '--mode', 'add',
    ] as const;
    const duplicatePath = join(photosRoot, 'page-copy.ppm');
    await writeFile(duplicatePath, pageBytes);
    const duplicateErrors: string[] = [];
    assert.equal(await runCollectionPhotoScanCommand(commandArguments, {
      stdout: () => undefined,
      stderr: (line) => duplicateErrors.push(line),
    }), 1);
    assert.equal((JSON.parse(duplicateErrors[0] ?? '{}') as { code?: string }).code, 'duplicate_photo');
    await rm(duplicatePath);

    const stdout: string[] = [];
    const stderr: string[] = [];
    const code = await runCollectionPhotoScanCommand(commandArguments, {
      stdout: (line) => stdout.push(line),
      stderr: (line) => stderr.push(line),
    });

    assert.equal(code, 0, stderr.join('\n'));
    assert.deepEqual(stderr, []);
    assert.deepEqual(JSON.parse(stdout.join('\n')), {
      collectionWritten: true,
      confirmed: 2,
      detected: 2,
      dismissed: 0,
      status: 'reconciled',
      unresolved: 0,
    });
    const collection = JSON.parse(await readFile(collectionOutPath, 'utf8')) as {
      counts: readonly Readonly<{ count: number }>[];
    };
    assert.equal(collection.counts[0]?.count, 3);
    const reportText = await readFile(reportPath, 'utf8');
    const report = JSON.parse(reportText) as { unresolvedDetectionIds: readonly string[] };
    assert.deepEqual(report.unresolvedDetectionIds, []);
    assert.equal(reportText.includes(collectionRoot), false);
    assert.equal(reportText.includes(authorityRoot), false);
  } finally {
    await Promise.all([
      removeFixture(authorityRoot, AUTHORITY_ROOT),
      removeFixture(collectionRoot, COLLECTION_ROOT),
    ]);
  }
});
