import assert from 'node:assert/strict';
import test from 'node:test';
import { DatabaseSync } from 'node:sqlite';

import { parseJsonWithDuplicateKeyCheck } from '../../src/authority/canonical-json.ts';
import { assertExactSourceHash, CARD_TRACKER_SCHEMA } from '../../src/catalog/card-tracker.ts';

test('large-feed parser keeps duplicate, depth, node, and byte guards', () => {
  assert.throws(() => parseJsonWithDuplicateKeyCheck('{"a":1,"a":2}', { validateCanonical: false }), /duplicate_key/u);
  assert.throws(() => parseJsonWithDuplicateKeyCheck(`${'['.repeat(66)}0${']'.repeat(66)}`, { validateCanonical: false }), /max_depth/u);
  assert.throws(() => parseJsonWithDuplicateKeyCheck('"too-long"', { validateCanonical: false, maxStringBytes: 3 }), /max_string_bytes/u);
  assert.throws(() => parseJsonWithDuplicateKeyCheck('[1,2]', { validateCanonical: false, maxNodes: 2 }), /max_nodes/u);
});

test('tracker source hashes require exact valid identities', () => {
  const hashA = `sha256:${'a'.repeat(64)}`;
  const hashB = `sha256:${'b'.repeat(64)}`;
  assert.doesNotThrow(() => assertExactSourceHash(hashA, hashA, 'synthetic source'));
  assert.throws(() => assertExactSourceHash(hashA, hashB, 'synthetic source'), /source hash mismatch/u);
  assert.throws(() => assertExactSourceHash('not-a-hash', hashB, 'synthetic source'), /SHA-256/u);
});

test('tracker schema preserves null frequencies and manual decisions while constraining exact joins', () => {
  const db = new DatabaseSync(':memory:', { enableForeignKeyConstraints: true });
  try {
    db.exec(CARD_TRACKER_SCHEMA);
    db.prepare('INSERT INTO capability_groups VALUES (?,?,?,?,?,?,?,?,1)')
      .run('group-a', 'Synthetic group', 1, 'synthetic', 1, 0, '{}', 'hash');
    db.prepare('INSERT INTO cards VALUES (?,?,?,?,?,?,?,?,?,?,?,?,?,1)')
      .run('card-a', null, 'Synthetic card', 'Unit', '[]', '{}', 'synthetic text', 'source', 'authority', 'revision', 'unbound', null, null);
    db.prepare('INSERT INTO codex_entries VALUES (?,?,?,?,?,?,?,?,1)')
      .run('entry-a', 'Synthetic entry', 'https://example.invalid', null, '[]', '', 'entry-hash', 'snapshot-hash');
    db.prepare('INSERT INTO rule_slices VALUES (?,?,?,?,?,?,?,?,1)')
      .run('slice-a', 'requirement-a', 'card-a', 'group-a', 'unverified-contextual-requirement', 'family-a', '{}', 'slice-hash');
    db.prepare('INSERT INTO card_requirements VALUES (?,?,?,1)')
      .run('card-a', 'slice-a', 'source');
    db.prepare('INSERT INTO codex_refs VALUES (?,?,?,?,?)')
      .run('slice', 'slice-a', 'entry-a', 'context-only', 'shared-family-context');
    db.prepare('INSERT INTO card_status VALUES (?,?,?,?,?,?,?,?,?,?,?)')
      .run('card-a', 'remaining', 'pending', 'blocked', 0, 'source-review-required', null, '[]', '[]', null, '{}');
    const addManual = db.prepare('INSERT INTO manual_card_status(card_id) VALUES (?) ON CONFLICT(card_id) DO NOTHING');
    addManual.run('card-a');
    db.prepare("UPDATE manual_card_status SET status='reviewed',note='synthetic decision' WHERE card_id='card-a'").run();
    addManual.run('card-a');

    assert.equal(db.prepare('SELECT count(*) AS n FROM cards WHERE is_current=1').get()!.n, 1);
    assert.equal(db.prepare('SELECT selected_deck_frequency AS n FROM card_status WHERE card_id=?').get('card-a')!.n, null);
    const manual = db.prepare('SELECT status,note FROM manual_card_status WHERE card_id=?').get('card-a')!;
    assert.equal(manual.status, 'reviewed');
    assert.equal(manual.note, 'synthetic decision');
    assert.throws(() => db.prepare('INSERT INTO codex_refs VALUES (?,?,?,?,?)')
      .run('slice', 'slice-a', 'missing-entry', 'context-only', 'shared-family-context'), /FOREIGN KEY/u);
    assert.equal(db.prepare('PRAGMA foreign_key_check').all().length, 0);
  } finally {
    db.close();
  }
});
