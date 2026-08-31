use serde_json::{Value, json};
use sorcery_engine::canonical::{canonical_json, identity_hash};
use sorcery_engine::contract::Seat;
use sorcery_engine::deck::{CanonicalDeck, DeckValidation};
use sorcery_engine::policy::{PolicySnapshot, parse_policy_snapshot};
use sorcery_engine::selfplay::{SelfPlayCase, train_and_promote};
use sorcery_engine::synthetic::synthetic_demo_manifest_json;

const HASH_B: &str = "sha256:bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";

fn authority_hash(manifest: &str) -> String {
    serde_json::from_str::<Value>(manifest).expect("manifest JSON")["authority"]["contentHash"]
        .as_str()
        .expect("authority content hash")
        .to_owned()
}

fn policy(authority_hash: &str) -> PolicySnapshot {
    let mut body = json!({
        "authorityHash": authority_hash,
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

fn deck(policy: &PolicySnapshot) -> DeckValidation {
    DeckValidation {
        deck: CanonicalDeck {
            avatar: "assigned-avatar".to_owned(),
            atlas: Vec::new(),
            spellbook: Vec::new(),
        },
        deck_id: policy.deck_id().clone(),
        format_legal: true,
        engine_supported: true,
        ranked_eligible: true,
        diagnostics: Vec::new(),
    }
}

fn pair<'a>(
    manifest_json: &'a str,
    seed: u32,
    opponent: &'a PolicySnapshot,
) -> [SelfPlayCase<'a>; 2] {
    [
        SelfPlayCase {
            seed,
            subgroup: "mirror",
            manifest_json,
            candidate_seat: Seat::North,
            opponent,
        },
        SelfPlayCase {
            seed,
            subgroup: "mirror",
            manifest_json,
            candidate_seat: Seat::South,
            opponent,
        },
    ]
}

#[test]
fn heldout_tie_should_keep_the_replay_verified_champion_deterministically() {
    let training_manifest = synthetic_demo_manifest_json(30).expect("training manifest");
    let heldout_manifest = synthetic_demo_manifest_json(31).expect("held-out manifest");
    let champion = policy(&authority_hash(&heldout_manifest));
    let deck = deck(&champion);
    let training = pair(&training_manifest, 30, &champion);
    let heldout = pair(&heldout_manifest, 31, &champion);

    let first = train_and_promote(&champion, &deck, &training, &heldout, 400)
        .expect("first promotion cycle");
    let second = train_and_promote(&champion, &deck, &training, &heldout, 400)
        .expect("repeat promotion cycle");

    assert_eq!(first, second);
    assert!(!first.promoted);
    assert_eq!(first.policy, champion);
    assert_eq!(first.champion_heldout.games(), 2);
    assert_eq!(first.nominee_heldout.games(), 2);
    assert_eq!(
        first.champion_heldout.half_points(),
        first.nominee_heldout.half_points()
    );
}

#[test]
fn suites_should_require_seat_pairs_and_disjoint_seeds() {
    let manifest = synthetic_demo_manifest_json(30).expect("training manifest");
    let champion = policy(&authority_hash(&manifest));
    let deck = deck(&champion);
    let pair = pair(&manifest, 30, &champion);

    assert!(train_and_promote(&champion, &deck, &pair[..1], &pair, 400).is_err());
    assert!(train_and_promote(&champion, &deck, &pair, &pair, 400).is_err());
}
