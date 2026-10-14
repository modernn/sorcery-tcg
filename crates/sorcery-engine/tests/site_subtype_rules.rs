//! Synthetic issued-action proofs for Site characteristics and Tower qualification.
//! Fate here tests the supported suppression operation, not the full official card.
use serde_json::{Value, json};
use sorcery_engine::canonical::{canonical_json, identity_hash};
use sorcery_engine::checkpoint::{
    create_game_checkpoint, parse_game_checkpoint, resume_game_checkpoint,
    serialize_game_checkpoint,
};
use sorcery_engine::contract::{ActionRequest, Receipt, Seat};
use sorcery_engine::session::{Session, StepResult};

fn site() -> Value {
    json!({"cardType":"site","elements":["earth"]})
}
fn minion() -> Value {
    json!({"cardType":"minion","attack":1,"defense":8,"manaCost":0,"thresholds":{"air":0,"earth":0,"fire":0,"water":0}})
}
fn magic(field: &str) -> Value {
    let mut v = json!({"cardType":"magic","manaCost":0,"thresholds":{"air":0,"earth":0,"fire":0,"water":0}});
    v[field] = json!(true);
    v
}
fn fate() -> Value {
    json!({"cardType":"aura","affectedNonOrdinarySitesAreFloodedProvideOnlyWaterAndLoseOtherAbilities":true,"manaCost":0,"thresholds":{"air":0,"earth":0,"fire":0,"water":0}})
}
fn named_seat(seat: Seat) -> &'static str {
    if seat == Seat::North {
        "north"
    } else {
        "south"
    }
}
fn cell(seat: Seat, north: &str) -> String {
    if seat == Seat::North {
        north.to_owned()
    } else {
        format!("{}{}", &north[..1], 5 - north[1..].parse::<u8>().unwrap())
    }
}
fn snapshot(s: &Session) -> Value {
    s.replay_value().unwrap()["state"].clone()
}
fn has(s: &Session, seat: Seat, zone: &str, id: &str) -> bool {
    snapshot(s)["players"][named_seat(seat)]["hand"][zone]
        .as_array()
        .unwrap()
        .iter()
        .any(|c| c["cardId"] == id)
}
fn act(s: &mut Session, predicate: impl Fn(&Value) -> bool) -> (Value, Receipt) {
    let actions = s.legal_actions().unwrap();
    let a = actions
        .iter()
        .find(|a| predicate(&a.descriptor))
        .unwrap_or_else(|| {
            panic!(
                "missing action: {:?}",
                actions.iter().map(|a| &a.descriptor).collect::<Vec<_>>()
            )
        });
    let d = a.descriptor.clone();
    let request = ActionRequest {
        action_id: a.action_id.to_string(),
        seat: a.seat,
        state_version: a.state_version,
    };
    let StepResult::Accepted(r) = s.step(request).unwrap() else {
        panic!("issued action rejected")
    };
    (d, r)
}
fn offers(s: &Session, p: impl Fn(&Value) -> bool) -> bool {
    s.legal_actions().unwrap().iter().any(|a| p(&a.descriptor))
}
fn turn(s: &mut Session, zone: &str) {
    act(s, |d| d["kind"] == "end-turn");
    act(s, |d| d["kind"] == "draw" && d["zone"] == zone);
}
fn need(s: &mut Session, seat: Seat, zone: &str, id: &str) {
    for _ in 0..32 {
        if has(s, seat, zone, id) {
            return;
        }
        turn(s, "spellbook");
        turn(s, zone);
    }
    panic!("bounded draw did not find {id}");
}
fn play(s: &mut Session, id: &str, c: &str) {
    act(s, |d| {
        d["kind"] == "play-site"
            && d["cardId"] == id
            && d["cell"] == c
            && d["genesisTokenChoice"] != "pay-one-mana"
    });
}
fn summon(s: &mut Session, id: &str, c: &str) -> String {
    act(s, |d| {
        d["kind"] == "summon-minion"
            && d["cardId"] == id
            && d["cell"] == c
            && (d["region"].is_null() || d["region"] == "surface")
    })
    .0["cardInstanceId"]
        .as_str()
        .unwrap()
        .to_owned()
}
fn unit(s: &Session, id: &str) -> Value {
    s.public_view(Seat::North).unwrap()["realm"]["units"]
        .as_array()
        .unwrap()
        .iter()
        .find(|u| u["instanceId"] == id)
        .unwrap()
        .clone()
}
fn checkpoint(s: &Session) -> Session {
    let text = serialize_game_checkpoint(&create_game_checkpoint(s).unwrap()).unwrap();
    let restored = resume_game_checkpoint(&parse_game_checkpoint(&text).unwrap()).unwrap();
    assert_eq!(s.replay_value().unwrap(), restored.replay_value().unwrap());
    assert_eq!(s.transcript(), restored.transcript());
    assert!(s.verify_replay().unwrap());
    restored
}
fn opening(seat: Seat, mut cards: Value, first_site: &str) -> Session {
    cards["avatar"] =
        json!({"cardType":"avatar","attack":1,"defense":1,"life":20,"drawSpell":false});
    cards["enemy-site"] = site();
    let mut target = minion();
    target["summonToAnySite"] = json!(true);
    cards["enemy-target"] = target;
    let atlas = cards
        .as_object()
        .unwrap()
        .iter()
        .filter(|(id, v)| id.as_str() != "enemy-site" && v["cardType"] == "site")
        .map(|(id, _)| id.clone())
        .collect::<Vec<_>>();
    let spells = cards
        .as_object()
        .unwrap()
        .iter()
        .filter(|(id, v)| {
            id.as_str() != "enemy-target"
                && v["cardType"] != "site"
                && v["cardType"] != "avatar"
                && v["token"] != true
        })
        .map(|(id, _)| id.clone())
        .collect::<Vec<_>>();
    let spells = if spells.is_empty() {
        vec!["enemy-target".to_owned()]
    } else {
        spells
    };
    for seed in 1..=4096 {
        let mut decks = json!({"north":{"avatar":"avatar","atlas":vec!["enemy-site";60],"spellbook":vec!["enemy-target";60]},"south":{"avatar":"avatar","atlas":vec!["enemy-site";60],"spellbook":vec!["enemy-target";60]}});
        decks[named_seat(seat)]["atlas"] = json!(
            atlas
                .iter()
                .cycle()
                .take(atlas.len() * 20)
                .collect::<Vec<_>>()
        );
        decks[named_seat(seat)]["spellbook"] = json!(
            spells
                .iter()
                .cycle()
                .take(spells.len() * 20)
                .collect::<Vec<_>>()
        );
        let mut m = json!({"schemaVersion":1,"engineVersion":"sorcery-core-v1","authority":{"mode":"synthetic","revisionId":"site-subtype-contract","contentHash":identity_hash(&cards).unwrap()},"cards":cards,"decks":decks,"seed":seed,"firstSeat":named_seat(seat)});
        m["manifestId"] = json!(identity_hash(&m).unwrap());
        let mut s = Session::new(&canonical_json(&m).unwrap()).unwrap();
        if !has(&s, seat, "atlas", first_site) {
            continue;
        }
        for _ in 0..2 {
            act(&mut s, |d| {
                d["kind"] == "mulligan"
                    && d["atlasOrder"] == json!([])
                    && d["spellbookOrder"] == json!([])
            });
        }
        return s;
    }
    panic!("bounded opening");
}
fn start_board(s: &mut Session, seat: Seat) {
    play(s, "plain", &cell(seat, "C4"));
    turn(s, "spellbook");
    play(s, "enemy-site", &cell(seat, "C1"));
    turn(s, "atlas");
}
fn cast_fate(s: &mut Session, seat: Seat, covered: &str, excluded: &str) {
    need(s, seat, "spellbook", "fate");
    act(s, |d| {
        d["kind"] == "cast-aura"
            && d["cardId"] == "fate"
            && d["cells"]
                .as_array()
                .is_some_and(|v| v.iter().any(|v| v == covered) && v.iter().all(|v| v != excluded))
    });
}

#[test]
fn played_site_metadata_is_sparse_public_and_checkpoint_owned_for_both_seats() {
    for seat in [Seat::North, Seat::South] {
        for names in [None, Some(json!([])), Some(json!(["Tower"]))] {
            let mut printed = site();
            printed["isTower"] = json!(true);
            if let Some(names) = &names {
                printed["subtypes"] = names.clone();
            }
            let mut s = opening(
                seat,
                json!({"printed":printed,"destroy":magic("destroyTargetSite")}),
                "printed",
            );
            checkpoint(&s);
            play(&mut s, "printed", &cell(seat, "C4"));
            for viewer in [Seat::North, Seat::South] {
                let view = s.public_view(viewer).unwrap();
                let actual = &view["realm"]["sites"][cell(seat, "C4")];
                assert_eq!(actual.get("subtypes"), names.as_ref());
                assert!(view.get("cards").is_none());
            }
            checkpoint(&s);
            need(&mut s, seat, "spellbook", "destroy");
            let home = cell(seat, "C4");
            let site_id =
                s.public_view(seat).unwrap()["realm"]["sites"][&home]["instanceId"].clone();
            act(&mut s, |d| {
                d["kind"] == "cast-magic"
                    && d["cardId"] == "destroy"
                    && d["targetSiteInstanceId"] == site_id
            });
            for viewer in [Seat::North, Seat::South] {
                let view = s.public_view(viewer).unwrap();
                assert_eq!(view["realm"]["sites"][&home]["rubble"], true);
                assert!(view["realm"]["sites"][&home].get("subtypes").is_none());
            }
            checkpoint(&s);
        }
    }
}

#[test]
fn printed_tower_survives_site_suppression_but_minion_suppression_still_applies() {
    for seat in [Seat::North, Seat::South] {
        for (names, legacy, expected) in [
            (json!(["Tower"]), false, true),
            (json!([]), true, false),
            (json!(["River"]), true, false),
            (json!(["River"]), false, false),
        ] {
            let mut tower = site();
            tower["subtypes"] = names;
            if legacy {
                tower["isTower"] = json!(true);
            }
            let mut watcher = minion();
            watcher["defense"] = json!(1);
            watcher["gainsPowerRangedAndSpellcasterAtopTower"] = json!(2);
            let mut bolt = magic("damageTargetUnit");
            bolt["damageTargetUnit"] = json!(1);
            let cards = json!({"plain":site(),"tower":tower,"watcher":watcher,"fate":fate(),"bolt":bolt,"disable":magic("disableTargetNearbyMinionUntilNextTurn"),"silence":magic("silenceAndTapNearbyMinionThenMayDrawSpell"),"untap":magic("untapTargetMinion")});
            let mut s = opening(seat, cards, "plain");
            start_board(&mut s, seat);
            need(&mut s, seat, "atlas", "tower");
            need(&mut s, seat, "spellbook", "watcher");
            cast_fate(&mut s, seat, &cell(seat, "C3"), &cell(seat, "C4"));
            play(&mut s, "tower", &cell(seat, "C3"));
            let watcher = summon(&mut s, "watcher", &cell(seat, "C3"));
            turn(&mut s, "spellbook");
            let target = summon(&mut s, "enemy-target", &cell(seat, "C4"));
            turn(&mut s, "spellbook");
            need(&mut s, seat, "spellbook", "bolt");
            let u = unit(&s, &watcher);
            assert_eq!(u["attack"], if expected { 3 } else { 1 });
            assert_eq!(u["defense"], if expected { 3 } else { 1 });
            assert_eq!(
                offers(&s, |d| d["kind"] == "shoot-projectile"
                    && d["shooterInstanceId"] == watcher),
                expected
            );
            assert_eq!(
                offers(&s, |d| d["kind"] == "cast-magic"
                    && d["casterInstanceId"] == watcher),
                expected
            );
            checkpoint(&s);
            if expected {
                let mut shot = checkpoint(&s);
                let (_, r) = act(&mut shot, |d| {
                    d["kind"] == "shoot-projectile"
                        && d["shooterInstanceId"] == watcher
                        && d["hit"]["instanceId"] == target
                });
                assert!(
                    r.events
                        .iter()
                        .any(|e| e.event_type == "strike-damage-allocated"
                            && e.payload["amount"] == 3)
                );
                checkpoint(&shot);
                let mut cast = checkpoint(&s);
                act(&mut cast, |d| {
                    d["kind"] == "cast-magic"
                        && d["casterInstanceId"] == watcher
                        && d["cardId"] == "bolt"
                });
                checkpoint(&cast);
                for effect in ["disable", "silence"] {
                    let mut suppressed = checkpoint(&s);
                    need(&mut suppressed, seat, "spellbook", effect);
                    need(&mut suppressed, seat, "spellbook", "untap");
                    act(&mut suppressed, |d| {
                        d["kind"] == "cast-magic"
                            && d["cardId"] == effect
                            && d["drawZone"].is_null()
                            && d["target"]["instanceId"] == watcher
                    });
                    while offers(&suppressed, |d| d["kind"] == "resolve-optional-draw") {
                        act(&mut suppressed, |d| {
                            d["kind"] == "resolve-optional-draw" && d["draw"] == false
                        });
                    }
                    if effect == "silence" {
                        act(&mut suppressed, |d| {
                            d["kind"] == "cast-magic"
                                && d["cardId"] == "untap"
                                && d["target"]["instanceId"] == watcher
                        });
                        assert_eq!(unit(&suppressed, &watcher)["tapped"], false);
                    }
                    assert_eq!(unit(&suppressed, &watcher)["attack"], 1);
                    assert!(!offers(&suppressed, |d| (d["kind"] == "shoot-projectile"
                        && d["shooterInstanceId"] == watcher)
                        || (d["kind"] == "cast-magic"
                            && d["casterInstanceId"] == watcher)));
                    checkpoint(&suppressed);
                }
            }
        }
    }
}

#[test]
fn suppressed_printed_tower_changes_prospective_oversized_entry_at_another_site() {
    for seat in [Seat::North, Seat::South] {
        for explicit in [false, true] {
            let mut tower = site();
            if explicit {
                tower["subtypes"] = json!(["Tower"]);
            } else {
                tower["isTower"] = json!(true);
            }
            let mut gate = site();
            gate["preventsUnitsWithPowerAtLeastFromEntering"] = json!(3);
            let mut giant = minion();
            giant["occupiesSquareArea"] = json!(2);
            giant["gainsPowerRangedAndSpellcasterAtopTower"] = json!(2);
            let mut s = opening(
                seat,
                json!({"plain":site(),"tower":tower,"gate":gate,"giant":giant,"fate":fate()}),
                "plain",
            );
            start_board(&mut s, seat);
            need(&mut s, seat, "atlas", "gate");
            play(&mut s, "gate", &cell(seat, "B4"));
            turn(&mut s, "spellbook");
            turn(&mut s, "atlas");
            need(&mut s, seat, "atlas", "tower");
            cast_fate(&mut s, seat, &cell(seat, "C3"), &cell(seat, "B4"));
            play(&mut s, "tower", &cell(seat, "C3"));
            turn(&mut s, "spellbook");
            turn(&mut s, "atlas");
            need(&mut s, seat, "atlas", "plain");
            play(&mut s, "plain", &cell(seat, "B3"));
            need(&mut s, seat, "spellbook", "giant");
            let predicate = |d: &Value| {
                d["kind"] == "summon-minion"
                    && d["cardId"] == "giant"
                    && d["cells"].as_array().is_some_and(|v| {
                        v.iter().any(|v| v == &cell(seat, "B4"))
                            && v.iter().any(|v| v == &cell(seat, "C3"))
                    })
            };
            assert_eq!(offers(&s, predicate), !explicit);
            if !explicit {
                act(&mut s, predicate);
            }
            checkpoint(&s);
        }
    }
}

#[test]
fn new_printed_tower_token_entry_checks_singleton_and_oversized_power_once() {
    for seat in [Seat::North, Seat::South] {
        for oversized in [false, true] {
            // 0: no Tower, 1: printed Tower, 2: legacy qualification.
            for (incoming_tower, existing_tower, threshold, off_anchor_blocker, allowed) in [
                (0, 0, 3, false, true),
                (1, 0, 3, false, false),
                (2, 0, 3, false, false),
                (1, 1, 3, false, false),
                (1, 1, 4, false, true),
                (2, 2, 3, false, false),
                (2, 2, 4, false, true),
                (0, 1, 3, false, false),
                (0, 2, 4, false, true),
                (0, 0, 4, true, false),
            ] {
                if (existing_tower != 0 || off_anchor_blocker) && !oversized {
                    continue;
                }
                let mut token = minion();
                token["token"] = json!(true);
                token["gainsPowerRangedAndSpellcasterAtopTower"] = json!(2);
                if oversized {
                    token["occupiesSquareArea"] = json!(2);
                }
                let mut gate = site();
                gate["ordinary"] = json!(true);
                gate["genesisPayOneManaToSummonToken"] = json!("token");
                gate["preventsUnitsWithPowerAtLeastFromEntering"] = json!(threshold);
                if incoming_tower == 1 {
                    gate["subtypes"] = json!(["Tower"]);
                } else if incoming_tower == 2 {
                    gate["isTower"] = json!(true);
                }
                let mut plain = site();
                if existing_tower == 1 {
                    plain["subtypes"] = json!(["Tower"]);
                } else if existing_tower == 2 {
                    plain["isTower"] = json!(true);
                }
                if off_anchor_blocker {
                    plain["preventsUnitsWithPowerAtLeastFromEntering"] = json!(1);
                }
                let mut s = opening(
                    seat,
                    json!({"plain":plain,"gate":gate,"token":token}),
                    "plain",
                );
                start_board(&mut s, seat);
                if oversized {
                    need(&mut s, seat, "atlas", "plain");
                    play(&mut s, "plain", &cell(seat, "C3"));
                    turn(&mut s, "spellbook");
                    turn(&mut s, "atlas");
                    need(&mut s, seat, "atlas", "plain");
                    play(
                        &mut s,
                        "plain",
                        if seat == Seat::North { "B4" } else { "B2" },
                    );
                    turn(&mut s, "spellbook");
                    turn(&mut s, "atlas");
                }
                need(&mut s, seat, "atlas", "gate");
                let dest = if oversized {
                    if seat == Seat::North { "B3" } else { "B1" }.to_owned()
                } else {
                    cell(seat, "C3")
                };
                let pay = |d: &Value| {
                    d["kind"] == "play-site"
                        && d["cardId"] == "gate"
                        && d["cell"] == dest
                        && d["genesisTokenChoice"] == "pay-one-mana"
                };
                assert_eq!(
                    offers(&s, pay),
                    allowed,
                    "seat={seat:?} oversized={oversized} incoming={incoming_tower} existing={existing_tower} threshold={threshold} off_blocker={off_anchor_blocker} dest={dest}"
                );
                if allowed {
                    let (_, r) = act(&mut s, pay);
                    assert!(r.events.iter().any(|e| e.event_type == "minion-summoned"));
                } else {
                    play(&mut s, "gate", &dest);
                }
                checkpoint(&s);
            }
        }
    }
}

#[test]
fn suppressed_new_site_has_no_token_genesis_even_with_printed_tower() {
    for seat in [Seat::North, Seat::South] {
        let mut gate = site();
        gate["subtypes"] = json!(["Tower"]);
        gate["genesisPayOneManaToSummonToken"] = json!("token");
        let mut token = minion();
        token["token"] = json!(true);
        let mut s = opening(
            seat,
            json!({"plain":site(),"gate":gate,"token":token,"fate":fate()}),
            "plain",
        );
        start_board(&mut s, seat);
        need(&mut s, seat, "atlas", "gate");
        cast_fate(&mut s, seat, &cell(seat, "C3"), &cell(seat, "C4"));
        assert!(!offers(&s, |d| d["kind"] == "play-site"
            && d["cardId"] == "gate"
            && d["cell"] == cell(seat, "C3")
            && d["genesisTokenChoice"] == "pay-one-mana"));
        let (_, r) = act(&mut s, |d| {
            d["kind"] == "play-site" && d["cardId"] == "gate" && d["cell"] == cell(seat, "C3")
        });
        assert!(!r.events.iter().any(|e| e.event_type == "minion-summoned"));
        checkpoint(&s);
    }
}

#[test]
fn replacement_token_entry_uses_incoming_site_instead_of_old_tower_or_blocker() {
    let mut observed = Vec::new();
    for seat in [Seat::North, Seat::South] {
        for old_tower in [true, false] {
            let mut old = site();
            let mut incoming = site();
            incoming["genesisPayOneManaToSummonToken"] = json!("token");
            incoming["subtypes"] = json!(["Village"]);
            if old_tower {
                old["subtypes"] = json!(["Tower"]);
                incoming["preventsUnitsWithPowerAtLeastFromEntering"] = json!(2);
            } else {
                old["preventsUnitsWithPowerAtLeastFromEntering"] = json!(1);
            }
            let mut token = minion();
            token["token"] = json!(true);
            token["gainsPowerRangedAndSpellcasterAtopTower"] = json!(2);
            let mut flood = fate();
            flood
                .as_object_mut()
                .unwrap()
                .remove("affectedNonOrdinarySitesAreFloodedProvideOnlyWaterAndLoseOtherAbilities");
            flood["affectedSitesAreFlooded"] = json!(true);
            let mut s = opening(
                seat,
                json!({"old":old,"incoming":incoming,"token":token,"flood":flood}),
                "old",
            );
            let home = cell(seat, "C4");
            play(&mut s, "old", &home);
            turn(&mut s, "spellbook");
            play(&mut s, "enemy-site", &cell(seat, "C1"));
            turn(&mut s, "atlas");
            need(&mut s, seat, "atlas", "incoming");
            need(&mut s, seat, "spellbook", "flood");
            act(&mut s, |d| {
                d["kind"] == "cast-aura"
                    && d["cardId"] == "flood"
                    && d["cells"]
                        .as_array()
                        .is_some_and(|v| v.iter().any(|c| c == &home))
            });
            checkpoint(&s);
            let pay = |d: &Value| {
                d["kind"] == "play-site"
                    && d["cardId"] == "incoming"
                    && d["cell"] == home
                    && d["genesisTokenChoice"] == "pay-one-mana"
            };
            observed.push((seat, old_tower, offers(&s, pay)));
            if !offers(&s, pay) {
                continue;
            }
            let (_, receipt) = act(&mut s, pay);
            assert!(
                receipt
                    .events
                    .iter()
                    .any(|e| e.event_type == "minion-summoned")
            );
            for viewer in [Seat::North, Seat::South] {
                assert_eq!(
                    s.public_view(viewer).unwrap()["realm"]["sites"][&home]["subtypes"],
                    json!(["Village"])
                );
            }
            checkpoint(&s);
        }
    }
    assert!(
        observed.iter().all(|(_, _, offered)| *offered),
        "incoming Site must own entry: {observed:?}"
    );
}

#[test]
fn oversized_token_entry_retains_missing_surface_and_invalid_anchor_guards() {
    for seat in [Seat::North, Seat::South] {
        for invalid_anchor in [false, true] {
            let mut token = minion();
            token["token"] = json!(true);
            token["occupiesSquareArea"] = json!(2);
            let mut gate = site();
            gate["subtypes"] = json!(["Tower"]);
            gate["genesisPayOneManaToSummonToken"] = json!("token");
            let mut s = opening(
                seat,
                json!({"plain":site(),"gate":gate,"token":token}),
                "plain",
            );
            start_board(&mut s, seat);
            if invalid_anchor {
                need(&mut s, seat, "atlas", "plain");
                play(&mut s, "plain", &cell(seat, "D4"));
                turn(&mut s, "spellbook");
                turn(&mut s, "atlas");
            }
            need(&mut s, seat, "atlas", "gate");
            let destination = cell(seat, if invalid_anchor { "E4" } else { "C3" });
            assert!(!offers(&s, |d| d["kind"] == "play-site"
                && d["cardId"] == "gate"
                && d["cell"] == destination
                && d["genesisTokenChoice"] == "pay-one-mana"));
            play(&mut s, "gate", &destination);
            checkpoint(&s);
        }
    }
}
