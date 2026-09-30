import { syncPrivateCardTracker, writeTrackerExport } from '../catalog/card-tracker.ts';

const mode = process.argv[2];

if (mode === 'sync') {
  const result = await syncPrivateCardTracker(process.cwd());
  console.log(JSON.stringify({
    cards: result.cardCount,
    codexEntries: result.codexEntryCount,
    groups: result.groupCount,
    requirements: result.requirementCount,
    sourceReviewCandidates: result.sourceReviewCandidateCount,
    sourceReviewed: result.sourceReviewedCount,
    currentlyBound: result.boundCount,
    databasePath: result.databasePath,
    authorityHash: result.authorityHash,
    revisionId: result.revisionId,
    feedHash: result.feedHash,
    engineHash: result.engineHash,
  }));
} else if (mode === 'export-json' || mode === 'export-csv') {
  const path = await writeTrackerExport(process.cwd(), mode === 'export-json' ? 'json' : 'csv');
  console.log(path);
} else {
  throw new Error('usage: cards:tracker-sync | cards:tracker-export-json | cards:tracker-export-csv');
}
