import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import test from 'node:test';

type Catalog = Readonly<{
  rules: readonly Readonly<{
    implementationStatus: 'rust-supported' | 'typescript-supported';
    plainLanguage: string;
    ruleId: string;
    scenarioProof: Readonly<{ file: string; testName: string }>;
  }>[];
  schemaVersion: number;
}>;

const catalogUrl = new URL('../../data/rules/catalog.json', import.meta.url);

test('the public rules catalog stays linked to direct scenario proofs', () => {
  const catalog = JSON.parse(readFileSync(catalogUrl, 'utf8')) as Catalog;
  assert.equal(catalog.schemaVersion, 1);
  assert.equal(catalog.rules.length > 0, true);
  assert.equal(new Set(catalog.rules.map(({ ruleId }) => ruleId)).size, catalog.rules.length);

  for (const rule of catalog.rules) {
    assert.match(rule.ruleId, /^RULE-CATALOG-\d{4}$/u);
    assert.equal(rule.plainLanguage.trim().length > 0, true);
    assert.match(
      rule.scenarioProof.file,
      rule.implementationStatus === 'rust-supported'
        ? /^crates\/sorcery-engine\/tests\/[\w-]+\.rs$/u
        : /^tests\/engine\/[\w-]+\.test\.ts$/u,
    );
    const proofUrl = new URL(`../../${rule.scenarioProof.file}`, import.meta.url);
    const proof = readFileSync(proofUrl, 'utf8').replaceAll("\\'", "'");
    assert.equal(proof.includes(rule.scenarioProof.testName), true);
  }
});
