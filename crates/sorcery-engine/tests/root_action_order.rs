use sorcery_engine::canonical::{IdentityHash, identity_hash};
use sorcery_engine::contract::ActionRequest;
use sorcery_engine::game::Game;
use sorcery_engine::policy::{PolicySnapshot, baseline_policy_snapshot};
use sorcery_engine::session::{Session, StepResult};
use sorcery_engine::simulator::{
    RootActionOrder, SimulatorError, replay_checkpoint_branch, replay_selected, run_game,
    search_from_checkpoint, search_from_checkpoint_with_order, search_root_actions,
};
use sorcery_engine::synthetic::synthetic_demo_manifest_json;

fn fixture() -> (Session, PolicySnapshot) {
    let manifest = synthetic_demo_manifest_json(31).expect("synthetic manifest");
    let game = Game::from_manifest_json(&manifest).expect("synthetic game");
    let policy =
        baseline_policy_snapshot(game.rules().authority_hash(), game.rules().engine_version())
            .expect("policy");
    let mut session = Session::new(&manifest).expect("session");
    for _ in 0..2 {
        let keep = session
            .legal_actions()
            .unwrap()
            .into_iter()
            .find(|action| {
                action.descriptor["kind"] == "mulligan"
                    && action.descriptor["atlasOrder"] == serde_json::json!([])
                    && action.descriptor["spellbookOrder"] == serde_json::json!([])
            })
            .expect("keep hand");
        assert!(matches!(
            session
                .step(ActionRequest {
                    action_id: keep.action_id.to_string(),
                    seat: keep.seat,
                    state_version: keep.state_version,
                })
                .unwrap(),
            StepResult::Accepted(_)
        ));
    }
    (session, policy)
}

fn order(session: &Session, action_ids: Vec<IdentityHash>) -> RootActionOrder {
    RootActionOrder {
        root_session_hash: session.session_hash().unwrap(),
        state_version: session.state_version(),
        action_ids,
    }
}

#[test]
fn preferred_later_action_survives_truncation_and_replays_its_original_index() {
    let (root, policy) = fixture();
    let before = root.session_hash().unwrap();
    let actions = root.legal_actions().unwrap();
    let last = actions.len() - 1;
    assert!(last > 0);
    let advice = order(&root, vec![actions[last].action_id.clone()]);
    let search = search_from_checkpoint_with_order(&root, &policy, &policy, 3, 1, Some(&advice))
        .expect("ordered root search");

    assert_eq!(search.rollouts().len(), 1);
    assert_eq!(search.rollouts()[0].action_indices()[0], last);
    let replay = replay_checkpoint_branch(&root, &search, 0).expect("exact replay");
    assert_eq!(
        replay.transcript()[root.transcript().len()].action_id,
        actions[last].action_id
    );
    assert!(replay.verify_replay().unwrap());
    assert_eq!(root.session_hash().unwrap(), before);
}

#[test]
fn every_reordered_branch_matches_its_canonical_branch_and_replays() {
    let (root, policy) = fixture();
    let actions = root.legal_actions().unwrap();
    let canonical = search_from_checkpoint(&root, &policy, &policy, 2, actions.len()).unwrap();
    let advice = order(
        &root,
        actions
            .iter()
            .rev()
            .map(|action| action.action_id.clone())
            .collect(),
    );
    let search =
        search_from_checkpoint_with_order(&root, &policy, &policy, 2, actions.len(), Some(&advice))
            .unwrap();

    for (branch, rollout) in search.rollouts().iter().enumerate() {
        let original = actions.len() - branch - 1;
        assert_eq!(rollout, &canonical.rollouts()[original]);
        replay_checkpoint_branch(&root, &search, branch).expect("every branch replays");
    }
}

#[test]
fn absent_empty_and_canonical_advice_preserve_existing_search_results() {
    let (root, policy) = fixture();
    let expected = search_from_checkpoint(&root, &policy, &policy, 3, 4).unwrap();
    let empty = order(&root, Vec::new());
    let canonical = order(
        &root,
        root.legal_actions()
            .unwrap()
            .into_iter()
            .map(|a| a.action_id)
            .collect(),
    );
    for advice in [None, Some(&empty), Some(&canonical)] {
        assert_eq!(
            search_from_checkpoint_with_order(&root, &policy, &policy, 3, 4, advice).unwrap(),
            expected
        );
    }

    let opening = Session::new(root.manifest_json()).unwrap();
    let game = Game::from_manifest_json(root.manifest_json()).unwrap();
    assert_eq!(
        search_root_actions(&game, &policy, &policy, 2, 3).unwrap(),
        search_from_checkpoint(&opening, &policy, &policy, 2, 3)
            .unwrap()
            .rollouts()
    );
}

#[test]
fn partial_prefix_appends_the_remaining_actions_canonically() {
    let (root, policy) = fixture();
    let actions = root.legal_actions().unwrap();
    let last = actions.len() - 1;
    let advice = order(&root, vec![actions[last].action_id.clone()]);
    let search =
        search_from_checkpoint_with_order(&root, &policy, &policy, 1, actions.len(), Some(&advice))
            .unwrap();
    assert_eq!(
        search
            .rollouts()
            .iter()
            .map(|r| r.action_indices()[0])
            .collect::<Vec<_>>(),
        std::iter::once(last).chain(0..last).collect::<Vec<_>>()
    );
}

#[test]
fn finished_sessions_preserve_empty_default_and_ordered_searches() {
    let (root, policy) = fixture();
    let rollout = run_game(
        Game::from_manifest_json(root.manifest_json()).unwrap(),
        &policy,
        &policy,
        400,
    )
    .unwrap();
    assert!(rollout.is_terminal());
    let finished = replay_selected(root.manifest_json(), &rollout).unwrap();
    let before = finished.session_hash().unwrap();
    assert!(finished.legal_actions().unwrap().is_empty());
    let expected = search_from_checkpoint(&finished, &policy, &policy, 8, 2).unwrap();
    assert!(expected.rollouts().is_empty());
    let empty = order(&finished, Vec::new());
    for advice in [None, Some(&empty)] {
        assert_eq!(
            search_from_checkpoint_with_order(&finished, &policy, &policy, 8, 2, advice).unwrap(),
            expected
        );
    }
    assert_eq!(finished.session_hash().unwrap(), before);
}

#[test]
fn invalid_advice_is_rejected_in_full_without_changing_the_root() {
    let (root, policy) = fixture();
    let before = root.replay_value().unwrap();
    let before_hash = root.session_hash().unwrap();
    let actions = root.legal_actions().unwrap();
    let first = actions[0].action_id.clone();
    let unknown = identity_hash(&serde_json::json!("unissued synthetic action")).unwrap();
    let mut stale_version = order(&root, vec![first.clone()]);
    stale_version.state_version += 1;
    let mut wrong_root = order(&root, vec![first.clone()]);
    wrong_root.root_session_hash = unknown.clone();
    let cases = [
        order(&root, vec![first.clone(), unknown]),
        order(&root, vec![first.clone(), first.clone()]),
        order(&root, vec![first; actions.len() + 1]),
        stale_version,
        wrong_root,
    ];
    for advice in cases {
        // Invalid entries after the one-branch budget must also reject.
        assert!(matches!(
            search_from_checkpoint_with_order(&root, &policy, &policy, 1, 1, Some(&advice)),
            Err(SimulatorError::InvalidRootActionOrder(_))
        ));
        assert_eq!(root.session_hash().unwrap(), before_hash);
        assert_eq!(root.replay_value().unwrap(), before);
    }
}

#[test]
fn prior_binding_becomes_stale_after_an_accepted_or_rejected_request() {
    let (root, policy) = fixture();
    let first = root.legal_actions().unwrap()[0].clone();
    let advice = order(&root, vec![first.action_id.clone()]);
    for state_version in [first.state_version, first.state_version + 1] {
        let mut changed = root.clone();
        changed
            .step(ActionRequest {
                action_id: first.action_id.to_string(),
                seat: first.seat,
                state_version,
            })
            .unwrap();
        let before = changed.replay_value().unwrap();
        assert!(matches!(
            search_from_checkpoint_with_order(&changed, &policy, &policy, 1, 1, Some(&advice)),
            Err(SimulatorError::InvalidRootActionOrder(_))
        ));
        assert_eq!(changed.replay_value().unwrap(), before);
    }
}
