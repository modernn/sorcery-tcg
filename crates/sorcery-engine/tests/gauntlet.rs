use serde_json::{Value, json};
use sorcery_engine::batch::BatchJob;
use sorcery_engine::canonical::{canonical_json, identity_hash};
use sorcery_engine::gauntlet::{GauntletOrientation, GauntletPair, run_gauntlet};
use sorcery_engine::policy::{PolicySnapshot, parse_policy_snapshot};

const HASH_B: &str = "sha256:bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";

fn manifest() -> String {
    let fixture: Value = serde_json::from_str(include_str!(
        "../../../tests/engine/fixtures/typescript-parity-v1.json"
    ))
    .expect("valid parity fixture");
    fixture["games"]
        .as_array()
        .and_then(|games| games.iter().find(|game| game["seed"] == 31))
        .and_then(|game| game["manifestJson"].as_str())
        .expect("seed-31 manifest")
        .to_owned()
}

fn policy(manifest: &str) -> PolicySnapshot {
    let manifest: Value = serde_json::from_str(manifest).expect("manifest JSON");
    let mut body = json!({
        "authorityHash": manifest["authority"]["contentHash"],
        "deckId": HASH_B,
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

#[test]
fn seat_pairs_should_aggregate_identically_across_worker_counts() {
    let manifest = manifest();
    let policy = policy(&manifest);
    let job = BatchJob {
        manifest_json: &manifest,
        north_policy: &policy,
        south_policy: &policy,
    };
    let pair = GauntletPair {
        seed: 31,
        orientations: [
            GauntletOrientation {
                job,
                north_deck_id: "deck-a",
                south_deck_id: "deck-b",
            },
            GauntletOrientation {
                job,
                north_deck_id: "deck-b",
                south_deck_id: "deck-a",
            },
        ],
    };

    let one = run_gauntlet(&[pair], 1).expect("one-worker gauntlet");
    let two = run_gauntlet(&[pair], 2).expect("two-worker gauntlet");

    assert_eq!(one, two);
    assert_eq!(one.games.len(), 2);
    assert_eq!(one.seeds, [31]);
    assert_eq!(one.by_seat[0].games, 2);
    assert_eq!(one.by_seat[1].games, 2);
    assert_eq!(one.by_deck["deck-a"].total.games, 2);
    assert_eq!(one.by_deck["deck-b"].total.games, 2);
    assert!(one.games.iter().all(|game| game.result.replay_verified));
}
