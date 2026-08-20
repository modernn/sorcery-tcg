import { z } from 'zod';

import {
  MAX_CANONICAL_DEPTH,
  MAX_CANONICAL_NODES,
  MAX_CANONICAL_STRING_BYTES,
  CanonicalJsonError,
  canonicalJson,
  parseJsonWithDuplicateKeyCheck,
  type JsonValue,
} from './canonical-json.ts';
import { identityHash } from './hash.ts';

export type Hash = ReturnType<typeof identityHash>;
export type ArtifactKind =
  | 'rule'
  | 'card'
  | 'format'
  | 'deck'
  | 'collection'
  | 'behavior'
  | 'experiment'
  | 'bundle'
  | 'source-manifest'
  | 'card-snapshot';

export type ArtifactRef = Readonly<{
  artifactKind: ArtifactKind;
  stableId: string;
  contentHash: Hash;
}>;

export type SourceRef = Readonly<{
  sourceId: string;
  byteHash: Hash;
}>;

export type IdentityDocument<T extends JsonValue> = Readonly<{
  artifactKind: ArtifactKind;
  stableId: string;
  schemaVersion: 1;
  parentRefs: readonly ArtifactRef[];
  sourceRefs: readonly SourceRef[];
  payload: T;
}>;

export type CanonicalArtifact<T extends JsonValue> = Readonly<{
  identity: IdentityDocument<T>;
  contentHash: Hash;
}>;

export type AuthorityClass = 'official' | 'community-provenance' | 'external-reference';
export type LicenseStatus = 'approved' | 'permission-required' | 'reference-only' | 'unknown';
export type DerivationMetadata = Readonly<{
  method: 'verbatim' | 'normalized' | 'manual-transcription';
  parentByteHashes: readonly Hash[];
  notes: string | null;
}>;

type SourceFields = Readonly<{
  sourceId: string;
  url: string;
  authorityClass: AuthorityClass;
  retrievedAt: string;
  effectiveDate: string | null;
  mediaType: string;
  byteHash: Hash;
  derivation: DerivationMetadata;
  licenseStatus: LicenseStatus;
}>;

export type StoredSourceRecord = SourceFields &
  Readonly<{
    storageMode: 'stored';
    storagePolicy: 'permitted';
    relativePath: string;
  }>;

export type ManifestOnlySourceRecord = SourceFields &
  Readonly<{
    storageMode: 'manifest-only';
    storagePolicy: 'manifest-only' | 'prohibited';
    durableLocator: string | null;
    acquisitionProcedureHash: Hash | null;
  }>;

export type SourceRecord = StoredSourceRecord | ManifestOnlySourceRecord;
export type SourceMetadata = SourceRecord;

export type RawCard = Readonly<{
  sourceCardId: string;
  name: string;
  cardType: 'avatar' | 'site' | 'minion' | 'aura' | 'artifact' | 'magic';
  elements: readonly ('earth' | 'fire' | 'water' | 'air')[];
  rarity: 'ordinary' | 'exceptional' | 'elite' | 'unique';
  manaCost: number | null;
  attack: number | null;
  defense: number | null;
  rulesText: string;
  printingSlugs: readonly string[];
  releasedAt: string | null;
}>;

export type NormalizedCard = Readonly<{
  stableId: string;
  officialSourceId: string | null;
  name: string;
  cardType: RawCard['cardType'];
  elements: RawCard['elements'];
  rarity: RawCard['rarity'];
  manaCost: number | null;
  attack: number | null;
  defense: number | null;
  rulesText: string;
  printingSlugs: readonly string[];
}>;

export type NormalizedCardSnapshot = Readonly<{
  cards: readonly NormalizedCard[];
}>;

export type FormatDefinition = Readonly<{
  name: string;
  effectiveDate: string;
  scope: string | null;
  parentFormatStableId: string | null;
  avatarCount: number;
  spellbookMinimum: number;
  atlasMinimum: number;
  copyLimits: Readonly<{
    ordinary: number;
    exceptional: number;
    elite: number;
    unique: number;
  }>;
}>;

export type FormatArtifact = CanonicalArtifact<FormatDefinition>;

export type AuthorityBundlePayload = Readonly<{
  effectiveDate: string;
  precedencePolicyVersion: 1;
  inputRootHash: Hash | null;
  sources: readonly SourceRecord[];
  artifacts: readonly CanonicalArtifact<JsonValue>[];
}>;

export type AuthorityBundle = CanonicalArtifact<AuthorityBundlePayload>;

export type Diagnostic = Readonly<{
  path: string;
  code: string;
  message: string;
}>;

export class AuthorityValidationError extends Error {
  readonly diagnostics: readonly Diagnostic[];

  constructor(diagnostics: readonly Diagnostic[]) {
    super('Authority validation failed');
    this.name = 'AuthorityValidationError';
    this.diagnostics = diagnostics;
  }
}

export type AuthorityJsonLimits = Readonly<{
  maxBytes: number;
  maxDepth: number;
  maxRecords: number;
  maxDiagnostics: number;
}>;

export const DEFAULT_AUTHORITY_JSON_LIMITS: AuthorityJsonLimits = Object.freeze({
  maxBytes: 10_000_000,
  maxDepth: MAX_CANONICAL_DEPTH,
  maxRecords: MAX_CANONICAL_NODES,
  maxDiagnostics: 100,
});

const HASH_PATTERN = /^sha256:[0-9a-f]{64}$/;
const STABLE_ID_PATTERN = /^[a-z][a-z0-9-]*(?::[a-z0-9][a-z0-9._-]*)+$/;
const SOURCE_ID_PATTERN = /^source:[a-z0-9][a-z0-9._-]*$/;
const SLUG_PATTERN = /^[a-z0-9]+(?:-[a-z0-9]+)*$/;
const MEDIA_TYPE_PATTERN = /^[a-z0-9!#$&^_.+-]+\/[a-z0-9!#$&^_.+-]+$/i;
const OFFICIAL_HOSTS = new Set([
  'sorcerytcg.com',
  'www.sorcerytcg.com',
  'api.sorcerytcg.com',
  'play.sorcerytcg.com',
  'curiosa.io',
  'www.curiosa.io',
]);

const hashSchema = z.string().regex(HASH_PATTERN);
const stableIdSchema = z.string().min(1).max(200).regex(STABLE_ID_PATTERN);
const sourceIdSchema = z.string().min(1).max(200).regex(SOURCE_ID_PATTERN);
const httpsUrlSchema = z.url().refine((value) => {
  const parsed = new URL(value);
  return parsed.protocol === 'https:' && parsed.username === '' && parsed.password === '';
}, 'URL must use HTTPS without credentials');
const durableLocatorSchema = z.union([
  httpsUrlSchema,
  z.string().regex(/^urn:sha256:[0-9a-f]{64}$/),
]);
const relativePathSchema = z.string().min(1).max(500).refine((value) => {
  if (value.startsWith('/') || value.startsWith('\\') || /^[a-z]:/i.test(value) || value.includes('\\')) return false;
  const segments = value.split('/');
  return segments.every((segment) => segment !== '' && segment !== '.' && segment !== '..');
}, 'path must be a confined slash-separated relative path');

export const artifactRefSchema = z.strictObject({
  artifactKind: z.enum([
    'rule',
    'card',
    'format',
    'deck',
    'collection',
    'behavior',
    'experiment',
    'bundle',
    'source-manifest',
    'card-snapshot',
  ]),
  stableId: stableIdSchema,
  contentHash: hashSchema,
});

export const sourceRefSchema = z.strictObject({
  sourceId: sourceIdSchema,
  byteHash: hashSchema,
});

const derivationSchema = z.strictObject({
  method: z.enum(['verbatim', 'normalized', 'manual-transcription']),
  parentByteHashes: z.array(hashSchema).max(100),
  notes: z.string().max(2_000).nullable(),
});

const sourceFields = {
  sourceId: sourceIdSchema,
  url: httpsUrlSchema,
  authorityClass: z.enum(['official', 'community-provenance', 'external-reference']),
  retrievedAt: z.iso.datetime({ offset: true }),
  effectiveDate: z.iso.date().nullable(),
  mediaType: z.string().min(1).max(200).regex(MEDIA_TYPE_PATTERN),
  byteHash: hashSchema,
  derivation: derivationSchema,
  licenseStatus: z.enum(['approved', 'permission-required', 'reference-only', 'unknown']),
};

const storedSourceRecordSchema = z.strictObject({
  ...sourceFields,
  storageMode: z.literal('stored'),
  storagePolicy: z.literal('permitted'),
  relativePath: relativePathSchema,
});

const manifestOnlySourceRecordSchema = z.strictObject({
  ...sourceFields,
  storageMode: z.literal('manifest-only'),
  storagePolicy: z.enum(['manifest-only', 'prohibited']),
  durableLocator: durableLocatorSchema.nullable(),
  acquisitionProcedureHash: hashSchema.nullable(),
});

export const sourceRecordSchema = z
  .discriminatedUnion('storageMode', [storedSourceRecordSchema, manifestOnlySourceRecordSchema])
  .superRefine((source, context) => {
    if (source.authorityClass === 'official' && !OFFICIAL_HOSTS.has(new URL(source.url).hostname)) {
      context.addIssue({
        code: 'custom',
        path: ['url'],
        message: 'official sources must use an allowlisted publisher host',
      });
    }
    if (
      source.storageMode === 'manifest-only' &&
      source.durableLocator === null &&
      source.acquisitionProcedureHash === null
    ) {
      context.addIssue({
        code: 'custom',
        path: ['durableLocator'],
        message: 'manifest-only sources require a durable locator or acquisition procedure hash',
      });
    }
  });

export const jsonValueSchema: z.ZodType<JsonValue> = z.lazy(() =>
  z.union([
    z.null(),
    z.boolean(),
    z.number(),
    z.string(),
    z.array(jsonValueSchema),
    z.record(z.string(), jsonValueSchema),
  ]),
);

function addIdentityIssues(
  identity: {
    parentRefs: readonly { stableId: string }[];
    sourceRefs: readonly { sourceId: string }[];
  },
  context: z.RefinementCtx,
): void {
  if (identity.parentRefs.length === 0 && identity.sourceRefs.length === 0) {
    context.addIssue({
      code: 'custom',
      path: ['sourceRefs'],
      message: 'identity requires at least one parent or source reference',
    });
  }

  const parentIds = new Set<string>();
  identity.parentRefs.forEach((reference, index) => {
    if (parentIds.has(reference.stableId)) {
      context.addIssue({
        code: 'custom',
        path: ['parentRefs', index, 'stableId'],
        message: 'duplicate parent reference',
        params: { diagnosticCode: 'duplicate_parent_ref' },
      });
    }
    parentIds.add(reference.stableId);
  });

  const sourceIds = new Set<string>();
  identity.sourceRefs.forEach((reference, index) => {
    if (sourceIds.has(reference.sourceId)) {
      context.addIssue({
        code: 'custom',
        path: ['sourceRefs', index, 'sourceId'],
        message: 'duplicate source reference',
        params: { diagnosticCode: 'duplicate_source_ref' },
      });
    }
    sourceIds.add(reference.sourceId);
  });
}

const identityFields = {
  artifactKind: artifactRefSchema.shape.artifactKind,
  stableId: stableIdSchema,
  schemaVersion: z.literal(1),
  parentRefs: z.array(artifactRefSchema).max(1_000),
  sourceRefs: z.array(sourceRefSchema).max(1_000),
};

export const identityDocumentSchema = z
  .strictObject({
    ...identityFields,
    payload: jsonValueSchema,
  })
  .superRefine(addIdentityIssues);

export const canonicalArtifactSchema = z.strictObject({
  identity: identityDocumentSchema,
  contentHash: hashSchema,
});

const nullableNonnegativeInteger = z.number().int().nonnegative().safe().nullable();
const cardFields = {
  name: z.string().min(1).max(300),
  cardType: z.enum(['avatar', 'site', 'minion', 'aura', 'artifact', 'magic']),
  elements: z.array(z.enum(['earth', 'fire', 'water', 'air'])).max(4),
  rarity: z.enum(['ordinary', 'exceptional', 'elite', 'unique']),
  manaCost: nullableNonnegativeInteger,
  attack: nullableNonnegativeInteger,
  defense: nullableNonnegativeInteger,
  rulesText: z.string().max(20_000),
  printingSlugs: z.array(z.string().min(1).max(200).regex(SLUG_PATTERN)).min(1).max(100),
};

function addPrintingSlugIssues(
  card: { printingSlugs: readonly string[] },
  context: z.RefinementCtx,
): void {
  const found = new Set<string>();
  card.printingSlugs.forEach((slug, index) => {
    if (found.has(slug)) {
      context.addIssue({
        code: 'custom',
        path: ['printingSlugs', index],
        message: 'duplicate printing slug',
        params: { diagnosticCode: 'duplicate_printing_slug' },
      });
    }
    found.add(slug);
  });
}

export const rawCardSchema = z
  .strictObject({
    sourceCardId: z.string().min(1).max(200),
    ...cardFields,
    releasedAt: z.iso.date().nullable(),
  })
  .superRefine(addPrintingSlugIssues);

export const normalizedCardSchema = z
  .strictObject({
    stableId: stableIdSchema,
    officialSourceId: z.string().min(1).max(200).nullable(),
    ...cardFields,
  })
  .superRefine(addPrintingSlugIssues);

export const normalizedCardSnapshotSchema = z
  .strictObject({
    cards: z.array(normalizedCardSchema).max(MAX_CANONICAL_NODES),
  })
  .superRefine((snapshot, context) => {
    const found = new Set<string>();
    snapshot.cards.forEach((card, index) => {
      if (found.has(card.stableId)) {
        context.addIssue({
          code: 'custom',
          path: ['cards', index, 'stableId'],
          message: 'duplicate card stable ID',
          params: { diagnosticCode: 'duplicate_stable_id' },
        });
      }
      found.add(card.stableId);
    });
  });

export const formatDefinitionSchema = z.strictObject({
  name: z.string().min(1).max(300),
  effectiveDate: z.iso.date(),
  scope: z.string().min(1).max(300).nullable(),
  parentFormatStableId: stableIdSchema.nullable(),
  avatarCount: z.number().int().nonnegative().safe(),
  spellbookMinimum: z.number().int().nonnegative().safe(),
  atlasMinimum: z.number().int().nonnegative().safe(),
  copyLimits: z.strictObject({
    ordinary: z.number().int().nonnegative().safe(),
    exceptional: z.number().int().nonnegative().safe(),
    elite: z.number().int().nonnegative().safe(),
    unique: z.number().int().nonnegative().safe(),
  }),
});

const formatIdentitySchema = z
  .strictObject({
    ...identityFields,
    artifactKind: z.literal('format'),
    payload: formatDefinitionSchema,
  })
  .superRefine(addIdentityIssues);

export const formatArtifactSchema = z.strictObject({
  identity: formatIdentitySchema,
  contentHash: hashSchema,
});

export const authorityBundlePayloadSchema = z
  .strictObject({
    effectiveDate: z.iso.date(),
    precedencePolicyVersion: z.literal(1),
    inputRootHash: hashSchema.nullable(),
    sources: z.array(sourceRecordSchema).max(MAX_CANONICAL_NODES),
    artifacts: z.array(canonicalArtifactSchema).max(MAX_CANONICAL_NODES),
  })
  .superRefine((bundle, context) => {
    const sourceIds = new Set<string>();
    bundle.sources.forEach((source, index) => {
      if (sourceIds.has(source.sourceId)) {
        context.addIssue({
          code: 'custom',
          path: ['sources', index, 'sourceId'],
          message: 'duplicate source ID',
          params: { diagnosticCode: 'duplicate_source_id' },
        });
      }
      sourceIds.add(source.sourceId);
    });

    const stableIds = new Set<string>();
    bundle.artifacts.forEach((artifact, index) => {
      if (stableIds.has(artifact.identity.stableId)) {
        context.addIssue({
          code: 'custom',
          path: ['artifacts', index, 'identity', 'stableId'],
          message: 'duplicate artifact stable ID',
          params: { diagnosticCode: 'duplicate_stable_id' },
        });
      }
      stableIds.add(artifact.identity.stableId);
    });
  });

const authorityBundleIdentitySchema = z
  .strictObject({
    ...identityFields,
    artifactKind: z.literal('bundle'),
    payload: authorityBundlePayloadSchema,
  })
  .superRefine(addIdentityIssues);

export const authorityBundleSchema = z.strictObject({
  identity: authorityBundleIdentitySchema,
  contentHash: hashSchema,
});

function compareText(left: string, right: string): number {
  return left < right ? -1 : left > right ? 1 : 0;
}

export function sortDiagnostics(diagnostics: readonly Diagnostic[]): readonly Diagnostic[] {
  return [...diagnostics].sort(
    (left, right) =>
      compareText(left.path, right.path) ||
      compareText(left.code, right.code) ||
      compareText(left.message, right.message),
  );
}

function jsonPointer(path: readonly PropertyKey[]): string {
  return path.reduce<string>(
    (result, segment) =>
      result + '/' + String(segment).replaceAll('~', '~0').replaceAll('/', '~1'),
    '',
  );
}

function diagnosticsFromZod(error: z.ZodError): readonly Diagnostic[] {
  const diagnostics: Diagnostic[] = [];
  for (const issue of error.issues) {
    if (issue.code === 'unrecognized_keys') {
      const keys = (issue as { keys?: readonly string[] }).keys ?? [];
      for (const key of keys) {
        diagnostics.push({
          path: jsonPointer([...issue.path, key]),
          code: 'unrecognized_key',
          message: 'Unrecognized key',
        });
      }
      continue;
    }

    const diagnosticCode =
      issue.code === 'custom'
        ? ((issue as { params?: { diagnosticCode?: string } }).params?.diagnosticCode ?? issue.code)
        : issue.code;
    diagnostics.push({
      path: jsonPointer(issue.path),
      code: diagnosticCode,
      message: issue.message,
    });
  }
  return diagnostics;
}

function raise(diagnostics: readonly Diagnostic[], maxDiagnostics = DEFAULT_AUTHORITY_JSON_LIMITS.maxDiagnostics): never {
  throw new AuthorityValidationError(sortDiagnostics(diagnostics).slice(0, maxDiagnostics));
}

function diagnosticFromCanonical(error: CanonicalJsonError): Diagnostic {
  return {
    path: error.path,
    code: error.code,
    message: error.message,
  };
}

function preflight(input: unknown, maxDiagnostics = DEFAULT_AUTHORITY_JSON_LIMITS.maxDiagnostics): void {
  try {
    canonicalJson(input as JsonValue);
  } catch (error: unknown) {
    if (error instanceof CanonicalJsonError) raise([diagnosticFromCanonical(error)], maxDiagnostics);
    throw error;
  }
}

function validateWithSchema<T>(
  input: unknown,
  schema: z.ZodType<T>,
  maxDiagnostics = DEFAULT_AUTHORITY_JSON_LIMITS.maxDiagnostics,
): T {
  preflight(input, maxDiagnostics);
  const result = schema.safeParse(input);
  if (!result.success) raise(diagnosticsFromZod(result.error), maxDiagnostics);
  return result.data;
}

function validateArtifactWithSchema<T extends { identity: unknown; contentHash: string }>(
  input: unknown,
  schema: z.ZodType<T>,
): T {
  const artifact = validateWithSchema(input, schema);
  const expected = identityHash(artifact.identity as JsonValue);
  if (artifact.contentHash !== expected) {
    raise([
      {
        path: '/contentHash',
        code: 'content_hash_mismatch',
        message: 'contentHash does not match the canonical identity document',
      },
    ]);
  }
  return artifact;
}

export function validateSourceRecord(input: unknown): SourceRecord {
  return validateWithSchema(input, sourceRecordSchema) as SourceRecord;
}

export function isNormativeSource(source: SourceRecord): boolean {
  return source.authorityClass === 'official';
}

export function validateRawCard(input: unknown): RawCard {
  return validateWithSchema(input, rawCardSchema) as RawCard;
}

export function validateNormalizedCard(input: unknown): NormalizedCard {
  return validateWithSchema(input, normalizedCardSchema) as NormalizedCard;
}

export function validateIdentityDocument(input: unknown): IdentityDocument<JsonValue> {
  return validateWithSchema(input, identityDocumentSchema) as IdentityDocument<JsonValue>;
}

export function createCanonicalArtifact<T extends JsonValue>(
  identity: IdentityDocument<T>,
): CanonicalArtifact<T> {
  const validated = validateWithSchema(identity, identityDocumentSchema) as unknown as IdentityDocument<T>;
  return Object.freeze({
    identity: validated,
    contentHash: identityHash(validated as JsonValue),
  });
}

export function validateCanonicalArtifact(input: unknown): CanonicalArtifact<JsonValue> {
  return validateArtifactWithSchema(input, canonicalArtifactSchema) as CanonicalArtifact<JsonValue>;
}

export function validateFormatArtifact(input: unknown): FormatArtifact {
  return validateArtifactWithSchema(input, formatArtifactSchema) as FormatArtifact;
}

export function validateAuthorityBundle(input: unknown): AuthorityBundle {
  return validateArtifactWithSchema(input, authorityBundleSchema) as AuthorityBundle;
}

function validateLimits(limits: AuthorityJsonLimits): void {
  const maxima: AuthorityJsonLimits = {
    maxBytes: DEFAULT_AUTHORITY_JSON_LIMITS.maxBytes,
    maxDepth: MAX_CANONICAL_DEPTH,
    maxRecords: MAX_CANONICAL_NODES,
    maxDiagnostics: DEFAULT_AUTHORITY_JSON_LIMITS.maxDiagnostics,
  };
  for (const key of ['maxBytes', 'maxDepth', 'maxRecords', 'maxDiagnostics'] as const) {
    const value = limits[key];
    if (!Number.isSafeInteger(value) || value < 1 || value > maxima[key]) {
      raise([
        {
          path: '/limits/' + key,
          code: 'invalid_limit',
          message: key + ' must be a positive integer no greater than the fixed maximum',
        },
      ]);
    }
  }
}

function measure(value: JsonValue, limits: AuthorityJsonLimits): void {
  let records = 0;

  function visit(current: JsonValue, depth: number, path: string): void {
    if (depth > limits.maxDepth) {
      raise([{ path, code: 'max_depth', message: 'JSON exceeds the configured depth limit' }], limits.maxDiagnostics);
    }
    if (current === null || typeof current !== 'object') return;

    const entries: readonly [string, JsonValue][] = Array.isArray(current)
      ? current.map((item, index) => [String(index), item] as const)
      : Object.entries(current);
    records += entries.length;
    if (records > limits.maxRecords) {
      raise([{ path, code: 'max_records', message: 'JSON exceeds the configured record limit' }], limits.maxDiagnostics);
    }
    for (const [segment, child] of entries) {
      const childPath = path + '/' + segment.replaceAll('~', '~0').replaceAll('/', '~1');
      visit(child, depth + 1, childPath);
    }
  }

  visit(value, 0, '');
}

export function parseAuthorityJson(bytes: Uint8Array, limits: AuthorityJsonLimits): JsonValue {
  validateLimits(limits);
  if (bytes.byteLength > limits.maxBytes) {
    raise([{ path: '', code: 'max_bytes', message: 'JSON exceeds the configured byte limit' }], limits.maxDiagnostics);
  }

  let text: string;
  try {
    text = new TextDecoder('utf-8', { fatal: true }).decode(bytes);
  } catch {
    raise([{ path: '', code: 'invalid_utf8', message: 'authority JSON must be valid UTF-8' }], limits.maxDiagnostics);
  }

  let parsed: JsonValue;
  try {
    parsed = parseJsonWithDuplicateKeyCheck(text);
  } catch (error: unknown) {
    if (error instanceof CanonicalJsonError) raise([diagnosticFromCanonical(error)], limits.maxDiagnostics);
    throw error;
  }
  measure(parsed, limits);

  const stringBytes = Buffer.byteLength(canonicalJson(parsed), 'utf8');
  if (stringBytes > MAX_CANONICAL_STRING_BYTES) {
    raise(
      [{ path: '', code: 'max_string_bytes', message: 'JSON strings exceed the fixed byte limit' }],
      limits.maxDiagnostics,
    );
  }
  return parsed;
}
