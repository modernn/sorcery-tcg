import assert from 'node:assert/strict';
import test from 'node:test';

import type { NormalizedCardSnapshot } from '../../src/authority/schemas.ts';
import {
  buildScanReport,
  createScanBinding,
  detectorOutputSchema,
  validateReviewManifest,
  validateSavedCollection,
  validateScanCatalog,
} from '../../src/collection/photo-collection.ts';

const HASH = `sha256:${'0'.repeat(64)}` as const;
const PHOTO_HASHES = [{ photoId: 'photo-0001', byteHash: HASH }] as const;
const snapshot: NormalizedCardSnapshot = {
  cards: [
    card('card:alpha', 'Alpha', ['alpha-base']),
    card('card:beta', 'Beta', ['beta-base']),
  ],
};

function card(
  stableId: string,
  name: string,
  printingSlugs: readonly string[],
): NormalizedCardSnapshot['cards'][number] {
  return {
    stableId,
    officialSourceId: null,
    name,
    cardType: 'minion',
    elements: ['earth'],
    rarity: 'ordinary',
    manaCost: 1,
    attack: 1,
    defense: 1,
    life: null,
    thresholds: { air: 0, earth: 1, fire: 0, water: 0 },
    rulesText: '',
    printingSlugs,
  };
}

function quad(offset: number): [[number, number], [number, number], [number, number], [number, number]] {
  return [[offset, 0], [offset + 10, 0], [offset + 10, 14], [offset, 14]];
}

function detector(status: 'confirmed' | 'review' = 'confirmed') {
  return detectorOutputSchema.parse({
    schemaVersion: 1,
    runtime: { python: 'synthetic', opencv: 'synthetic' },
    photos: [{
      photoId: 'photo-0001',
      detections: [
        { detectionId: 'photo-0001:0001', quad: quad(0), candidates: [{ labelId: 'label-00001', confidence: 0.91 }], status: 'confirmed' },
        { detectionId: 'photo-0001:0002', quad: quad(20), candidates: [{ labelId: 'label-00001', confidence: 0.89 }], status: 'confirmed' },
        { detectionId: 'photo-0001:0003', quad: quad(40), candidates: [], status },
      ],
    }],
  });
}

const catalog = validateScanCatalog({
  schemaVersion: 1,
  entries: [{
    cardId: 'card:alpha',
    printingSlug: 'alpha-base',
    finish: 'standard',
    referenceImages: ['references/alpha.ppm'],
  }],
}, snapshot);

const saved = validateSavedCollection({
  schemaVersion: 1,
  counts: [
    { cardId: 'card:alpha', printingSlug: 'alpha-base', finish: 'standard', count: 1 },
    { cardId: 'card:beta', printingSlug: 'beta-base', finish: 'standard', count: 4 },
  ],
}, snapshot);

test('confirmed references and review decisions count duplicates and reconcile exact additions', () => {
  const detected = detector('review');
  const review = validateReviewManifest({
    schemaVersion: 1,
    scanBinding: createScanBinding(detected, PHOTO_HASHES, catalog),
    decisions: [{
      detectionId: 'photo-0001:0003',
      decision: 'card',
      cardId: 'card:beta',
      printingSlug: 'beta-base',
      finish: 'foil',
    }],
  }, snapshot);
  const report = buildScanReport({
    detector: detected,
    catalog,
    snapshot,
    savedCollection: saved,
    review,
    mode: 'add',
    photoHashes: PHOTO_HASHES,
  });

  assert.deepEqual(report.unresolvedDetectionIds, []);
  assert.deepEqual(report.confirmedCounts.map(({ cardId, finish, count }) => ({ cardId, finish, count })), [
    { cardId: 'card:alpha', finish: 'standard', count: 2 },
    { cardId: 'card:beta', finish: 'foil', count: 1 },
  ]);
  assert.deepEqual(report.proposedCollection?.counts, [
    { cardId: 'card:alpha', printingSlug: 'alpha-base', finish: 'standard', count: 3 },
    { cardId: 'card:beta', printingSlug: 'beta-base', finish: 'foil', count: 1 },
    { cardId: 'card:beta', printingSlug: 'beta-base', finish: 'standard', count: 4 },
  ]);
});

test('unresolved detections are reported and prevent a collection proposal', () => {
  const detected = detector('review');
  const report = buildScanReport({
    detector: detected,
    catalog,
    snapshot,
    savedCollection: saved,
    review: validateReviewManifest({
      schemaVersion: 1,
      scanBinding: createScanBinding(detected, PHOTO_HASHES, catalog),
      decisions: [],
    }, snapshot),
    mode: 'replace',
    photoHashes: PHOTO_HASHES,
  });

  assert.deepEqual(report.unresolvedDetectionIds, ['photo-0001:0003']);
  assert.equal(report.proposedCollection, null);
  assert.equal(report.reconciliation.find(({ cardId }) => cardId === 'card:beta')?.next, 0);
});

test('human review can dismiss a false card detection without counting it', () => {
  const detected = detector('review');
  const report = buildScanReport({
    detector: detected,
    catalog,
    snapshot,
    savedCollection: saved,
    review: validateReviewManifest({
      schemaVersion: 1,
      scanBinding: createScanBinding(detected, PHOTO_HASHES, catalog),
      decisions: [
        { detectionId: 'photo-0001:0001', decision: 'not-card' },
        { detectionId: 'photo-0001:0003', decision: 'not-card' },
      ],
    }, snapshot),
    mode: 'add',
    photoHashes: PHOTO_HASHES,
  });

  assert.deepEqual(report.unresolvedDetectionIds, []);
  assert.equal(report.detections[0]?.disposition, 'dismissed');
  assert.equal(report.detections[0]?.confirmedBy, null);
  assert.deepEqual(report.confirmedCounts.map(({ cardId, count }) => ({ cardId, count })), [
    { cardId: 'card:alpha', count: 1 },
  ]);
  assert.ok(report.proposedCollection);
});

test('review decisions are rejected when the photo scan binding changed', () => {
  const detected = detector('review');
  assert.throws(() => buildScanReport({
    detector: detected,
    catalog,
    snapshot,
    savedCollection: saved,
    review: validateReviewManifest({
      schemaVersion: 1,
      scanBinding: HASH,
      decisions: [{ detectionId: 'photo-0001:0003', decision: 'not-card' }],
    }, snapshot),
    mode: 'add',
    photoHashes: PHOTO_HASHES,
  }), /review does not match the current scan/);
});

test('catalog, collection, and review identities must exist in the authority snapshot', () => {
  assert.throws(() => validateScanCatalog({
    schemaVersion: 1,
    entries: [{
      cardId: 'card:missing',
      printingSlug: 'missing-base',
      finish: 'standard',
      referenceImages: ['references/missing.ppm'],
    }],
  }, snapshot), /unknown authority card/);
  assert.throws(() => validateSavedCollection({
    schemaVersion: 1,
    counts: [
      { cardId: 'card:alpha', printingSlug: 'wrong-printing', finish: 'standard', count: 1 },
    ],
  }, snapshot), /unknown authority printing/);
  assert.throws(() => validateReviewManifest({
    schemaVersion: 1,
    scanBinding: HASH,
    decisions: [
      { detectionId: 'one', decision: 'card', cardId: 'card:alpha', printingSlug: 'alpha-base', finish: 'standard' },
      { detectionId: 'one', decision: 'card', cardId: 'card:alpha', printingSlug: 'alpha-base', finish: 'standard' },
    ],
  }, snapshot), /duplicate review decision/);
  assert.throws(() => validateScanCatalog({
    schemaVersion: 1,
    entries: [{
      cardId: 'card:alpha',
      printingSlug: 'alpha-base',
      finish: 'Foil',
      referenceImages: ['references/alpha.ppm'],
    }],
  }, snapshot));
});
