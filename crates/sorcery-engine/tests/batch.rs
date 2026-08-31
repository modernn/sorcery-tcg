use serde_json::{Value, json};
use sorcery_engine::batch::{BatchError, BatchJob, run_batch};
use sorcery_engine::canonical::{canonical_json, identity_hash};
use sorcery_engine::policy::{PolicySnapshot, parse_policy_snapshot};

const HASH_B: &str = "sha256:bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";

fn fixture() -> Value {
    serde_json::from_str(include_str!(
        "../../../tests/engine/fixtures/typescript-parity-v1.json"
    ))
    .expect("valid parity fixture")
}

fn template(fixture: &Value) -> &str {
    fixture["games"]
        .as_array()
        .and_then(|games| games.iter().find(|game| game["seed"] == 31))
        .and_then(|game| game["manifestJson"].as_str())
        .expect("seed-31 manifest")
}

fn manifest_for_seed(template: &str, seed: u32) -> String {
    let mut manifest: Value = serde_json::from_str(template).expect("manifest JSON");
    let body = manifest.as_object_mut().expect("manifest object");
    body.remove("manifestId").expect("manifest identity");
    body.insert("seed".to_owned(), json!(seed));
    manifest["manifestId"] = json!(identity_hash(&manifest).expect("manifest identity"));
    canonical_json(&manifest).expect("canonical manifest")
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
fn worker_counts_should_produce_identical_ordered_authoritative_results() {
    let fixture = fixture();
    let first_manifest = manifest_for_seed(template(&fixture), 31);
    let second_manifest = manifest_for_seed(template(&fixture), 23);
    let policy = policy(&first_manifest);
    let jobs = [
        BatchJob {
            manifest_json: &first_manifest,
            north_policy: &policy,
            south_policy: &policy,
        },
        BatchJob {
            manifest_json: &second_manifest,
            north_policy: &policy,
            south_policy: &policy,
        },
    ];

    let one = run_batch(&jobs, 400, 1).expect("one-worker batch");
    let two = run_batch(&jobs, 400, 2).expect("two-worker batch");

    assert_eq!(one, two);
    assert_eq!(
        one.iter()
            .map(|result| result.job_index)
            .collect::<Vec<_>>(),
        [0, 1]
    );
    assert!(
        one.iter()
            .all(|result| result.accepted_action_count > 0 && result.replay_verified)
    );
    assert!(matches!(
        run_batch(&jobs, 400, 9),
        Err(BatchError::Invalid(_))
    ));
    assert!(matches!(
        run_batch(&jobs, 1, 2),
        Err(BatchError::NonTerminal(0))
    ));
}
