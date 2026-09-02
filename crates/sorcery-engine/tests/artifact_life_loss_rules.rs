//! Direct proofs for the life an Artifact costs its current site controller as each turn ends
//! (RULE-CATALOG-0155), the carried cell that loss follows and the bearer Disable it survives
//! (RULE-CATALOG-0156), and the regions, Rubble, stacking, and Death's Door it respects
//! (RULE-CATALOG-0157).

use serde_json::{Value, json};
use sorcery_engine::canonical::{IdentityHash, canonical_json, identity_hash};
use sorcery_engine::contract::{ActionRequest, Receipt};
use sorcery_engine::session::{Session, StepResult};

fn avatar(life: u64, extra: Value) -> Value {
    let mut value = json!({
        "attack": 1,
        "cardType": "avatar",
        "defense": 1,
        "drawSpell": false,
        "life": life,
    });
    let Value::Object(extra) = extra else {
        panic!("extra avatar facts must be an object");
    };
    value.as_object_mut().expect("avatar facts").extend(extra);
    value
}

fn site(elements: &[&str]) -> Value {
    json!({ "cardType": "site", "elements": elements })
}

fn minion(extra: Value) -> Value {
    let mut value = json!({
        "attack": 1,
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

fn magic(extra: Value) -> Value {
    let mut value = json!({
        "cardType": "magic",
        "manaCost": 0,
        "thresholds": { "air": 0, "earth": 0, "fire": 0, "water": 0 },
    });
    let Value::Object(extra) = extra else {
        panic!("extra Magic facts must be an object");
    };
    value.as_object_mut().expect("Magic facts").extend(extra);
    value
}

/// A Devil's Egg: the Artifact whose site controller loses `amount` life as each turn ends.
fn devils_egg(amount: u64) -> Value {
    json!({
        "atEndOfEachTurnSiteControllerLosesLife": amount,
        "cardType": "artifact",
        "manaCost": 0,
        "thresholds": { "air": 0, "earth": 0, "fire": 0, "water": 0 },
    })
}

/// A Lucky Charm: an Artifact effect this slice still refuses to honor.
fn lucky_charm() -> Value {
    json!({
        "bearerControllerChoosesExtraRandomOutcome": true,
        "cardType": "artifact",
        "manaCost": 0,
        "thresholds": { "air": 0, "earth": 0, "fire": 0, "water": 0 },
    })
}

fn manifest(
    seed: u32,
    cards: &Value,
    north_spellbook: &[&str],
    south_spellbook: &[&str],
) -> String {
    let mut value = json!({
        "authority": {
            "contentHash": identity_hash(&json!({ "fixture": "artifact-life-loss" }))
                .expect("synthetic authority identity"),
            "mode": "synthetic",
            "revisionId": "synthetic-artifact-life-loss-v1",
        },
        "cards": cards,
        "decks": {
            "north": {
                "atlas": vec!["north-site"; 6],
                "avatar": "north-avatar",
                "spellbook": north_spellbook,
            },
            "south": {
                "atlas": vec!["south-site"; 6],
                "avatar": "south-avatar",
                "spellbook": south_spellbook,
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

fn payloads_of(receipt: &Receipt, event_type: &str) -> Vec<Value> {
    receipt
        .events
        .iter()
        .filter(|event| event.event_type == event_type)
        .map(|event| event.payload.clone())
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

fn life(current: &Value, seat: &str) -> Value {
    current["players"][seat]["avatar"]["life"].clone()
}

fn site_instance_id(current: &Value, cell: &str) -> Value {
    current["realm"]["sites"][cell]["instanceId"].clone()
}

fn realm_artifacts(current: &Value) -> Vec<Value> {
    current["realm"]["artifacts"]
        .as_array()
        .cloned()
        .unwrap_or_default()
}

/// The realm unit with this identity, absent once it has died.
fn realm_unit(current: &Value, instance_id: &str) -> Option<Value> {
    current["realm"]["units"]
        .as_array()
        .expect("realm units")
        .iter()
        .find(|unit| unit["instanceId"] == instance_id)
        .cloned()
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

fn draw_site(session: &mut Session) {
    accept_where(session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "atlas"
    });
}

fn end_turn(session: &mut Session) -> Receipt {
    accept_where(session, |descriptor| descriptor["kind"] == "end-turn").1
}

fn summon(session: &mut Session, card_id: &str, cell: &str) -> String {
    let (summoned, _) = accept_where(session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["cardId"] == card_id
            && descriptor["cell"] == cell
    });
    summoned["cardInstanceId"]
        .as_str()
        .expect("summoned identity")
        .to_owned()
}

/// Conjures one Artifact loose on a controlled site and returns its realm identity.
fn cast_loose_artifact(session: &mut Session, card_id: &str, cell: &str) -> String {
    let (cast, _) = accept_where(session, |descriptor| {
        descriptor["kind"] == "cast-artifact"
            && descriptor["cardId"] == card_id
            && descriptor["cell"] == cell
    });
    cast["cardInstanceId"]
        .as_str()
        .expect("conjured identity")
        .to_owned()
}

/// Conjures one Artifact onto a local bearer and returns its realm identity.
fn cast_carried_artifact(session: &mut Session, card_id: &str, bearer_instance_id: &str) -> String {
    let (cast, _) = accept_where(session, |descriptor| {
        descriptor["kind"] == "cast-artifact"
            && descriptor["cardId"] == card_id
            && descriptor["bearer"]["instanceId"] == bearer_instance_id
    });
    cast["cardInstanceId"]
        .as_str()
        .expect("conjured identity")
        .to_owned()
}

/// Walks one unit a single cardinal step and closes the attack window it opens.
fn step_unit(session: &mut Session, instance_id: &str, from: &str, to: &str) {
    accept_where(session, |descriptor| {
        descriptor["kind"] == "move-and-attack"
            && descriptor["unitInstanceId"] == instance_id
            && descriptor["from"]["cell"] == from
            && descriptor["to"]["cell"] == to
    });
    if !descriptors_of_kind(session, "decline-attack").is_empty() {
        accept_where(session, |descriptor| descriptor["kind"] == "decline-attack");
    }
}

fn devils_egg_cards() -> Value {
    json!({
        "north-avatar": avatar(20, json!({})),
        "north-egg": devils_egg(1),
        "north-site": site(&["earth"]),
        "south-avatar": avatar(20, json!({})),
        "south-egg": devils_egg(1),
        "south-site": site(&["earth"]),
    })
}

#[test]
#[expect(
    clippy::too_many_lines,
    reason = "one direct rule proof keeps both seats' charges and their exact ordering together"
)]
fn end_turn_artifacts_should_cost_their_current_site_controller_life() {
    let mut unbounded = devils_egg_cards();
    unbounded["north-egg"]["atEndOfEachTurnSiteControllerLosesLife"] = json!(0);
    let rejected = Session::new(&manifest(
        73,
        &unbounded,
        &["north-egg"; 6],
        &["south-egg"; 6],
    ))
    .map(|_| ())
    .expect_err("a life loss below one is not a supported Artifact fact");
    assert!(
        rejected
            .to_string()
            .contains("atEndOfEachTurnSiteControllerLosesLife must be between 1 and 100"),
        "unexpected rejection: {rejected}"
    );

    let mut session = Session::new(&manifest(
        73,
        &devils_egg_cards(),
        &["north-egg"; 6],
        &["south-egg"; 6],
    ))
    .expect("valid Devil's Egg scenario");
    keep(&mut session);
    keep(&mut session);
    play_site(&mut session, "C4");
    let north_egg = cast_loose_artifact(&mut session, "north-egg", "C4");
    let north_site = site_instance_id(&state(&session), "C4");

    let north_ended = end_turn(&mut session);
    assert_eq!(
        event_types(&north_ended),
        [
            "end-turn-site-life-loss-triggered",
            "avatar-life-lost",
            "turn-ended",
            "turn-started",
        ]
    );
    assert_eq!(
        north_ended.events[..2]
            .iter()
            .map(|event| event.payload.clone())
            .collect::<Vec<_>>(),
        [
            json!({
                "amount": 1,
                "seat": "north",
                "siteInstanceId": north_site,
                "sourceInstanceId": north_egg,
            }),
            json!({
                "amount": 1,
                "life": 19,
                "seat": "north",
                "sourceInstanceId": north_egg,
            }),
        ]
    );
    assert!(north_ended.random_draws.is_empty());

    draw_site(&mut session);
    play_site(&mut session, "C1");
    let south_egg = cast_loose_artifact(&mut session, "south-egg", "C1");
    let south_site = site_instance_id(&state(&session), "C1");

    // The ending turn charges the waiting seat's Artifacts first, then the active seat's.
    let south_ended = end_turn(&mut session);
    assert_eq!(
        event_types(&south_ended),
        [
            "end-turn-site-life-loss-triggered",
            "avatar-life-lost",
            "end-turn-site-life-loss-triggered",
            "avatar-life-lost",
            "turn-ended",
            "turn-started",
        ]
    );
    assert_eq!(
        south_ended.events[..4]
            .iter()
            .map(|event| event.payload.clone())
            .collect::<Vec<_>>(),
        [
            json!({
                "amount": 1,
                "seat": "north",
                "siteInstanceId": north_site,
                "sourceInstanceId": north_egg,
            }),
            json!({
                "amount": 1,
                "life": 18,
                "seat": "north",
                "sourceInstanceId": north_egg,
            }),
            json!({
                "amount": 1,
                "seat": "south",
                "siteInstanceId": south_site,
                "sourceInstanceId": south_egg,
            }),
            json!({
                "amount": 1,
                "life": 19,
                "seat": "south",
                "sourceInstanceId": south_egg,
            }),
        ]
    );
    let ended = state(&session);
    assert_eq!(
        (life(&ended, "north"), life(&ended, "south")),
        (json!(18), json!(19))
    );
    assert!(south_ended.random_draws.is_empty());
    assert_exact_replay(&session);
    unmodeled_artifact_effects_should_offer_no_conjuration();
}

/// The remaining Artifact effects stay inert instead of becoming silent no-ops in play.
fn unmodeled_artifact_effects_should_offer_no_conjuration() {
    let cards = json!({
        "north-avatar": avatar(20, json!({})),
        "north-charm": lucky_charm(),
        "north-site": site(&["earth"]),
        "south-avatar": avatar(20, json!({})),
        "south-egg": devils_egg(1),
        "south-site": site(&["earth"]),
    });
    let mut session = Session::new(&manifest(
        74,
        &cards,
        &["north-charm"; 6],
        &["south-egg"; 6],
    ))
    .expect("a Lucky Charm is an admitted fact");
    keep(&mut session);
    keep(&mut session);
    play_site(&mut session, "C4");
    assert!(descriptors_of_kind(&session, "cast-artifact").is_empty());
    let quiet = end_turn(&mut session);
    assert_eq!(event_types(&quiet), ["turn-ended", "turn-started"]);
}

fn carried_egg_cards(bearer: &Value) -> Value {
    json!({
        "north-avatar": avatar(20, json!({})),
        "north-bearer": bearer.clone(),
        "north-egg": devils_egg(1),
        "north-freeze": magic(json!({ "disableTargetNearbyMinionUntilNextTurn": true })),
        "north-site": site(&["earth"]),
        "south-avatar": avatar(20, json!({})),
        "south-filler": minion(json!({})),
        "south-site": site(&["earth"]),
    })
}

#[test]
fn end_turn_artifact_life_loss_should_use_its_carried_cell_and_survive_bearer_disable() {
    let mut session = Session::new(&manifest(
        4,
        &carried_egg_cards(&minion(json!({}))),
        &["north-bearer", "north-egg", "north-freeze"],
        &["south-filler"; 3],
    ))
    .expect("valid carried Devil's Egg scenario");
    keep(&mut session);
    keep(&mut session);
    play_site(&mut session, "C4");
    let bearer = summon(&mut session, "north-bearer", "C4");
    let egg = cast_carried_artifact(&mut session, "north-egg", &bearer);
    let opening = state(&session);
    assert_eq!(
        realm_artifacts(&opening)
            .into_iter()
            .map(|artifact| artifact["bearer"]["instanceId"].clone())
            .collect::<Vec<_>>(),
        [json!(bearer)]
    );
    let home_site = site_instance_id(&opening, "C4");

    let first_end = end_turn(&mut session);
    assert_eq!(
        payloads_of(&first_end, "end-turn-site-life-loss-triggered"),
        [json!({
            "amount": 1,
            "seat": "north",
            "siteInstanceId": home_site,
            "sourceInstanceId": egg,
        })]
    );

    draw_site(&mut session);
    play_site(&mut session, "C1");
    end_turn(&mut session);

    draw_site(&mut session);
    play_site(&mut session, "C3");
    step_unit(&mut session, &bearer, "C4", "C3");
    let moved = state(&session);
    let walked_site = site_instance_id(&moved, "C3");
    assert_ne!(walked_site, home_site);

    // The loss follows the cell the bearer now stands on rather than where the Egg was conjured.
    let walked_end = end_turn(&mut session);
    assert_eq!(
        payloads_of(&walked_end, "end-turn-site-life-loss-triggered"),
        [json!({
            "amount": 1,
            "seat": "north",
            "siteInstanceId": walked_site,
            "sourceInstanceId": egg,
        })]
    );
    let walked = state(&session);
    assert_eq!(life(&walked, "north"), json!(17));
    assert_exact_replay(&session);

    carried_egg_should_outlast_a_disabled_bearer();
    carried_egg_should_charge_before_its_bearer_dies();
}

/// A Disabled bearer stops dying and acting, but the Artifact it holds keeps charging its site.
fn carried_egg_should_outlast_a_disabled_bearer() {
    let bearer = minion(json!({ "diesAtEndOfControllerTurn": true }));
    let mut session = Session::new(&manifest(
        5,
        &carried_egg_cards(&bearer),
        &["north-bearer", "north-egg", "north-freeze"],
        &["south-filler"; 3],
    ))
    .expect("valid Disabled bearer scenario");
    keep(&mut session);
    keep(&mut session);
    play_site(&mut session, "C4");
    let bearer_id = summon(&mut session, "north-bearer", "C4");
    let egg = cast_carried_artifact(&mut session, "north-egg", &bearer_id);
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "cast-magic"
            && descriptor["cardId"] == "north-freeze"
            && descriptor["target"]["instanceId"] == bearer_id
    });
    let frozen = state(&session);
    assert_eq!(
        realm_unit(&frozen, &bearer_id).expect("Disabled bearer")["disableEffects"]
            .as_array()
            .map(Vec::len),
        Some(1)
    );
    let home_site = site_instance_id(&frozen, "C4");

    let ended = end_turn(&mut session);
    assert_eq!(
        event_types(&ended),
        [
            "end-turn-site-life-loss-triggered",
            "avatar-life-lost",
            "turn-ended",
            "turn-started",
        ]
    );
    assert_eq!(
        ended.events[0].payload,
        json!({
            "amount": 1,
            "seat": "north",
            "siteInstanceId": home_site,
            "sourceInstanceId": egg,
        })
    );
    let after = state(&session);
    assert!(realm_unit(&after, &bearer_id).is_some());
    assert_eq!(life(&after, "north"), json!(19));
    assert_eq!(
        realm_artifacts(&after)
            .into_iter()
            .map(|artifact| artifact["bearer"]["instanceId"].clone())
            .collect::<Vec<_>>(),
        [json!(bearer_id)]
    );
    assert_exact_replay(&session);
}

/// The charge lands before the end-phase deaths that drop the Artifact where its bearer fell.
fn carried_egg_should_charge_before_its_bearer_dies() {
    let bearer = minion(json!({ "diesAtEndOfControllerTurn": true }));
    let mut session = Session::new(&manifest(
        6,
        &carried_egg_cards(&bearer),
        &["north-bearer", "north-egg", "north-freeze"],
        &["south-filler"; 3],
    ))
    .expect("valid dying bearer scenario");
    keep(&mut session);
    keep(&mut session);
    play_site(&mut session, "C4");
    let bearer_id = summon(&mut session, "north-bearer", "C4");
    let egg = cast_carried_artifact(&mut session, "north-egg", &bearer_id);

    let ended = end_turn(&mut session);
    assert_eq!(
        event_types(&ended),
        [
            "end-turn-site-life-loss-triggered",
            "avatar-life-lost",
            "artifact-dropped",
            "minion-died",
            "turn-ended",
            "turn-started",
        ]
    );
    let after = state(&session);
    assert!(realm_unit(&after, &bearer_id).is_none());
    assert_eq!(
        realm_artifacts(&after)
            .into_iter()
            .map(|artifact| {
                json!({
                    "bearer": artifact["bearer"].clone(),
                    "instanceId": artifact["instanceId"].clone(),
                    "location": artifact["location"].clone(),
                    "region": artifact["region"].clone(),
                })
            })
            .collect::<Vec<_>>(),
        [json!({
            "bearer": Value::Null,
            "instanceId": egg,
            "location": "C4",
            "region": "surface",
        })]
    );
    assert_eq!(life(&after, "north"), json!(19));
    assert_exact_replay(&session);
}

#[test]
fn end_turn_artifact_life_loss_should_respect_regions_rubble_stacking_and_deaths_door() {
    let cards = json!({
        "north-avatar": avatar(2, json!({})),
        "north-egg": devils_egg(1),
        "north-site": site(&["earth"]),
        "south-avatar": avatar(20, json!({})),
        "south-filler": minion(json!({})),
        "south-site": site(&["earth"]),
    });
    let mut session = Session::new(&manifest(
        91,
        &cards,
        &["north-egg"; 6],
        &["south-filler"; 3],
    ))
    .expect("valid stacked Devil's Egg scenario");
    keep(&mut session);
    keep(&mut session);
    play_site(&mut session, "C4");
    let mut eggs: Vec<String> = (0..3)
        .map(|_| cast_loose_artifact(&mut session, "north-egg", "C4"))
        .collect();
    eggs.sort();
    let stacked = state(&session);
    let home_site = site_instance_id(&stacked, "C4");
    let turn_number = stacked["turnNumber"].clone();

    // Three Eggs charge one site three separate times, and the third finds nothing left to take.
    let ended = end_turn(&mut session);
    assert_eq!(
        event_types(&ended),
        [
            "end-turn-site-life-loss-triggered",
            "avatar-life-lost",
            "end-turn-site-life-loss-triggered",
            "avatar-life-lost",
            "avatar-reached-deaths-door",
            "end-turn-site-life-loss-triggered",
            "turn-ended",
            "turn-started",
        ]
    );
    assert_eq!(
        payloads_of(&ended, "end-turn-site-life-loss-triggered"),
        eggs.iter()
            .map(|egg| json!({
                "amount": 1,
                "seat": "north",
                "siteInstanceId": home_site,
                "sourceInstanceId": egg,
            }))
            .collect::<Vec<_>>()
    );
    assert_eq!(
        payloads_of(&ended, "avatar-reached-deaths-door"),
        [json!({
            "seat": "north",
            "sourceInstanceId": eggs[1],
            "turnNumber": turn_number,
        })]
    );
    let door = state(&session);
    assert_eq!(life(&door, "north"), json!(0));
    assert_eq!(
        door["players"]["north"]["avatar"]["deathDoorTurn"],
        turn_number
    );
    assert_eq!(door["terminal"]["status"], "active");

    // An Avatar already on Death's Door only records the trigger: the charge is not damage.
    draw_site(&mut session);
    play_site(&mut session, "C1");
    let repeated = end_turn(&mut session);
    assert_eq!(
        event_types(&repeated),
        [
            "end-turn-site-life-loss-triggered",
            "end-turn-site-life-loss-triggered",
            "end-turn-site-life-loss-triggered",
            "turn-ended",
            "turn-started",
        ]
    );
    let survived = state(&session);
    assert_eq!(life(&survived, "north"), json!(0));
    assert_eq!(survived["terminal"]["status"], "active");
    assert_exact_replay(&session);

    end_turn_artifact_life_loss_should_skip_rubble();
    end_turn_artifact_life_loss_should_charge_a_submerged_bearers_site();
}

/// Rubble has no controller, so an Artifact resting on it charges nobody.
fn end_turn_artifact_life_loss_should_skip_rubble() {
    let cards = json!({
        "north-avatar": avatar(20, json!({ "earthSitePlayCreatesAdjacentRubble": true })),
        "north-bearer": minion(json!({})),
        "north-egg": devils_egg(1),
        "north-site": site(&["earth"]),
        "south-avatar": avatar(20, json!({})),
        "south-filler": minion(json!({})),
        "south-site": site(&["earth"]),
    });
    let mut session = Session::new(&manifest(
        92,
        &cards,
        &["north-bearer", "north-egg", "north-egg"],
        &["south-filler"; 3],
    ))
    .expect("valid Rubble scenario");
    keep(&mut session);
    keep(&mut session);
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site"
            && descriptor["cell"] == "C4"
            && descriptor["createRubbleAt"] == "C3"
    });
    let bearer = summon(&mut session, "north-bearer", "C4");
    let egg = cast_carried_artifact(&mut session, "north-egg", &bearer);
    let opening = state(&session);
    assert_eq!(opening["realm"]["sites"]["C3"]["rubble"], json!(true));
    let home_site = site_instance_id(&opening, "C4");

    let first_end = end_turn(&mut session);
    assert_eq!(
        payloads_of(&first_end, "end-turn-site-life-loss-triggered"),
        [json!({
            "amount": 1,
            "seat": "north",
            "siteInstanceId": home_site,
            "sourceInstanceId": egg,
        })]
    );

    draw_site(&mut session);
    play_site(&mut session, "C1");
    end_turn(&mut session);

    draw_site(&mut session);
    step_unit(&mut session, &bearer, "C4", "C3");
    let rubbled = end_turn(&mut session);
    assert_eq!(event_types(&rubbled), ["turn-ended", "turn-started"]);
    let after = state(&session);
    assert_eq!(life(&after, "north"), json!(18));
    assert_eq!(
        realm_unit(&after, &bearer).expect("bearer on Rubble")["location"],
        json!("C3")
    );
    assert_exact_replay(&session);
}

/// Only the Void escapes the charge: a subsurface Artifact still charges the site above it.
fn end_turn_artifact_life_loss_should_charge_a_submerged_bearers_site() {
    let cards = json!({
        "north-avatar": avatar(20, json!({})),
        "north-bearer": minion(json!({ "submerge": true })),
        "north-egg": devils_egg(1),
        "north-site": site(&["earth", "water"]),
        "south-avatar": avatar(20, json!({})),
        "south-drown": magic(json!({ "submergeTargetMinion": true })),
        "south-site": site(&["earth"]),
    });
    let mut session = Session::new(&manifest(
        93,
        &cards,
        &["north-bearer", "north-egg", "north-egg"],
        &["south-drown"; 3],
    ))
    .expect("valid submerged bearer scenario");
    keep(&mut session);
    keep(&mut session);
    play_site(&mut session, "C4");
    let bearer = summon(&mut session, "north-bearer", "C4");
    let egg = cast_carried_artifact(&mut session, "north-egg", &bearer);
    let home_site = site_instance_id(&state(&session), "C4");
    end_turn(&mut session);

    draw_site(&mut session);
    play_site(&mut session, "C1");
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "cast-magic"
            && descriptor["cardId"] == "south-drown"
            && descriptor["target"]["instanceId"] == bearer
    });
    let submerged = state(&session);
    assert_eq!(
        realm_unit(&submerged, &bearer).expect("submerged bearer")["region"],
        json!("underwater")
    );
    assert_eq!(
        realm_artifacts(&submerged)
            .into_iter()
            .map(|artifact| artifact["bearer"]["instanceId"].clone())
            .collect::<Vec<_>>(),
        [json!(bearer)]
    );

    let ended = end_turn(&mut session);
    assert_eq!(
        payloads_of(&ended, "end-turn-site-life-loss-triggered"),
        [json!({
            "amount": 1,
            "seat": "north",
            "siteInstanceId": home_site,
            "sourceInstanceId": egg,
        })]
    );
    let after = state(&session);
    assert_eq!(life(&after, "north"), json!(18));
    assert_eq!(life(&after, "south"), json!(20));
    assert_exact_replay(&session);
}
