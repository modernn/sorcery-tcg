# RULE-CATALOG-0037 Proof Attempt Notes

## Rule
**RULE-CATALOG-0037**: Aura-loss deaths cannot restore stale combat during a defender path.

**Plain Language**: When a defender with `movementBonus` walks a multi-step defend path, and nearby minions die from losing an aura mid-path (triggering `deathrite-order`), the defender path continues to completion and the `defender-joined` event comes after all `minion-died` events.

**Current Status**: `typescript-supported` (proof exists in `tests/engine/game-setup.test.ts` around line 6082)

## Handoff Context
According to `OVERNIGHT-HANDOFF.md`:
- All machinery exists in Rust: stepped defender paths, Deathrite ordering, `defender-joined` event
- This is **proof-only work** - no engine changes needed
- Should mirror the TypeScript test structure

## Test Scenario Requirements
The test needs to prove both halves of the rule:

1. **Aura-loss deaths occur mid-defender-path**:
   - Source minion with `movementBonus: 2` and `otherNearbyAlliesPowerBonus: 1` at B4
   - Two fragile minions (1/1 with `deathriteHeal: 3`) at C4, kept alive by +1 aura
   - Cast `damageEachAbovegroundMinion: 1` (Rain of Arrows) - wounds fragiles to 1 damage on 2 effective defense
   - Source defends with 4-cell path: B4 → A4 → B4 → C4
   - When source moves away, fragiles lose +1 defense bonus and die (1 damage on 1 defense)

2. **Defender path completes despite deaths**:
   - Game enters `deathrite-order` phase mid-movement
   - After ordering deathrites, movement resumes
   - `defender-joined` event fires **after** the last `minion-died` event
   - The defender successfully joins combat

## Implementation Challenges Encountered

### Manifest Validation Issues
The Rust engine requires exact card-to-deck matching. Every card defined in `cards` must be referenced in `decks`, and vice versa. The TypeScript test searches seeds 1-4096 to find one where specific cards appear in opening hands.

### Test Patterns
Existing Rust tests use two patterns:
1. **Simple manifests** (`defend_rules.rs`): Single card type repeated 8 times, with known working seeds
2. **Seed searching** (`static_power_rules.rs`): Helper function searches 1-4096 to find seeds where specific cards are in opening hands

### Attempted Solutions
1. Fixed manifest with seed searching - needs implementation of `seeded()` helper
2. Simplified scenarios - still requires right cards at right times
3. Multiple card copies - doesn't guarantee draw order

## Recommended Next Steps

### Option 1: Implement Seed Search (Most Robust)
Create a helper similar to `static_power_rules.rs::seeded()`:
```rust
fn find_working_seed(wanted_cards: &[&str]) -> String {
    (1..=4096)
        .map(|seed| create_manifest(seed))
        .find(|manifest| {
            let session = Session::new(manifest).expect("valid");
            wanted_cards.iter().all(|card_spec| {
                // Check if card is in opening hand or early draws
            })
        })
        .expect("found working seed")
}
```

Use to find a seed where North's opening + first draw contains: `north-target`, `north-fragile` (2x), `north-source`
And South's opening + first 2 draws contain: `south-attacker`, `south-rain`

### Option 2: Use Synthetic Demo Manifest (Simpler)
Base the test on `synthetic_demo_manifest_json()` like `deathrite_rules.rs` does:
- Uses `base_manifest(seed)` helper
- `set_all_minions()` to replace cards with custom facts
- Guarantees structure and reduces manifest validation issues

### Option 3: Exhaustive Seed Trial (Pragmatic)
Try seeds 1-500 systematically until one works with the current test structure. Document the working seed.

## Files to Modify

### Test File
- **Location**: `crates/sorcery-engine/tests/defend_rules.rs` (add at end) OR new file `defend_rules_0037.rs`
- **Test name**: `rule_catalog_0037_aura_loss_deaths_cannot_restore_stale_combat_during_defender_path`
- **Pattern**: Follow `movement_bonus_one_should_issue_and_apply_exact_two_step_defend_path` structure but with deathrite-order handling

### Catalog Update
After test passes, update `data/rules/catalog.json`:
```json
{
  "ruleId": "RULE-CATALOG-0037",
  "implementationStatus": "rust-supported",
  "scenarioProof": {
    "file": "crates/sorcery-engine/tests/defend_rules.rs",
    "testName": "rule_catalog_0037_aura_loss_deaths_cannot_restore_stale_combat_during_defender_path"
  }
}
```

## Key Assertions Needed
1. After source starts defend path: `assert_eq!(state["phase"], "deathrite-order")`
2. Fragiles are dead but not in cemetery during deathrite-order
3. After ordering: `assert_eq!(minion_died_count, 2)`
4. After movement continues: `assert_eq!(state["phase"], "defend")`
5. Defender successfully joined: `assert_eq!(pendingCombat.defenders, [source_id])`
6. Event ordering: `defender_joined_idx > last_minion_died_idx`

## TypeScript Test Reference
See `tests/engine/game-setup.test.ts:6082-6297` for complete reference implementation.

## Status
**Incomplete** - Test framework prepared but needs working seed or manifest restructuring. Engine implementation is confirmed working per handoff notes. This is purely a test authoring challenge, not an engine implementation issue.

## Time Investment
Approximately 2 hours investigating manifest validation, test patterns, and attempting multiple implementation strategies.
