//! Public numeric `takesLessDamage` proofs for avatar damage and Death's Door.

use serde_json::{Value, json};
use sorcery_engine::canonical::{canonical_json, identity_hash};
use sorcery_engine::checkpoint::{create_game_checkpoint, resume_game_checkpoint};
use sorcery_engine::contract::{ActionRequest, Receipt};
use sorcery_engine::session::{Session, StepResult};

fn act(session: &mut Session, predicate: impl Fn(&Value) -> bool) -> Receipt {
    let action = session
        .legal_actions()
        .unwrap()
        .into_iter()
        .find(|a| predicate(&a.descriptor))
        .unwrap_or_else(|| {
            panic!(
                "issued action: {:?}",
                session
                    .legal_actions()
                    .unwrap()
                    .iter()
                    .map(|a| &a.descriptor)
                    .collect::<Vec<_>>()
            )
        });
    let StepResult::Accepted(receipt) = session
        .step(ActionRequest {
            action_id: action.action_id.to_string(),
            seat: action.seat,
            state_version: action.state_version,
        })
        .unwrap()
    else {
        panic!("accepted")
    };
    receipt
}

fn manifest(life: u8, attack: bool) -> String {
    let zero = json!({"air":0,"earth":0,"fire":0,"water":0});
    let mut cards = json!({
        "north-avatar":{"cardType":"avatar","attack":1,"defense":1,"drawSpell":false,"life":20},
        "south-avatar":{"cardType":"avatar","attack":1,"defense":1,"drawSpell":false,"life":life,"takesLessDamage":2},
        "north-site":{"cardType":"site","elements":["earth"]},"south-site":{"cardType":"site","elements":["earth"]},
        "caster":{"cardType":"minion","attack":1,"defense":2,"manaCost":0,"summonToAnySite":true,"spellcaster":true,"thresholds":zero},
        "damage-1":{"cardType":"magic","manaCost":0,"thresholds":zero,"damageTargetUnit":1},
        "damage-2":{"cardType":"magic","manaCost":0,"thresholds":zero,"damageTargetUnit":2},
        "damage-3":{"cardType":"magic","manaCost":0,"thresholds":zero,"damageTargetUnit":3}
    });
    if attack {
        cards["attacker"] = json!({"cardType":"minion","attack":3,"defense":3,"manaCost":0,"charge":true,"summonToAnySite":true,"thresholds":zero});
        for key in ["caster", "damage-1", "damage-2", "damage-3"] {
            cards.as_object_mut().unwrap().remove(key);
        }
    }
    let north_spells = if attack {
        vec!["attacker"; 3]
    } else {
        vec!["damage-1", "damage-2", "damage-3"]
    };
    let mut value = json!({"schemaVersion":1,"engineVersion":"sorcery-core-v1","seed":31,"firstSeat":"north",
        "authority":{"mode":"synthetic","revisionId":"numeric-prevention-v1","contentHash":identity_hash(&json!({"fixture":"numeric-prevention"})).unwrap()},"cards":cards,
        "decks":{"north":{"avatar":"north-avatar","atlas":vec!["north-site"; 6],"spellbook":north_spells},"south":{"avatar":"south-avatar","atlas":vec!["south-site"; 6],"spellbook":if attack { vec!["attacker"; 3] } else { vec!["caster"; 3] }}}});
    value["manifestId"] = json!(identity_hash(&value).unwrap());
    canonical_json(&value).unwrap()
}

fn keep(s: &mut Session) {
    act(s, |d| {
        d["kind"] == "mulligan" && d["atlasOrder"] == json!([]) && d["spellbookOrder"] == json!([])
    });
}
fn state(s: &Session) -> Value {
    s.replay_value().unwrap()["state"].clone()
}
fn setup(s: &mut Session) {
    keep(s);
    keep(s);
    act(s, |d| d["kind"] == "play-site" && d["cell"] == "C4");
    act(s, |d| d["kind"] == "end-turn");
    act(s, |d| d["kind"] == "draw" && d["zone"] == "atlas");
    act(s, |d| d["kind"] == "play-site" && d["cell"] == "C1");
    act(s, |d| d["kind"] == "end-turn");
    act(s, |d| d["kind"] == "draw" && d["zone"] == "atlas");
}

#[test]
fn avatar_takes_less_damage_two_reduces_magic_one_two_three_and_replays() {
    for (card, expected) in [("damage-1", 0), ("damage-2", 0), ("damage-3", 1)] {
        let mut s = Session::new(&manifest(20, false)).unwrap();
        setup(&mut s);
        let r = act(&mut s, |d| {
            d["kind"] == "cast-magic"
                && d["cardId"] == card
                && d["target"]["kind"] == "avatar"
                && d["target"]["seat"] == "south"
        });
        assert_eq!(
            r.events
                .iter()
                .find(|e| e.event_type == "damage-dealt")
                .unwrap()
                .payload["amount"],
            expected
        );
        assert_eq!(
            state(&s)["players"]["south"]["avatar"]["life"],
            if expected == 1 { 19 } else { 20 }
        );
        let replay = resume_game_checkpoint(&create_game_checkpoint(&s).unwrap()).unwrap();
        assert_eq!(replay.state_hash().unwrap(), s.state_hash().unwrap());
    }
}

#[test]
fn ordinary_site_attack_is_not_reduced_by_avatar_takes_less_damage() {
    let mut s = Session::new(&manifest(20, true)).unwrap();
    setup(&mut s);
    act(&mut s, |d| {
        d["kind"] == "summon-minion" && d["cardId"] == "attacker" && d["cell"] == "C1"
    });
    act(&mut s, |d| {
        d["kind"] == "move-and-attack" && d["unitInstanceId"].is_string() && d["to"]["cell"] == "C1"
    });
    while s
        .legal_actions()
        .unwrap()
        .iter()
        .any(|a| a.descriptor["kind"] == "continue-basic-movement")
    {
        act(&mut s, |d| d["kind"] == "continue-basic-movement");
    }
    act(&mut s, |d| {
        d["kind"] == "declare-attack" && d["target"]["kind"] == "site"
    });
    act(&mut s, |d| {
        d["kind"] == "close-defend" && d["originalTargetParticipates"] == false
    });
    assert_eq!(state(&s)["players"]["south"]["avatar"]["life"], 17);
    let replay = resume_game_checkpoint(&create_game_checkpoint(&s).unwrap()).unwrap();
    assert_eq!(replay.state_hash().unwrap(), s.state_hash().unwrap());
}

#[test]
fn reduced_damage_reaches_deaths_door_and_replays() {
    let mut s = Session::new(&manifest(1, false)).unwrap();
    setup(&mut s);
    let r = act(&mut s, |d| {
        d["kind"] == "cast-magic"
            && d["cardId"] == "damage-3"
            && d["target"]["kind"] == "avatar"
            && d["target"]["seat"] == "south"
    });
    assert!(
        r.events
            .iter()
            .any(|e| e.event_type == "avatar-reached-deaths-door")
    );
    assert_eq!(state(&s)["players"]["south"]["avatar"]["life"], 0);
    let replay = resume_game_checkpoint(&create_game_checkpoint(&s).unwrap()).unwrap();
    assert_eq!(replay.state_hash().unwrap(), s.state_hash().unwrap());
}
