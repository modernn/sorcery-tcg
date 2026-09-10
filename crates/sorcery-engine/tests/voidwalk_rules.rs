//! Direct proofs for the void: the summons and steps Voidwalk grants (RULE-CATALOG-0119), the
//! temporary Voidwalk a Planar Gate lends until a minion leaves the void (RULE-CATALOG-0120), the
//! outer-column cast restriction that filters those summons but not movement (RULE-CATALOG-0121),
//! the settlement that kills inhospitable minions and banishes stranded void occupants
//! (RULE-CATALOG-0050), and the loose Artifacts a newly played site lifts out of the void it
//! covers (RULE-CATALOG-0051).

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
            && descriptor["atlasOrder"] == json!([])
            && descriptor["spellbookOrder"] == json!([])
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
fn waterbound_voidwalk_should_stop_at_the_first_inhospitable_void_cell() {
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
