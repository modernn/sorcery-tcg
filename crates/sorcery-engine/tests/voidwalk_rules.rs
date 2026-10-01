//! Direct proofs for the void: the summons and steps Voidwalk grants (RULE-CATALOG-0119), the
//! temporary Voidwalk a Planar Gate lends until a minion leaves the void (RULE-CATALOG-0120), the
//! outer-column cast restriction that filters those summons but not movement (RULE-CATALOG-0121),
//! the settlement that kills inhospitable minions and banishes stranded void occupants
//! (RULE-CATALOG-0050), the loose Artifacts a newly played site lifts out of the void it
//! covers (RULE-CATALOG-0051), oversized Voidwalk (RULE-CATALOG-0316–0317), flooded Secret Tunnel
//! hops (RULE-CATALOG-0902), and state-based void banishment when a site covers part of a 2×2
//! footprint (RULE-CATALOG-0918).

use serde_json::{Value, json};
use sorcery_engine::canonical::{IdentityHash, canonical_json, identity_hash};
use sorcery_engine::checkpoint::{
    create_game_checkpoint, parse_game_checkpoint, resume_game_checkpoint,
    serialize_game_checkpoint,
};
use sorcery_engine::contract::{ActionRequest, Receipt, Seat};
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

fn site(elements: &[&str]) -> Value {
    json!({ "cardType": "site", "elements": elements })
}

fn minion(extra: Value) -> Value {
    let mut value = json!({
        "attack": 2,
        "cardType": "minion",
        "defense": 2,
        "manaCost": 0,
        "thresholds": { "air": 0, "earth": 0, "fire": 0, "water": 0 },
    });
    let Value::Object(extra) = extra else {
        panic!("extra minion facts must be an object");
    };
    value.as_object_mut().expect("minion facts").extend(extra);
    value
}

/// A minion that walks the void and drowns anywhere but a Water site.
fn waterbound_voidwalker() -> Value {
    minion(json!({
        "burrowing": true,
        "deathriteDrawSite": true,
        "voidwalk": true,
        "waterbound": true,
    }))
}

fn manifest(seed: u32, cards: &Value, north_site: &str, north_spellbook: &[&str]) -> String {
    let mut value = json!({
        "authority": {
            "contentHash": identity_hash(&json!({ "fixture": "voidwalk-rules" }))
                .expect("synthetic authority identity"),
            "mode": "synthetic",
            "revisionId": "synthetic-voidwalk-rules-v1",
        },
        "cards": cards,
        "decks": {
            "north": {
                "atlas": vec![north_site; 9],
                "avatar": "north-avatar",
                "spellbook": north_spellbook,
            },
            "south": {
                "atlas": vec!["south-site"; 9],
                "avatar": "south-avatar",
                "spellbook": vec!["south-filler"; 8],
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
            && descriptor["atlasOrder"]
                .as_array()
                .is_some_and(std::vec::Vec::is_empty)
            && descriptor["spellbookOrder"]
                .as_array()
                .is_some_and(std::vec::Vec::is_empty)
    });
}

fn state(session: &Session) -> Value {
    session.replay_value().expect("authoritative replay value")["state"].clone()
}

fn event_types(receipt: &Receipt) -> Vec<&str> {
    receipt
        .events
        .iter()
        .map(|event| event.event_type.as_str())
        .collect()
}

fn descriptors_of_kind(session: &Session, kind: &str) -> Vec<Value> {
    session
        .legal_actions()
        .expect("legal actions")
        .into_iter()
        .map(|action| action.descriptor)
        .filter(|descriptor| descriptor["kind"] == kind)
        .collect()
}

fn offers(session: &Session, predicate: impl Fn(&Value) -> bool) -> bool {
    session
        .legal_actions()
        .expect("legal actions")
        .iter()
        .any(|action| predicate(&action.descriptor))
}

fn path_locations(descriptor: &Value) -> Vec<String> {
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
        .collect()
}

fn is_void_summon_at(cell: &'static str) -> impl Fn(&Value) -> bool {
    move |descriptor: &Value| {
        descriptor["kind"] == "summon-minion"
            && descriptor["cell"] == cell
            && descriptor["region"] == "void"
    }
}

fn realm_unit(current: &Value, instance_id: &str) -> Option<Value> {
    current["realm"]["units"]
        .as_array()
        .expect("realm units")
        .iter()
        .find(|unit| unit["instanceId"] == instance_id)
        .cloned()
}

fn in_cemetery(current: &Value, seat: &str, instance_id: &str) -> bool {
    current["players"][seat]["cemetery"]
        .as_array()
        .expect("cemetery")
        .iter()
        .any(|card| card["instanceId"] == instance_id)
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

fn public_session_fingerprint(session: &Session) -> Value {
    json!({
        "legalActions": session.legal_actions().expect("legal frontier"),
        "northView": session.public_view(Seat::North).expect("north public view"),
        "replay": session.replay_value().expect("replay value"),
        "sessionHash": session.session_hash().expect("session hash"),
        "southView": session.public_view(Seat::South).expect("south public view"),
        "stateHash": session.state_hash().expect("state hash"),
        "transcript": session.transcript(),
        "transcriptHash": session.transcript_hash().expect("transcript hash"),
    })
}

fn play_site(session: &mut Session, cell: &str) {
    accept_where(session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == cell
    });
}

fn end_turn(session: &mut Session) {
    accept_where(session, |descriptor| descriptor["kind"] == "end-turn");
}

fn draw_spell(session: &mut Session) {
    accept_where(session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    });
}

/// Hands the turn to the other seat and takes its opening draw.
fn pass_turn(session: &mut Session) {
    end_turn(session);
    draw_spell(session);
}

fn draw_atlas(session: &mut Session) {
    accept_where(session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "atlas"
    });
}

fn pass_turn_with_site_draw(session: &mut Session) {
    end_turn(session);
    draw_atlas(session);
}

fn voidwalk_cards(voidwalker: &Value, north_site: &Value) -> Value {
    json!({
        "north-avatar": avatar(),
        "north-site": north_site.clone(),
        "north-spell": voidwalker.clone(),
        "south-avatar": avatar(),
        "south-filler": minion(json!({})),
        "south-site": site(&["earth"]),
    })
}

fn oriented_step_manifest(seed: u32, first_seat: &str, walker: &Value) -> String {
    let north_spellbook = if first_seat == "north" {
        vec!["walker"; 8]
    } else {
        vec!["filler"; 8]
    };
    let south_spellbook = if first_seat == "south" {
        vec!["walker"; 8]
    } else {
        vec!["filler"; 8]
    };
    let value = json!({
        "authority": {
            "contentHash": identity_hash(&json!({ "fixture": "airborne-voidwalk-step" }))
                .expect("synthetic authority identity"),
            "mode": "synthetic",
            "revisionId": "synthetic-airborne-voidwalk-step-v1",
        },
        "cards": {
            "filler": minion(json!({})),
            "north-avatar": avatar(),
            "north-site": site(&["earth"]),
            "south-avatar": avatar(),
            "south-site": site(&["earth"]),
            "walker": walker.clone(),
        },
        "decks": {
            "north": {
                "atlas": vec!["north-site"; 9],
                "avatar": "north-avatar",
                "spellbook": north_spellbook,
            },
            "south": {
                "atlas": vec!["south-site"; 9],
                "avatar": "south-avatar",
                "spellbook": south_spellbook,
            },
        },
        "engineVersion": "sorcery-core-v1",
        "firstSeat": first_seat,
        "schemaVersion": 1,
        "seed": seed,
    });
    let mut value = value;
    value["manifestId"] = json!(identity_hash(&value).expect("manifest identity"));
    canonical_json(&value).expect("canonical synthetic manifest")
}

/// Opens both domains through issued Site actions and returns the first seat's ready walker.
fn ready_oriented_walker(
    seed: u32,
    first_seat: &str,
    walker: &Value,
    summon_in_void: bool,
) -> (Session, String, String, String, String) {
    let mut session = Session::new(&oriented_step_manifest(seed, first_seat, walker))
        .expect("valid oriented movement fixture");
    keep(&mut session);
    keep(&mut session);

    let (
        site_cell,
        other_site_cell,
        second_site_cell,
        walker_card_id,
        surface_cell,
        diagonal_cell,
        void_cell,
        surface_diagonal,
    ) = if first_seat == "north" {
        ("C4", "C1", "C3", "walker", "C4", "B3", "B4", "C3")
    } else {
        ("C1", "C4", "C2", "walker", "C1", "B2", "B2", "C1")
    };
    play_site(&mut session, site_cell);
    let (summoned, _) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["cardId"] == walker_card_id
            && if summon_in_void {
                descriptor["region"] == "void" && descriptor["cell"] == void_cell
            } else {
                descriptor["region"].is_null() && descriptor["cell"] == surface_cell
            }
    });
    let walker_id = summoned["cardInstanceId"]
        .as_str()
        .expect("summoned walker identity")
        .to_owned();

    pass_turn(&mut session);
    play_site(&mut session, other_site_cell);
    pass_turn(&mut session);
    if summon_in_void {
        play_site(&mut session, second_site_cell);
        pass_turn(&mut session);
        pass_turn(&mut session);
    }

    (
        session,
        walker_id,
        diagonal_cell.to_owned(),
        void_cell.to_owned(),
        surface_diagonal.to_owned(),
    )
}

fn planar_gate_site() -> Value {
    json!({
        "cardType": "site",
        "elements": ["air"],
        "minionsHereGainVoidwalkUntilLeavingVoid": true,
    })
}

fn planar_gate_cards() -> Value {
    json!({
        "north-avatar": avatar(),
        "north-site": planar_gate_site(),
        "north-spell": minion(json!({
            "attack": 2,
            "defense": 2,
            "manaCost": 1,
            "movementBonus": 2,
        })),
        "south-avatar": avatar(),
        "south-filler": minion(json!({})),
        "south-site": site(&["earth"]),
    })
}

fn decline_attack_if_needed(session: &mut Session) {
    if offers(session, |descriptor| descriptor["kind"] == "decline-attack") {
        accept_where(session, |descriptor| descriptor["kind"] == "decline-attack");
    }
}

#[test]
fn rule_catalog_0120_planar_gate_should_grant_voidwalk_only_until_leaving_the_void() {
    let cards = planar_gate_cards();
    let mut session = Session::new(&manifest(162, &cards, "north-site", &["north-spell"; 8]))
        .expect("valid Planar Gate scenario");
    keep(&mut session);
    keep(&mut session);

    play_site(&mut session, "C4");
    let (summoned, _) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "summon-minion" && descriptor["cell"] == "C4"
    });
    let walker = summoned["cardInstanceId"]
        .as_str()
        .expect("summoned identity")
        .to_owned();

    pass_turn(&mut session);
    play_site(&mut session, "C1");
    pass_turn(&mut session);

    let into_void = |descriptor: &Value| {
        descriptor["kind"] == "move-and-attack"
            && descriptor["unitInstanceId"] == walker.as_str()
            && path_locations(descriptor) == ["C4/surface", "B4/void"]
    };
    accept_where(&mut session, into_void);
    decline_attack_if_needed(&mut session);
    let borrowed = realm_unit(&state(&session), &walker).expect("void borrower");
    assert_eq!(borrowed["planarGateVoidwalk"], json!(true));

    pass_turn(&mut session);
    play_site(&mut session, "C2");
    pass_turn(&mut session);

    let deeper_void = |descriptor: &Value| {
        descriptor["kind"] == "move-and-attack"
            && descriptor["unitInstanceId"] == walker.as_str()
            && path_locations(descriptor) == ["B4/void", "B3/void"]
    };
    assert!(offers(&session, deeper_void));
    accept_where(&mut session, deeper_void);
    decline_attack_if_needed(&mut session);

    pass_turn(&mut session);
    play_site(&mut session, "C3");
    pass_turn(&mut session);

    let reenter_void = |descriptor: &Value| {
        descriptor["kind"] == "move-and-attack"
            && descriptor["unitInstanceId"] == walker.as_str()
            && path_locations(descriptor) == ["B3/void", "C3/surface", "D3/void"]
    };
    assert!(!offers(&session, reenter_void));

    let exit_void = |descriptor: &Value| {
        descriptor["kind"] == "move-and-attack"
            && descriptor["unitInstanceId"] == walker.as_str()
            && path_locations(descriptor) == ["B3/void", "C3/surface"]
    };
    accept_where(&mut session, exit_void);
    decline_attack_if_needed(&mut session);
    let surfaced = realm_unit(&state(&session), &walker).expect("surfaced borrower");
    assert!(surfaced.get("planarGateVoidwalk").is_none());

    pass_turn(&mut session);
    pass_turn(&mut session);

    let back_to_void = |descriptor: &Value| {
        descriptor["kind"] == "move-and-attack"
            && descriptor["unitInstanceId"] == walker.as_str()
            && path_locations(descriptor) == ["C3/surface", "D3/void"]
    };
    assert!(!offers(&session, back_to_void));
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_0119_voidwalk_should_summon_to_any_void_and_step_between_void_and_surface() {
    let cards = voidwalk_cards(&minion(json!({ "voidwalk": true })), &site(&["earth"]));
    let mut session = Session::new(&manifest(141, &cards, "north-site", &["north-spell"; 8]))
        .expect("valid Voidwalk scenario");
    keep(&mut session);
    keep(&mut session);
    play_site(&mut session, "C4");

    // The void belongs to no site, so every cell the realm has not covered is on offer.
    assert!(offers(&session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["cell"] == "C4"
            && descriptor["region"].is_null()
    }));
    assert!(offers(&session, is_void_summon_at("B4")));
    assert!(offers(&session, is_void_summon_at("A1")));
    assert!(!offers(&session, is_void_summon_at("C4")));

    let (summoned, _) = accept_where(&mut session, is_void_summon_at("B4"));
    let walker = summoned["cardInstanceId"]
        .as_str()
        .expect("summoned identity")
        .to_owned();
    let placed = realm_unit(&state(&session), &walker).expect("void occupant");
    assert_eq!(
        (&placed["location"], &placed["region"]),
        (&json!("B4"), &json!("void"))
    );

    pass_turn(&mut session);
    play_site(&mut session, "C1");
    pass_turn(&mut session);

    let void_step = |descriptor: &Value| {
        descriptor["kind"] == "move-and-attack"
            && descriptor["unitInstanceId"] == walker.as_str()
            && path_locations(descriptor) == ["B4/void", "A4/void"]
    };
    let surface_step = |descriptor: &Value| {
        descriptor["kind"] == "move-and-attack"
            && descriptor["unitInstanceId"] == walker.as_str()
            && path_locations(descriptor) == ["B4/void", "C4/surface"]
    };
    assert!(offers(&session, void_step));
    assert!(offers(&session, surface_step));

    accept_where(&mut session, surface_step);
    if !descriptors_of_kind(&session, "decline-attack").is_empty() {
        accept_where(&mut session, |descriptor| {
            descriptor["kind"] == "decline-attack"
        });
    }
    let surfaced = realm_unit(&state(&session), &walker).expect("surfaced walker");
    assert_eq!(
        (&surfaced["location"], &surfaced["region"]),
        (&json!("C4"), &json!("surface"))
    );
    assert_exact_replay(&session);

    ordinary_minions_should_reach_no_void();
}

/// Without Voidwalk the void is unreachable, so no summon may name it.
fn ordinary_minions_should_reach_no_void() {
    let cards = voidwalk_cards(&minion(json!({})), &site(&["earth"]));
    let mut session = Session::new(&manifest(142, &cards, "north-site", &["north-spell"; 8]))
        .expect("valid ordinary summon scenario");
    keep(&mut session);
    keep(&mut session);
    play_site(&mut session, "C4");

    assert!(!descriptors_of_kind(&session, "summon-minion").is_empty());
    assert!(
        descriptors_of_kind(&session, "summon-minion")
            .iter()
            .all(|descriptor| descriptor["region"].is_null())
    );
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_0121_an_outer_column_restriction_should_filter_summons_but_not_movement() {
    let outer = minion(json!({ "mustBeCastToOuterColumn": true, "voidwalk": true }));
    let cards = voidwalk_cards(&outer, &site(&["earth"]));
    let mut session = Session::new(&manifest(143, &cards, "north-site", &["north-spell"; 8]))
        .expect("valid outer-column scenario");
    keep(&mut session);
    keep(&mut session);

    play_site(&mut session, "C4");
    pass_turn(&mut session);
    play_site(&mut session, "C1");
    pass_turn(&mut session);
    play_site(&mut session, "B4");
    pass_turn(&mut session);
    pass_turn(&mut session);
    play_site(&mut session, "A4");

    let summons = descriptors_of_kind(&session, "summon-minion");
    assert!(!summons.is_empty());
    assert!(summons.iter().all(|descriptor| {
        let cell = descriptor["cell"].as_str().expect("summon cell");
        cell.starts_with('A') || cell.starts_with('E')
    }));
    assert!(offers(&session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["cell"] == "A4"
            && descriptor["region"].is_null()
    }));
    assert!(!offers(&session, |descriptor| {
        descriptor["kind"] == "summon-minion" && descriptor["cell"] == "C4"
    }));
    assert!(offers(&session, is_void_summon_at("E2")));
    assert!(!offers(&session, is_void_summon_at("D2")));

    let (summoned, _) = accept_where(&mut session, is_void_summon_at("E2"));
    let walker = summoned["cardInstanceId"]
        .as_str()
        .expect("summoned identity")
        .to_owned();
    pass_turn(&mut session);
    pass_turn(&mut session);

    // The restriction governs casting alone: the walker still steps into an inner column.
    let inward = |descriptor: &Value| {
        descriptor["kind"] == "move-and-attack"
            && descriptor["unitInstanceId"] == walker.as_str()
            && path_locations(descriptor) == ["E2/void", "D2/void"]
    };
    assert!(offers(&session, inward));
    accept_where(&mut session, inward);
    if !descriptors_of_kind(&session, "decline-attack").is_empty() {
        accept_where(&mut session, |descriptor| {
            descriptor["kind"] == "decline-attack"
        });
    }
    let moved = realm_unit(&state(&session), &walker).expect("inner-column walker");
    assert_eq!(
        (&moved["location"], &moved["region"]),
        (&json!("D2"), &json!("void"))
    );
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_0050_region_settlement_should_kill_inhospitable_minions_and_banish_void_ones() {
    let cards = voidwalk_cards(&waterbound_voidwalker(), &site(&["earth"]));

    let mut drowned = Session::new(&manifest(144, &cards, "north-site", &["north-spell"; 8]))
        .expect("valid land settlement scenario");
    keep(&mut drowned);
    keep(&mut drowned);
    play_site(&mut drowned, "C4");
    let (summoned, receipt) = accept_where(&mut drowned, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["cell"] == "C4"
            && descriptor["region"] == "underground"
    });
    let underground = summoned["cardInstanceId"]
        .as_str()
        .expect("summoned identity")
        .to_owned();
    // Disabled by the dry site, the minion cannot hold its burrow and its Deathrite never fires.
    assert_eq!(event_types(&receipt), ["minion-summoned", "minion-died"]);
    let after_death = state(&drowned);
    assert!(realm_unit(&after_death, &underground).is_none());
    assert!(in_cemetery(&after_death, "north", &underground));
    assert_exact_replay(&drowned);

    let mut banished = Session::new(&manifest(144, &cards, "north-site", &["north-spell"; 8]))
        .expect("valid void settlement scenario");
    keep(&mut banished);
    keep(&mut banished);
    play_site(&mut banished, "C4");
    let (summoned, receipt) = accept_where(&mut banished, is_void_summon_at("A4"));
    let stranded = summoned["cardInstanceId"]
        .as_str()
        .expect("summoned identity")
        .to_owned();
    // The void banishes instead of killing, so the card leaves the game rather than the realm.
    assert_eq!(
        event_types(&receipt),
        ["minion-summoned", "minion-banished"]
    );
    let after_banishment = state(&banished);
    assert!(realm_unit(&after_banishment, &stranded).is_none());
    assert!(!in_cemetery(&after_banishment, "north", &stranded));
    assert_exact_replay(&banished);
}

#[test]
fn rule_catalog_0836_waterbound_voidwalk_stops_at_inhospitable_void() {
    let walker = minion(json!({
        "burrowing": true,
        "deathriteDrawSite": true,
        "movementBonus": 1,
        "voidwalk": true,
        "waterbound": true,
    }));
    let cards = voidwalk_cards(&walker, &site(&["water"]));
    let mut session = Session::new(&manifest(146, &cards, "north-site", &["north-spell"; 8]))
        .expect("valid truncated voidwalk scenario");
    keep(&mut session);
    keep(&mut session);
    play_site(&mut session, "C4");
    let (summoned, _) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["cell"] == "C4"
            && descriptor["region"].is_null()
    });
    let walker_id = summoned["cardInstanceId"]
        .as_str()
        .expect("walker identity")
        .to_owned();
    pass_turn(&mut session);
    play_site(&mut session, "C1");
    pass_turn(&mut session);
    let (_, receipt) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "move-and-attack"
            && descriptor["unitInstanceId"] == walker_id.as_str()
            && path_locations(descriptor) == ["C4/surface", "B4/void", "A4/void"]
    });
    assert_eq!(
        event_types(&receipt),
        ["move-and-attack-activated", "minion-banished"]
    );
    assert_eq!(
        receipt.events[0].payload,
        json!({
            "from": { "cell": "C4", "region": "surface" },
            "path": [
                { "cell": "C4", "region": "surface" },
                { "cell": "B4", "region": "void" },
            ],
            "seat": "north",
            "steps": 1,
            "to": { "cell": "B4", "region": "void" },
            "unitInstanceId": walker_id,
        })
    );
    let after = state(&session);
    assert!(realm_unit(&after, &walker_id).is_none());
    assert!(!in_cemetery(&after, "north", &walker_id));
    assert_eq!(after["phase"], "main");
    assert!(after["pendingCombat"].is_null());
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_0051_playing_a_site_should_surface_the_artifacts_its_void_held() {
    let cards = json!({
        "north-avatar": avatar(),
        "north-blade": {
            "cardType": "artifact",
            "grantsBearerLethal": true,
            "manaCost": 0,
            "thresholds": { "air": 0, "earth": 0, "fire": 0, "water": 0 },
        },
        "north-site": site(&["water"]),
        "north-spell": waterbound_voidwalker(),
        "south-avatar": avatar(),
        "south-filler": minion(json!({})),
        "south-site": site(&["earth"]),
    });
    let mut session = Session::new(&manifest(
        145,
        &cards,
        "north-site",
        &[
            "north-spell",
            "north-blade",
            "north-spell",
            "north-blade",
            "north-spell",
            "north-blade",
            "north-spell",
            "north-blade",
        ],
    ))
    .expect("valid covered-void Artifact scenario");
    keep(&mut session);
    keep(&mut session);
    play_site(&mut session, "C4");
    let (summoned, _) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["cell"] == "C4"
            && descriptor["region"].is_null()
    });
    let bearer = summoned["cardInstanceId"]
        .as_str()
        .expect("summoned identity")
        .to_owned();

    pass_turn(&mut session);
    play_site(&mut session, "C1");
    pass_turn(&mut session);

    // Wait for the Artifact the walker will carry into the void.
    let (cast, _) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "cast-artifact"
            && descriptor["bearer"]["instanceId"] == bearer.as_str()
    });
    let blade = cast["cardInstanceId"]
        .as_str()
        .expect("conjured identity")
        .to_owned();

    let into_void = |descriptor: &Value| {
        descriptor["kind"] == "move-and-attack"
            && descriptor["unitInstanceId"] == bearer.as_str()
            && path_locations(descriptor) == ["C4/surface", "B4/void"]
    };
    let (_, receipt) = accept_where(&mut session, into_void);
    // Leaving its Water site Disables the walker, so the void drops what it carried and banishes it.
    assert_eq!(
        event_types(&receipt),
        [
            "move-and-attack-activated",
            "artifact-dropped",
            "minion-banished"
        ]
    );
    let dropped = state(&session);
    assert!(realm_unit(&dropped, &bearer).is_none());
    let loose = dropped["realm"]["artifacts"]
        .as_array()
        .expect("realm Artifacts")
        .iter()
        .find(|artifact| artifact["instanceId"] == blade.as_str())
        .cloned()
        .expect("dropped Artifact");
    assert_eq!(
        (&loose["location"], &loose["region"]),
        (&json!("B4"), &json!("void"))
    );

    play_site(&mut session, "B4");
    let covered = state(&session);
    let surfaced = covered["realm"]["artifacts"]
        .as_array()
        .expect("realm Artifacts")
        .iter()
        .find(|artifact| artifact["instanceId"] == blade.as_str())
        .cloned()
        .expect("surfaced Artifact");
    assert_eq!(
        (&surfaced["location"], &surfaced["region"]),
        (&json!("B4"), &json!("surface"))
    );
    assert_exact_replay(&session);
}

fn oversized_voidwalker() -> Value {
    minion(json!({
        "occupiesSquareArea": 2,
        "voidwalk": true,
    }))
}

fn tunnel_flood_manifest(seed: u64) -> String {
    let mut value = json!({
        "authority": {
            "contentHash": identity_hash(&json!({ "fixture": "voidwalk-tunnel-flood" }))
                .expect("authority identity"),
            "mode": "synthetic",
            "revisionId": "synthetic-voidwalk-tunnel-flood-v1",
        },
        "cards": {
            "north-avatar": avatar(),
            "north-crosser": json!({
                "attack": 2,
                "burrowing": true,
                "cardType": "minion",
                "defense": 2,
                "manaCost": 0,
                "movementBonus": 1,
                "submerge": true,
                "thresholds": { "air": 0, "earth": 0, "fire": 0, "water": 0 },
            }),
            "north-flood": json!({
                "affectedSitesAreFlooded": true,
                "cardType": "aura",
                "manaCost": 0,
                "thresholds": { "air": 0, "earth": 0, "fire": 0, "water": 0 },
            }),
            "north-land": site(&["earth"]),
            "north-tunnel": json!({
                "cardType": "site",
                "connectsBurrowedAllies": true,
                "elements": ["earth"],
            }),
            "north-water": site(&["water"]),
            "south-avatar": avatar(),
            "south-plain": json!({
                "attack": 2,
                "cardType": "minion",
                "defense": 2,
                "manaCost": 0,
                "thresholds": { "air": 0, "earth": 0, "fire": 0, "water": 0 },
            }),
            "south-site": site(&["earth"]),
        },
        "decks": {
            "north": {
                "atlas": [
                    "north-tunnel", "north-water", "north-land",
                    "north-tunnel", "north-water", "north-land",
                    "north-tunnel", "north-water", "north-land",
                ],
                "avatar": "north-avatar",
                "spellbook": [
                    "north-crosser", "north-flood", "north-crosser", "north-crosser",
                    "north-crosser", "north-crosser", "north-crosser", "north-crosser",
                ],
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
    canonical_json(&value).expect("canonical manifest")
}

fn play_site_card(session: &mut Session, card_id: &str, cell: &str) {
    accept_where(session, |descriptor| {
        descriptor["kind"] == "play-site"
            && descriptor["cardId"] == card_id
            && descriptor["cell"] == cell
    });
}

fn cast_flood_covering(session: &mut Session, cell: &str) {
    accept_where(session, |descriptor| {
        descriptor["kind"] == "cast-aura"
            && descriptor["cardId"] == "north-flood"
            && descriptor["cells"]
                .as_array()
                .is_some_and(|cells| cells.len() == 4 && cells.iter().any(|value| value == cell))
    });
}

fn underground_path(descriptor: &Value) -> String {
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

fn is_square_void_summon_at(cell: &'static str, cells: &[&str]) -> impl Fn(&Value) -> bool {
    let expected = json!(cells);
    move |descriptor: &Value| {
        descriptor["kind"] == "summon-minion"
            && descriptor["cell"] == cell
            && descriptor["region"] == "void"
            && descriptor["cells"] == expected
    }
}

#[test]
fn rule_catalog_0316_oversized_voidwalk_summons_only_onto_an_all_void_square() {
    let cards = voidwalk_cards(&oversized_voidwalker(), &site(&["earth"]));
    let mut session = Session::new(&manifest(141, &cards, "north-site", &["north-spell"; 8]))
        .expect("valid oversized Voidwalk scenario");
    keep(&mut session);
    keep(&mut session);
    play_site(&mut session, "C4");

    assert!(offers(
        &session,
        is_square_void_summon_at("A1", &["A1", "A2", "B1", "B2"])
    ));
    assert!(!offers(&session, is_void_summon_at("B3")));
    assert!(!offers(&session, is_void_summon_at("C4")));
    assert!(
        descriptors_of_kind(&session, "summon-minion")
            .iter()
            .all(|descriptor| descriptor["region"] == "void")
    );

    let (summoned, _) = accept_where(
        &mut session,
        is_square_void_summon_at("A1", &["A1", "A2", "B1", "B2"]),
    );
    let giant = summoned["cardInstanceId"]
        .as_str()
        .expect("summoned identity")
        .to_owned();
    let placed = realm_unit(&state(&session), &giant).expect("void occupant");
    assert_eq!(
        (
            &placed["location"],
            &placed["region"],
            &placed["occupiedCells"]
        ),
        (
            &json!("A1"),
            &json!("void"),
            &json!(["A1", "A2", "B1", "B2"])
        )
    );
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_0317_oversized_voidwalk_steps_between_void_squares_not_onto_surface() {
    let cards = voidwalk_cards(&oversized_voidwalker(), &site(&["earth"]));
    let mut session = Session::new(&manifest(141, &cards, "north-site", &["north-spell"; 8]))
        .expect("valid oversized Voidwalk movement scenario");
    keep(&mut session);
    keep(&mut session);
    play_site(&mut session, "C4");
    let (summoned, _) = accept_where(
        &mut session,
        is_square_void_summon_at("A1", &["A1", "A2", "B1", "B2"]),
    );
    let giant = summoned["cardInstanceId"]
        .as_str()
        .expect("summoned identity")
        .to_owned();
    pass_turn(&mut session);
    play_site(&mut session, "C1");
    pass_turn(&mut session);

    let void_step = |descriptor: &Value| {
        descriptor["kind"] == "move-and-attack"
            && descriptor["unitInstanceId"] == giant.as_str()
            && path_locations(descriptor) == ["A1/void", "A2/void"]
    };
    let onto_surface = |descriptor: &Value| {
        descriptor["kind"] == "move-and-attack"
            && descriptor["unitInstanceId"] == giant.as_str()
            && path_locations(descriptor)
                .iter()
                .any(|step| step.ends_with("/surface"))
    };
    assert!(offers(&session, void_step));
    assert!(!offers(&session, onto_surface));

    accept_where(&mut session, void_step);
    decline_attack_if_needed(&mut session);
    let stepped = realm_unit(&state(&session), &giant).expect("void walker");
    assert_eq!(
        (
            &stepped["location"],
            &stepped["region"],
            &stepped["occupiedCells"]
        ),
        (
            &json!("A2"),
            &json!("void"),
            &json!(["A2", "A3", "B2", "B3"])
        )
    );
    assert_exact_replay(&session);
}

#[test]
#[expect(
    clippy::too_many_lines,
    reason = "the paired seat and isolated keyword control cases form one boundary proof"
)]
fn airborne_voidwalk_allows_only_surface_departure_diagonal_into_void() {
    for (index, seat) in ["north", "south"].into_iter().enumerate() {
        let walker = minion(json!({ "airborne": true, "voidwalk": true }));
        let (mut session, walker_id, diagonal, _, _) =
            ready_oriented_walker(840 + u32::try_from(index).unwrap(), seat, &walker, false);
        let from = if seat == "north" { "C4" } else { "C1" };
        let path = [format!("{from}/surface"), format!("{diagonal}/void")];
        let movement = |descriptor: &Value| {
            descriptor["kind"] == "move-and-attack"
                && descriptor["unitInstanceId"] == walker_id
                && path_locations(descriptor) == path
        };
        let parent_session = session.clone();
        let parent_fingerprint = public_session_fingerprint(&parent_session);
        let mut branch = parent_session.clone();
        let parent_state = state(&session);
        let old = realm_unit(&parent_state, &walker_id).expect("ready Airborne Voidwalker");
        assert_eq!(old["controller"], seat);
        assert_eq!(old["owner"], seat);
        assert_eq!(old["tapped"], false);
        assert!(offers(&session, movement));

        let checkpoint = create_game_checkpoint(&session).expect("pre-step checkpoint");
        let encoded = serialize_game_checkpoint(&checkpoint).expect("serialized checkpoint");
        let parsed = parse_game_checkpoint(&encoded).expect("parsed checkpoint");
        let mut resumed = resume_game_checkpoint(&parsed).expect("resumed pre-step checkpoint");
        assert_eq!(
            resumed.replay_value().unwrap(),
            session.replay_value().unwrap()
        );
        assert_eq!(
            resumed.legal_actions().unwrap(),
            session.legal_actions().unwrap()
        );

        let (descriptor, receipt) = accept_where(&mut session, movement);
        let (_, resumed_receipt) = accept_where(&mut resumed, movement);
        let (_, branch_receipt) = accept_where(&mut branch, movement);
        assert_eq!(receipt, resumed_receipt);
        assert_eq!(receipt, branch_receipt);
        decline_attack_if_needed(&mut session);
        decline_attack_if_needed(&mut resumed);
        decline_attack_if_needed(&mut branch);
        assert_eq!(descriptor["path"].as_array().unwrap().len(), 2);
        assert_eq!(receipt.events[0].event_type, "move-and-attack-activated");
        let moved = realm_unit(&state(&session), &walker_id).expect("moved Voidwalker");
        assert_eq!(moved["location"], diagonal);
        assert_eq!(moved["region"], "void");
        assert_eq!(moved["controller"], seat);
        assert_eq!(moved["owner"], seat);
        assert_eq!(moved["tapped"], true);
        assert_eq!(moved["instanceId"], walker_id);
        assert_eq!(
            public_session_fingerprint(&parent_session),
            parent_fingerprint
        );
        let expected_fingerprint = public_session_fingerprint(&session);
        assert_eq!(public_session_fingerprint(&resumed), expected_fingerprint);
        assert_eq!(public_session_fingerprint(&branch), expected_fingerprint);
        assert_exact_replay(&session);
        assert_exact_replay(&resumed);
        assert_exact_replay(&branch);
    }

    for (index, seat) in ["north", "south"].into_iter().enumerate() {
        let walker = minion(json!({ "voidwalk": true }));
        let (session, walker_id, diagonal, _, _) =
            ready_oriented_walker(850 + u32::try_from(index).unwrap(), seat, &walker, false);
        assert!(!offers(&session, |descriptor| {
            descriptor["kind"] == "move-and-attack"
                && descriptor["unitInstanceId"] == walker_id
                && path_locations(descriptor).last() == Some(&format!("{diagonal}/void"))
        }));
        assert!(offers(&session, |descriptor| {
            descriptor["kind"] == "move-and-attack"
                && descriptor["unitInstanceId"] == walker_id
                && path_locations(descriptor)
                    .last()
                    .is_some_and(|location| location.ends_with("/void"))
        }));

        let walker = minion(json!({ "airborne": true }));
        let (session, walker_id, diagonal, _, _) =
            ready_oriented_walker(860 + u32::try_from(index).unwrap(), seat, &walker, false);
        assert!(!offers(&session, |descriptor| {
            descriptor["kind"] == "move-and-attack"
                && descriptor["unitInstanceId"] == walker_id
                && path_locations(descriptor).last() == Some(&format!("{diagonal}/void"))
        }));
    }

    for (index, seat) in ["north", "south"].into_iter().enumerate() {
        let walker = minion(json!({ "airborne": true, "voidwalk": true }));
        let (session, walker_id, _, void_source, surface_diagonal) =
            ready_oriented_walker(870 + u32::try_from(index).unwrap(), seat, &walker, true);
        let surface_from_void = |descriptor: &Value| {
            descriptor["kind"] == "move-and-attack"
                && descriptor["unitInstanceId"] == walker_id
                && path_locations(descriptor)
                    == [
                        format!("{void_source}/void"),
                        format!("{surface_diagonal}/surface"),
                    ]
        };
        let diagonal_void = if seat == "north" { "A3" } else { "A1" };
        let void_from_void = |descriptor: &Value| {
            descriptor["kind"] == "move-and-attack"
                && descriptor["unitInstanceId"] == walker_id
                && path_locations(descriptor)
                    == [
                        format!("{void_source}/void"),
                        format!("{diagonal_void}/void"),
                    ]
        };
        assert!(!offers(&session, surface_from_void));
        assert!(!offers(&session, void_from_void));
        assert!(offers(&session, |descriptor| {
            descriptor["kind"] == "move-and-attack"
                && descriptor["unitInstanceId"] == walker_id
                && path_locations(descriptor).len() == 2
                && path_locations(descriptor).last().is_some_and(|location| {
                    location.ends_with("/void") && location != &format!("{diagonal_void}/void")
                })
        }));
    }
}

#[test]
fn airborne_voidwalk_multistep_permission_restarts_after_surface_arrival() {
    for (index, seat) in ["north", "south"].into_iter().enumerate() {
        let walker = minion(json!({
            "airborne": true,
            "movementBonus": 1,
            "voidwalk": true,
        }));
        let (mut session, walker_id, diagonal, _, _) =
            ready_oriented_walker(880 + u32::try_from(index).unwrap(), seat, &walker, false);
        let (from, next_void) = if seat == "north" {
            ("C4", "A3")
        } else {
            ("C1", "A2")
        };
        let two_steps = [
            format!("{from}/surface"),
            format!("{diagonal}/void"),
            format!("{next_void}/void"),
        ];
        let allowed = |descriptor: &Value| {
            descriptor["kind"] == "move-and-attack"
                && descriptor["unitInstanceId"] == walker_id
                && path_locations(descriptor) == two_steps
        };
        let forbidden_second_diagonal = if seat == "north" {
            ["C4/surface", "B3/void", "A2/void"]
        } else {
            ["C1/surface", "B2/void", "A3/void"]
        };
        assert!(!offers(&session, |descriptor| {
            descriptor["kind"] == "move-and-attack"
                && descriptor["unitInstanceId"] == walker_id
                && path_locations(descriptor) == forbidden_second_diagonal
        }));
        assert!(offers(&session, allowed));
        let (descriptor, _) = accept_where(&mut session, allowed);
        assert_eq!(descriptor["path"].as_array().unwrap().len(), 3);
        decline_attack_if_needed(&mut session);
        let moved = realm_unit(&state(&session), &walker_id).expect("two-step walker");
        assert_eq!(moved["location"], next_void);
        assert_eq!(moved["region"], "void");
        assert_exact_replay(&session);

        let (mut session, walker_id, _, void_source, _) =
            ready_oriented_walker(890 + u32::try_from(index).unwrap(), seat, &walker, true);
        let (surface, diagonal_void) = if seat == "north" {
            ("C4", "B3")
        } else {
            ("C2", "B3")
        };
        let restart = [
            format!("{void_source}/void"),
            format!("{surface}/surface"),
            format!("{diagonal_void}/void"),
        ];
        let allowed_after_surface = |descriptor: &Value| {
            descriptor["kind"] == "move-and-attack"
                && descriptor["unitInstanceId"] == walker_id
                && path_locations(descriptor) == restart
        };
        assert!(offers(&session, allowed_after_surface));
        let (descriptor, _) = accept_where(&mut session, allowed_after_surface);
        assert_eq!(descriptor["path"].as_array().unwrap().len(), 3);
        decline_attack_if_needed(&mut session);
        assert_eq!(
            realm_unit(&state(&session), &walker_id).unwrap()["location"],
            diagonal_void
        );
        assert_exact_replay(&session);
    }
}

#[test]
fn airborne_uses_the_effective_planar_gate_voidwalk_loan_until_surface_exit() {
    let mut cards = planar_gate_cards();
    cards["north-spell"]["airborne"] = json!(true);
    let mut session = Session::new(&manifest(900, &cards, "north-site", &["north-spell"; 8]))
        .expect("Airborne Planar Gate scenario");
    keep(&mut session);
    keep(&mut session);
    play_site(&mut session, "C4");
    let (summoned, _) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "summon-minion" && descriptor["cell"] == "C4"
    });
    let walker = summoned["cardInstanceId"]
        .as_str()
        .expect("borrower identity")
        .to_owned();
    pass_turn(&mut session);
    play_site(&mut session, "C1");
    pass_turn(&mut session);

    let diagonal_into_void = |descriptor: &Value| {
        descriptor["kind"] == "move-and-attack"
            && descriptor["unitInstanceId"] == walker
            && path_locations(descriptor) == ["C4/surface", "B3/void"]
    };
    assert!(offers(&session, diagonal_into_void));
    accept_where(&mut session, diagonal_into_void);
    decline_attack_if_needed(&mut session);
    assert_eq!(
        realm_unit(&state(&session), &walker).unwrap()["planarGateVoidwalk"],
        true
    );

    pass_turn(&mut session);
    pass_turn(&mut session);
    play_site(&mut session, "C3");
    assert_eq!(
        realm_unit(&state(&session), &walker).unwrap()["planarGateVoidwalk"],
        true
    );
    let exit_to_surface = |descriptor: &Value| {
        descriptor["kind"] == "move-and-attack"
            && descriptor["unitInstanceId"] == walker
            && path_locations(descriptor) == ["B3/void", "C3/surface"]
    };
    assert!(offers(&session, exit_to_surface));
    accept_where(&mut session, exit_to_surface);
    decline_attack_if_needed(&mut session);
    assert!(
        realm_unit(&state(&session), &walker)
            .unwrap()
            .get("planarGateVoidwalk")
            .is_none()
    );
    assert_exact_replay(&session);

    let cards = planar_gate_cards();
    let mut grounded = Session::new(&manifest(901, &cards, "north-site", &["north-spell"; 8]))
        .expect("grounded Planar Gate control");
    keep(&mut grounded);
    keep(&mut grounded);
    play_site(&mut grounded, "C4");
    let (summoned, _) = accept_where(&mut grounded, |descriptor| {
        descriptor["kind"] == "summon-minion" && descriptor["cell"] == "C4"
    });
    let grounded_id = summoned["cardInstanceId"].as_str().unwrap().to_owned();
    pass_turn(&mut grounded);
    play_site(&mut grounded, "C1");
    pass_turn(&mut grounded);
    assert!(!offers(&grounded, |descriptor| {
        descriptor["kind"] == "move-and-attack"
            && descriptor["unitInstanceId"] == grounded_id
            && path_locations(descriptor) == ["C4/surface", "B3/void"]
    }));
    assert!(offers(&grounded, |descriptor| {
        descriptor["kind"] == "move-and-attack"
            && descriptor["unitInstanceId"] == grounded_id
            && path_locations(descriptor) == ["C4/surface", "B4/void"]
    }));
}

#[test]
fn rule_catalog_0918_playing_site_on_void_square_banishes_oversized_voidwalk_footprint() {
    let cards = voidwalk_cards(&oversized_voidwalker(), &site(&["earth"]));
    let mut session = Session::new(&manifest(141, &cards, "north-site", &["north-spell"; 8]))
        .expect("valid oversized void site-cover scenario");
    keep(&mut session);
    keep(&mut session);
    play_site(&mut session, "C4");
    let (summoned, _) = accept_where(
        &mut session,
        is_square_void_summon_at("A1", &["A1", "A2", "B1", "B2"]),
    );
    let giant = summoned["cardInstanceId"]
        .as_str()
        .expect("summoned identity")
        .to_owned();
    pass_turn(&mut session);
    play_site(&mut session, "C1");
    pass_turn_with_site_draw(&mut session);
    play_site(&mut session, "B4");
    pass_turn_with_site_draw(&mut session);
    play_site(&mut session, "C2");
    pass_turn_with_site_draw(&mut session);
    play_site(&mut session, "B3");
    pass_turn_with_site_draw(&mut session);
    play_site(&mut session, "D1");
    pass_turn_with_site_draw(&mut session);
    // Cover a non-anchor void cell the 2×2 still occupies; settlement banishes the stranded walker.
    let (_, receipt) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "B2"
    });
    assert_eq!(event_types(&receipt), ["site-played", "minion-banished"]);
    let after = state(&session);
    assert!(realm_unit(&after, &giant).is_none());
    assert!(!in_cemetery(&after, "north", &giant));
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_0902_flooded_earth_routes_secret_tunnel_hop_underwater() {
    let mut session =
        Session::new(&tunnel_flood_manifest(105)).expect("valid flooded Secret Tunnel scenario");
    keep(&mut session);
    keep(&mut session);
    play_site_card(&mut session, "north-tunnel", "C4");
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
    play_site_card(&mut session, "south-site", "C1");
    end_turn(&mut session);
    draw_spell(&mut session);
    play_site_card(&mut session, "north-land", "C3");
    cast_flood_covering(&mut session, "C3");
    end_turn(&mut session);
    draw_spell(&mut session);
    end_turn(&mut session);
    draw_spell(&mut session);

    let hop = |descriptor: &Value| {
        descriptor["kind"] == "move-and-attack"
            && descriptor["unitInstanceId"] == crosser_id.as_str()
            && underground_path(descriptor) == "C4/underground,C3/underwater"
    };
    assert!(offers(&session, hop));
    accept_where(&mut session, hop);
    decline_attack_if_needed(&mut session);
    let landed = realm_unit(&state(&session), &crosser_id).expect("flooded hop occupant");
    assert_eq!(
        (&landed["location"], &landed["region"]),
        (&json!("C3"), &json!("underwater"))
    );
    assert_exact_replay(&session);
}
