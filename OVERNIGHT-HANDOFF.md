# Overnight handoff — Phase 3 / Rust cutover

Branch: `cursor/phase3-drown-bury-artifacts-36d3` (nothing pushed).
Catalog status at handoff: **127 of 161 cataloged scenarios have direct Rust proofs** (34 remain).

## Commits landed this session

Newest first; every one was committed only after `cargo fmt --all -- --check`, `cargo clippy --workspace
--all-targets --all-features --locked -- -D warnings`, `cargo test --workspace --all-features --locked`,
and `pnpm verify` were green.

| Commit | Subject |
| --- | --- |
| `58bf8ca` | `test(engine): prove Magic targets stay in region and skip enemy Stealth` (RULE-CATALOG-0023) |
| `a587166` | `test(engine): prove aura-loss Deathrite ends the game after its Magic resolves` (RULE-CATALOG-0036) |
| `cf1d92d` | `test(engine): prove Blink owes its draw until ordered Deathrites resolve` (RULE-CATALOG-0040 deferred branch) |
| `e7ffa0d` | `feat(engine): support Blink teleport-then-private-draw in the Rust engine` (RULE-CATALOG-0040) |
| `282735c` | `feat(engine): support Genesis immobilize-nearby areas in the Rust engine` (RULE-CATALOG-0063) |
| `962161b` | `feat(engine): support Drown forced submersion in the Rust engine` (RULE-CATALOG-0045) |
| `cd01e5c` | `feat(engine): support discard-funded random damage in the Rust engine` |
| `63e2f0d` | `feat(engine): support the Tower archetype in the Rust engine` |
| `5e1b002` | `feat(engine): support conditional end-turn Stealth` |

Earlier in the same overnight run: Leap Attack cutover plus typecheck fix, the Pudge/Teleport/Lure parity
harness scaffolds, the catalog reconcile pass, ShootDragProjectile, and Teleport + Lure. The queue items
P0-1 through P1-C from `OVERNIGHT-QUEUE.md` are all done.

## Verification state at handoff

- Last full green sweep was at commit `58bf8ca`: Rust fmt/clippy/test all clean, `pnpm verify` 403/403 tests pass.
- On the dirty tree at handoff: `cargo check --workspace --all-targets --all-features --locked` and
  `cargo test --workspace --all-features --locked` both pass, and `pnpm game:demo` finishes a 33-turn
  game with `replayVerified: true`. `cargo fmt --all -- --check` fails on one uncommitted Craterize
  hunk in `game.rs` around the `discard_site_instance_ids` binding — that belongs to the other agent,
  so `cargo fmt --all` before their next commit will settle it.
- The working tree is **currently dirty with another agent's in-progress work** (see below), so re-run the
  gates before trusting the tree.

## Important: a sibling agent is editing `crates/sorcery-engine/src/game.rs`

While I was starting the Artifact cluster, another agent began implementing Craterize
(RULE-CATALOG-0160) inside the same files. The uncommitted diff at handoff is theirs, not mine:

- `crates/sorcery-engine/src/action.rs` — new optional `discard_site_instance_id` on `CastMagic`,
  with canonical comparison and label support.
- `crates/sorcery-engine/src/policy.rs` — test fixtures updated for the new field.
- `crates/sorcery-engine/src/game.rs` — ~320 lines, including an extracted
  `lower_region_minion_deaths` helper.

I backed my own Artifact scaffolding out of `game.rs` so their work sits alone and the workspace
compiles (`cargo check --workspace --all-targets --all-features --locked` is clean as of handoff).
**Do not `git checkout` `game.rs`** without checking whether that agent has finished; you would
discard their Craterize work.

## Remaining 34 rules, grouped by the work they actually need

### Artifact cluster — 12 rules, the largest single prize
0043, 0051, 0140, 0141, 0142, 0143, 0144, 0145, 0155, 0156, 0157, 0161, plus 0028 and 0076 which
depend on Artifacts existing.

The Rust engine currently fails closed on Artifacts: `unsupported_selfplay_fact` returns
`cardType:artifact` and there is no realm Artifact storage at all. `parse_artifact` in `facts.rs`
already parses every Artifact fact, so only the engine half is missing. I mapped the TypeScript
semantics before backing off; recommended first slice proves RULE-CATALOG-0140 and 0141 together
because they share one code path:

1. Add `Position.artifacts: Vec<ArtifactPosition>` where placement is either
   `Carried { bearer: UnitTarget, bearer_cell: Option<Cell> }` or `Loose(Location)`.
2. Add `last_picked_up_artifacts_turn` and `last_dropped_artifacts_turn` to both `UnitPosition`
   and `AvatarPosition`.
3. New descriptors `CastArtifact`, `PickUpArtifacts`, `DropArtifacts`. TypeScript reference:
   `artifactDescriptors`, `pickUpArtifactDescriptors`, `dropArtifactDescriptors` in
   `src/engine/game.ts` (around lines 1335, 1493, 1520) and their apply arms near lines 9345,
   9425, and 9482. Note the asymmetry in the legality gates — Pick Up is blocked only by
   `lastPickedUpArtifactsTurn`, while Drop is blocked by `lastDroppedArtifactsTurn` *and*
   `lastInteractedTurn`. Both enumerate every nonempty subset of the local Artifact identities.
4. Feed a bearer bonus into `minion_current_stats` and the Avatar equivalent. In TypeScript,
   `bearerPowerBonus` adds to attack *and* defense, which is why dropping a power Artifact can
   kill a lethally wounded bearer (RULE-CATALOG-0141) — route Drop through the existing death
   settlement so that falls out for free.
5. Serialize `realm.artifacts` only when nonempty, mirroring how `realm.immobileAreas` is emitted
   in `authoritative_state`.
6. Keep failing closed on the Artifact effects you have not implemented yet; only stop returning
   `cardType:artifact` once the exercised effect is genuinely supported.

Checkpoints need no schema work: `checkpoint.rs` replays from the manifest plus action identities.

### Region cluster — 11 rules
0049, 0050, 0113, 0114, 0115, 0116, 0117, 0118, 0119, 0120, 0121.

`ActionDescriptor::SummonMinion` has no `region` field, so underwater and underground summons need a
descriptor extension before 0113 and 0114 can land; 0049 (Waterbound) depends on underwater summons
even though `minion_is_disabled` already derives Waterbound's Disabled state from terrain. The
`burrowing`, `submerge`, `voidwalk`, and `waterbound` facts are still on the
`unsupported_selfplay_minion` list — clear each one only when its rules are actually supported.

### Aura and footprint cluster — 4 rules
0064, 0065, 0066, 0101. Rust already has `SquareArea` footprints and `footprint_rules.rs`, so 0066
may be closest to a proof-only reconcile.

### One-offs
- 0037 (aura-loss deaths cannot restore stale combat during a defender path). Every piece of
  machinery exists in Rust — stepped defender paths, Deathrite ordering, `defender-joined`. This is
  proof-only work; mirror `RULE-04 aura-loss deaths cannot restore stale combat during a defender
  path` in `tests/engine/game-setup.test.ts` (around line 6082). Expect a long scenario: a
  `movementBonus: 2` aura source walking a four-cell defender path while two wounded allies die
  mid-path and owe a Deathrite order, then asserting `defender-joined` lands after the last
  `minion-died`.
- 0070, 0158, 0159, 0160. 0160 is the slice the other agent is mid-way through.

## Exact next step

1. Run `git status`. If `game.rs`, `action.rs`, and `policy.rs` are still dirty, either wait for the
   Craterize agent to commit or finish and verify that slice yourself before starting anything else.
2. Once the tree is clean, take the Artifact foundation slice described above; it unlocks the most
   catalog rules per unit of work.
3. If you want a quick win first, RULE-CATALOG-0037 needs no engine change at all — only the
   scenario proof.
