use serde_json::{Value, json};
use sorcery_engine::canonical::{canonical_json, identity_hash};
use sorcery_engine::policy::{
    MAX_ATLAS_RESERVE, MAX_POLICY_GENERATION, ObservationVersion, POLICY_SCHEMA_VERSION,
    PolicyFeature, TieBreak, parse_policy_snapshot, serialize_policy_snapshot,
};

const HASH_A: &str = "sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
const HASH_B: &str = "sha256:bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";
const HASH_C: &str = "sha256:cccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc";

fn feature_names() -> Value {
    json!([
        "keep-mulligan",
        "play-site",
        "summon-minion",
        "preferred-draw",
        "powered-movement",
        "beneficial-tactic",
        "move-toward-enemy",
        "end-turn",
        "canonical-fallback"
    ])
}

fn body() -> Value {
    json!({
        "authorityHash": HASH_A,
        "deckId": HASH_B,
        "engineVersion": "sorcery-core-v1",
        "generation": 7,
        "observationVersion": "seat-observation-v1",
        "parentPolicyId": HASH_C,
        "schemaVersion": 1,
        "selector": {
            "atlasReserve": 3,
            "featurePriority": feature_names()
        },
        "tieBreak": "canonical-action-order-v1"
    })
}

fn canonical_snapshot(mut body: Value) -> String {
    let policy_id = identity_hash(&body).expect("hash policy body");
    body.as_object_mut()
        .expect("policy body object")
        .insert("policyId".to_owned(), json!(policy_id));
    canonical_json(&body).expect("canonical policy snapshot")
}

fn mutate_body(mutator: impl FnOnce(&mut serde_json::Map<String, Value>)) -> String {
    let mut value = body();
    mutator(value.as_object_mut().expect("policy body object"));
    canonical_snapshot(value)
}

#[test]
fn canonical_snapshot_should_parse_hash_and_round_trip() {
    let encoded = canonical_snapshot(body());
    let snapshot = parse_policy_snapshot(&encoded).expect("valid policy snapshot");

    assert_eq!(snapshot.schema_version(), POLICY_SCHEMA_VERSION);
    assert_eq!(
        snapshot.policy_id().as_str(),
        identity_hash(&body()).unwrap().as_str()
    );
    assert_eq!(snapshot.parent_policy_id().unwrap().as_str(), HASH_C);
    assert_eq!(snapshot.generation(), 7);
    assert_eq!(snapshot.engine_version(), "sorcery-core-v1");
    assert_eq!(snapshot.authority_hash().as_str(), HASH_A);
    assert_eq!(snapshot.deck_id().as_str(), HASH_B);
    assert_eq!(
        snapshot.observation_version(),
        ObservationVersion::SeatObservationV1
    );
    assert_eq!(
        snapshot.observation_version().as_str(),
        "seat-observation-v1"
    );
    assert_eq!(snapshot.tie_break(), TieBreak::CanonicalActionOrderV1);
    assert_eq!(snapshot.tie_break().as_str(), "canonical-action-order-v1");
    assert_eq!(snapshot.selector().atlas_reserve(), 3);
    assert_eq!(snapshot.selector().feature_priority(), &PolicyFeature::ALL);
    assert_eq!(
        serialize_policy_snapshot(&snapshot).expect("serialize valid policy"),
        encoded
    );
}

#[test]
fn changed_body_with_stale_policy_id_should_be_rejected() {
    let encoded = canonical_snapshot(body());
    let mut value: Value = serde_json::from_str(&encoded).expect("policy JSON");
    value["generation"] = json!(8);
    let tampered = canonical_json(&value).expect("canonical tampered policy");

    let error = parse_policy_snapshot(&tampered).expect_err("stale policyId must fail");

    assert!(error.to_string().contains("policyId"));
}

#[test]
fn unknown_and_extension_fields_should_be_rejected() {
    let cases = [
        ("script", json!("select(actions)"), false),
        ("ruleExtensions", json!(["RULE-99"]), false),
        ("cardOverrides", json!({"card-1": 10}), false),
        ("weights", json!([1, 2, 3]), true),
    ];

    for (field, value, selector_field) in cases {
        let encoded = mutate_body(|body| {
            if selector_field {
                body["selector"]
                    .as_object_mut()
                    .expect("selector object")
                    .insert(field.to_owned(), value);
            } else {
                body.insert(field.to_owned(), value);
            }
        });

        assert!(
            parse_policy_snapshot(&encoded).is_err(),
            "field {field} must be rejected"
        );
    }
}

#[test]
fn duplicate_and_noncanonical_json_should_be_rejected() {
    let encoded = canonical_snapshot(body());
    let duplicate = encoded.replacen("\"generation\":7", "\"generation\":7,\"generation\":7", 1);
    assert!(
        parse_policy_snapshot(&duplicate)
            .expect_err("duplicate field must fail")
            .to_string()
            .contains("duplicate_key:generation")
    );

    let value: Value = serde_json::from_str(&encoded).expect("policy JSON");
    let pretty = serde_json::to_string_pretty(&value).expect("pretty policy JSON");
    assert!(
        parse_policy_snapshot(&pretty)
            .expect_err("noncanonical JSON must fail")
            .to_string()
            .contains("canonical JSON")
    );
    assert!(parse_policy_snapshot(&format!("{encoded}\n")).is_err());
}

#[test]
fn identity_fields_should_require_canonical_sha256_ids() {
    for field in ["policyId", "authorityHash", "deckId", "parentPolicyId"] {
        let mut value: Value = serde_json::from_str(&canonical_snapshot(body())).unwrap();
        value[field] = json!("SHA256:not-an-identity");
        let encoded = canonical_json(&value).unwrap();

        assert!(
            parse_policy_snapshot(&encoded).is_err(),
            "invalid {field} must be rejected"
        );
    }
}

#[test]
fn feature_priority_should_contain_every_closed_feature_once() {
    let missing = mutate_body(|body| {
        body["selector"]["featurePriority"]
            .as_array_mut()
            .unwrap()
            .pop();
    });
    assert!(parse_policy_snapshot(&missing).is_err());

    let duplicate = mutate_body(|body| {
        body["selector"]["featurePriority"][8] = json!("keep-mulligan");
    });
    assert!(parse_policy_snapshot(&duplicate).is_err());

    let unknown = mutate_body(|body| {
        body["selector"]["featurePriority"][8] = json!("card-specific-combo");
    });
    assert!(parse_policy_snapshot(&unknown).is_err());
}

#[test]
fn numeric_and_fixed_contract_fields_should_be_bounded() {
    let cases = [
        ("schemaVersion", json!(2)),
        ("generation", json!(u64::from(MAX_POLICY_GENERATION) + 1)),
    ];
    for (field, value) in cases {
        let encoded = mutate_body(|body| {
            body.insert(field.to_owned(), value);
        });
        assert!(parse_policy_snapshot(&encoded).is_err(), "invalid {field}");
    }

    let reserve = mutate_body(|body| {
        body["selector"]["atlasReserve"] = json!(u64::from(MAX_ATLAS_RESERVE) + 1);
    });
    assert!(parse_policy_snapshot(&reserve).is_err());

    for (field, value) in [
        ("observationVersion", json!("omniscient-observation-v1")),
        ("tieBreak", json!("random-v1")),
        ("engineVersion", json!("javascript:select()")),
    ] {
        let encoded = mutate_body(|body| {
            body.insert(field.to_owned(), value);
        });
        assert!(parse_policy_snapshot(&encoded).is_err(), "invalid {field}");
    }
}

#[test]
fn absent_parent_should_be_canonical_but_null_should_not() {
    let without_parent = mutate_body(|body| {
        body.remove("parentPolicyId");
    });
    let snapshot = parse_policy_snapshot(&without_parent).expect("root policy");
    assert_eq!(snapshot.parent_policy_id(), None);

    let with_null = mutate_body(|body| {
        body.insert("parentPolicyId".to_owned(), Value::Null);
    });
    assert!(parse_policy_snapshot(&with_null).is_err());
}
