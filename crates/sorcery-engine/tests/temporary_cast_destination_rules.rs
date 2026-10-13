//! Synthetic proofs for a global-turn temporary paid-cast destination permission.

use serde_json::{Value, json};
use sorcery_engine::canonical::{canonical_json, identity_hash};
use sorcery_engine::checkpoint::{create_game_checkpoint, resume_game_checkpoint};
use sorcery_engine::contract::{ActionRequest, Receipt};
use sorcery_engine::session::{Session, StepResult};

fn finish_manifest(mut value: Value) -> String {
    value["manifestId"] = json!(identity_hash(&value).expect("synthetic manifest identity"));
    canonical_json(&value).expect("canonical synthetic manifest")
}

fn minion(attack: u8, defense: u8, extra: Value) -> Value {
    let mut value = json!({
        "attack": attack,
        "cardType": "minion",
        "defense": defense,
        "manaCost": 0,
        "thresholds": { "air": 0, "earth": 0, "fire": 0, "water": 0 },
    });
    let Value::Object(extra) = extra else {
        panic!("minion additions must be an object");
    };
    value.as_object_mut().expect("minion object").extend(extra);
    value
}

fn manifest(seed: u32, prevent_entry_at_c1: bool, permission: Option<bool>) -> String {
    let mut site = json!({
        "cardType": "site",
        "elements": ["earth"],
        "sacrificeToDestroyNearbySite": true,
    });
    if let Some(permission) = permission {
        site["genesisCastMinionsUpToPowerTwoToAnySiteThisTurn"] = json!(permission);
    }
    if prevent_entry_at_c1 {
        site["preventsUnitsWithPowerAtLeastFromEntering"] = json!(3);
    }
    let mut split_power = minion(4, 1, json!({}));
    split_power["genesisProgram"] = json!({"effects":[{
        "op":"grant",
        "recipients":{"query":{
            "area":"source",
            "kind":"minion",
            "controller":"any",
            "excludeSource":false
        }},
        "modifier":"power",
        "amount":2,
        "duration":"until-your-next-turn"
    }]});
    finish_manifest(json!({
        "authority": {
            "contentHash": identity_hash(&json!({"fixture":"temporary-cast-destination"}))
                .expect("synthetic authority identity"),
            "mode": "synthetic",
            "revisionId": "synthetic-temporary-cast-destination-v1",
        },
        "cards": {
            "north-avatar":{"attack":1,"cardType":"avatar","defense":1,"drawSpell":false,"life":20},
            "north-dummy":{"attack":1,"cardType":"minion","defense":1,"manaCost":0,"thresholds":{"air":0,"earth":0,"fire":0,"water":0}},
            "north-site":{"cardType":"site","elements":["earth"]},
            "south-avatar":{"attack":1,"cardType":"avatar","defense":1,"drawSpell":false,"earthSitePlayCreatesAdjacentRubble":true,"life":20,"replaceAdjacentRubbleWithTopAtlasSite":true},
            "south-permission-site":site,
            "cast-two":minion(2,2,json!({})),
            "cast-above":minion(3,3,json!({})),
            "cast-split":split_power,
        },
        "decks": {
            "north":{"atlas":vec!["north-site";12],"avatar":"north-avatar","spellbook":vec!["north-dummy";8]},
            "south":{"atlas":vec!["south-permission-site";12],"avatar":"south-avatar","spellbook":[
                "cast-two","cast-above","cast-split","cast-two",
                "cast-above","cast-split","cast-two","cast-above",
                "cast-split","cast-two","cast-above","cast-split",
                "cast-two","cast-above","cast-split","cast-two",
            ]},
        },
        "engineVersion":"sorcery-core-v1",
        "firstSeat":"north",
        "schemaVersion":1,
        "seed":seed,
    }))
}

fn state(session: &Session) -> Value {
    session.replay_value().expect("authoritative replay")["state"].clone()
}

fn try_accept_where(
    session: &mut Session,
    predicate: impl Fn(&Value) -> bool,
) -> Option<(Value, Receipt)> {
    let action = session
        .legal_actions()
        .ok()?
        .into_iter()
        .find(|action| predicate(&action.descriptor))?;
    let descriptor = action.descriptor.clone();
    let StepResult::Accepted(receipt) = session
        .step(ActionRequest {
            action_id: action.action_id.to_string(),
            seat: action.seat,
            state_version: action.state_version,
        })
        .ok()?
    else {
        return None;
    };
    Some((descriptor, receipt))
}

fn accept_where(session: &mut Session, predicate: impl Fn(&Value) -> bool) -> (Value, Receipt) {
    try_accept_where(session, predicate).expect("expected engine-issued action")
}

fn keep(session: &mut Session) {
    accept_where(session, |descriptor| {
        descriptor["kind"] == "mulligan"
            && descriptor["atlasOrder"] == json!([])
            && descriptor["spellbookOrder"] == json!([])
    });
}

fn offers(session: &Session, card_id: &str, cell: &str) -> bool {
    session
        .legal_actions()
        .expect("legal actions")
        .iter()
        .any(|action| {
            action.descriptor["kind"] == "summon-minion"
                && action.descriptor["cardId"] == card_id
                && action.descriptor["cell"] == cell
        })
}

fn opening_session(prevent_entry_at_c1: bool) -> (String, Session) {
    for seed in 1..=4096 {
        let encoded = manifest(seed, prevent_entry_at_c1, Some(true));
        let session = Session::new(&encoded).expect("valid synthetic manifest");
        let current_state = state(&session);
        let hand = current_state["players"]["south"]["hand"]["spellbook"]
            .as_array()
            .expect("south opening hand");
        let has = |card_id: &str| hand.iter().any(|card| card["cardId"] == card_id);
        if ["cast-two", "cast-above", "cast-split"]
            .into_iter()
            .all(has)
        {
            return (encoded, session);
        }
    }
    panic!("bounded seed with all synthetic cast-power examples in hand");
}

fn begin_south_turn(session: &mut Session) {
    keep(session);
    keep(session);
    accept_where(session, |descriptor| {
        descriptor["kind"] == "play-site"
            && descriptor["cardId"] == "north-site"
            && descriptor["cell"] == "C4"
    });
    accept_where(session, |descriptor| descriptor["kind"] == "end-turn");
    accept_where(session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    });
    accept_where(session, |descriptor| {
        descriptor["kind"] == "play-site"
            && descriptor["cardId"] == "south-permission-site"
            && descriptor["cell"] == "C1"
            && descriptor["createRubbleAt"] == "C2"
    });
}

fn assert_replay(session: &Session) {
    let action_ids = session
        .transcript()
        .iter()
        .map(|receipt| receipt.action_id.clone())
        .collect::<Vec<_>>();
    let replayed =
        Session::replay(session.manifest_json(), &action_ids).expect("deterministic replay");
    assert_eq!(replayed.transcript(), session.transcript());
    assert_eq!(
        replayed.replay_value().expect("replayed result"),
        session.replay_value().expect("session result")
    );
    assert!(session.verify_replay().expect("verified replay"));
}

#[test]
fn feature_absent_initial_state_matches_the_frozen_default_hash() {
    let session = Session::new(&manifest(31, false, None)).expect("feature-absent manifest");
    let snapshot = state(&session);
    let canonical_state = canonical_json(&snapshot).expect("canonical initial state");
    assert!(
        snapshot["players"]["north"]
            .get("castMinionsUpToPowerTwoToAnySiteTurn")
            .is_none()
    );
    assert!(
        snapshot["players"]["south"]
            .get("castMinionsUpToPowerTwoToAnySiteTurn")
            .is_none()
    );
    assert_eq!(
        identity_hash(&snapshot)
            .expect("initial state hash")
            .to_string(),
        "sha256:bd91c7432ecf20d38d1072b09ee3fa4e1d2650ea8e747c9dc2c1d29b2e15977a",
        "feature-absent state matches the frozen v2 baseline hash"
    );
    assert_eq!(canonical_state.len(), 9_359);
}

fn assert_permission_expires(session: &mut Session) {
    let stale = session
        .legal_actions()
        .expect("pre-expiry actions")
        .into_iter()
        .find(|action| {
            action.descriptor["kind"] == "summon-minion"
                && action.descriptor["cardId"] == "cast-two"
                && action.descriptor["cell"] == "C4"
        })
        .expect("cross-site cast before expiry");
    accept_where(session, |descriptor| descriptor["kind"] == "end-turn");
    let expired = state(session);
    assert!(
        expired["players"]["south"]
            .get("castMinionsUpToPowerTwoToAnySiteTurn")
            .is_none()
    );
    accept_where(session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    });
    accept_where(session, |descriptor| descriptor["kind"] == "end-turn");
    accept_where(session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    });
    assert!(
        !offers(session, "cast-two", "C4"),
        "permission expired before South's next turn"
    );
    assert!(
        offers(session, "cast-two", "C1"),
        "the player's own-Site destination remains legal"
    );
    let StepResult::Rejected(rejection) = session
        .step(ActionRequest {
            action_id: stale.action_id.to_string(),
            seat: stale.seat,
            state_version: state(session)["stateVersion"]
                .as_u64()
                .expect("new South state version"),
        })
        .expect("expired action rejection")
    else {
        panic!("an action from the prior global turn must be rejected");
    };
    assert_eq!(
        rejection.code,
        sorcery_engine::contract::RejectionCode::UnknownAction
    );
    assert_replay(session);
}

#[test]
fn genesis_permission_uses_cast_power_for_any_site_and_expires_at_global_turn_end() {
    let (encoded, mut session) = opening_session(false);
    let initial = state(&session);
    for seat in ["north", "south"] {
        assert!(
            initial["players"][seat]
                .get("castMinionsUpToPowerTwoToAnySiteTurn")
                .is_none()
        );
    }
    begin_south_turn(&mut session);

    let grant = session.transcript().last().expect("site play receipt");
    // The accepted site action's grant is present in its receipt and uses the captured seat.
    let receipt = grant.clone();
    let grant_event = receipt
        .events
        .iter()
        .find(|event| event.event_type == "cast-destination-permission-granted")
        .expect("permission grant event");
    let current_turn = state(&session)["turnNumber"]
        .as_u64()
        .expect("global turn number");
    assert_eq!(grant_event.payload["seat"], "south");
    assert_eq!(grant_event.payload["turnNumber"], current_turn);
    assert_eq!(
        state(&session)["players"]["south"]["castMinionsUpToPowerTwoToAnySiteTurn"],
        json!(current_turn)
    );
    assert!(
        state(&session)["players"]["north"]
            .get("castMinionsUpToPowerTwoToAnySiteTurn")
            .is_none()
    );
    assert!(
        !session
            .legal_actions()
            .expect("active permission actions")
            .iter()
            .any(|action| {
                action.descriptor["kind"] == "summon-minion"
                    && action.descriptor["cardId"] == "cast-two"
                    && action.descriptor["region"] == "void"
            }),
        "any Site permission does not extend to the Void"
    );

    let checkpoint = create_game_checkpoint(&session).expect("checkpoint with active grant");
    session = resume_game_checkpoint(&checkpoint).expect("resume active grant checkpoint");
    assert_eq!(
        state(&session)["players"]["south"]["castMinionsUpToPowerTwoToAnySiteTurn"],
        json!(current_turn)
    );

    assert!(
        offers(&session, "cast-two", "C1"),
        "low-power cast to own Site remains legal"
    );
    assert!(
        offers(&session, "cast-two", "C4"),
        "permission reaches the opponent-controlled Site"
    );
    assert!(
        offers(&session, "cast-split", "C4"),
        "split power uses the rounded-down midpoint"
    );
    assert!(
        !offers(&session, "cast-above", "C4"),
        "power above two is not covered"
    );
    assert!(
        offers(&session, "cast-above", "C1"),
        "ordinary own-Site casting is unchanged"
    );

    let (summon, _) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["cardId"] == "cast-split"
            && descriptor["cell"] == "C4"
    });
    let unit_id = summon["cardInstanceId"]
        .as_str()
        .expect("summoned instance");
    let powered_state = state(&session);
    let powered = powered_state["realm"]["units"]
        .as_array()
        .expect("realm units")
        .iter()
        .find(|unit| unit["instanceId"] == unit_id)
        .expect("summoned minion");
    assert!(
        powered["temporaryModifiers"]
            .as_array()
            .is_some_and(|modifiers| {
                modifiers
                    .iter()
                    .any(|modifier| modifier["kind"] == "power" && modifier["amount"] == 2)
            }),
        "split cast power two is checked before its Genesis power increase"
    );
    assert!(
        offers(&session, "cast-two", "C4"),
        "permission is not consumed by a cast"
    );

    assert_permission_expires(&mut session);
    assert_eq!(session.manifest_json(), encoded);
}

#[test]
fn atlas_rubble_replacement_grants_permission_and_source_removal_does_not_revoke_it() {
    let (_, mut session) = opening_session(false);
    begin_south_turn(&mut session);
    let original_turn = state(&session)["turnNumber"].as_u64().expect("global turn");
    accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn");
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    });
    accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn");
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    });
    let current_turn = state(&session)["turnNumber"].as_u64().expect("global turn");
    assert!(current_turn > original_turn);
    assert!(
        state(&session)["players"]["south"]
            .get("castMinionsUpToPowerTwoToAnySiteTurn")
            .is_none()
    );

    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "replace-rubble-with-top-atlas-site"
            && descriptor["targetCell"] == "C2"
    });
    let replacement_site_id = state(&session)["realm"]["sites"]["C2"]["instanceId"]
        .as_str()
        .expect("Atlas replacement Site instance")
        .to_owned();
    let grant_event = session
        .transcript()
        .last()
        .expect("replacement receipt")
        .events
        .iter()
        .find(|event| event.event_type == "cast-destination-permission-granted")
        .expect("replacement Site Genesis grant");
    assert_eq!(grant_event.payload["seat"], "south");
    assert_eq!(grant_event.payload["sourceInstanceId"], replacement_site_id);
    assert_eq!(grant_event.payload["turnNumber"], current_turn);

    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "activate-site-destruction"
            && descriptor["sourceSiteInstanceId"] == replacement_site_id
            && descriptor["targetCell"] == "C2"
    });
    assert_eq!(state(&session)["realm"]["sites"]["C2"]["rubble"], true);
    assert_eq!(
        state(&session)["players"]["south"]["castMinionsUpToPowerTwoToAnySiteTurn"],
        json!(current_turn),
        "permission survives its source Site leaving play"
    );
    assert!(offers(&session, "cast-two", "C4"));
    assert_replay(&session);
}

#[test]
fn site_entry_power_threshold_still_uses_destination_prospective_power() {
    let (_, mut session) = opening_session(true);
    begin_south_turn(&mut session);
    assert!(
        offers(&session, "cast-split", "C4"),
        "cast power is two despite attack four"
    );
    assert!(
        !offers(&session, "cast-split", "C1"),
        "destination entry power still blocks the high-attack split-power minion"
    );
    assert!(
        offers(&session, "cast-two", "C1"),
        "power below the Site entry threshold is legal"
    );
}
