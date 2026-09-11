//! Direct proofs for free-summon prospective power (RULE-CATALOG-0329–0330).
//!
//! Cemetery free placement uses the power the minion would have on that cell or
//! 2x2 footprint, not printed attack. A printed-1 minion that would become 3
//! atop a Tower cannot Raise Dead onto a threshold-3 Tower. A 2x2 cannot occupy
//! a square that includes a threshold site just because another cell of the
//! square is a Tower that would raise its power.

use serde_json::{Value, json};
use sorcery_engine::canonical::identity_hash;
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

fn site() -> Value {
    json!({ "cardType": "site", "elements": ["earth"] })
}

fn tower() -> Value {
    json!({
        "cardType": "site",
        "elements": ["earth"],
        "isTower": true,
    })
}

fn blocked() -> Value {
    json!({
        "cardType": "site",
        "elements": ["earth"],
        "preventsUnitsWithPowerAtLeastFromEntering": 3,
    })
}

fn tower_blocked() -> Value {
    json!({
        "cardType": "site",
        "elements": ["earth"],
        "isTower": true,
        "preventsUnitsWithPowerAtLeastFromEntering": 3,
    })
}

fn raise_dead() -> Value {
    json!({
        "cardType": "magic",
        "manaCost": 0,
        "summonRandomMinionFromAnyCemetery": true,
        "thresholds": { "air": 0, "earth": 0, "fire": 0, "water": 0 },
    })
}

fn dummy() -> Value {
    json!({
        "attack": 1,
        "cardType": "minion",
        "defense": 1,
        "manaCost": 0,
        "thresholds": { "air": 0, "earth": 0, "fire": 0, "water": 0 },
    })
}

fn corpse(occupies_square: bool) -> Value {
    let mut value = json!({
        "attack": 1,
        "cardType": "minion",
        "defense": 1,
        "diesAtEndOfControllerTurn": true,
        "gainsPowerRangedAndSpellcasterAtopTower": 2,
        "manaCost": 0,
        "thresholds": { "air": 0, "earth": 0, "fire": 0, "water": 0 },
    });
    if occupies_square {
        value["occupiesSquareArea"] = json!(2);
    }
    value
}

const OPEN_SQUARE: [&str; 4] = ["B3", "B4", "C3", "C4"];
const MIXED_SQUARE: [&str; 4] = ["A3", "A4", "B3", "B4"];

fn finish_manifest(mut value: Value) -> String {
    value["manifestId"] = json!(identity_hash(&value).expect("manifest identity"));
    sorcery_engine::canonical::canonical_json(&value).expect("canonical synthetic manifest")
}

fn tower_manifest(seed: u32) -> String {
    finish_manifest(json!({
        "authority": {
            "contentHash": identity_hash(&json!({ "fixture": "free-summon-tower" }))
                .expect("synthetic authority identity"),
            "mode": "synthetic",
            "revisionId": "synthetic-free-summon-tower-v1",
        },
        "cards": {
            "north-avatar": avatar(),
            "north-corpse": corpse(false),
            "north-open": site(),
            "north-raise": raise_dead(),
            "north-tower": tower_blocked(),
            "south-avatar": avatar(),
            "south-dummy": dummy(),
            "south-site": site(),
        },
        "decks": {
            "north": {
                "atlas": [
                    "north-open",
                    "north-tower",
                    "north-open",
                    "north-tower",
                    "north-open",
                    "north-tower",
                ],
                "avatar": "north-avatar",
                "spellbook": [
                    "north-corpse",
                    "north-raise",
                    "north-corpse",
                    "north-raise",
                    "north-corpse",
                    "north-raise",
                ],
            },
            "south": {
                "atlas": vec!["south-site"; 6],
                "avatar": "south-avatar",
                "spellbook": vec!["south-dummy"; 6],
            },
        },
        "engineVersion": "sorcery-core-v1",
        "firstSeat": "north",
        "schemaVersion": 1,
        "seed": seed,
    }))
}

fn square_manifest(seed: u32) -> String {
    finish_manifest(json!({
        "authority": {
            "contentHash": identity_hash(&json!({ "fixture": "free-summon-square" }))
                .expect("synthetic authority identity"),
            "mode": "synthetic",
            "revisionId": "synthetic-free-summon-square-v1",
        },
        "cards": {
            "north-avatar": avatar(),
            "north-blocked": blocked(),
            "north-corpse": corpse(true),
            "north-open": site(),
            "north-raise": raise_dead(),
            "north-tower": tower(),
            "south-avatar": avatar(),
            "south-dummy": dummy(),
            "south-site": site(),
        },
        "decks": {
            "north": {
                "atlas": [
                    "north-open",
                    "north-open",
                    "north-open",
                    "north-open",
                    "north-open",
                    "north-open",
                    "north-open",
                    "north-open",
                    "north-tower",
                    "north-blocked",
                    "north-open",
                    "north-open",
                    "north-tower",
                    "north-blocked",
                ],
                "avatar": "north-avatar",
                "spellbook": [
                    "north-corpse",
                    "north-raise",
                    "north-corpse",
                    "north-raise",
                    "north-corpse",
                    "north-raise",
                    "north-corpse",
                    "north-raise",
                ],
            },
            "south": {
                "atlas": vec!["south-site"; 10],
                "avatar": "south-avatar",
                "spellbook": vec!["south-dummy"; 8],
            },
        },
        "engineVersion": "sorcery-core-v1",
        "firstSeat": "north",
        "schemaVersion": 1,
        "seed": seed,
    }))
}

fn accept_where(session: &mut Session, predicate: impl Fn(&Value) -> bool) -> (Value, Receipt) {
    let action = session
        .legal_actions()
        .expect("legal actions")
        .into_iter()
        .find(|action| predicate(&action.descriptor))
        .unwrap_or_else(|| {
            panic!(
                "expected engine-issued action among {:?}",
                session
                    .legal_actions()
                    .expect("legal actions")
                    .iter()
                    .map(|action| action.descriptor.clone())
                    .collect::<Vec<_>>()
            )
        });
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

fn offers(session: &Session, predicate: impl Fn(&Value) -> bool) -> bool {
    session
        .legal_actions()
        .expect("legal actions")
        .iter()
        .any(|action| predicate(&action.descriptor))
}

fn opening_ids(session: &Session, zone: &str) -> Vec<String> {
    session.replay_value().expect("authoritative replay")["state"]["players"]["north"]["hand"][zone]
        .as_array()
        .expect("north hand zone")
        .iter()
        .filter_map(|card| card["cardId"].as_str().map(ToOwned::to_owned))
        .collect()
}

fn end_and_draw(session: &mut Session, zone: &str) {
    accept_where(session, |descriptor| descriptor["kind"] == "end-turn");
    accept_where(session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == zone
    });
}

fn play_named_site(session: &mut Session, card_id: &str, cell: &str) {
    accept_where(session, |descriptor| {
        descriptor["kind"] == "play-site"
            && descriptor["cardId"] == card_id
            && descriptor["cell"] == cell
    });
}

fn can_play_named_site(session: &Session, card_id: &str, cell: &str) -> bool {
    offers(session, |descriptor| {
        descriptor["kind"] == "play-site"
            && descriptor["cardId"] == card_id
            && descriptor["cell"] == cell
    })
}

fn raise_on(card_id: &str, cell: &str) -> impl Fn(&Value) -> bool + '_ {
    move |descriptor: &Value| {
        descriptor["kind"] == "summon-minion"
            && descriptor["cardId"] == card_id
            && descriptor["cell"] == cell
            && descriptor["manaCost"] == 0
    }
}

fn raise_square(card_id: &str, cells: &'static [&str; 4]) -> impl Fn(&Value) -> bool + '_ {
    move |descriptor: &Value| {
        descriptor["kind"] == "summon-minion"
            && descriptor["cardId"] == card_id
            && descriptor["cells"] == json!(cells)
            && descriptor["manaCost"] == 0
    }
}

fn assert_exact_replay(session: &Session) {
    let action_ids: Vec<_> = session
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

fn tower_opening() -> Session {
    (1..=4096)
        .map(tower_manifest)
        .find_map(|candidate| {
            let session = Session::new(&candidate).expect("free-summon Tower candidate");
            let atlas = opening_ids(&session, "atlas");
            let spells = opening_ids(&session, "spellbook");
            (atlas.contains(&"north-open".to_owned())
                && atlas.contains(&"north-tower".to_owned())
                && spells.contains(&"north-corpse".to_owned())
                && spells.contains(&"north-raise".to_owned()))
            .then_some(session)
        })
        .expect("bounded seed opening with Raise Dead, a Tower-bonus corpse, and both sites")
}

fn south_plays_c1_and_passes(session: &mut Session) {
    end_and_draw(session, "atlas");
    accept_where(session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C1"
    });
    end_and_draw(session, "spellbook");
}

fn south_passes_north_draws_atlas(session: &mut Session) {
    end_and_draw(session, "spellbook");
    end_and_draw(session, "atlas");
}

fn try_square_setup(mut session: Session) -> Option<Session> {
    keep(&mut session);
    keep(&mut session);
    if !can_play_named_site(&session, "north-open", "C4") {
        return None;
    }
    play_named_site(&mut session, "north-open", "C4");
    end_and_draw(&mut session, "atlas");
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C1"
    });
    end_and_draw(&mut session, "atlas");
    if !can_play_named_site(&session, "north-open", "B4") {
        return None;
    }
    play_named_site(&mut session, "north-open", "B4");
    south_passes_north_draws_atlas(&mut session);
    if !can_play_named_site(&session, "north-open", "C3") {
        return None;
    }
    play_named_site(&mut session, "north-open", "C3");
    south_passes_north_draws_atlas(&mut session);
    if !can_play_named_site(&session, "north-open", "B3") {
        return None;
    }
    play_named_site(&mut session, "north-open", "B3");
    if !offers(&session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["cardId"] == "north-corpse"
            && descriptor["cells"] == json!(OPEN_SQUARE)
    }) {
        return None;
    }
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["cardId"] == "north-corpse"
            && descriptor["cells"] == json!(OPEN_SQUARE)
    });
    south_passes_north_draws_atlas(&mut session);
    if !can_play_named_site(&session, "north-tower", "A4") {
        return None;
    }
    play_named_site(&mut session, "north-tower", "A4");
    south_passes_north_draws_atlas(&mut session);
    if !can_play_named_site(&session, "north-blocked", "A3") {
        return None;
    }
    play_named_site(&mut session, "north-blocked", "A3");
    if !offers(&session, |descriptor| {
        descriptor["kind"] == "cast-magic" && descriptor["cardId"] == "north-raise"
    }) {
        return None;
    }
    Some(session)
}

fn square_opening() -> Session {
    (1..=4096)
        .map(square_manifest)
        .find_map(|candidate| {
            let session = Session::new(&candidate).ok()?;
            let spells = opening_ids(&session, "spellbook");
            if !spells.contains(&"north-corpse".to_owned())
                || !spells.contains(&"north-raise".to_owned())
            {
                return None;
            }
            try_square_setup(session)
        })
        .expect("bounded seed that can Raise Dead a 2x2 onto an open square beside a mixed square")
}

#[test]
fn rule_catalog_0329_free_placement_uses_prospective_power_on_a_tower() {
    let mut session = tower_opening();
    keep(&mut session);
    keep(&mut session);
    play_named_site(&mut session, "north-open", "C4");
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["cardId"] == "north-corpse"
            && descriptor["cell"] == "C4"
    });
    south_plays_c1_and_passes(&mut session);
    play_named_site(&mut session, "north-tower", "C3");
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "cast-magic" && descriptor["cardId"] == "north-raise"
    });
    assert!(
        !offers(&session, raise_on("north-corpse", "C3")),
        "printed 1 plus Tower bonus is 3, so threshold-3 Tower stays illegal"
    );
    assert!(
        offers(&session, raise_on("north-corpse", "C4")),
        "the same printed-1 corpse can still land on an open site"
    );
    assert!(
        offers(&session, raise_on("north-corpse", "C1")),
        "free placement may still choose an enemy open site"
    );
    accept_where(&mut session, raise_on("north-corpse", "C4"));
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_0330_free_two_by_two_uses_footprint_power() {
    let mut session = square_opening();
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "cast-magic" && descriptor["cardId"] == "north-raise"
    });
    assert!(
        !offers(&session, raise_square("north-corpse", &MIXED_SQUARE)),
        "Tower on A4 raises the whole 2x2 to 3, so threshold-3 A3 blocks occupancy"
    );
    assert!(
        offers(&session, raise_square("north-corpse", &OPEN_SQUARE)),
        "the same printed-1 2x2 can still occupy an all-open square"
    );
    accept_where(&mut session, raise_square("north-corpse", &OPEN_SQUARE));
    assert_exact_replay(&session);
}
