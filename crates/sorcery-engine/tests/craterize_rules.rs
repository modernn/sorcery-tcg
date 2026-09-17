//! Direct proofs for Craterize discard-cost site destruction and damage grid
//! (RULE-CATALOG-0160, 0705–0706).
//!
//! 0657–0658 cover the Session replay slice. 0705 is the unprotected
//! discard/destroy/grid happy path; 0706 is discard-cost refusal plus
//! protected-site damage without destroying the site.

#[test]
fn rule_catalog_0705_craterize_discards_destroy_target_and_applies_damage_grid() {
    sorcery_engine::game::catalog_proofs::rule_catalog_0705_craterize_discards_destroy_target_and_applies_damage_grid();
}

#[test]
fn rule_catalog_0706_craterize_enforces_discard_cost_and_still_damages_protected_sites() {
    sorcery_engine::game::catalog_proofs::rule_catalog_0706_craterize_enforces_discard_cost_and_still_damages_protected_sites();
}
