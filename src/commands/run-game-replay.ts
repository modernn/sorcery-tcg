import { runRustEngineCommand } from '../engine/rust-engine.ts';

function isRecord(value: unknown): value is Record<string, unknown> {
  return value !== null && typeof value === 'object' && !Array.isArray(value);
}

export type ReplayMismatch =
  | 'authority'
  | 'engine-version'
  | 'events-hash'
  | 'final-state-hash'
  | 'manifest-identity'
  | 'replay-rejected'
  | 'schema-version'
  | 'transcript-hash';

export type ArtifactReplayReport = Readonly<{
  classification: 'unranked_partial_rules_unverified_authority';
  matched: boolean;
  mismatch?: ReplayMismatch;
  replayVerified: boolean;
  schemaVersion: 1;
}>;

const MISMATCHES = new Set<ReplayMismatch>([
  'authority',
  'engine-version',
  'events-hash',
  'final-state-hash',
  'manifest-identity',
  'replay-rejected',
  'schema-version',
  'transcript-hash',
]);

function parseReplay(value: unknown): ArtifactReplayReport {
  if (!isRecord(value)
    || value.classification !== 'unranked_partial_rules_unverified_authority'
    || value.schemaVersion !== 1
    || (value.matched !== true && value.matched !== false)
    || (value.replayVerified !== true && value.replayVerified !== false)) {
    throw new Error('Rust artifact replay report did not match the expected contract');
  }
  if (value.matched) {
    if (value.mismatch !== undefined || value.replayVerified !== true) {
      throw new Error('matched artifact replay must be replay-verified and have no mismatch');
    }
    return Object.freeze({
      classification: 'unranked_partial_rules_unverified_authority',
      matched: true,
      replayVerified: true,
      schemaVersion: 1,
    });
  }
  if (typeof value.mismatch !== 'string' || !MISMATCHES.has(value.mismatch as ReplayMismatch)
    || value.replayVerified !== false) {
    throw new Error('mismatched artifact replay must name a classified mismatch');
  }
  return Object.freeze({
    classification: 'unranked_partial_rules_unverified_authority',
    matched: false,
    mismatch: value.mismatch as ReplayMismatch,
    replayVerified: false,
    schemaVersion: 1,
  });
}

/** Replays one SIM-03 artifact directory and classifies hash or version mismatches. */
export function replayGameArtifacts(artifactsDir: string): ArtifactReplayReport {
  if (artifactsDir.trim() === '') {
    throw new RangeError('artifacts directory is empty');
  }
  return parseReplay(runRustEngineCommand(['replay', artifactsDir]));
}
