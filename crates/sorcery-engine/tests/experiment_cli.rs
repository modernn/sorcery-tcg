//! The LLM boundary accepts deck composition only and preserves reproducible games.

use std::io::Write;
use std::process::{Command, Output, Stdio};

use serde_json::{Value, json};
use sorcery_engine::canonical::canonical_json;
use sorcery_engine::game::{Game, recompose_manifest_json};
use sorcery_engine::synthetic::selfplay_manifest_with;

fn cli(command: &str, input: Option<&Value>) -> Output {
    let mut child = Command::new(env!("CARGO_BIN_EXE_sorcery-engine"))
        .arg(command)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("start CLI");
    if let Some(input) = input {
        child
            .stdin
            .take()
            .expect("stdin")
            .write_all(&serde_json::to_vec(input).expect("request JSON"))
            .expect("write request");
    }
    child.wait_with_output().expect("CLI output")
}

fn success(output: &Output) -> Value {
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    serde_json::from_slice(&output.stdout).expect("response JSON")
}

fn example() -> Value {
    success(&cli("experiment-example", None))
}

#[test]
fn experiments_preserve_facts_and_replay_across_workers_and_deck_encodings() {
    let mut request = example();
    request["seeds"] = json!([31]);
    let spells = request["candidate"]["spellbook"]
        .as_array_mut()
        .expect("spells");
    let removed = spells.remove(1)["cardId"]
        .as_str()
        .expect("card ID")
        .to_owned();
    spells[0]["copies"] = json!(2);
    let dir = std::env::temp_dir().join(format!("sorcery-experiment-cli-{}", std::process::id()));
    request["artifactsDir"] = json!(dir);
    let mut first = success(&cli("experiment-json", Some(&request)));
    assert_eq!(first["gameCount"], 2);
    assert_eq!(first["allReplayVerified"], true);
    assert_eq!(first["ranked"], false);
    assert_eq!(first["candidate"]["engineSupported"], true);
    assert_eq!(first["candidate"]["formatLegal"], false);
    assert_eq!(first["bySeat"]["north"]["games"], 2);
    let candidate_id = first["candidate"]["deckId"].as_str().expect("candidate ID");
    assert_eq!(first["byDeck"][candidate_id]["asNorth"]["games"], 1);
    assert_eq!(first["byDeck"][candidate_id]["asSouth"]["games"], 1);

    let saved: Value = serde_json::from_slice(
        &std::fs::read(dir.join("0/manifest.json")).expect("saved manifest"),
    )
    .expect("manifest JSON");
    assert!(saved["cards"].get(&removed).is_none());
    for (card_id, facts) in saved["cards"].as_object().expect("saved cards") {
        assert_eq!(facts, &request["baseManifest"]["cards"][card_id]);
    }
    let replay = Command::new(env!("CARGO_BIN_EXE_sorcery-engine"))
        .arg("replay")
        .arg(dir.join("0"))
        .output()
        .expect("replay CLI");
    assert_eq!(success(&replay)["matched"], true);

    request
        .as_object_mut()
        .expect("request")
        .remove("artifactsDir");
    first
        .as_object_mut()
        .expect("response")
        .remove("artifactsDir");
    request["workers"] = json!(2);
    for role in ["candidate", "opponent"] {
        for zone in ["atlas", "spellbook"] {
            request[role][zone] = json!(
                request[role][zone]
                    .as_array()
                    .expect("counts")
                    .iter()
                    .rev()
                    .flat_map(|row| {
                        std::iter::repeat_n(
                            row["cardId"].clone(),
                            usize::try_from(row["copies"].as_u64().expect("copies"))
                                .expect("bounded copies"),
                        )
                    })
                    .collect::<Vec<_>>()
            );
        }
    }
    let second = success(&cli("experiment-json", Some(&request)));
    assert_eq!(
        canonical_json(&first).expect("first JSON"),
        canonical_json(&second).expect("second JSON")
    );
    std::fs::remove_dir_all(dir).expect("clean synthetic artifacts");
}

#[test]
fn experiments_reject_invalid_compositions_facts_bounds_and_private_artifact_writes() {
    let base = example();
    for (pointer, replacement, expected) in [
        ("/schemaVersion", json!(2), "schemaVersion"),
        ("/workers", json!(9), "workers"),
        ("/seeds", json!([]), "seeds"),
        (
            "/candidate/avatar",
            json!("missing"),
            "cannot be instantiated",
        ),
        (
            "/candidate/spellbook/0/copies",
            json!(0),
            "cannot be instantiated",
        ),
        ("/candidate/spellbook/0/copies", json!(u32::MAX), "overflow"),
        (
            "/candidate",
            json!({"avatar":"north-avatar","atlas":[],"spellbook":[],"cards":{}}),
            "unknown field",
        ),
        (
            "/baseManifest/cards/north-avatar/attack",
            json!(99),
            "not canonical",
        ),
    ] {
        let mut request = base.clone();
        *request.pointer_mut(pointer).expect("existing field") = replacement;
        let output = cli("experiment-json", Some(&request));
        assert!(!output.status.success(), "accepted {pointer}");
        assert!(
            String::from_utf8_lossy(&output.stderr).contains(expected),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        assert!(output.stdout.is_empty());
    }
    let mut private = base;
    private["baseManifest"]["authority"]["mode"] = json!("private-local");
    private["artifactsDir"] = json!("/tmp/must-not-write-private-facts");
    let output = cli("experiment-json", Some(&private));
    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("only for synthetic"));
}

#[test]
fn recomposition_uses_engine_token_dependencies_and_rejects_missing_or_unused_facts() {
    let base = selfplay_manifest_with(1, |manifest| {
        manifest["cards"]["north-site-1"]["genesisPayOneManaToSummonToken"] = json!("scout");
        manifest["cards"]["scout"] = manifest["cards"]["north-spell-1"].clone();
        manifest["cards"]["scout"]["token"] = json!(true);
    });
    let value: Value = serde_json::from_str(&base).expect("base JSON");
    let mut decks = value["decks"].clone();
    let recomposed = recompose_manifest_json(&base, &decks, 31).expect("token-aware recompose");
    Game::from_manifest_json(&recomposed).expect("valid recomposition");
    let retained: Value = serde_json::from_str(&recomposed).expect("recomposed JSON");
    assert_eq!(retained["cards"]["scout"], value["cards"]["scout"]);
    decks["north"]["atlas"][0] = json!("north-site-2");
    let pruned: Value =
        serde_json::from_str(&recompose_manifest_json(&base, &decks, 31).expect("pruned token"))
            .expect("pruned JSON");
    assert!(pruned["cards"].get("scout").is_none());
    assert!(pruned["cards"].get("north-site-1").is_none());
    decks["north"]["spellbook"][0] = json!("scout");
    assert!(recompose_manifest_json(&base, &decks, 31).is_err());

    for missing in [true, false] {
        let bad = selfplay_manifest_with(1, |manifest| {
            if missing {
                manifest["cards"]
                    .as_object_mut()
                    .expect("cards")
                    .remove("north-spell-1");
            } else {
                manifest["cards"]["unused"] = manifest["cards"]["north-spell-1"].clone();
            }
        });
        assert!(recompose_manifest_json(&bad, &value["decks"], 31).is_err());
    }
}
