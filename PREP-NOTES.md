# PREP-NOTES — P1-B / P1-C parity harness prep

Prepared by parallel prep agent. **Not committed** because the Opus shipper has staged WIP on `game.rs`, `catalog.json`, and related Leap Attack files.

## Added files

| Path | Purpose |
|------|---------|
| `scripts/capture-drag-projectile-action-parity.ts` | Regenerates Pudge drag fixture (seeds **5** main, **218** Deathrite) |
| `scripts/capture-teleport-action-parity.ts` | Regenerates Teleport cast-magic fixture (seed **154**) |
| `scripts/capture-lure-action-parity.ts` | Regenerates Lure cast-magic fixture (seed **100**) |
| `tests/engine/fixtures/drag-projectile-action-v1.json` | P1-B: `shoot-drag-projectile` enumeration, no-fight/fight transitions, `drag-projectile` Deathrite resume |
| `tests/engine/fixtures/teleport-action-v1.json` | P1-C: six `cast-magic` ally/site pairs + avatar no-op + ally teleport receipts |
| `tests/engine/fixtures/lure-action-v1.json` | P1-C: two Lure choices (C3/D4), first lure, second lure, paid no-op |
| `tests/engine/drag-projectile-action-parity.test.ts` | Byte-identical regeneration + shape asserts |
| `tests/engine/teleport-action-parity.test.ts` | Same for Teleport |
| `tests/engine/lure-action-parity.test.ts` | Same for Lure |
| `crates/sorcery-engine/tests/drag_projectile_rules.rs` | `#[ignore]` stubs for RULE-CATALOG-0092/0093 |
| `crates/sorcery-engine/tests/targeted_relocation_magic_rules.rs` | `#[ignore]` stubs for RULE-CATALOG-0038/0039 |

## Verification run

```text
node --test tests/engine/drag-projectile-action-parity.test.ts \
         tests/engine/teleport-action-parity.test.ts \
         tests/engine/lure-action-parity.test.ts   # pass
pnpm typecheck                                     # pass
cargo test -p sorcery-engine --test drag_projectile_rules \
          --test targeted_relocation_magic_rules --locked  # compile; 4 ignored
```

Regenerate fixtures after TS engine changes:

```text
node scripts/capture-drag-projectile-action-parity.ts --write
node scripts/capture-teleport-action-parity.ts --write
node scripts/capture-lure-action-parity.ts --write
```

## Shipper next steps — P1-B (drag projectile)

1. Add `ShootDragProjectile { direction, fight_on_arrival, hit, path, shooter_instance_id }` to `crates/sorcery-engine/src/action.rs` and wire legality in `game.rs` (mirror TS `shoot-drag-projectile`).
2. Implement `drag-projectile` continuation resume (reuse Leap Attack Deathrite pattern; fixture `movementDeathrite` block is the contract).
3. Fill in `drag_projectile_rules.rs` ignored tests; optionally extend `action_parity.rs` once descriptors deserialize.
4. Flip RULE-CATALOG-0092/0093 in `catalog.json` to `rust-supported` pointing at `drag_projectile_rules.rs`.

## Shipper next steps — P1-C (Teleport + Lure)

1. Extend `CastMagic` in `action.rs` with `tempted_enemy`, `tempted_destination` (Lure) — Teleport already has `target_location` + `target_site_instance_id`.
2. Implement `teleportAllyToTargetSite` and `lureEnemyMinionOneStepCloser` in `game.rs` (remove from unsupported list).
3. Fill in `targeted_relocation_magic_rules.rs` ignored tests using fixture replays.
4. Flip RULE-CATALOG-0038/0039 in `catalog.json` to `rust-supported`.

## Note on existing assets

- `shoot-projectile-action-v1.json` remains the **ordinary Ranged** parity fixture; drag uses the new `drag-projectile-action-v1.json` (`shoot-drag-projectile` kind).
- `cast-magic-action-v1.json` is unchanged; Teleport/Lure have dedicated fixtures with full transition receipts.
