//! Synthetic issued-action proofs for ordered affinity and independent forced submerge.
//! These generic effect fixtures do not establish complete real-card admission.
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
    if cards.get("enemy-site").is_none() {
        cards["enemy-site"] = site();
    }
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
        let mut m = json!({"schemaVersion":1,"engineVersion":"sorcery-core-v1","authority":{"mode":"synthetic","revisionId":"ordered-overlay-contract","contentHash":identity_hash(&cards).unwrap()},"cards":cards,"decks":decks,"seed":seed,"firstSeat":named_seat(seat)});
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

fn overlay(name: &str) -> Value {
    let mut v = fate();
    v.as_object_mut()
        .unwrap()
        .remove("affectedNonOrdinarySitesAreFloodedProvideOnlyWaterAndLoseOtherAbilities");
    v[match name {
        "flood" => "affectedSitesAreFlooded",
        "drought" => "affectedSitesAreNotWaterSitesAndProvideNoWaterThreshold",
        "fate" => "affectedNonOrdinarySitesAreFloodedProvideOnlyWaterAndLoseOtherAbilities",
        _ => panic!("unknown synthetic overlay"),
    }] = json!(true);
    v
}
fn mixed_site() -> Value {
    json!({"cardType":"site","elements":["earth","fire","water"],
        "siteAffinity":{"earth":2,"fire":2,"water":2,"air":0}})
}
fn overlay_board(seat: Seat, subject: &Value) -> Session {
    let mut probe = minion();
    probe["burrowing"] = json!(true);
    probe["submerge"] = json!(true);
    let mut s = opening(
        seat,
        json!({"subject":subject,"flood":overlay("flood"),
        "drought":overlay("drought"),"fate":fate(),"probe":probe,
        "destroy":magic("destroyTargetAura"),"return":magic("returnTargetAuraToOwnerHand"),
        "plain":{"cardType":"site","elements":[],"ordinary":true},"shift":magic("teleportTargetMinionArtifactOrAuraOneDiagonal"),
        "victim":minion(),"artifact":{"cardType":"artifact","grantsBearerPower":2,"manaCost":0,
        "thresholds":{"earth":0,"fire":0,"water":0,"air":0}}}),
        "subject",
    );
    play(&mut s, "subject", &cell(seat, "C4"));
    turn(&mut s, "spellbook");
    play(&mut s, "enemy-site", &cell(seat, "C1"));
    turn(&mut s, "spellbook");
    need(&mut s, seat, "spellbook", "probe");
    s
}
fn affinity(s: &Session, seat: Seat) -> [u64; 4] {
    let v = s.public_view(seat).unwrap();
    ["earth", "fire", "water", "air"].map(|e| {
        v["players"][named_seat(seat)]["affinity"][e]
            .as_u64()
            .unwrap()
    })
}
fn assert_affinity_and_regions(s: &Session, seat: Seat, expected: [u64; 4]) {
    assert_eq!(affinity(s, seat), expected);
    // The opponent's distant printed Earth site is outside every subject overlay.
    let opponent = if seat == Seat::North {
        Seat::South
    } else {
        Seat::North
    };
    assert_eq!(affinity(s, opponent), [1, 0, 0, 0]);
    for (region, exists) in [
        ("underwater", expected[2] > 0),
        ("underground", expected[2] == 0),
    ] {
        assert_eq!(
            offers(s, |d| d["kind"] == "summon-minion"
                && d["cardId"] == "probe"
                && d["cell"] == cell(seat, "C4")
                && d["region"] == region),
            exists,
            "{seat:?} {region}"
        );
    }
    checkpoint(s);
}
fn cast_overlay(s: &mut Session, seat: Seat, name: &str) -> String {
    need(s, seat, "spellbook", name);
    *s = checkpoint(s);
    let (d, _) = act(s, |d| {
        d["kind"] == "cast-aura"
            && d["cardId"] == name
            && d["cells"]
                .as_array()
                .is_some_and(|cs| cs.iter().any(|c| c == &cell(seat, "C4")))
    });
    checkpoint(s);
    d["cardInstanceId"].as_str().unwrap().to_owned()
}
fn remove_overlay(s: &mut Session, seat: Seat, id: &str, return_to_hand: bool) {
    let name = if return_to_hand { "return" } else { "destroy" };
    need(s, seat, "spellbook", name);
    act(s, |d| {
        d["kind"] == "cast-magic" && d["cardId"] == name && d["targetAuraInstanceId"] == id
    });
    checkpoint(s);
}

#[test]
fn ordered_affinity_pairs_and_triples_use_each_current_value_both_seats() {
    let cases: &[(&[&str], [u64; 4])] = &[
        (&["flood", "drought"], [2, 2, 0, 0]),
        (&["drought", "flood"], [2, 2, 1, 0]),
        (&["flood", "fate"], [0, 0, 2, 0]),
        (&["fate", "flood"], [0, 0, 2, 0]),
        (&["drought", "fate"], [0, 0, 1, 0]),
        (&["fate", "drought"], [0, 0, 0, 0]),
        (&["flood", "drought", "fate"], [0, 0, 1, 0]),
        (&["flood", "fate", "drought"], [0, 0, 0, 0]),
        (&["drought", "flood", "fate"], [0, 0, 1, 0]),
        (&["drought", "fate", "flood"], [0, 0, 1, 0]),
        (&["fate", "flood", "drought"], [0, 0, 0, 0]),
        (&["fate", "drought", "flood"], [0, 0, 1, 0]),
    ];
    for seat in [Seat::North, Seat::South] {
        for (sequence, expected) in cases {
            let mut s = overlay_board(seat, &mixed_site());
            assert_affinity_and_regions(&s, seat, [2, 2, 2, 0]);
            for name in *sequence {
                cast_overlay(&mut s, seat, name);
            }
            assert_affinity_and_regions(&s, seat, *expected);
        }
    }
}

#[test]
fn ordinary_ignores_only_fate_and_protected_keeps_printed_affinity() {
    for seat in [Seat::North, Seat::South] {
        for (sequence, ordinary_water) in [
            (["flood", "drought", "fate"], 0),
            (["flood", "fate", "drought"], 0),
            (["drought", "flood", "fate"], 1),
            (["drought", "fate", "flood"], 1),
            (["fate", "flood", "drought"], 0),
            (["fate", "drought", "flood"], 1),
        ] {
            for protected in [false, true] {
                let mut subject = mixed_site();
                if protected {
                    subject["cannotBeMovedDestroyedOrModified"] = json!(true);
                } else {
                    subject["ordinary"] = json!(true);
                }
                let mut s = overlay_board(seat, &subject);
                for name in sequence {
                    cast_overlay(&mut s, seat, name);
                }
                assert_affinity_and_regions(
                    &s,
                    seat,
                    [2, 2, if protected { 2 } else { ordinary_water }, 0],
                );
            }
        }
    }
}

#[test]
fn removal_recomputes_surviving_effects_and_recast_gets_new_timestamp() {
    for seat in [Seat::North, Seat::South] {
        let mut base = overlay_board(seat, &mixed_site());
        let fate_id = cast_overlay(&mut base, seat, "fate");
        let drought_id = cast_overlay(&mut base, seat, "drought");
        let flood_id = cast_overlay(&mut base, seat, "flood");
        assert_affinity_and_regions(&base, seat, [0, 0, 1, 0]);
        for (id, expected) in [
            (&fate_id, [2, 2, 1, 0]),
            (&drought_id, [0, 0, 2, 0]),
            (&flood_id, [0, 0, 0, 0]),
        ] {
            let mut branch = checkpoint(&base);
            remove_overlay(&mut branch, seat, id, false);
            assert_affinity_and_regions(&branch, seat, expected);
        }
        let mut all = checkpoint(&base);
        for id in [&fate_id, &drought_id, &flood_id] {
            remove_overlay(&mut all, seat, id, false);
        }
        assert_affinity_and_regions(&all, seat, [2, 2, 2, 0]);
        let mut recast = checkpoint(&base);
        remove_overlay(&mut recast, seat, &drought_id, true);
        assert_affinity_and_regions(&recast, seat, [0, 0, 2, 0]);
        cast_overlay(&mut recast, seat, "drought");
        assert_affinity_and_regions(&recast, seat, [0, 0, 0, 0]);
    }
}

#[test]
fn fate_genesis_checks_ordinary_and_protected_water_without_site_immunity() {
    for seat in [Seat::North, Seat::South] {
        for protected in [false, true] {
            for water in [false, true] {
                let mut subject =
                    json!({"cardType":"site","elements":[if water {"water"} else {"earth"}]});
                if protected {
                    subject["cannotBeMovedDestroyedOrModified"] = json!(true);
                } else {
                    subject["ordinary"] = json!(true);
                }
                let mut s = overlay_board(seat, &subject);
                let id = summon(&mut s, "probe", &cell(seat, "C4"));
                cast_overlay(&mut s, seat, "fate");
                assert_eq!(
                    unit(&s, &id)["region"],
                    if water { "underwater" } else { "surface" }
                );
                assert_eq!(
                    affinity(&s, seat),
                    if water { [0, 0, 1, 0] } else { [1, 0, 0, 0] }
                );
                checkpoint(&s);
            }
        }
    }
}

#[test]
fn duplicate_overlays_remain_individually_ordered_after_one_is_removed() {
    for seat in [Seat::North, Seat::South] {
        let mut s = overlay_board(seat, &mixed_site());
        let first = cast_overlay(&mut s, seat, "flood");
        cast_overlay(&mut s, seat, "drought");
        let last = cast_overlay(&mut s, seat, "flood");
        assert_affinity_and_regions(&s, seat, [2, 2, 1, 0]);
        let mut remove_first = checkpoint(&s);
        remove_overlay(&mut remove_first, seat, &first, false);
        assert_affinity_and_regions(&remove_first, seat, [2, 2, 1, 0]);
        remove_overlay(&mut s, seat, &last, false);
        assert_affinity_and_regions(&s, seat, [2, 2, 0, 0]);
        let first_fate = cast_overlay(&mut s, seat, "fate");
        let last_fate = cast_overlay(&mut s, seat, "fate");
        for id in [&first_fate, &last_fate] {
            let mut branch = checkpoint(&s);
            remove_overlay(&mut branch, seat, id, false);
            assert_affinity_and_regions(&branch, seat, [0, 0, 1, 0]);
        }
    }
}

#[test]
fn relocation_changes_coverage_without_refreshing_passive_timestamp() {
    for seat in [Seat::North, Seat::South] {
        let mut s = overlay_board(seat, &mixed_site());
        for north in ["B4", "B3", "B2", "C2", "C3", "D2", "D3", "D4"] {
            need(&mut s, seat, "atlas", "plain");
            play(&mut s, "plain", &cell(seat, north));
            turn(&mut s, "spellbook");
            turn(&mut s, "atlas");
        }
        let old = cast_overlay(&mut s, seat, "fate");
        let before = snapshot(&s)["realm"]["auras"]
            .as_array()
            .unwrap()
            .iter()
            .find(|a| a["instanceId"] == old)
            .unwrap()["cells"]
            .clone();
        cast_overlay(&mut s, seat, "drought");
        assert_affinity_and_regions(&s, seat, [0, 0, 0, 0]);
        need(&mut s, seat, "spellbook", "shift");
        act(&mut s, |d| {
            d["kind"] == "cast-magic"
                && d["cardId"] == "shift"
                && d["targetAuraInstanceId"] == old
                && d["allyDestinationCells"]
                    .as_array()
                    .is_some_and(|cs| cs.iter().all(|c| c != &cell(seat, "C4")))
        });
        assert_affinity_and_regions(&s, seat, [2, 2, 0, 0]);
        need(&mut s, seat, "spellbook", "shift");
        act(&mut s, |d| {
            d["kind"] == "cast-magic"
                && d["cardId"] == "shift"
                && d["targetAuraInstanceId"] == old
                && d["allyDestinationCells"] == before
        });
        assert_affinity_and_regions(&s, seat, [0, 0, 0, 0]);
        let auras = snapshot(&s)["realm"]["auras"].as_array().unwrap().clone();
        assert_eq!(auras[0]["instanceId"], old);
        checkpoint(&s);
    }
}

#[test]
fn ordinary_water_genesis_kills_plain_minion_and_submerges_loose_artifact_only() {
    for seat in [Seat::North, Seat::South] {
        for water in [false, true] {
            let subject = json!({"cardType":"site","ordinary":true,
                "elements":[if water {"water"} else {"earth"}]});
            let mut s = overlay_board(seat, &subject);
            need(&mut s, seat, "spellbook", "victim");
            let victim = summon(&mut s, "victim", &cell(seat, "C4"));
            need(&mut s, seat, "spellbook", "artifact");
            let (descriptor, _) = act(&mut s, |d| {
                d["kind"] == "cast-artifact" && d["cardId"] == "artifact" && d["bearer"].is_null()
            });
            let artifact = descriptor["cardInstanceId"].as_str().unwrap().to_owned();
            need(&mut s, seat, "spellbook", "fate");
            let (_, receipt) = act(&mut s, |d| {
                d["kind"] == "cast-aura"
                    && d["cardId"] == "fate"
                    && d["cells"]
                        .as_array()
                        .is_some_and(|cs| cs.iter().any(|c| c == &cell(seat, "C4")))
            });
            for event in ["minion-submerged", "minion-died", "artifact-submerged"] {
                assert_eq!(
                    receipt.events.iter().any(|e| e.event_type == event),
                    water,
                    "{seat:?} {event}"
                );
            }
            let state = snapshot(&s);
            assert_eq!(
                state["realm"]["units"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .any(|u| u["instanceId"] == victim),
                !water
            );
            let loose = state["realm"]["artifacts"]
                .as_array()
                .unwrap()
                .iter()
                .find(|a| a["instanceId"] == artifact)
                .unwrap();
            assert_eq!(
                loose["region"],
                if water { "underwater" } else { "surface" }
            );
            assert_eq!(
                state["players"][named_seat(seat)]["avatar"]["region"],
                "surface"
            );
            checkpoint(&s);
        }
    }
}

#[test]
fn forced_submerge_requires_whole_footprint_and_deduplicates_overlapping_cells() {
    for seat in [Seat::North, Seat::South] {
        for all_water in [false, true] {
            for already_underwater in [false, true] {
                if already_underwater && !all_water {
                    continue;
                }
                let plain = json!({"cardType":"site","ordinary":true,
                    "elements":[if all_water {"water"} else {"earth"}]});
                let mut giant = minion();
                giant["submerge"] = json!(true);
                giant["occupiesSquareArea"] = json!(2);
                let mut s = opening(
                    seat,
                    json!({"plain":plain,"special":site(),
                    "giant":giant,"fate":fate()}),
                    "plain",
                );
                play(&mut s, "plain", &cell(seat, "C4"));
                turn(&mut s, "spellbook");
                play(&mut s, "enemy-site", &cell(seat, "C1"));
                turn(&mut s, "atlas");
                for (id, destination) in [("plain", "B4"), ("special", "C3"), ("plain", "B3")] {
                    need(&mut s, seat, "atlas", id);
                    // A preexisting underwater negative needs all four Sites Water before Fate.
                    // Use plain instead of special when the minion starts underwater.
                    let played = if already_underwater { "plain" } else { id };
                    need(&mut s, seat, "atlas", played);
                    play(&mut s, played, &cell(seat, destination));
                    turn(&mut s, "spellbook");
                    turn(&mut s, "atlas");
                }
                need(&mut s, seat, "spellbook", "giant");
                let anchor = if seat == Seat::North { "B3" } else { "B1" };
                let (summoned, _) = act(&mut s, |d| {
                    d["kind"] == "summon-minion"
                        && d["cardId"] == "giant"
                        && d["cell"] == anchor
                        && if already_underwater {
                            d["region"] == "underwater"
                        } else {
                            d["region"].is_null()
                        }
                });
                let id = summoned["cardInstanceId"].as_str().unwrap().to_owned();
                let before = unit(&s, &id);
                need(&mut s, seat, "spellbook", "fate");
                let (_, receipt) = act(&mut s, |d| {
                    d["kind"] == "cast-aura"
                        && d["cardId"] == "fate"
                        && d["cells"].as_array().is_some_and(|cs| {
                            cs.iter().any(|c| c == &cell(seat, "C3"))
                                && cs.iter().all(|c| c != &cell(seat, "C4"))
                        })
                });
                let after = unit(&s, &id);
                assert_eq!(after["occupiedCells"], before["occupiedCells"]);
                assert_eq!(
                    after["region"],
                    if all_water { "underwater" } else { "surface" }
                );
                assert_eq!(
                    receipt
                        .events
                        .iter()
                        .filter(|e| e.event_type == "minion-submerged")
                        .count(),
                    usize::from(all_water && !already_underwater)
                );
                assert!(
                    !receipt.events.iter().any(|e| matches!(
                        e.event_type.as_str(),
                        "minion-died" | "minion-banished"
                    ))
                );
                checkpoint(&s);
            }
        }
    }
}

#[test]
fn threshold_prechecks_use_preinsertion_affinity_then_issued_choices_use_composed_counts() {
    for seat in [Seat::North, Seat::South] {
        let mut paid = overlay("drought");
        paid["manaCost"] = json!(1);
        paid["thresholds"]["water"] = json!(2);
        let mut one = minion();
        one["thresholds"]["water"] = json!(1);
        let mut two = minion();
        two["thresholds"]["water"] = json!(2);
        let mut earth = minion();
        earth["thresholds"]["earth"] = json!(2);
        let mut fire = minion();
        fire["thresholds"]["fire"] = json!(2);
        let mut air = minion();
        air["thresholds"]["air"] = json!(1);
        let mut subject = mixed_site();
        subject["elements"] = json!(["earth", "fire", "water", "air"]);
        subject["siteAffinity"]["air"] = json!(1);
        let mut s = opening(
            seat,
            json!({"subject":subject,"paid":paid,"fate":fate(),
            "one":one,"two":two,"earth":earth,"fire":fire,"air":air}),
            "subject",
        );
        play(&mut s, "subject", &cell(seat, "C4"));
        turn(&mut s, "spellbook");
        play(&mut s, "enemy-site", &cell(seat, "C1"));
        turn(&mut s, "spellbook");
        for id in ["paid", "one", "two", "earth", "fire", "air", "fate"] {
            need(&mut s, seat, "spellbook", id);
        }
        let can_cast = |s: &Session, id: &str| {
            offers(s, |d| d["kind"] == "summon-minion" && d["cardId"] == id)
        };
        assert!(
            can_cast(&s, "one")
                && can_cast(&s, "two")
                && can_cast(&s, "earth")
                && can_cast(&s, "fire")
        );
        assert!(can_cast(&s, "air"));
        let mana = snapshot(&s)["players"][named_seat(seat)]["mana"]
            .as_u64()
            .unwrap();
        cast_overlay(&mut s, seat, "paid");
        assert_eq!(snapshot(&s)["players"][named_seat(seat)]["mana"], mana - 1);
        assert_eq!(affinity(&s, seat), [2, 2, 0, 1]);
        assert!(
            !can_cast(&s, "one")
                && !can_cast(&s, "two")
                && can_cast(&s, "earth")
                && can_cast(&s, "fire")
        );
        assert!(can_cast(&s, "air"));
        cast_overlay(&mut s, seat, "fate");
        assert_eq!(affinity(&s, seat), [0, 0, 1, 0]);
        assert!(
            can_cast(&s, "one")
                && !can_cast(&s, "two")
                && !can_cast(&s, "earth")
                && !can_cast(&s, "fire")
                && !can_cast(&s, "air")
        );
        summon(&mut s, "one", &cell(seat, "C4"));
        checkpoint(&s);
    }
}

#[test]
fn zero_and_single_and_double_water_keep_multiplicity_until_drought() {
    for seat in [Seat::North, Seat::South] {
        for water in [0, 1, 2] {
            let mut subject = json!({"cardType":"site","elements":[]});
            if water > 0 {
                subject["elements"] = json!(["water"]);
                subject["siteAffinity"] = json!({"earth":0,"fire":0,"water":water,"air":0});
            }
            let mut s = overlay_board(seat, &subject);
            assert_affinity_and_regions(&s, seat, [0, 0, water, 0]);
            cast_overlay(&mut s, seat, "flood");
            assert_affinity_and_regions(&s, seat, [0, 0, water.max(1), 0]);
            cast_overlay(&mut s, seat, "drought");
            assert_affinity_and_regions(&s, seat, [0, 0, 0, 0]);
            cast_overlay(&mut s, seat, "flood");
            assert_affinity_and_regions(&s, seat, [0, 0, 1, 0]);
        }
    }
}

#[test]
fn rubble_changes_lower_region_without_contributing_affinity() {
    for seat in [Seat::North, Seat::South] {
        let mut dual = minion();
        dual["submerge"] = json!(true);
        dual["burrowing"] = json!(true);
        let mut s = opening(
            seat,
            json!({"subject":site(),"dual":dual,
            "destroy":magic("destroyTargetSite"),"flood":overlay("flood"),"drought":overlay("drought")}),
            "subject",
        );
        play(&mut s, "subject", &cell(seat, "C4"));
        turn(&mut s, "spellbook");
        play(&mut s, "enemy-site", &cell(seat, "C1"));
        turn(&mut s, "spellbook");
        need(&mut s, seat, "spellbook", "dual");
        let (d, _) = act(&mut s, |d| {
            d["kind"] == "summon-minion"
                && d["cardId"] == "dual"
                && d["cell"] == cell(seat, "C4")
                && d["region"] == "underground"
        });
        let id = d["cardInstanceId"].as_str().unwrap().to_owned();
        need(&mut s, seat, "spellbook", "destroy");
        let site_id =
            s.public_view(seat).unwrap()["realm"]["sites"][cell(seat, "C4")]["instanceId"].clone();
        act(&mut s, |d| {
            d["kind"] == "cast-magic"
                && d["cardId"] == "destroy"
                && d["targetSiteInstanceId"] == site_id
        });
        assert_eq!(
            s.public_view(seat).unwrap()["realm"]["sites"][cell(seat, "C4")]["rubble"],
            true
        );
        assert_eq!(affinity(&s, seat), [0; 4]);
        cast_overlay(&mut s, seat, "flood");
        assert_eq!(unit(&s, &id)["region"], "underwater");
        assert_eq!(affinity(&s, seat), [0; 4]);
        cast_overlay(&mut s, seat, "drought");
        assert_eq!(unit(&s, &id)["region"], "underground");
        assert_eq!(affinity(&s, seat), [0; 4]);
        checkpoint(&s);
    }
}

#[test]
fn genesis_crosses_controllers_and_keeps_carried_and_outside_objects_distinct() {
    for seat in [Seat::North, Seat::South] {
        let water = json!({"cardType":"site","elements":["water"],"ordinary":true});
        let mut diver = minion();
        diver["submerge"] = json!(true);
        let artifact = json!({"cardType":"artifact","grantsBearerPower":2,"manaCost":0,
            "thresholds":{"earth":0,"fire":0,"water":0,"air":0}});
        let mut s = opening(
            seat,
            json!({"subject":water,"enemy-site":water,"diver":diver,
            "artifact":artifact,"fate":fate()}),
            "subject",
        );
        play(&mut s, "subject", &cell(seat, "C4"));
        turn(&mut s, "spellbook");
        play(&mut s, "enemy-site", &cell(seat, "C1"));
        turn(&mut s, "atlas");
        need(&mut s, seat, "atlas", "subject");
        play(&mut s, "subject", &cell(seat, "C3"));
        turn(&mut s, "spellbook");
        play(&mut s, "enemy-site", &cell(seat, "C2"));
        let enemy = summon(&mut s, "enemy-target", &cell(seat, "C2"));
        turn(&mut s, "spellbook");
        need(&mut s, seat, "spellbook", "diver");
        let carried_by = summon(&mut s, "diver", &cell(seat, "C3"));
        need(&mut s, seat, "spellbook", "diver");
        let outside = summon(&mut s, "diver", &cell(seat, "C4"));
        need(&mut s, seat, "spellbook", "artifact");
        let (held, _) = act(&mut s, |d| {
            d["kind"] == "cast-artifact"
                && d["cardId"] == "artifact"
                && d["bearer"]["instanceId"] == carried_by
        });
        need(&mut s, seat, "spellbook", "artifact");
        let (loose, _) = act(&mut s, |d| {
            d["kind"] == "cast-artifact"
                && d["cardId"] == "artifact"
                && d["bearer"].is_null()
                && d["cell"] == cell(seat, "C4")
        });
        let before = snapshot(&s);
        let outside_artifact = before["realm"]["artifacts"]
            .as_array()
            .unwrap()
            .iter()
            .find(|a| a["instanceId"] == loose["cardInstanceId"])
            .unwrap();
        assert_eq!(outside_artifact["location"], cell(seat, "C4"));
        assert_eq!(outside_artifact["region"], "surface");
        checkpoint(&s);
        need(&mut s, seat, "spellbook", "fate");
        let (_, receipt) = act(&mut s, |d| {
            d["kind"] == "cast-aura"
                && d["cardId"] == "fate"
                && d["cells"].as_array().is_some_and(|cs| {
                    cs.iter().any(|c| c == &cell(seat, "C3"))
                        && cs.iter().any(|c| c == &cell(seat, "C2"))
                        && cs.iter().all(|c| c != &cell(seat, "C4"))
                })
        });
        assert_eq!(unit(&s, &carried_by)["region"], "underwater");
        assert_eq!(unit(&s, &outside)["region"], "surface");
        let state = snapshot(&s);
        assert!(
            !state["realm"]["units"]
                .as_array()
                .unwrap()
                .iter()
                .any(|u| u["instanceId"] == enemy)
        );
        let artifacts = state["realm"]["artifacts"].as_array().unwrap();
        let held = artifacts
            .iter()
            .find(|a| a["instanceId"] == held["cardInstanceId"])
            .unwrap();
        let loose = artifacts
            .iter()
            .find(|a| a["instanceId"] == loose["cardInstanceId"])
            .unwrap();
        assert_eq!(held["bearer"]["instanceId"], carried_by);
        assert_eq!(loose["region"], "surface");
        assert!(
            !receipt
                .events
                .iter()
                .any(|e| e.event_type == "artifact-submerged")
        );
        assert_eq!(
            receipt
                .events
                .iter()
                .filter(|e| e.event_type == "minion-submerged")
                .count(),
            2
        );
        checkpoint(&s);
    }
}

#[test]
fn ordinary_water_tower_keeps_entry_restriction_and_tower_grant_under_fate() {
    for seat in [Seat::North, Seat::South] {
        for printed in [false, true] {
            let mut tower = json!({"cardType":"site","elements":["water"],"ordinary":true,
                "preventsUnitsWithPowerAtLeastFromEntering":4});
            if printed {
                tower["subtypes"] = json!(["Tower"]);
            } else {
                tower["isTower"] = json!(true);
            }
            let mut watcher = minion();
            watcher["defense"] = json!(1);
            watcher["gainsPowerRangedAndSpellcasterAtopTower"] = json!(2);
            let mut heavy = minion();
            heavy["attack"] = json!(4);
            heavy["defense"] = json!(4);
            let mut bolt = magic("damageTargetUnit");
            bolt["damageTargetUnit"] = json!(1);
            let mut s = opening(
                seat,
                json!({"subject":tower,"watcher":watcher,
                "heavy":heavy,"fate":fate(),"bolt":bolt}),
                "subject",
            );
            play(&mut s, "subject", &cell(seat, "C4"));
            turn(&mut s, "spellbook");
            play(&mut s, "enemy-site", &cell(seat, "C1"));
            turn(&mut s, "spellbook");
            for id in ["watcher", "heavy", "fate", "bolt"] {
                need(&mut s, seat, "spellbook", id);
            }
            let blocked = |d: &Value| {
                d["kind"] == "summon-minion"
                    && d["cardId"] == "heavy"
                    && d["cell"] == cell(seat, "C4")
            };
            assert!(!offers(&s, blocked));
            cast_overlay(&mut s, seat, "fate");
            assert!(!offers(&s, blocked));
            let watcher = summon(&mut s, "watcher", &cell(seat, "C4"));
            turn(&mut s, "spellbook");
            let enemy = summon(&mut s, "enemy-target", &cell(seat, "C4"));
            turn(&mut s, "spellbook");
            assert_eq!(unit(&s, &watcher)["attack"], 3);
            assert_eq!(unit(&s, &watcher)["defense"], 3);
            assert!(offers(&s, |d| d["kind"] == "shoot-projectile"
                && d["shooterInstanceId"] == watcher
                && d["hit"]["instanceId"] == enemy));
            assert!(offers(&s, |d| d["kind"] == "cast-magic"
                && d["casterInstanceId"] == watcher
                && d["cardId"] == "bolt"));
            assert_eq!(affinity(&s, seat), [0, 0, 1, 0]);
            checkpoint(&s);
        }
    }
}
