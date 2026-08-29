import { z } from 'zod';

import { identityHash } from '../authority/hash.ts';
import type { JsonValue } from '../authority/canonical-json.ts';
import type { NormalizedCardSnapshot } from '../authority/schemas.ts';

const stableIdSchema = z.string().regex(/^[a-z][a-z0-9-]*(?::[a-z0-9][a-z0-9._-]*)+$/);
const slugSchema = z.string().regex(/^[a-z0-9]+(?:[-_][a-z0-9]+)*$/);
const finishSchema = z.string().min(1).max(80).regex(/^[a-z0-9][a-z0-9._ -]*$/);
const hashSchema = z.string().regex(/^sha256:[0-9a-f]{64}$/);
const relativeImagePathSchema = z.string().min(1).max(500).refine((value) => {
  if (value.startsWith('/') || value.startsWith('\\') || /^[a-z]:/i.test(value) || value.includes('\\')) return false;
  return value.split('/').every((segment) => segment !== '' && segment !== '.' && segment !== '..');
}, 'reference image must be a confined slash-separated relative path');

const identityFields = {
  cardId: stableIdSchema,
  printingSlug: slugSchema,
  finish: finishSchema,
} as const;

const catalogEntrySchema = z.strictObject({
  ...identityFields,
  referenceImages: z.array(relativeImagePathSchema).min(1).max(20),
});

export const scanCatalogSchema = z.strictObject({
  schemaVersion: z.literal(1),
  entries: z.array(catalogEntrySchema).min(1).max(5_000),
}).superRefine((catalog, context) => {
  const identities = new Set<string>();
  catalog.entries.forEach((entry, index) => {
    const key = collectionKey(entry);
    if (identities.has(key)) {
      context.addIssue({ code: 'custom', path: ['entries', index], message: 'duplicate catalog identity' });
    }
    identities.add(key);
  });
  if (catalog.entries.reduce((total, entry) => total + entry.referenceImages.length, 0) > 5_000) {
    context.addIssue({ code: 'custom', path: ['entries'], message: 'catalog exceeds 5,000 total reference images' });
  }
});

const countSchema = z.strictObject({
  ...identityFields,
  count: z.number().int().nonnegative().safe(),
});

export const savedCollectionSchema = z.strictObject({
  schemaVersion: z.literal(1),
  counts: z.array(countSchema).max(20_000),
}).superRefine((collection, context) => {
  const identities = new Set<string>();
  collection.counts.forEach((entry, index) => {
    const key = collectionKey(entry);
    if (identities.has(key)) {
      context.addIssue({ code: 'custom', path: ['counts', index], message: 'duplicate collection identity' });
    }
    identities.add(key);
  });
});

const cardReviewDecisionSchema = z.strictObject({
  detectionId: z.string().min(1).max(300),
  decision: z.literal('card'),
  ...identityFields,
});

const notCardReviewDecisionSchema = z.strictObject({
  detectionId: z.string().min(1).max(300),
  decision: z.literal('not-card'),
});

const reviewDecisionSchema = z.discriminatedUnion('decision', [
  cardReviewDecisionSchema,
  notCardReviewDecisionSchema,
]);

export const reviewManifestSchema = z.strictObject({
  schemaVersion: z.literal(1),
  scanBinding: hashSchema,
  decisions: z.array(reviewDecisionSchema).max(20_000),
}).superRefine((review, context) => {
  const detectionIds = new Set<string>();
  review.decisions.forEach((decision, index) => {
    if (detectionIds.has(decision.detectionId)) {
      context.addIssue({ code: 'custom', path: ['decisions', index, 'detectionId'], message: 'duplicate review decision' });
    }
    detectionIds.add(decision.detectionId);
  });
});

const detectorCandidateSchema = z.strictObject({
  labelId: z.string().min(1).max(100),
  confidence: z.number().min(0).max(1),
});

const pointSchema = z.tuple([
  z.number().int().nonnegative().safe(),
  z.number().int().nonnegative().safe(),
]);

const detectorDetectionSchema = z.strictObject({
  detectionId: z.string().min(1).max(300),
  quad: z.tuple([pointSchema, pointSchema, pointSchema, pointSchema]),
  candidates: z.array(detectorCandidateSchema).max(3),
  status: z.enum(['confirmed', 'review']),
});

const detectorPhotoSchema = z.strictObject({
  photoId: z.string().min(1).max(200),
  detections: z.array(detectorDetectionSchema).max(10_000),
});

export const detectorOutputSchema = z.strictObject({
  schemaVersion: z.literal(1),
  runtime: z.strictObject({
    python: z.string().min(1).max(100),
    opencv: z.string().min(1).max(100),
  }),
  photos: z.array(detectorPhotoSchema).max(100),
}).superRefine((output, context) => {
  const photoIds = new Set<string>();
  const detectionIds = new Set<string>();
  output.photos.forEach((photo, photoIndex) => {
    if (photoIds.has(photo.photoId)) {
      context.addIssue({ code: 'custom', path: ['photos', photoIndex, 'photoId'], message: 'duplicate photo ID' });
    }
    photoIds.add(photo.photoId);
    photo.detections.forEach((detection, detectionIndex) => {
      if (detectionIds.has(detection.detectionId)) {
        context.addIssue({
          code: 'custom',
          path: ['photos', photoIndex, 'detections', detectionIndex, 'detectionId'],
          message: 'duplicate detection ID',
        });
      }
      detectionIds.add(detection.detectionId);
    });
  });
});

export type ScanCatalog = z.infer<typeof scanCatalogSchema>;
export type SavedCollection = z.infer<typeof savedCollectionSchema>;
export type ReviewManifest = z.infer<typeof reviewManifestSchema>;
export type DetectorOutput = z.infer<typeof detectorOutputSchema>;
export type CollectionMode = 'add' | 'replace';
export type PhotoHash = Readonly<{ photoId: string; byteHash: `sha256:${string}` }>;

export type ValidatedCatalogEntry = Readonly<{
  labelId: string;
  cardId: string;
  printingSlug: string;
  finish: string;
  referenceImages: readonly string[];
}>;

type Identity = Readonly<{
  cardId: string;
  printingSlug: string;
  finish: string;
}>;

function collectionKey(identity: Identity): string {
  return `${identity.cardId}\0${identity.printingSlug}\0${identity.finish}`;
}

function compareIdentity(left: Identity, right: Identity): number {
  return compareText(left.cardId, right.cardId)
    || compareText(left.printingSlug, right.printingSlug)
    || compareText(left.finish, right.finish);
}

function compareText(left: string, right: string): number {
  return left < right ? -1 : left > right ? 1 : 0;
}

function authorityCardMap(snapshot: NormalizedCardSnapshot): ReadonlyMap<string, NormalizedCardSnapshot['cards'][number]> {
  return new Map(snapshot.cards.map((card) => [card.stableId, card]));
}

function validateIdentity(
  identity: Identity,
  cards: ReadonlyMap<string, NormalizedCardSnapshot['cards'][number]>,
  context: string,
): NormalizedCardSnapshot['cards'][number] {
  const card = cards.get(identity.cardId);
  if (!card) throw new Error(`${context} references an unknown authority card`);
  if (!card.printingSlugs.includes(identity.printingSlug)) {
    throw new Error(`${context} references an unknown authority printing`);
  }
  return card;
}

export function validateScanCatalog(
  input: unknown,
  snapshot: NormalizedCardSnapshot,
): readonly ValidatedCatalogEntry[] {
  const parsed = scanCatalogSchema.parse(input);
  const cards = authorityCardMap(snapshot);
  return parsed.entries.map((entry, index) => {
    validateIdentity(entry, cards, `catalog entry ${index}`);
    return Object.freeze({
      ...entry,
      labelId: `label-${String(index + 1).padStart(5, '0')}`,
    });
  });
}

export function validateSavedCollection(
  input: unknown,
  snapshot: NormalizedCardSnapshot,
): SavedCollection {
  const parsed = savedCollectionSchema.parse(input);
  const cards = authorityCardMap(snapshot);
  parsed.counts.forEach((count, index) => validateIdentity(count, cards, `saved count ${index}`));
  return parsed;
}

export function validateReviewManifest(
  input: unknown,
  snapshot: NormalizedCardSnapshot,
): ReviewManifest {
  const parsed = reviewManifestSchema.parse(input);
  const cards = authorityCardMap(snapshot);
  parsed.decisions.forEach((decision, index) => {
    if (decision.decision === 'card') validateIdentity(decision, cards, `review decision ${index}`);
  });
  return parsed;
}

export type ScanReport = Readonly<{
  schemaVersion: 1;
  scanBinding: `sha256:${string}`;
  runtime: DetectorOutput['runtime'];
  photoHashes: readonly PhotoHash[];
  detections: readonly Readonly<{
    detectionId: string;
    photoId: string;
    quad: DetectorOutput['photos'][number]['detections'][number]['quad'];
    candidates: readonly Readonly<Identity & { confidence: number }>[];
    confirmed: Identity | null;
    confirmedBy: 'automatic' | 'review' | null;
    disposition: 'confirmed' | 'dismissed' | 'review';
  }>[];
  confirmedCounts: readonly Readonly<Identity & { count: number }>[];
  unresolvedDetectionIds: readonly string[];
  reconciliation: readonly Readonly<Identity & {
    previous: number;
    scanned: number;
    delta: number;
    next: number;
  }>[];
  proposedCollection: SavedCollection | null;
}>;

export function buildScanReport(args: Readonly<{
  detector: DetectorOutput;
  catalog: readonly ValidatedCatalogEntry[];
  snapshot: NormalizedCardSnapshot;
  savedCollection: SavedCollection;
  review: ReviewManifest;
  mode: CollectionMode;
  photoHashes: readonly PhotoHash[];
}>): ScanReport {
  const scanBinding = createScanBinding(args.detector, args.photoHashes, args.catalog);
  if (args.review.scanBinding !== scanBinding) throw new Error('review does not match the current scan');
  const cards = authorityCardMap(args.snapshot);
  const labels = new Map(args.catalog.map((entry) => [entry.labelId, entry]));
  const reviewByDetection = new Map(args.review.decisions.map((decision) => [decision.detectionId, decision]));
  const detectorIds = new Set(args.detector.photos.flatMap((photo) => photo.detections.map(({ detectionId }) => detectionId)));
  for (const detectionId of reviewByDetection.keys()) {
    if (!detectorIds.has(detectionId)) throw new Error('review references an unknown detection');
  }

  const detections: Array<ScanReport['detections'][number]> = [];
  for (const photo of args.detector.photos) {
    for (const detection of photo.detections) {
      const candidates = detection.candidates.map((candidate) => {
        const label = labels.get(candidate.labelId);
        if (!label) throw new Error('detector returned an unknown catalog label');
        return {
          cardId: label.cardId,
          printingSlug: label.printingSlug,
          finish: label.finish,
          confidence: candidate.confidence,
        };
      });
      const reviewed = reviewByDetection.get(detection.detectionId);
      const automatic = detection.status === 'confirmed' ? candidates[0] : undefined;
      const confirmed = reviewed?.decision === 'not-card'
        ? null
        : reviewed
        ? (() => {
            validateIdentity(reviewed, cards, 'review');
            return {
              cardId: reviewed.cardId,
              printingSlug: reviewed.printingSlug,
              finish: reviewed.finish,
            };
          })()
        : automatic
          ? {
              cardId: automatic.cardId,
              printingSlug: automatic.printingSlug,
              finish: automatic.finish,
            }
          : null;
      detections.push({
        detectionId: detection.detectionId,
        photoId: photo.photoId,
        quad: detection.quad,
        candidates,
        confirmed,
        confirmedBy: reviewed?.decision === 'not-card'
          ? null
          : reviewed?.decision === 'card'
            ? 'review'
            : automatic
              ? 'automatic'
              : null,
        disposition: reviewed?.decision === 'not-card' ? 'dismissed' : confirmed ? 'confirmed' : 'review',
      });
    }
  }

  const counts = new Map<string, Identity & { count: number }>();
  for (const detection of detections) {
    if (!detection.confirmed) continue;
    const key = collectionKey(detection.confirmed);
    const existing = counts.get(key);
    counts.set(key, { ...detection.confirmed, count: (existing?.count ?? 0) + 1 });
  }
  const confirmedCounts = [...counts.values()].sort(compareIdentity);
  const unresolvedDetectionIds = detections
    .filter(({ disposition }) => disposition === 'review')
    .map(({ detectionId }) => detectionId)
    .sort();

  const previous = new Map(args.savedCollection.counts.map((count) => [collectionKey(count), count]));
  const identities = new Map<string, Identity>();
  args.savedCollection.counts.forEach((count) => identities.set(collectionKey(count), count));
  confirmedCounts.forEach((count) => identities.set(collectionKey(count), count));
  const reconciliation = [...identities.values()].sort(compareIdentity).map((identity) => {
    const previousCount = previous.get(collectionKey(identity))?.count ?? 0;
    const scanned = counts.get(collectionKey(identity))?.count ?? 0;
    const next = args.mode === 'add' ? previousCount + scanned : scanned;
    return {
      ...identity,
      previous: previousCount,
      scanned,
      delta: next - previousCount,
      next,
    };
  });
  const proposedCollection = unresolvedDetectionIds.length === 0
    ? {
        schemaVersion: 1 as const,
        counts: reconciliation
          .filter(({ next }) => next > 0)
          .map(({ cardId, printingSlug, finish, next }) => ({ cardId, printingSlug, finish, count: next })),
      }
    : null;

  return Object.freeze({
    schemaVersion: 1,
    scanBinding,
    runtime: args.detector.runtime,
    photoHashes: [...args.photoHashes].sort((left, right) => compareText(left.photoId, right.photoId)),
    detections,
    confirmedCounts,
    unresolvedDetectionIds,
    reconciliation,
    proposedCollection,
  });
}

export function createScanBinding(
  detector: DetectorOutput,
  photoHashes: readonly PhotoHash[],
  catalog: readonly ValidatedCatalogEntry[],
): `sha256:${string}` {
  return identityHash({
    catalog: catalog.map(({ labelId, cardId, printingSlug, finish }) => ({
      labelId,
      cardId,
      printingSlug,
      finish,
    })),
    detector,
    photoHashes: [...photoHashes].sort((left, right) => compareText(left.photoId, right.photoId)),
  } as unknown as JsonValue);
}
