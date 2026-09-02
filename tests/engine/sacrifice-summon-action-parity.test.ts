import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import test from 'node:test';

import {
  serializeSacrificeSummonActionParityFixture,
} from '../../scripts/capture-sacrifice-summon-action-parity.ts';
import {
  opaqueActionId,
  orderLegalActions,
  type EngineActionDescriptor,
} from '../../src/engine/contract.ts';

type FixtureAction = Readonly<{
  actionId: string;
  descriptor: EngineActionDescriptor;
  label: string;
  seat?: 'north';
  stateVersion?: number;
}>;

type FixtureReceipt = Readonly<{
  actionId: string;
  events: readonly Readonly<{ eventId: string; payload: unknown; type: string }>[];
  postStateHash: string;
  randomDraws: readonly unknown[];
  receiptId: string;
}>;

type Fixture = Readonly<{
  actions: readonly FixtureAction[];
  canonicalActionIds: readonly string[];
  contract: string;
  deathrite: Readonly<{
    manifestId: string;
    paymentAction: FixtureAction;
    pending: Readonly<{
      checkpointId: string;
      checkpointRoundTrip: boolean;
      decisionSeat: string;
      expectedSessionHash: string;
      orderActions: readonly FixtureAction[];
      phase: string;
      receipt: FixtureReceipt;
      serializedCheckpointHash: string;
      stateHash: string;
      stateVersion: number;
    }>;
    resolved: Readonly<{
      checkpointId: string;
      decisionSeat: string;
      expectedSessionHash: string;
      phase: string;
      receipt: FixtureReceipt;
      replayVerified: boolean;
      selectedOrderActionId: string;
      serializedCheckpointHash: string;
      stateHash: string;
      stateVersion: number;
    }>;
  }>;
  eligibleSacrificeCandidateIds: readonly string[];
  schemaVersion: number;
  seat: 'north';
  source: string;
  stateVersion: number;
  transition: Readonly<{
    receipt: FixtureReceipt;
    selectedActionId: string;
  }>;
}>;

const fixtureUrl = new URL('./fixtures/sacrifice-summon-action-v1.json', import.meta.url);
const fixture = JSON.parse(readFileSync(fixtureUrl, 'utf8')) as Fixture;

test('sacrifice-summon fixture regenerates byte-identically from TypeScript legality', () => {
  assert.equal(serializeSacrificeSummonActionParityFixture(), readFileSync(fixtureUrl, 'utf8'));
  assert.equal(fixture.schemaVersion, 1);
  assert.equal(fixture.source, 'typescript-legality-engine');
  assert.deepEqual(fixture.eligibleSacrificeCandidateIds, [
    'sha256:6ba77ca7089f0021f72e8a10e69836c169b49d2816d0b7cc4bcd063a167a59a0',
    'sha256:6d54db9a8afec1f6eb3c1dc1a184b0b058485f4b927e61480b0c09207a3ab1b9',
    'sha256:f945832fc1de3d436ea4e0435e8fd050142bed6baa0746f90e3181d044b686d0',
  ]);
  assert.deepEqual(
    fixture.actions.map(({ descriptor }) => descriptor.kind),
    Array(8).fill('summon-minion'),
  );
  assert.deepEqual(
    fixture.actions.map(({ descriptor }) => descriptor.kind === 'summon-minion'
      ? descriptor.sacrificedMinionInstanceIds ?? []
      : undefined),
    [
      fixture.eligibleSacrificeCandidateIds,
      fixture.eligibleSacrificeCandidateIds.slice(0, 2),
      [fixture.eligibleSacrificeCandidateIds[0], fixture.eligibleSacrificeCandidateIds[2]],
      fixture.eligibleSacrificeCandidateIds.slice(1),
      [fixture.eligibleSacrificeCandidateIds[0]],
      [fixture.eligibleSacrificeCandidateIds[1]],
      [fixture.eligibleSacrificeCandidateIds[2]],
      [],
    ],
  );
  assert.deepEqual(
    fixture.actions.map(({ descriptor }) => descriptor.kind === 'summon-minion'
      ? descriptor.manaCost
      : -1),
    [0, 2, 2, 2, 4, 4, 4, 6],
  );
  assert.deepEqual(fixture.actions.map(({ actionId }) => actionId), [
    'sha256:1278a34a14cafa4e2c9bacf1eb620fedeb291b149c9cd1a72448afe9d877f86b',
    'sha256:ac2769f4e82f351fe8c64b95c53eecffd252e38364964f3a75a67f0aa3aa8b2d',
    'sha256:ff3009104639d2b0493669a16c5276cd28d7dc46a75cc9beefb3d2dd3b4b5d16',
    'sha256:22dced7c08317dce29f7b514b79a83cb4e2d00dec0081b846195d575c806743f',
    'sha256:ce30e85718f534960e962e6bfd7245f672316a168a988170a841d71c93ad9f72',
    'sha256:3d28d6c78087b3fba27f5f74f51fcd4fd1e5eb2ba5721c18b31e01a21c053426',
    'sha256:1dbca6c8d972d299421d4696fce333e728f36b3fe656ac1a56bd8186aaac6d27',
    'sha256:0e881775c6062ad2345f83391d158cd307a59cdbea85868e05a4a6f2e28cca7d',
  ]);
  assert.deepEqual(fixture.actions.map(({ label }) => label), [
    'Summon synthetic-tithe-beast at C4 (0 mana + sacrifice 3 minions)',
    'Summon synthetic-tithe-beast at C4 (2 mana + sacrifice 2 minions)',
    'Summon synthetic-tithe-beast at C4 (2 mana + sacrifice 2 minions)',
    'Summon synthetic-tithe-beast at C4 (2 mana + sacrifice 2 minions)',
    'Summon synthetic-tithe-beast at C4 (4 mana + sacrifice 1 minion)',
    'Summon synthetic-tithe-beast at C4 (4 mana + sacrifice 1 minion)',
    'Summon synthetic-tithe-beast at C4 (4 mana + sacrifice 1 minion)',
    'Summon synthetic-tithe-beast at C4 (6 mana)',
  ]);
  for (const action of fixture.actions) {
    assert.equal(
      opaqueActionId(fixture.contract, fixture.seat, fixture.stateVersion, action.descriptor),
      action.actionId,
    );
  }
  const ordered = orderLegalActions(fixture.actions.map((action) => ({
    ...action,
    seat: fixture.seat,
    stateVersion: fixture.stateVersion,
  })));
  assert.deepEqual(ordered.map(({ actionId }) => actionId), fixture.canonicalActionIds);
  assert.equal(fixture.transition.selectedActionId, fixture.actions[0]?.actionId);
  assert.equal(fixture.transition.receipt.actionId, fixture.transition.selectedActionId);
  assert.deepEqual(
    fixture.transition.receipt.events.map(({ type }) => type),
    [
      'minion-sacrificed',
      'minion-sacrificed',
      'minion-sacrificed',
      'minion-died',
      'minion-died',
      'minion-died',
      'minion-summoned',
    ],
  );
  assert.deepEqual(fixture.transition.receipt.randomDraws, []);
  assert.equal(
    fixture.transition.receipt.postStateHash,
    'sha256:be3c563da371d2359fa4185a89160575c88817ad1cd0a93df3fc4fbca3c669f4',
  );
  assert.equal(
    fixture.transition.receipt.receiptId,
    'sha256:b9975441ec5806d8245a1ace8d9219d9ad66f08cf295e45a7c37fbd9ed7dc2f8',
  );

  assert.equal(
    fixture.deathrite.manifestId,
    'sha256:1198723f35681225d227323e60fa33decbd2282938ec75a517a3c57bff01029e',
  );
  assert.equal(
    fixture.deathrite.paymentAction.actionId,
    'sha256:aaed65f351f69e074b92f440af1d73c7f5702c930dcc3e7a3d382787d69655ee',
  );
  assert.equal(fixture.deathrite.paymentAction.label,
    'Summon synthetic-tithe-beast at C4 (2 mana + sacrifice 2 minions)');
  assert.deepEqual(
    fixture.deathrite.paymentAction.descriptor.kind === 'summon-minion'
      ? fixture.deathrite.paymentAction.descriptor.sacrificedMinionInstanceIds
      : undefined,
    [
      'sha256:1f833cdf1a322b5e9900977fed2896159ab8397488447b0c1044b915253d2f2d',
      'sha256:7e59c6b4df41bb2084481b3790eabfdf5640ed893f25d56174ce8d84cdd07090',
    ],
  );
  assert.deepEqual(
    fixture.deathrite.pending.receipt.events.map(({ type }) => type),
    ['minion-sacrificed', 'minion-sacrificed'],
  );
  assert.deepEqual(fixture.deathrite.pending.receipt.randomDraws, []);
  assert.equal(fixture.deathrite.pending.phase, 'deathrite-order');
  assert.equal(fixture.deathrite.pending.decisionSeat, 'north');
  assert.equal(fixture.deathrite.pending.stateVersion, 13);
  assert.equal(
    fixture.deathrite.pending.stateHash,
    'sha256:1a31c6f450858ded6c7b35eb79efe07e01c9ef6a893cdb542fab653635efa604',
  );
  assert.equal(
    fixture.deathrite.pending.checkpointId,
    'sha256:0910994adb36e09ece47dfcd14719463afcd1b815ff6b0c938d36579d61a33f8',
  );
  assert.equal(
    fixture.deathrite.pending.expectedSessionHash,
    'sha256:a8c3365ee85beb3c9ea8f4687d7a673f2dbe039f85bd59f0ba2bc2022c418d05',
  );
  assert.equal(fixture.deathrite.pending.checkpointRoundTrip, true);
  assert.equal(
    fixture.deathrite.pending.serializedCheckpointHash,
    'sha256:bbb246138e7ec589c9cb72a747f3a165352754eb7b795673fc377c7c3af68150',
  );
  assert.deepEqual(
    fixture.deathrite.pending.orderActions.map(({ actionId, descriptor, label }) => ({
      actionId,
      label,
      sourceInstanceId: descriptor.kind === 'order-deathrites'
        ? descriptor.sourceInstanceId
        : undefined,
    })),
    [
      {
        actionId: 'sha256:a5f2d154d4f3f48f15cf1eee0e0ba57e4641a542520ba5aa7b99a0db088cc16c',
        label: 'Order synthetic-fodder-minion first within your Deathrites',
        sourceInstanceId: 'sha256:1f833cdf1a322b5e9900977fed2896159ab8397488447b0c1044b915253d2f2d',
      },
      {
        actionId: 'sha256:d77eb088e2e59baed21adc13c80c52a4ad1c7fff86691de565cff42d620b9736',
        label: 'Order synthetic-fodder-minion first within your Deathrites',
        sourceInstanceId: 'sha256:7e59c6b4df41bb2084481b3790eabfdf5640ed893f25d56174ce8d84cdd07090',
      },
    ],
  );
  assert.equal(
    fixture.deathrite.resolved.selectedOrderActionId,
    fixture.deathrite.pending.orderActions[0]?.actionId,
  );
  assert.deepEqual(
    fixture.deathrite.resolved.receipt.events.map(({ type }) => type),
    [
      'deathrite-order-committed',
      'site-drawn',
      'site-drawn',
      'minion-died',
      'minion-died',
      'minion-summoned',
    ],
  );
  assert.deepEqual(fixture.deathrite.resolved.receipt.randomDraws, []);
  assert.equal(fixture.deathrite.resolved.phase, 'main');
  assert.equal(fixture.deathrite.resolved.decisionSeat, 'north');
  assert.equal(fixture.deathrite.resolved.stateVersion, 14);
  assert.equal(
    fixture.deathrite.resolved.stateHash,
    'sha256:db957786f73adf17ceab12ab6f7b5e7754a60d19d411a1dc1cd88e05165cb31a',
  );
  assert.equal(
    fixture.deathrite.resolved.expectedSessionHash,
    'sha256:0f3b2449c39f174c00695582ef6098e89c29d03d13c2226d5e7e84db817fc8b2',
  );
  assert.equal(
    fixture.deathrite.resolved.serializedCheckpointHash,
    'sha256:ae785b73f062821904129270eec29e18458b6f5997d4086b02e50bf8ee050df7',
  );
  assert.equal(fixture.deathrite.resolved.replayVerified, true);
});
