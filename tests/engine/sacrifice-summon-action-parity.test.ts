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

test('sacrifice-summon fixture regenerates byte-identically from Rust legality', async () => {
  assert.equal(await serializeSacrificeSummonActionParityFixture(), readFileSync(fixtureUrl, 'utf8'));
  assert.equal(fixture.schemaVersion, 1);
  assert.equal(fixture.source, 'rust-legality-engine');
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
    'sha256:1a751baae437b35e13467fea9167bed26533b1b5f55dc333036a890c97c0f03d',
  );
  assert.equal(
    fixture.transition.receipt.receiptId,
    'sha256:d2fd020796391b1aa2f8c44232fc7e368a368bba769ad25ff82b35367ba7fbb9',
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
    'sha256:e19f242cb5d0afd939fea8b31fc5605b7ebf54139607b46932e0a9123284bd95',
  );
  assert.equal(
    fixture.deathrite.pending.checkpointId,
    'sha256:d4b17649e7aebe57b097e41587bbda0b8ea19615360000c71acf38e1bd338c29',
  );
  assert.equal(
    fixture.deathrite.pending.expectedSessionHash,
    'sha256:0949c1d532d5130a6840c210a6973b441dc8a957bcaebc8bfe783248266b3ceb',
  );
  assert.equal(fixture.deathrite.pending.checkpointRoundTrip, true);
  assert.equal(
    fixture.deathrite.pending.serializedCheckpointHash,
    'sha256:cf6ca9b67adb3708cd34c704c6912979e1e0f0835b44653f99d53d611cb2d3bb',
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
    'sha256:401b1b56ebefddfc906255011d93f0299d5070ab5f95bc728e12e6e63a7ae456',
  );
  assert.equal(
    fixture.deathrite.resolved.expectedSessionHash,
    'sha256:542989126a2a1396337414368b83825f597b6e3c7eb71bb36d1aea1e9575b988',
  );
  assert.equal(
    fixture.deathrite.resolved.serializedCheckpointHash,
    'sha256:d955347329d35c517b0e8954a33c1a5d96ddd307b9aad5e0d4b44d1d0539f4b0',
  );
  assert.equal(fixture.deathrite.resolved.replayVerified, true);
});
