import assert from 'node:assert/strict';
import test from 'node:test';

import { canonicalJson, type JsonValue } from '../../src/authority/canonical-json.ts';
import {
  buildMechanicWorkload,
  type MechanicWorkloadCardInput,
  type PresetCardDemand,
} from '../../src/mechanics/mechanic-workload.ts';

const BLANK: MechanicWorkloadCardInput = {
  cardType: 'minion',
  name: 'Synthetic Blank',
  rulesText: '',
  stableId: 'card:synthetic-blank',
};

const SENTINEL = 'SYNTHETIC-SENTINEL-PROSE must never survive report serialization';
const MULTILABEL: MechanicWorkloadCardInput = {
  cardType: 'magic',
  name: 'Synthetic Many Effects',
  rulesText: `${SENTINEL}. Genesis: pay mana to summon a token at a nearby site, then choose a random target and deal damage.`,
  stableId: 'card:synthetic-multilabel',
};

const UNCLASSIFIED: MechanicWorkloadCardInput = {
  cardType: 'artifact',
  name: 'Synthetic Unknown',
  rulesText: 'Zorbles become quuxed.',
  stableId: 'card:synthetic-unclassified',
};

const FACET_ONLY: MechanicWorkloadCardInput = {
  cardType: 'artifact',
  name: 'Synthetic Facet Only',
  rulesText: 'Whenever zorbles become quuxed.',
  stableId: 'card:synthetic-facet-only',
};

const INFLECTIONS: MechanicWorkloadCardInput = {
  cardType: 'minion',
  name: 'Synthetic Inflections',
  rulesText: 'Provides mana at sites, strikes projectiles, and becomes disabled after being moved between locations.',
  stableId: 'card:synthetic-inflections',
};

const KEYWORD_ONLY: MechanicWorkloadCardInput = {
  cardType: 'minion',
  name: 'Synthetic Keyword Only',
  rulesText: 'Waterbound',
  stableId: 'card:synthetic-keyword-only',
};

const DEMAND: readonly PresetCardDemand[] = [{
  northCopies: 2,
  presetId: 'synthetic-preset',
  southCopies: 1,
  stableId: MULTILABEL.stableId,
}];

test('mechanic workload includes every card without prose and is byte-identical under shuffled input', () => {
  const cards = [BLANK, FACET_ONLY, INFLECTIONS, KEYWORD_ONLY, MULTILABEL, UNCLASSIFIED];
  const report = buildMechanicWorkload(cards, DEMAND);

  assert.equal(report.totals.cards, cards.length);
  assert.equal(report.totals.blank, 1);
  assert.equal(report.totals.classified, 3);
  assert.equal(report.totals.unclassified, 2);
  assert.equal(report.totals.presetDemandPresent, 1);
  assert.equal(report.totals.presetDemandAbsent, 5);
  assert.equal(
    report.scopeDisclaimer,
    'Lexical workload labels and preset demand only; this report proves neither engine implementation nor scenario verification.',
  );
  assert.deepEqual(report.cards.map(({ stableId }) => stableId), [
    BLANK.stableId,
    FACET_ONLY.stableId,
    INFLECTIONS.stableId,
    KEYWORD_ONLY.stableId,
    MULTILABEL.stableId,
    UNCLASSIFIED.stableId,
  ]);

  const blank = report.cards.find(({ stableId }) => stableId === BLANK.stableId);
  assert.ok(blank);
  assert.equal(blank.classification, 'blank');
  assert.deepEqual(blank.familyIds, []);
  assert.deepEqual(blank.facetIds, []);

  const facetOnly = report.cards.find(({ stableId }) => stableId === FACET_ONLY.stableId);
  assert.ok(facetOnly);
  assert.equal(facetOnly.classification, 'unclassified');
  assert.deepEqual(facetOnly.familyIds, []);
  assert.deepEqual(facetOnly.facetIds, ['trigger-timing']);

  const inflections = report.cards.find(({ stableId }) => stableId === INFLECTIONS.stableId);
  assert.ok(inflections);
  assert.equal(inflections.classification, 'classified');
  assert.deepEqual(inflections.familyIds, [
    'combat-projectiles',
    'movement-regions',
    'resource-casting',
    'site-terrain-domain',
    'status-ability-modifiers',
  ]);
  assert.deepEqual(inflections.facetIds, ['spatial-scope', 'trigger-timing']);

  const keywordOnly = report.cards.find(({ stableId }) => stableId === KEYWORD_ONLY.stableId);
  assert.ok(keywordOnly);
  assert.equal(keywordOnly.classification, 'classified');
  assert.deepEqual(keywordOnly.familyIds, ['movement-regions']);
  assert.deepEqual(keywordOnly.facetIds, []);

  const multilabel = report.cards.find(({ stableId }) => stableId === MULTILABEL.stableId);
  assert.ok(multilabel);
  assert.equal(multilabel.classification, 'classified');
  assert.equal(multilabel.presetDemandStatus, 'present');
  assert.deepEqual(multilabel.presetDemand, [{
    northCopies: 2,
    presetId: 'synthetic-preset',
    southCopies: 1,
    totalCopies: 3,
  }]);
  assert.deepEqual(multilabel.familyIds, [
    'artifact-token-carry',
    'damage-life-death',
    'resource-casting',
    'site-terrain-domain',
    'unit-placement-summoning',
  ]);
  assert.deepEqual(multilabel.facetIds, [
    'randomness',
    'spatial-scope',
    'target-choice',
    'trigger-timing',
  ]);

  const unclassified = report.cards.find(({ stableId }) => stableId === UNCLASSIFIED.stableId);
  assert.ok(unclassified);
  assert.equal(unclassified.classification, 'unclassified');
  assert.equal(unclassified.presetDemandStatus, 'absent');
  assert.deepEqual(unclassified.familyIds, []);
  assert.deepEqual(unclassified.facetIds, []);

  const serialized = canonicalJson(report as unknown as JsonValue);
  const shuffled = canonicalJson(buildMechanicWorkload(
    [...cards].reverse(),
    [...DEMAND].reverse(),
  ) as unknown as JsonValue);
  assert.equal(shuffled, serialized);
  assert.equal(serialized.includes(SENTINEL), false);
  assert.equal(serialized.includes('rulesText'), false);
});
