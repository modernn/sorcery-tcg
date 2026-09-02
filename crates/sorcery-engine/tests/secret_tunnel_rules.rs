use serde_json::{Value, json};
use sorcery_engine::canonical::{IdentityHash, canonical_json, identity_hash};
use sorcery_engine::contract::{ActionRequest, Receipt};
use sorcery_engine::session::{Session, StepResult};

fn avatar() -> Value {
    json!({
        "attack": 1,
        "cardType": "avatar",
        "defense": 1,
        "drawSpell": false,
        "life": 20,
    })
}

fn site(water: bool, tunnel: bool) -> Value {
    let mut value = json!({
        "cardType": "site",
        "elements": if water { ["water"] } else { ["earth"] },
    });
    if tunnel {
        value["connectsBurrowedAllies"] = json!(true);
    }
    value
}

fn crosser() -> Value {
    json!({
        "attack": 2,
        "burrowing": true,
        "cardType": "minion",
        "defense": 2,
        "manaCost": 0,
        "movementBonus": 1,
        "submerge": true,
        "thresholds": { "air": 0, "earth": 0, "fire": 0, "water": 0 },
    })
}

fn plain() -> Value {
    json!({
        "attack": 2,
        "cardType": "minion",
        "defense": 2,
        "manaCost": 0,
        "thresholds": { "air": 0, "earth": 0, "fire": 0, "water": 0 },
    })
}

fn manifest(seed: u64) -> String {
    let cards = json!({
        "north-avatar": avatar(),
        "north-crosser": crosser(),
        "north-land": site(false, false),
        "north-tunnel": site(false, true),
        "north-water": site(true, false),
        "south-avatar": avatar(),
        "south-plain": plain(),
        "south-site": site(false, false),
    });
    let mut value = json!({
        "authority": {
            "contentHash": identity_hash(&json!({ "fixture": "secret-tunnel-rules" }))
                .expect("synthetic authority identity"),
            "mode": "synthetic",
            "revisionId": "synthetic-secret-tunnel-rules-v1",
        },
        "cards": cards,
        "decks": {
            "north": {
                "atlas": [
                    "north-tunnel",
                    "north-water",
                    "north-land",
                    "north-tunnel",
                    "north-water",
                    "north-land",
                    "north-tunnel",
                    "north-water",
                    "north-land",
                ],
                "avatar": "north-avatar",
                "spellbook": vec!["north-crosser"; 8],
            },
            "south": {
                "atlas": vec!["south-site"; 9],
                "avatar": "south-avatar",
                "spellbook": vec!["south-plain"; 8],
            },
        },
        "engineVersion": "sorcery-core-v1",
        "firstSeat": "north",
        "schemaVersion": 1,
        "seed": seed,
    });
    value["manifestId"] = json!(identity_hash(&value).expect("manifest identity"));
    canonical_json(&value).expect("canonical synthetic manifest")
}

fn accept_where(session: &mut Session, predicate: impl Fn(&Value) -> bool) -> (Value, Receipt) {
    let action = session
        .legal_actions()
        .expect("legal actions")
        .into_iter()
        .find(|action| predicate(&action.descriptor))
        .expect("expected engine-issued action");
    let descriptor = action.descriptor.clone();
    let StepResult::Accepted(receipt) = session
        .step(ActionRequest {
            action_id: action.action_id.to_string(),
            seat: action.seat,
            state_version: action.state_version,
        })
        .expect("authoritative step")
    else {
        panic!("engine-issued action must be accepted");
    };
    (descriptor, receipt)
}

fn keep(session: &mut Session) {
    accept_where(session, |descriptor| {
        descriptor["kind"] == "mulligan"
            && descriptor["atlasOrder"] == json!([])
            && descriptor["spellbookOrder"] == json!([])
    });
}

fn draw_spell(session: &mut Session) {
    accept_where(session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    });
}

fn play_site(session: &mut Session, card_id: &str, cell: &str) {
    accept_where(session, |descriptor| {
        descriptor["kind"] == "play-site"
            && descriptor["cardId"] == card_id
            && descriptor["cell"] == cell
    });
}

fn end_turn(session: &mut Session) {
    accept_where(session, |descriptor| descriptor["kind"] == "end-turn");
}

fn state(session: &Session) -> Value {
    session.replay_value().expect("authoritative replay value")["state"].clone()
}

fn path_locations(descriptor: &Value) -> String {
    descriptor["path"]
        .as_array()
        .expect("movement path")
        .iter()
        .map(|location| {
            format!(
                "{}/{}",
                location["cell"].as_str().expect("path cell"),
                location["region"].as_str().expect("path region")
            )
        })
        .collect::<Vec<_>>()
        .join(",")
}

fn unit_paths(session: &Session, instance_id: &str) -> Vec<String> {
    session
        .legal_actions()
        .expect("legal actions")
        .iter()
        .filter(|action| {
            action.descriptor["kind"] == "move-and-attack"
                && action.descriptor["unitInstanceId"] == instance_id
        })
        .map(|action| path_locations(&action.descriptor))
        .collect()
}

fn assert_exact_replay(session: &Session) {
    let action_ids: Vec<IdentityHash> = session
        .transcript()
        .iter()
        .map(|receipt| receipt.action_id.clone())
        .collect();
    let replayed = Session::replay(session.manifest_json(), &action_ids).expect("exact replay");
    assert_eq!(replayed.transcript(), session.transcript());
    assert_eq!(
        replayed.replay_value().expect("replayed value"),
        session.replay_value().expect("session value")
    );
    assert!(session.verify_replay().expect("verified replay"));
}

#[test]
fn rule_catalog_0115_secret_tunnel_should_connect_burrowed_allies_to_controlled_sites_only() {
    let mut session = Session::new(&manifest(105)).expect("valid Secret Tunnel scenario");
    keep(&mut session);
    keep(&mut session);
    play_site(&mut session, "north-tunnel", "C4");
    let (summoned, _) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["cell"] == "C4"
            && descriptor["region"] == "underground"
    });
    let crosser_id = summoned["cardInstanceId"]
        .as_str()
        .expect("burrowed identity")
        .to_owned();
    end_turn(&mut session);

    draw_spell(&mut session);
    play_site(&mut session, "south-site", "C1");
    end_turn(&mut session);

    draw_spell(&mut session);
    play_site(&mut session, "north-land", "C3");
    end_turn(&mut session);
    draw_spell(&mut session);
    end_turn(&mut session);

    draw_spell(&mut session);
    play_site(&mut session, "north-water", "C2");
    end_turn(&mut session);
    draw_spell(&mut session);
    end_turn(&mut session);
    draw_spell(&mut session);

    let paths = unit_paths(&session, &crosser_id);
    assert_eq!(
        paths
            .iter()
            .filter(|path| *path == "C4/underground,C3/underground")
            .count(),
        1
    );
    assert!(paths.contains(&"C4/underground,C2/underwater".to_owned()));
    assert!(!paths.contains(&"C4/underground,C1/underground".to_owned()));
    assert!(!paths.contains(&"C4/underground,C2/underwater,C4/underground".to_owned()));

    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "move-and-attack"
            && descriptor["unitInstanceId"] == crosser_id.as_str()
            && path_locations(descriptor) == "C4/underground,C2/underwater"
    });
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "decline-attack"
    });
    let units = state(&session)["realm"]["units"]
        .as_array()
        .expect("realm units")
        .clone();
    assert_eq!(units[0]["location"], "C2");
    assert_eq!(units[0]["region"], "underwater");
    assert_exact_replay(&session);
}
