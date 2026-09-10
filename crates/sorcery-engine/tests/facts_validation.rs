use serde_json::{Value, json};
use sorcery_engine::synthetic::synthetic_demo_manifest_json;

#[expect(
    dead_code,
    reason = "the focused test imports the complete future public module"
)]
#[path = "../src/facts.rs"]
mod facts;

use facts::{
    CardFacts, Element, EndTurnStealth, MagicEffect, MinionGenesis, RequiredCastRegion, Thresholds,
    parse_card_definition,
};

fn thresholds() -> Value {
    json!({ "earth": 0, "fire": 0, "water": 0, "air": 0 })
}

fn avatar() -> Value {
    json!({
        "attack": 1,
        "cardType": "avatar",
        "defense": 2,
        "drawSpell": false,
        "life": 20
    })
}

fn minion() -> Value {
    json!({
        "attack": 2,
        "cardType": "minion",
        "defense": 3,
        "manaCost": 1,
        "thresholds": thresholds()
    })
}

fn spell(card_type: &str, effect: (&str, Value)) -> Value {
    let mut value = json!({
        "cardType": card_type,
        "manaCost": 1,
        "thresholds": thresholds()
    });
    value
        .as_object_mut()
        .expect("spell fixture object")
        .insert(effect.0.to_owned(), effect.1);
    value
}

fn with(mut value: Value, field: &str, added: Value) -> Value {
    value
        .as_object_mut()
        .expect("card fixture object")
        .insert(field.to_owned(), added);
    value
}

#[test]
fn parse_should_accept_each_typed_card_kind() {
    let cases = [
        ("avatar", avatar(), "avatar"),
        (
            "site",
            json!({
                "cardType": "site",
                "connectsBurrowedAllies": false,
                "elements": ["earth", "water"],
                "genesisGainMana": 2
            }),
            "site",
        ),
        (
            "artifact",
            spell("artifact", ("grantsBearerPower", json!(2))),
            "artifact",
        ),
        (
            "aura",
            spell(
                "aura",
                (
                    "immobilizeAndGroundMinionsAtAffectedSitesForThreeControllerTurns",
                    json!(true),
                ),
            ),
            "aura",
        ),
        (
            "magic",
            spell("magic", ("damageTargetUnit", json!(2))),
            "magic",
        ),
        ("minion", minion(), "minion"),
    ];

    for (card_id, definition, expected) in cases {
        let actual = match parse_card_definition(card_id, &definition).expect("valid card facts") {
            CardFacts::Avatar(_) => "avatar",
            CardFacts::Artifact(_) => "artifact",
            CardFacts::Aura(_) => "aura",
            CardFacts::Magic(_) => "magic",
            CardFacts::Minion(_) => "minion",
            CardFacts::Site(_) => "site",
        };
        assert_eq!(actual, expected, "{card_id}");
    }
}

#[test]
fn parse_should_normalize_optional_false_and_canonical_element_order() {
    let definition = with(
        with(
            with(minion(), "airborne", json!(false)),
            "charge",
            json!(false),
        ),
        "thresholds",
        json!({ "air": 4, "water": 3, "fire": 2, "earth": 1 }),
    );

    let CardFacts::Minion(facts) =
        parse_card_definition("normalized", &definition).expect("valid normalized facts")
    else {
        panic!("expected minion facts");
    };
    assert_eq!(facts.thresholds.canonical(), [1, 2, 3, 4]);
    assert!(!facts.airborne && !facts.charge);
}

#[test]
fn parse_should_use_utf16_length_and_ecmascript_whitespace_for_card_ids() {
    let valid = "😀".repeat(128);
    let too_long = "😀".repeat(129);
    let next_line = "\u{0085}";
    let cases = [
        (valid.as_str(), true),
        (too_long.as_str(), false),
        ("\u{FEFF}", false),
        (next_line, true),
    ];

    for (card_id, accepted) in cases {
        assert_eq!(
            parse_card_definition(card_id, &avatar()).is_ok(),
            accepted,
            "card ID {card_id:?}"
        );
    }
}

#[test]
#[expect(clippy::too_many_lines)]
fn parse_should_accept_every_artifact_aura_and_magic_effect_shape() {
    let artifact_effects = [
        ("atEndOfEachTurnSiteControllerLosesLife", json!(2)),
        ("bearerControllerChoosesExtraRandomOutcome", json!(true)),
        ("grantsBearerLethal", json!(true)),
        ("grantsBearerPower", json!(2)),
        (
            "tapBearerAndAnotherAllyHereAndDiscardCardToDamageEachUnitAtLocationWithinThreeSteps",
            json!(true),
        ),
        (
            "tapBearerAndAnotherAllyHereToDamageTargetWithinTwoSteps",
            json!(3),
        ),
        (
            "tapUnitHereToRollInCardinalDirectionAndDamageOtherUnitsAlongPath",
            json!(4),
        ),
    ];
    for (field, value) in artifact_effects {
        assert!(
            parse_card_definition(field, &spell("artifact", (field, value))).is_ok(),
            "Artifact effect {field}"
        );
    }

    let aura_effects = [
        (
            "atEndOfControllerTurnDamageRandomUnitAtAffectedSitesThenMayMoveOneStep",
            json!(3),
        ),
        (
            "immobilizeAndGroundMinionsAtAffectedSitesForThreeControllerTurns",
            json!(true),
        ),
    ];
    for (field, value) in aura_effects {
        assert!(
            parse_card_definition(field, &spell("aura", (field, value))).is_ok(),
            "Aura effect {field}"
        );
    }

    let magic_effects = [
        ("burrowAllMinionsAndArtifactsAtTargetLandSite", json!(true)),
        ("burrowTargetMinionOrArtifact", json!(true)),
        ("damageChainNearbyUnits", json!(true)),
        ("damageEachAbovegroundMinion", json!(1)),
        ("damageEachUnitAtLocationWithinTwoSteps", json!(2)),
        ("damageRandomUnitAtLocation", json!(2)),
        ("damageTargetUnit", json!(2)),
        ("destroyTargetArtifact", json!(true)),
        ("destroyTargetSite", json!(true)),
        ("disableTargetNearbyMinionUntilNextTurn", json!(true)),
        ("fightAllyWithAdjacentEnemy", json!(true)),
        ("gainControlOfTargetNearbyMinion", json!(true)),
        ("grantChargeToAllyThisTurn", json!(true)),
        ("grantPowerToAllyThisTurn", json!(2)),
        ("grantStealthToTargetMinion", json!(true)),
        ("grantWardToTargetMinion", json!(true)),
        ("healController", json!(2)),
        ("drawSites", json!(2)),
        ("drawSpells", json!(2)),
        ("killTargetMinion", json!(true)),
        ("killTargetWoundedMinion", json!(true)),
        ("leapAttackAlly", json!(true)),
        ("lureEnemyMinionOneStepCloser", json!(true)),
        ("millSites", json!(2)),
        ("millSpells", json!(2)),
        ("returnMinionFromOwnCemetery", json!(true)),
        ("returnTargetArtifactFromOwnCemetery", json!(true)),
        ("returnTargetMagicFromOwnCemetery", json!(true)),
        ("returnTargetArtifactToOwnerHand", json!(true)),
        ("returnTargetMinionToOwnerHand", json!(true)),
        ("returnTargetSiteFromOwnCemetery", json!(true)),
        ("returnTargetSiteToOwnerHand", json!(true)),
        ("submergeTargetMinion", json!(true)),
        ("summonRandomMinionFromAnyCemetery", json!(true)),
        (
            "summonTokenToEachControlledSiteBorderingEnemySite",
            json!("foot-soldier"),
        ),
        ("targetPlayerGainsLife", json!(2)),
        ("targetPlayerLosesLife", json!(2)),
        ("tapTargetMinion", json!(true)),
        ("teleportAllyToTargetSite", json!(true)),
        ("teleportNearbyAllyThenDrawCard", json!(true)),
        ("untapTargetMinion", json!(true)),
    ];
    for (field, value) in magic_effects {
        assert!(
            parse_card_definition(field, &spell("magic", (field, value))).is_ok(),
            "Magic effect {field}"
        );
    }

    let grid = with(
        with(
            spell(
                "magic",
                (
                    "damageUnitsAboveAndBelowTargetSiteByManhattanDistance",
                    json!([1, 2, 3, 4, 5]),
                ),
            ),
            "discardSiteAsAdditionalCost",
            json!(true),
        ),
        "destroyTargetSite",
        json!(true),
    );
    assert!(parse_card_definition("damage-grid", &grid).is_ok());
}

#[test]
fn rule_06_should_reject_unknown_or_noncanonical_facts() {
    let invalid = [
        (
            "unknown field",
            with(avatar(), "futureRule", json!(true)),
            "futureRule",
        ),
        (
            "obsolete fact",
            with(minion(), "genesisDrawSpell", json!(true)),
            "obsolete",
        ),
        (
            "unknown card type",
            json!({ "cardType": "realm" }),
            "cardType",
        ),
        (
            "true-only false",
            with(minion(), "token", json!(false)),
            "must be true",
        ),
        (
            "fractional integer",
            with(minion(), "manaCost", json!(1.5)),
            "safe integer",
        ),
        (
            "unsafe integer",
            with(minion(), "manaCost", json!(9_007_199_254_740_992_u64)),
            "safe integer",
        ),
    ];

    for (name, definition, expected_error) in invalid {
        let error = parse_card_definition(name, &definition).expect_err(name);
        assert!(
            error.to_string().contains(expected_error),
            "{name}: {error}"
        );
    }
}

#[test]
fn rule_06_typescript_parity_fixture_card_facts_should_parse() {
    let manifest_json = synthetic_demo_manifest_json(31).expect("synthetic manifest");
    let manifest: Value = serde_json::from_str(&manifest_json).expect("valid manifest JSON");
    let cards = manifest["cards"].as_object().expect("manifest cards");

    for (card_id, definition) in cards {
        parse_card_definition(card_id, definition).expect("TypeScript-supported card facts");
    }
}

#[test]
fn rule_06_should_validate_elements_thresholds_and_token_reference_ids() {
    let invalid = [
        (
            "element order",
            json!({ "cardType": "site", "elements": ["air", "earth"] }),
            "canonical order",
        ),
        (
            "duplicate element",
            json!({ "cardType": "site", "elements": ["earth", "earth"] }),
            "canonical order",
        ),
        (
            "unknown element",
            json!({ "cardType": "site", "elements": ["aether"] }),
            "unsupported element",
        ),
        (
            "missing threshold",
            with(
                minion(),
                "thresholds",
                json!({ "earth": 0, "fire": 0, "water": 0 }),
            ),
            "thresholds.air",
        ),
        (
            "unknown threshold",
            with(
                minion(),
                "thresholds",
                json!({ "earth": 0, "fire": 0, "water": 0, "air": 0, "aether": 0 }),
            ),
            "aether",
        ),
        (
            "negative threshold",
            with(
                minion(),
                "thresholds",
                json!({ "earth": -1, "fire": 0, "water": 0, "air": 0 }),
            ),
            "between 0",
        ),
        (
            "blank token reference",
            spell(
                "magic",
                (
                    "summonTokenToEachControlledSiteBorderingEnemySite",
                    json!("\u{FEFF}"),
                ),
            ),
            "UTF-16",
        ),
    ];

    for (name, definition, expected_error) in invalid {
        let error = parse_card_definition(name, &definition).expect_err(name);
        assert!(
            error.to_string().contains(expected_error),
            "{name}: {error}"
        );
    }
}

#[test]
#[expect(
    clippy::too_many_lines,
    reason = "one table documents every fail-closed Magic auxiliary pairing"
)]
fn exclusive_effects_and_magic_auxiliary_facts_should_fail_closed() {
    let invalid = [
        (
            "artifact no effect",
            json!({ "cardType": "artifact", "manaCost": 0, "thresholds": thresholds() }),
            "exactly one",
        ),
        (
            "artifact two effects",
            with(
                spell("artifact", ("grantsBearerPower", json!(2))),
                "grantsBearerLethal",
                json!(true),
            ),
            "exactly one",
        ),
        (
            "aura two effects",
            with(
                spell(
                    "aura",
                    (
                        "immobilizeAndGroundMinionsAtAffectedSitesForThreeControllerTurns",
                        json!(true),
                    ),
                ),
                "atEndOfControllerTurnDamageRandomUnitAtAffectedSitesThenMayMoveOneStep",
                json!(3),
            ),
            "exactly one",
        ),
        (
            "magic two effects",
            with(
                spell("magic", ("damageTargetUnit", json!(2))),
                "healController",
                json!(2),
            ),
            "exactly one",
        ),
        (
            "targetNearby false still needs target damage",
            with(
                spell("magic", ("healController", json!(2))),
                "targetNearby",
                json!(false),
            ),
            "requires damageTargetUnit",
        ),
        (
            "untap needs target damage",
            with(
                spell("magic", ("healController", json!(2))),
                "untapTargetMinionAfterDamage",
                json!(true),
            ),
            "requires damageTargetUnit",
        ),
        (
            "incomplete damage grid",
            spell(
                "magic",
                (
                    "damageUnitsAboveAndBelowTargetSiteByManhattanDistance",
                    json!([1, 2, 3, 4, 5]),
                ),
            ),
            "defined together",
        ),
        (
            "destroy plus discard without grid",
            with(
                spell("magic", ("destroyTargetSite", json!(true))),
                "discardSiteAsAdditionalCost",
                json!(true),
            ),
            "defined together",
        ),
        (
            "bad damage grid value",
            with(
                with(
                    spell(
                        "magic",
                        (
                            "damageUnitsAboveAndBelowTargetSiteByManhattanDistance",
                            json!([1, 2, 0, 4, 5]),
                        ),
                    ),
                    "discardSiteAsAdditionalCost",
                    json!(true),
                ),
                "destroyTargetSite",
                json!(true),
            ),
            "positive damage",
        ),
    ];

    for (name, definition, expected_error) in invalid {
        let error = parse_card_definition(name, &definition).expect_err(name);
        assert!(
            error.to_string().contains(expected_error),
            "{name}: {error}"
        );
    }
}

#[test]
#[expect(
    clippy::too_many_lines,
    reason = "one visible table documents the cross-field fail-closed contract"
)]
fn site_and_minion_mutual_exclusions_should_fail_closed() {
    let paid_token = with(
        json!({ "cardType": "site", "elements": [] }),
        "genesisPayOneManaToSummonToken",
        json!("token"),
    );
    let invalid = [
        (
            "site mana modes",
            with(
                with(
                    json!({ "cardType": "site", "elements": [] }),
                    "genesisGainMana",
                    json!(2),
                ),
                "genesisGainManaIfOnlyControlledCopy",
                json!(1),
            ),
            "unconditional and conditional",
        ),
        (
            "paid token plus Genesis",
            with(paid_token, "genesisHealNearbyAvatars", json!(3)),
            "paid-token",
        ),
        (
            "site discard and draw",
            with(
                with(
                    json!({ "cardType": "site", "elements": [] }),
                    "genesisDiscardTopSpells",
                    json!(2),
                ),
                "genesisDrawSpellPerAdjacentSameCard",
                json!(true),
            ),
            "discard and draw",
        ),
        (
            "alternative payments",
            with(
                with(minion(), "discardRandomCardInsteadOfMana", json!(true)),
                "sacrificeMinionAtSummoningLocationForManaDiscount",
                json!(2),
            ),
            "alternative summon payments",
        ),
        (
            "Genesis effects",
            with(
                with(minion(), "genesisDrawSite", json!(true)),
                "genesisHealController",
                json!(2),
            ),
            "Genesis effects",
        ),
        (
            "Genesis disable and Stealth",
            with(
                with(minion(), "genesisDisableSelfUntilDamaged", json!(true)),
                "stealth",
                json!(true),
            ),
            "disable with Stealth",
        ),
        (
            "targeted Genesis and payment",
            with(
                with(minion(), "genesisMayDamageTargetAdjacentUnit", json!(2)),
                "discardRandomCardInsteadOfMana",
                json!(true),
            ),
            "targeted Genesis",
        ),
        (
            "end-turn Stealth modes",
            with(
                with(minion(), "gainsStealthAtEndOfTurn", json!(true)),
                "gainsStealthAtEndOfTurnIfNoEnemiesNearby",
                json!(true),
            ),
            "end-turn Stealth",
        ),
        (
            "Ranged movement requires Ranged",
            with(
                minion(),
                "mayRangedStrikeOnceDuringBasicMovement",
                json!(true),
            ),
            "requires ranged",
        ),
        (
            "random teleport requires Voidwalk",
            with(
                minion(),
                "atStartOfControllerTurnTeleportToRandomSiteOrVoid",
                json!(true),
            ),
            "requires voidwalk",
        ),
        (
            "oversized ability",
            with(
                with(minion(), "occupiesSquareArea", json!(2)),
                "voidwalk",
                json!(true),
            ),
            "unsupported ability combination",
        ),
        (
            "movement restrictions",
            with(
                with(minion(), "movesOnlyForward", json!(true)),
                "movesOnlySideways",
                json!(true),
            ),
            "only forward and only sideways",
        ),
        (
            "cast region ability",
            with(minion(), "mustBeCastBurrowed", json!(true)),
            "requires Burrowing",
        ),
        (
            "damage prevention",
            with(
                with(minion(), "ward", json!(true)),
                "takesLessDamage",
                json!(1),
            ),
            "damage prevention",
        ),
        (
            "token Genesis",
            with(
                with(minion(), "token", json!(true)),
                "genesisDrawSite",
                json!(true),
            ),
            "token Genesis",
        ),
    ];

    for (name, definition, expected_error) in invalid {
        let error = parse_card_definition(name, &definition).expect_err(name);
        assert!(
            error.to_string().contains(expected_error),
            "{name}: {error}"
        );
    }
}

#[test]
fn typed_effects_should_retain_only_normalized_values() {
    let definition = with(
        with(
            spell("magic", ("damageTargetUnit", json!(3.0))),
            "targetNearby",
            json!(true),
        ),
        "untapTargetMinionAfterDamage",
        json!(true),
    );
    let CardFacts::Magic(facts) =
        parse_card_definition("lash", &definition).expect("valid targeted Magic")
    else {
        panic!("expected Magic facts");
    };
    assert_eq!(
        facts.effect,
        MagicEffect::DamageTargetUnit {
            amount: 3,
            target_nearby: true,
            untap_target_minion_after_damage: true,
        }
    );

    let CardFacts::Minion(facts) =
        parse_card_definition("draw-two", &with(minion(), "genesisDrawSpells", json!(2)))
            .expect("valid Genesis minion")
    else {
        panic!("expected minion facts");
    };
    assert_eq!(facts.genesis, Some(MinionGenesis::DrawSpells(2)));
    assert_eq!(facts.thresholds, Thresholds::default());
    assert_eq!(facts.provides, None::<Element>);

    let CardFacts::Minion(facts) = parse_card_definition(
        "oversized-draw",
        &with(
            with(minion(), "occupiesSquareArea", json!(2)),
            "genesisDrawSpells",
            json!(1),
        ),
    )
    .expect("valid oversized Genesis minion") else {
        panic!("expected minion facts");
    };
    assert!(facts.occupies_square_area_two);
    assert_eq!(facts.genesis, Some(MinionGenesis::DrawSpells(1)));

    let CardFacts::Minion(facts) = parse_card_definition(
        "oversized-caster",
        &with(
            with(minion(), "occupiesSquareArea", json!(2)),
            "spellcaster",
            json!(true),
        ),
    )
    .expect("valid oversized Spellcaster") else {
        panic!("expected minion facts");
    };
    assert!(facts.occupies_square_area_two);
    assert!(facts.spellcaster);

    let CardFacts::Minion(facts) = parse_card_definition(
        "oversized-deathrite",
        &with(
            with(minion(), "occupiesSquareArea", json!(2)),
            "deathriteDamageEachUnitHere",
            json!(1),
        ),
    )
    .expect("valid oversized Deathrite minion") else {
        panic!("expected minion facts");
    };
    assert!(facts.occupies_square_area_two);
    assert_eq!(facts.deathrite_damage_each_unit_here, Some(1));

    let CardFacts::Minion(facts) = parse_card_definition(
        "oversized-here",
        &with(
            with(minion(), "occupiesSquareArea", json!(2)),
            "genesisDamageEachOtherUnitHere",
            json!(1),
        ),
    )
    .expect("valid oversized Genesis here minion") else {
        panic!("expected minion facts");
    };
    assert!(facts.occupies_square_area_two);
    assert_eq!(
        facts.genesis,
        Some(MinionGenesis::DamageEachOtherUnitHereOne)
    );
}

#[test]
fn oversized_discard_here_and_any_site_should_parse() {
    let CardFacts::Minion(facts) = parse_card_definition(
        "oversized-discard-here",
        &with(
            with(minion(), "occupiesSquareArea", json!(2)),
            "discardSpellToDamageRandomOtherUnitHere",
            json!(1),
        ),
    )
    .expect("valid oversized discard-here minion") else {
        panic!("expected minion facts");
    };
    assert!(facts.occupies_square_area_two);
    assert_eq!(
        facts.discard_spell_to_damage_random_other_unit_here,
        Some(1)
    );

    let CardFacts::Minion(facts) = parse_card_definition(
        "oversized-any-site",
        &with(
            with(minion(), "occupiesSquareArea", json!(2)),
            "summonToAnySite",
            json!(true),
        ),
    )
    .expect("valid oversized summon-to-any-site minion") else {
        panic!("expected minion facts");
    };
    assert!(facts.occupies_square_area_two);
    assert!(facts.summon_to_any_site);
}

#[test]
fn oversized_waterbound_and_threshold_suppression_should_parse() {
    let CardFacts::Minion(facts) = parse_card_definition(
        "oversized-waterbound",
        &with(
            with(minion(), "occupiesSquareArea", json!(2)),
            "waterbound",
            json!(true),
        ),
    )
    .expect("valid oversized Waterbound minion") else {
        panic!("expected minion facts");
    };
    assert!(facts.occupies_square_area_two);
    assert!(facts.waterbound);

    let CardFacts::Minion(facts) = parse_card_definition(
        "oversized-rats",
        &with(
            with(minion(), "occupiesSquareArea", json!(2)),
            "siteProvidesNoThreshold",
            json!(true),
        ),
    )
    .expect("valid oversized threshold-suppression minion") else {
        panic!("expected minion facts");
    };
    assert!(facts.occupies_square_area_two);
    assert!(facts.site_provides_no_threshold);
}

#[test]
fn oversized_ranged_and_tower_should_parse() {
    let CardFacts::Minion(facts) = parse_card_definition(
        "oversized-ranged",
        &with(
            with(minion(), "occupiesSquareArea", json!(2)),
            "ranged",
            json!(true),
        ),
    )
    .expect("valid oversized Ranged minion") else {
        panic!("expected minion facts");
    };
    assert!(facts.occupies_square_area_two);
    assert!(facts.ranged);

    let CardFacts::Minion(facts) = parse_card_definition(
        "oversized-tower",
        &with(
            with(minion(), "occupiesSquareArea", json!(2)),
            "gainsPowerRangedAndSpellcasterAtopTower",
            json!(2),
        ),
    )
    .expect("valid oversized Tower-conditional minion") else {
        panic!("expected minion facts");
    };
    assert!(facts.occupies_square_area_two);
    assert!(facts.gains_power_ranged_and_spellcaster_atop_tower);
}

#[test]
fn oversized_ordinary_and_sacrifice_should_parse() {
    let CardFacts::Minion(facts) = parse_card_definition(
        "oversized-ordinary",
        &with(
            with(minion(), "occupiesSquareArea", json!(2)),
            "ordinary",
            json!(true),
        ),
    )
    .expect("valid oversized Ordinary minion") else {
        panic!("expected minion facts");
    };
    assert!(facts.occupies_square_area_two);
    assert!(facts.ordinary);

    let CardFacts::Minion(facts) = parse_card_definition(
        "oversized-sacrifice",
        &with(
            with(minion(), "occupiesSquareArea", json!(2)),
            "sacrificeMinionAtSummoningLocationForManaDiscount",
            json!(2),
        ),
    )
    .expect("valid oversized sacrifice-discount minion") else {
        panic!("expected minion facts");
    };
    assert!(facts.occupies_square_area_two);
    assert_eq!(
        facts.alternative_summon_payment,
        Some(facts::AlternativeSummonPayment::SacrificeMinionAtSummoningLocationForManaDiscountTwo)
    );
}

#[test]
fn oversized_water_site_cast_should_parse() {
    let CardFacts::Minion(facts) = parse_card_definition(
        "oversized-water-cast",
        &with(
            with(minion(), "occupiesSquareArea", json!(2)),
            "mustBeCastToWaterSite",
            json!(true),
        ),
    )
    .expect("valid oversized water-site cast minion") else {
        panic!("expected minion facts");
    };
    assert!(facts.occupies_square_area_two);
    assert!(facts.must_be_cast_to_water_site);

    let CardFacts::Minion(facts) = parse_card_definition(
        "oversized-water-cast-any-site",
        &with(
            with(
                with(minion(), "occupiesSquareArea", json!(2)),
                "mustBeCastToWaterSite",
                json!(true),
            ),
            "summonToAnySite",
            json!(true),
        ),
    )
    .expect("valid oversized water-site any-site minion") else {
        panic!("expected minion facts");
    };
    assert!(facts.occupies_square_area_two);
    assert!(facts.must_be_cast_to_water_site);
    assert!(facts.summon_to_any_site);
}

#[test]
fn oversized_activated_and_drag_projectiles_should_parse() {
    let CardFacts::Minion(facts) = parse_card_definition(
        "oversized-activated-projectile",
        &with(
            with(minion(), "occupiesSquareArea", json!(2)),
            "tapToShootProjectileDamage",
            json!(1),
        ),
    )
    .expect("valid oversized activated-projectile minion") else {
        panic!("expected minion facts");
    };
    assert!(facts.occupies_square_area_two);
    assert_eq!(facts.tap_to_shoot_projectile_damage, Some(1));

    let CardFacts::Minion(facts) = parse_card_definition(
        "oversized-drag-projectile",
        &with(
            with(minion(), "occupiesSquareArea", json!(2)),
            "shootsDragProjectile",
            json!(true),
        ),
    )
    .expect("valid oversized drag-projectile minion") else {
        panic!("expected minion facts");
    };
    assert!(facts.occupies_square_area_two);
    assert!(facts.shoots_drag_projectile);
}

#[test]
fn oversized_burrowing_and_submerge_should_parse() {
    let CardFacts::Minion(facts) = parse_card_definition(
        "oversized-burrowing",
        &with(
            with(minion(), "occupiesSquareArea", json!(2)),
            "burrowing",
            json!(true),
        ),
    )
    .expect("valid oversized Burrowing minion") else {
        panic!("expected minion facts");
    };
    assert!(facts.occupies_square_area_two);
    assert!(facts.burrowing);

    let CardFacts::Minion(facts) = parse_card_definition(
        "oversized-submerge",
        &with(
            with(minion(), "occupiesSquareArea", json!(2)),
            "submerge",
            json!(true),
        ),
    )
    .expect("valid oversized Submerge minion") else {
        panic!("expected minion facts");
    };
    assert!(facts.occupies_square_area_two);
    assert!(facts.submerge);
}

#[test]
fn oversized_required_cast_regions_should_parse() {
    let CardFacts::Minion(facts) = parse_card_definition(
        "oversized-burrowed-only",
        &with(
            with(
                with(minion(), "occupiesSquareArea", json!(2)),
                "burrowing",
                json!(true),
            ),
            "mustBeCastBurrowed",
            json!(true),
        ),
    )
    .expect("valid oversized burrowed-only minion") else {
        panic!("expected minion facts");
    };
    assert!(facts.occupies_square_area_two);
    assert!(facts.burrowing);
    assert_eq!(
        facts.required_cast_region,
        Some(RequiredCastRegion::Underground)
    );

    let CardFacts::Minion(facts) = parse_card_definition(
        "oversized-submerged-only",
        &with(
            with(
                with(minion(), "occupiesSquareArea", json!(2)),
                "submerge",
                json!(true),
            ),
            "mustBeCastSubmerged",
            json!(true),
        ),
    )
    .expect("valid oversized submerged-only minion") else {
        panic!("expected minion facts");
    };
    assert!(facts.occupies_square_area_two);
    assert!(facts.submerge);
    assert_eq!(
        facts.required_cast_region,
        Some(RequiredCastRegion::Underwater)
    );
}

#[test]
fn oversized_area_damage_should_parse() {
    let CardFacts::Minion(facts) = parse_card_definition(
        "oversized-area-damage",
        &with(
            with(minion(), "occupiesSquareArea", json!(2)),
            "tapToDamageEachUnitAtAdjacentLocation",
            json!(2),
        ),
    )
    .expect("valid oversized area-damage minion") else {
        panic!("expected minion facts");
    };
    assert!(facts.occupies_square_area_two);
    assert!(facts.tap_to_damage_each_unit_at_adjacent_location);
}

#[test]
fn oversized_nearby_aura_and_stealth_loss_should_parse() {
    let CardFacts::Minion(facts) = parse_card_definition(
        "oversized-nearby-aura",
        &with(
            with(minion(), "occupiesSquareArea", json!(2)),
            "otherNearbyAlliesPowerBonus",
            json!(1),
        ),
    )
    .expect("valid oversized nearby-aura minion") else {
        panic!("expected minion facts");
    };
    assert!(facts.occupies_square_area_two);
    assert!(facts.other_nearby_allies_power_bonus);

    let CardFacts::Minion(facts) = parse_card_definition(
        "oversized-scent-hounds",
        &with(
            with(minion(), "occupiesSquareArea", json!(2)),
            "nearbyEnemiesPermanentlyLoseStealth",
            json!(true),
        ),
    )
    .expect("valid oversized Scent Hounds minion") else {
        panic!("expected minion facts");
    };
    assert!(facts.occupies_square_area_two);
    assert!(facts.nearby_enemies_permanently_lose_stealth);
}

#[test]
fn oversized_conditional_end_turn_stealth_should_parse() {
    let CardFacts::Minion(facts) = parse_card_definition(
        "oversized-conditional-stealth",
        &with(
            with(minion(), "occupiesSquareArea", json!(2)),
            "gainsStealthAtEndOfTurnIfNoEnemiesNearby",
            json!(true),
        ),
    )
    .expect("valid oversized conditional end-turn Stealth minion") else {
        panic!("expected minion facts");
    };
    assert!(facts.occupies_square_area_two);
    assert_eq!(
        facts.end_turn_stealth,
        Some(EndTurnStealth::IfNoEnemiesNearby)
    );
}

#[test]
fn oversized_during_movement_and_post_ranged_step_should_parse() {
    let CardFacts::Minion(facts) = parse_card_definition(
        "oversized-during-movement-ranged",
        &with(
            with(
                with(minion(), "occupiesSquareArea", json!(2)),
                "ranged",
                json!(true),
            ),
            "mayRangedStrikeOnceDuringBasicMovement",
            json!(true),
        ),
    )
    .expect("valid oversized during-movement Ranged minion") else {
        panic!("expected minion facts");
    };
    assert!(facts.occupies_square_area_two);
    assert!(facts.ranged);
    assert!(facts.may_ranged_strike_once_during_basic_movement);

    let CardFacts::Minion(facts) = parse_card_definition(
        "oversized-post-ranged-step",
        &with(
            with(
                with(minion(), "occupiesSquareArea", json!(2)),
                "ranged",
                json!(true),
            ),
            "mayStepAfterRangedStrike",
            json!(true),
        ),
    )
    .expect("valid oversized post-Ranged step minion") else {
        panic!("expected minion facts");
    };
    assert!(facts.occupies_square_area_two);
    assert!(facts.ranged);
    assert!(facts.may_step_after_ranged_strike);
}
