use super::{GameError, IdentityHash, Seat};

/// A trigger source that can be ordered by its controller.
pub(super) trait TriggerSource {
    fn controller(&self) -> Seat;
    fn instance_id(&self) -> &IdentityHash;
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum TriggerOrderStage {
    ActiveOrder,
    NonActiveOrder,
    Resolve,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(super) struct TriggerBatch<T: TriggerSource> {
    pub(super) active_order: Vec<T>,
    pub(super) active_remaining: Vec<T>,
    pub(super) non_active_order: Vec<T>,
    pub(super) non_active_remaining: Vec<T>,
    pub(super) resolving: Vec<T>,
    pub(super) stage: TriggerOrderStage,
}

impl<T: TriggerSource> TriggerBatch<T> {
    pub(super) fn new(sources: Vec<T>, active_seat: Seat) -> Option<Self> {
        if sources.is_empty() {
            return None;
        }
        let (active, non_active): (Vec<_>, Vec<_>) = sources
            .into_iter()
            .partition(|source| source.controller() == active_seat);
        let active_needs_order = active.len() > 1;
        let non_active_needs_order = non_active.len() > 1;
        let stage = if active_needs_order {
            TriggerOrderStage::ActiveOrder
        } else if non_active_needs_order {
            TriggerOrderStage::NonActiveOrder
        } else {
            TriggerOrderStage::Resolve
        };
        let (active_order, active_remaining) = if active_needs_order {
            (Vec::new(), active)
        } else {
            (active, Vec::new())
        };
        let (non_active_order, non_active_remaining) = if non_active_needs_order {
            (Vec::new(), non_active)
        } else {
            (non_active, Vec::new())
        };
        let mut batch = Self {
            active_order,
            active_remaining,
            non_active_order,
            non_active_remaining,
            resolving: Vec::new(),
            stage,
        };
        if stage == TriggerOrderStage::Resolve {
            batch.start_resolving();
        }
        Some(batch)
    }

    fn start_resolving(&mut self) {
        self.resolving = std::mem::take(&mut self.non_active_order);
        self.resolving.append(&mut self.active_order);
        self.stage = TriggerOrderStage::Resolve;
    }

    pub(super) fn pending_order(&self) -> Option<&[T]> {
        match self.stage {
            TriggerOrderStage::ActiveOrder if self.active_remaining.len() > 1 => {
                Some(&self.active_remaining)
            }
            TriggerOrderStage::NonActiveOrder if self.non_active_remaining.len() > 1 => {
                Some(&self.non_active_remaining)
            }
            TriggerOrderStage::ActiveOrder
            | TriggerOrderStage::NonActiveOrder
            | TriggerOrderStage::Resolve => None,
        }
    }

    pub(super) fn commit(&mut self, source_instance_id: &IdentityHash) -> Result<(), GameError> {
        let active_stage = self.stage == TriggerOrderStage::ActiveOrder;
        let (committed, remaining) = if active_stage {
            (&mut self.active_order, &mut self.active_remaining)
        } else if self.stage == TriggerOrderStage::NonActiveOrder {
            (&mut self.non_active_order, &mut self.non_active_remaining)
        } else {
            return Err(GameError::IllegalAction);
        };
        if remaining.len() < 2 {
            return Err(GameError::IllegalAction);
        }
        let index = remaining
            .iter()
            .position(|source| source.instance_id() == source_instance_id)
            .ok_or(GameError::IllegalAction)?;
        committed.push(remaining.remove(index));
        if remaining.len() == 1 {
            committed.push(remaining.remove(0));
        }
        if remaining.len() > 1 {
            return Ok(());
        }
        if active_stage && self.non_active_remaining.len() > 1 {
            self.stage = TriggerOrderStage::NonActiveOrder;
            return Ok(());
        }
        self.start_resolving();
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Clone, Debug, Eq, PartialEq)]
    struct Source {
        controller: Seat,
        instance_id: IdentityHash,
    }

    impl TriggerSource for Source {
        fn controller(&self) -> Seat {
            self.controller
        }

        fn instance_id(&self) -> &IdentityHash {
            &self.instance_id
        }
    }

    fn source(controller: Seat, number: u8) -> Source {
        let digest = format!("sha256:{number:02x}{:0>62}", "");
        Source {
            controller,
            instance_id: IdentityHash::parse(&digest).unwrap(),
        }
    }

    #[test]
    fn active_then_non_active_priority() {
        let active_first = source(Seat::North, 1);
        let active_second = source(Seat::North, 2);
        let non_active_first = source(Seat::South, 3);
        let non_active_second = source(Seat::South, 4);
        let mut batch = TriggerBatch::new(
            vec![
                active_first.clone(),
                non_active_first.clone(),
                active_second.clone(),
                non_active_second.clone(),
            ],
            Seat::North,
        )
        .unwrap();

        assert_eq!(
            batch.pending_order().unwrap(),
            &[active_first, active_second]
        );
        let chosen = batch.pending_order().unwrap()[0].instance_id().clone();
        batch.commit(&chosen).unwrap();
        assert_eq!(batch.stage, TriggerOrderStage::NonActiveOrder);
        assert_eq!(
            batch.pending_order().unwrap(),
            &[non_active_first, non_active_second]
        );
    }

    #[test]
    fn caller_chosen_order_is_first_to_resolve_within_side() {
        let first = source(Seat::North, 1);
        let second = source(Seat::North, 2);
        let third = source(Seat::North, 3);
        let non_active = source(Seat::South, 4);
        let mut batch = TriggerBatch::new(
            vec![
                first.clone(),
                second.clone(),
                third.clone(),
                non_active.clone(),
            ],
            Seat::North,
        )
        .unwrap();

        batch.commit(second.instance_id()).unwrap();
        batch.commit(first.instance_id()).unwrap();
        assert_eq!(batch.stage, TriggerOrderStage::Resolve);
        assert_eq!(batch.resolving, vec![non_active, second, first, third]);
    }

    #[test]
    fn invalid_or_duplicate_selection_is_rejected() {
        let first = source(Seat::North, 1);
        let second = source(Seat::North, 2);
        let third = source(Seat::North, 3);
        let mut batch = TriggerBatch::new(vec![first.clone(), second, third], Seat::North).unwrap();

        let unknown = source(Seat::South, 9);
        assert!(matches!(
            batch.commit(unknown.instance_id()),
            Err(GameError::IllegalAction)
        ));
        batch.commit(first.instance_id()).unwrap();
        assert!(matches!(
            batch.commit(first.instance_id()),
            Err(GameError::IllegalAction)
        ));
    }
}
