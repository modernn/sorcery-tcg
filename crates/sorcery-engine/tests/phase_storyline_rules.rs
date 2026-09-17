//! Direct proofs for Phase and Storyline cleanup (RULE-CATALOG-0728,
//! RULE-CATALOG-0731).
//!
//! End-turn cleanup resets both players' air-threshold cast counters, not just
//! the ending player. Extends the Sparkmage per-turn counter slice in 0149.
//! Post-action settlement tails slot before `magic-resolved`, so terminal
//! `game-ended` follows magic completion rather than `magic-cast`.

#[test]
fn rule_catalog_0728_end_turn_cleanup_resets_both_players_air_threshold_counts() {
    sorcery_engine::game::catalog_proofs::rule_catalog_0728_end_turn_cleanup_resets_both_players_air_threshold_counts();
}

#[test]
fn rule_catalog_0731_post_action_terminal_event_should_follow_magic_completion() {
    sorcery_engine::game::catalog_proofs::rule_catalog_0731_post_action_terminal_event_should_follow_magic_completion();
}
