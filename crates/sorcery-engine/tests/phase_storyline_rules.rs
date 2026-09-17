//! Direct proofs for Phase and Storyline cleanup (RULE-CATALOG-0728, 0732).
//!
//! End-turn cleanup resets both players' air-threshold cast counters, not just
//! the ending player. Extends the Sparkmage per-turn counter slice in 0149.
//! Ordered terminal cleanup omits resolved Chain Magic from authoritative state.

#[test]
fn rule_catalog_0728_end_turn_cleanup_resets_both_players_air_threshold_counts() {
    sorcery_engine::game::catalog_proofs::rule_catalog_0728_end_turn_cleanup_resets_both_players_air_threshold_counts();
}

#[test]
fn rule_catalog_0732_ordered_terminal_cleanup_should_omit_resolved_chain_magic() {
    sorcery_engine::game::catalog_proofs::rule_catalog_0732_ordered_terminal_cleanup_should_omit_resolved_chain_magic();
}
