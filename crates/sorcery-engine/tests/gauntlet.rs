use serde_json::{Value, json};
use sorcery_engine::batch::BatchJob;
use sorcery_engine::canonical::{canonical_json, identity_hash};
use sorcery_engine::gauntlet::{GauntletOrientation, GauntletPair, GauntletReport, run_gauntlet};
use sorcery_engine::policy::{PolicySnapshot, parse_policy_snapshot};
use sorcery_engine::synthetic::synthetic_demo_manifest_json;

const HASH_B: &str = "sha256:bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";
const HASH_C: &str = "sha256:cccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc";

fn swapped_manifest(manifest: &str) -> String {
    let mut manifest: Value = serde_json::from_str(manifest).expect("manifest JSON");
    manifest
        .as_object_mut()
        .expect("manifest object")
        .remove("manifestId")
        .expect("manifest identity");
    let decks = manifest["decks"].as_object_mut().expect("manifest decks");
    let north = decks.remove("north").expect("north deck");
    let south = decks.remove("south").expect("south deck");
    decks.insert("north".to_owned(), south);
    decks.insert("south".to_owned(), north);
    manifest["manifestId"] = json!(identity_hash(&manifest).expect("manifest identity"));
    canonical_json(&manifest).expect("canonical manifest")
}

fn manifest_with_first_seat(manifest: &str, first_seat: &str) -> String {
    let mut manifest: Value = serde_json::from_str(manifest).expect("manifest JSON");
    manifest
        .as_object_mut()
        .expect("manifest object")
        .remove("manifestId")
        .expect("manifest identity");
    manifest["firstSeat"] = json!(first_seat);
    manifest["manifestId"] = json!(identity_hash(&manifest).expect("manifest identity"));
    canonical_json(&manifest).expect("canonical manifest")
}

fn policy(manifest: &str, deck_id: &str) -> PolicySnapshot {
    let manifest: Value = serde_json::from_str(manifest).expect("manifest JSON");
    let mut body = json!({
        "authorityHash": manifest["authority"]["contentHash"],
        "deckId": deck_id,
        "engineVersion": "sorcery-core-v1",
        "generation": 0,
        "observationVersion": "seat-observation-v1",
        "schemaVersion": 1,
        "selector": {
            "atlasReserve": 3,
            "featurePriority": [
                "keep-mulligan", "play-site", "summon-minion", "preferred-draw",
                "powered-movement", "beneficial-tactic", "move-toward-enemy",
                "end-turn", "canonical-fallback"
            ]
        },
        "tieBreak": "canonical-action-order-v1"
    });
    body["policyId"] = json!(identity_hash(&body).expect("policy identity"));
    parse_policy_snapshot(&canonical_json(&body).expect("canonical policy")).expect("valid policy")
}

fn assert_invalid_pair_metadata<'a>(
    pair: GauntletPair<'a>,
    manifest: &'a str,
    swapped_manifest: &'a str,
    second_job: BatchJob<'a>,
) {
    let unswapped_second_job = BatchJob {
        manifest_json: manifest,
        ..second_job
    };
    let unswapped_pair = GauntletPair {
        orientations: [
            pair.orientations[0],
            GauntletOrientation {
                job: unswapped_second_job,
                ..pair.orientations[1]
            },
        ],
        ..pair
    };
    assert!(run_gauntlet(&[unswapped_pair], 1).is_err());

    let south_first_manifest = manifest_with_first_seat(swapped_manifest, "south");
    let south_first_pair = GauntletPair {
        orientations: [
            pair.orientations[0],
            GauntletOrientation {
                job: BatchJob {
                    manifest_json: &south_first_manifest,
                    ..second_job
                },
                ..pair.orientations[1]
            },
        ],
        ..pair
    };
    assert!(run_gauntlet(&[south_first_pair], 1).is_err());

    let mismatched_seed = GauntletPair { seed: 32, ..pair };
    assert!(run_gauntlet(&[mismatched_seed], 1).is_err());

    let different_pair = GauntletPair {
        orientations: [
            GauntletOrientation {
                north_deck_id: "deck-c",
                ..pair.orientations[0]
            },
            GauntletOrientation {
                south_deck_id: "deck-c",
                ..pair.orientations[1]
            },
        ],
        ..pair
    };
    assert!(run_gauntlet(&[pair, different_pair], 1).is_err());
}

fn assert_report_contract(report: &GauntletReport, value: &Value) {
    assert_eq!(report.games.len(), 2);
    assert_eq!(report.game_count, 2);
    assert_eq!(report.seeds, [31]);
    assert_eq!(report.by_seat.north.games, 2);
    assert_eq!(report.by_seat.south.games, 2);
    assert_eq!(report.by_deck["deck-a"].games, 2);
    assert_eq!(report.by_deck["deck-b"].games, 2);
    assert_eq!(
        value["bySeat"],
        json!({
            "north": { "draws": 0, "games": 2, "losses": 2, "wins": 0 },
            "south": { "draws": 0, "games": 2, "losses": 0, "wins": 2 },
        })
    );
    let expected_deck_counts = json!({
        "asNorth": { "draws": 0, "games": 1, "losses": 1, "wins": 0 },
        "asSouth": { "draws": 0, "games": 1, "losses": 0, "wins": 1 },
        "draws": 0,
        "games": 2,
        "losses": 1,
        "wins": 1,
    });
    assert_eq!(value["byDeck"]["deck-a"], expected_deck_counts);
    assert_eq!(value["byDeck"]["deck-b"], expected_deck_counts);
    assert!(
        report
            .games
            .iter()
            .all(|game| game.result.report.replay_verified)
    );
    assert_eq!(
        report
            .games
            .iter()
            .map(|game| {
                (
                    game.north_deck_id.as_str(),
                    game.seed,
                    game.south_deck_id.as_str(),
                )
            })
            .collect::<Vec<_>>(),
        [("deck-a", 31, "deck-b"), ("deck-b", 31, "deck-a"),]
    );
    let expected_average = report
        .games
        .iter()
        .map(|game| {
            u32::try_from(game.result.report.turn_count)
                .map(f64::from)
                .expect("fixture turn count fits u32")
        })
        .sum::<f64>()
        / 2.0;
    assert!((report.average_turns - expected_average).abs() < f64::EPSILON);
    assert_eq!(value["gameCount"], 2);
    assert_eq!(
        value["classification"],
        "unranked_partial_rules_unverified_authority"
    );
    assert_eq!(value["games"][0]["jobIndex"], 0);
    assert_eq!(value["games"][1]["jobIndex"], 1);
    assert!(value["games"][0].get("result").is_none());
    assert_eq!(value["games"][0]["report"]["replayVerified"], true);
    assert!(value["bySeat"].get("north").is_some());
    assert!(value["byDeck"]["deck-a"].get("total").is_none());
}

#[test]
fn seat_pairs_should_aggregate_identically_across_worker_counts() {
    let manifest = synthetic_demo_manifest_json(31).expect("seed-31 manifest");
    let swapped_manifest = swapped_manifest(&manifest);
    let baseline_policy = policy(&manifest, HASH_B);
    let alternate_policy = policy(&manifest, HASH_C);
    let first_job = BatchJob {
        manifest_json: &manifest,
        north_deck_id: baseline_policy.deck_id(),
        north_policy: &baseline_policy,
        south_deck_id: alternate_policy.deck_id(),
        south_policy: &alternate_policy,
    };
    let second_job = BatchJob {
        manifest_json: &swapped_manifest,
        north_deck_id: alternate_policy.deck_id(),
        north_policy: &alternate_policy,
        south_deck_id: baseline_policy.deck_id(),
        south_policy: &baseline_policy,
    };
    let pair = GauntletPair {
        seed: 31,
        orientations: [
            GauntletOrientation {
                job: first_job,
                north_deck_id: "deck-a",
                south_deck_id: "deck-b",
            },
            GauntletOrientation {
                job: second_job,
                north_deck_id: "deck-b",
                south_deck_id: "deck-a",
            },
        ],
    };

    let one = run_gauntlet(&[pair], 1).expect("one-worker gauntlet");
    let two = run_gauntlet(&[pair], 2).expect("two-worker gauntlet");
    let one_value = serde_json::to_value(&one).expect("serializable one-worker gauntlet");
    let two_value = serde_json::to_value(&two).expect("serializable two-worker gauntlet");

    assert_eq!(one, two);
    assert_eq!(
        serde_json::to_string(&one_value).expect("ordered one-worker gauntlet JSON"),
        serde_json::to_string(&two_value).expect("ordered two-worker gauntlet JSON")
    );
    assert_report_contract(&one, &one_value);

    assert_invalid_pair_metadata(pair, &manifest, &swapped_manifest, second_job);
}
