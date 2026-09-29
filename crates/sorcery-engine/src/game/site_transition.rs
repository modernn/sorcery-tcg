//! Deterministic site-incarnation boundaries derived from complete old/new footprints.

use super::effect::RealmReference;

#[derive(Clone, Debug, Eq, PartialEq)]
pub(super) struct SiteTransition {
    pub(super) entered: Vec<RealmReference>,
    pub(super) exited: Vec<RealmReference>,
}

impl SiteTransition {
    pub(super) fn between(before: &[RealmReference], after: &[RealmReference]) -> Self {
        Self {
            entered: difference(after, before),
            exited: difference(before, after),
        }
    }
}

fn difference(references: &[RealmReference], previous: &[RealmReference]) -> Vec<RealmReference> {
    references
        .iter()
        .fold(Vec::new(), |mut changed, reference| {
            if !previous.contains(reference) && !changed.contains(reference) {
                changed.push(reference.clone());
            }
            changed
        })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::canonical::identity_hash;
    use crate::game::{CardId, CardInstance, CardSource, Seat};

    fn reference(label: &str, realm_entry: u64) -> RealmReference {
        RealmReference::from_card(&CardInstance {
            card_id: CardId(0),
            instance_id: identity_hash(&serde_json::json!({"site-transition": label}))
                .expect("test identity"),
            owner: Seat::North,
            realm_entry,
            source: CardSource::Atlas,
        })
    }

    #[test]
    fn classifies_entered_and_exited_site_incarnations() {
        let shared = reference("shared", 1);
        let departed = reference("departed", 1);
        let arrived = reference("arrived", 1);
        let transition = SiteTransition::between(
            &[shared.clone(), departed.clone()],
            &[shared, arrived.clone()],
        );

        assert_eq!(transition.entered, [arrived]);
        assert_eq!(transition.exited, [departed]);
    }

    #[test]
    fn duplicate_footprint_cells_do_not_duplicate_a_site_boundary() {
        let same = reference("same", 1);
        let arrived = reference("arrived", 1);
        let transition = SiteTransition::between(
            &[same.clone(), same.clone()],
            &[same.clone(), same, arrived.clone(), arrived],
        );

        assert_eq!(transition.entered.len(), 1);
        assert!(transition.exited.is_empty());
    }

    #[test]
    fn realm_layer_change_is_a_no_op_for_the_same_incarnation() {
        let site = reference("site", 1);
        let transition =
            SiteTransition::between(std::slice::from_ref(&site), std::slice::from_ref(&site));

        assert!(transition.entered.is_empty());
        assert!(transition.exited.is_empty());
    }

    #[test]
    fn reentered_physical_card_is_a_new_site_incarnation() {
        let before = reference("same-card", 1);
        let after = reference("same-card", 2);
        let transition =
            SiteTransition::between(std::slice::from_ref(&before), std::slice::from_ref(&after));

        assert_eq!(transition.entered, [after]);
        assert_eq!(transition.exited, [before]);
    }

    #[test]
    fn differences_retain_footprint_order_without_duplicate_boundaries() {
        let a = reference("a", 1);
        let b = reference("b", 1);
        let c = reference("c", 1);
        let d = reference("d", 1);
        let transition = SiteTransition::between(
            &[b.clone(), a.clone(), b.clone()],
            &[d.clone(), c.clone(), d.clone()],
        );

        assert_eq!(transition.entered, [d, c]);
        assert_eq!(transition.exited, [b, a]);
    }
}
