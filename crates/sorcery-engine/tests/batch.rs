use serde_json::{Value, json};
use sorcery_engine::batch::{
    BatchError, BatchJob, default_batch_workers, run_batch, run_game_batch, run_game_batch_to_dir,
};
use sorcery_engine::canonical::{canonical_json, identity_hash};
use sorcery_engine::game_record::GAME_ARTIFACT_FILES;
use sorcery_engine::policy::{PolicySnapshot, parse_policy_snapshot};
use sorcery_engine::synthetic::synthetic_demo_manifest_json;

const HASH_B: &str = "sha256:bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";
const HASH_C: &str = "sha256:cccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc";

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
    let first_manifest = synthetic_demo_manifest_json(31).expect("seed-31 manifest");
    let second_manifest = synthetic_demo_manifest_json(23).expect("seed-23 manifest");
    let policy = policy(&first_manifest);
    let jobs = [
        BatchJob {
            manifest_json: &first_manifest,
            north_deck_id: policy.deck_id(),
            north_policy: &policy,
            south_deck_id: policy.deck_id(),
            south_policy: &policy,
        },
        BatchJob {
            manifest_json: &second_manifest,
            north_deck_id: policy.deck_id(),
            north_policy: &policy,
            south_deck_id: policy.deck_id(),
            south_policy: &policy,
        },
    ];

    let one = run_game_batch(&jobs, 1).expect("one-worker batch");
    let two = run_game_batch(&jobs, 2).expect("two-worker batch");
    let one_value = serde_json::to_value(&one).expect("serializable one-worker batch");
    let two_value = serde_json::to_value(&two).expect("serializable two-worker batch");

    assert_eq!(one, two);
    assert_eq!(
        canonical_json(&one_value).expect("canonical one-worker batch"),
        canonical_json(&two_value).expect("canonical two-worker batch")
    );
    assert_eq!(
        one.iter()
            .map(|result| result.job_index)
            .collect::<Vec<_>>(),
        [0, 1]
    );
    assert!(
        one.iter()
            .all(|result| result.report.accepted_action_count > 0 && result.report.replay_verified)
    );
    assert_eq!(one_value[0]["jobIndex"], 0);
    assert_eq!(
        one_value[0]["manifestId"],
        serde_json::from_str::<Value>(&first_manifest).expect("manifest JSON")["manifestId"]
    );
    assert_eq!(one_value[0]["report"]["replayVerified"], true);
    assert_eq!(
        one_value[0]["report"]["classification"],
        "unranked_partial_rules_unverified_authority"
    );
    assert_eq!(one_value[0]["report"]["terminal"]["status"], "finished");
    assert!(one_value[0].get("acceptedActionCount").is_none());
    assert!(matches!(run_batch(&jobs, 9), Err(BatchError::Invalid(_))));
    assert!((1..=8).contains(&default_batch_workers()));

    let invalid_jobs = [
        jobs[0],
        BatchJob {
            manifest_json: "{}",
            ..jobs[1]
        },
    ];
    assert!(matches!(
        run_batch(&invalid_jobs, 2),
        Err(BatchError::Job { job_index: 1, .. })
    ));

    let wrong_deck_id =
        sorcery_engine::canonical::IdentityHash::parse(HASH_C).expect("valid wrong deck identity");
    let wrong_binding = [BatchJob {
        north_deck_id: &wrong_deck_id,
        ..jobs[0]
    }];
    assert!(matches!(
        run_batch(&wrong_binding, 1),
        Err(BatchError::Job { job_index: 0, .. })
    ));
}

#[test]
fn batch_to_dir_writes_one_artifact_folder_per_job() {
    let first_manifest = synthetic_demo_manifest_json(31).expect("seed-31 manifest");
    let second_manifest = synthetic_demo_manifest_json(23).expect("seed-23 manifest");
    let policy = policy(&first_manifest);
    let jobs = [
        BatchJob {
            manifest_json: &first_manifest,
            north_deck_id: policy.deck_id(),
            north_policy: &policy,
            south_deck_id: policy.deck_id(),
            south_policy: &policy,
        },
        BatchJob {
            manifest_json: &second_manifest,
            north_deck_id: policy.deck_id(),
            north_policy: &policy,
            south_deck_id: policy.deck_id(),
            south_policy: &policy,
        },
    ];
    let dir = std::env::temp_dir().join(format!("sorcery-batch-artifacts-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    let results = run_game_batch_to_dir(&jobs, 2, &dir).expect("batch artifacts");
    assert_eq!(results.len(), 2);
    for (job_index, result) in results.iter().enumerate() {
        let job_dir = dir.join(job_index.to_string());
        for file in GAME_ARTIFACT_FILES {
            assert!(job_dir.join(file).is_file(), "{job_index}/{file}");
        }
        let outcome: Value = serde_json::from_str(
            &std::fs::read_to_string(job_dir.join("outcome.json")).expect("outcome"),
        )
        .expect("outcome JSON");
        assert_eq!(outcome["manifestId"], result.manifest_id.as_str());
        assert_eq!(
            outcome["finalStateHash"],
            result.report.final_state_hash.as_str()
        );
        assert_eq!(
            outcome["transcriptHash"],
            result.report.transcript_hash.as_str()
        );
    }
    let _ = std::fs::remove_dir_all(&dir);
}
