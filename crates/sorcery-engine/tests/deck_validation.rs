use sorcery_engine::deck::{
    CandidateDeck, CardCatalogEntry, CardCount, CardType, DeckCostStatus, DeckDiagnostic, DeckZone,
    FormatContext, OfficialCardMapping, PriceKey, PriceScope, PriceSnapshot, PrintingPrice, Rarity,
    price_deck, validate_deck,
};

fn entry(id: &str, card_type: CardType, rarity: Option<Rarity>) -> CardCatalogEntry {
    CardCatalogEntry {
        stable_id: id.to_owned(),
        card_type,
        rarity,
        engine_supported: true,
        official_mapping: OfficialCardMapping::Exact(format!("official:{id}")),
        token: false,
    }
}

fn counted(prefix: &str, count: usize) -> Vec<CardCount> {
    (0..count)
        .map(|index| CardCount {
            card_id: format!("card:{prefix}-{index:02}"),
            copies: 1,
        })
        .collect()
}

fn valid_fixture() -> (CandidateDeck, Vec<CardCatalogEntry>) {
    let candidate = CandidateDeck {
        avatar: "card:avatar".to_owned(),
        atlas: counted("site", 30),
        spellbook: counted("spell", 60),
    };
    let mut cards = vec![entry("card:avatar", CardType::Avatar, None)];
    cards.extend(
        candidate
            .atlas
            .iter()
            .map(|row| entry(&row.card_id, CardType::Site, Some(Rarity::Ordinary))),
    );
    cards.extend(
        candidate
            .spellbook
            .iter()
            .map(|row| entry(&row.card_id, CardType::Minion, Some(Rarity::Ordinary))),
    );
    (candidate, cards)
}

fn scope() -> PriceScope {
    PriceScope {
        variant: "standard".to_owned(),
        condition: "near-mint".to_owned(),
        currency: "USD".to_owned(),
        source: "market:test".to_owned(),
    }
}

fn price(card_id: &str, printing_id: &str, unit_price_cents: u64) -> PrintingPrice {
    PrintingPrice {
        key: PriceKey {
            official_card_id: format!("official:{card_id}"),
            printing_id: printing_id.to_owned(),
            variant: "standard".to_owned(),
            condition: "near-mint".to_owned(),
            currency: "USD".to_owned(),
            source: "market:test".to_owned(),
        },
        unit_price_cents,
    }
}

#[test]
fn modeled_constructed_accepts_exact_thirty_card_atlas_and_sixty_card_spellbook() {
    let (candidate, cards) = valid_fixture();

    let validation = validate_deck(candidate, &cards, FormatContext::constructed())
        .expect("valid modeled Constructed deck");

    assert!(validation.ranked_eligible, "{:#?}", validation.diagnostics);
}

#[test]
fn modeled_constructed_enforces_each_rarity_copy_limit() {
    let expected = [
        (Rarity::Ordinary, 4),
        (Rarity::Exceptional, 3),
        (Rarity::Elite, 2),
        (Rarity::Unique, 1),
    ];

    for (rarity, limit) in expected {
        let (mut candidate, mut cards) = valid_fixture();
        candidate.atlas[0].copies = limit + 1;
        cards
            .iter_mut()
            .find(|card| card.stable_id == candidate.atlas[0].card_id)
            .expect("featured site catalog entry")
            .rarity = Some(rarity);

        let validation = validate_deck(candidate, &cards, FormatContext::constructed())
            .expect("copy-limit diagnostic");

        assert!(
            validation.diagnostics.iter().any(|diagnostic| matches!(
                diagnostic,
                DeckDiagnostic::CopyLimitExceeded {
                    actual,
                    limit: actual_limit,
                    rarity: actual_rarity,
                    ..
                } if *actual == limit + 1 && *actual_limit == limit && *actual_rarity == rarity
            )),
            "missing {rarity:?} limit diagnostic: {:#?}",
            validation.diagnostics
        );
    }
}

#[test]
fn zone_shape_rejects_a_minion_in_the_atlas() {
    let (candidate, mut cards) = valid_fixture();
    cards
        .iter_mut()
        .find(|card| card.stable_id == candidate.atlas[0].card_id)
        .expect("first site catalog entry")
        .card_type = CardType::Minion;

    let validation = validate_deck(candidate, &cards, FormatContext::constructed())
        .expect("wrong-zone diagnostic");

    assert!(validation.diagnostics.iter().any(|diagnostic| matches!(
        diagnostic,
        DeckDiagnostic::WrongCardType {
            zone: DeckZone::Atlas,
            ..
        }
    )));
}

#[test]
fn unsupported_mechanics_preserve_format_legality_but_block_ranked_use() {
    let (candidate, mut cards) = valid_fixture();
    cards
        .iter_mut()
        .find(|card| card.stable_id == candidate.spellbook[0].card_id)
        .expect("first spell catalog entry")
        .engine_supported = false;

    let validation = validate_deck(candidate, &cards, FormatContext::constructed())
        .expect("unsupported-card diagnostic");

    assert_eq!(
        (
            validation.format_legal,
            validation.engine_supported,
            validation.ranked_eligible,
        ),
        (true, false, false)
    );
}

#[test]
fn canonical_identity_and_rows_ignore_candidate_row_order() {
    let (candidate, cards) = valid_fixture();
    let mut reversed = candidate.clone();
    reversed.atlas.reverse();
    reversed.spellbook.reverse();

    let forward =
        validate_deck(candidate, &cards, FormatContext::constructed()).expect("forward deck");
    let backward =
        validate_deck(reversed, &cards, FormatContext::constructed()).expect("reversed deck");

    assert_eq!(
        (forward.deck, forward.deck_id),
        (backward.deck, backward.deck_id)
    );
}

#[test]
fn pricing_selects_the_deterministic_minimum_printing_and_totals_exact_cents() {
    let (mut candidate, cards) = valid_fixture();
    candidate.spellbook[0].copies = 4;
    candidate.spellbook.truncate(57);
    let validation =
        validate_deck(candidate, &cards, FormatContext::constructed()).expect("valid counted deck");
    let mut prices = vec![
        price("card:avatar", "printing:avatar-expensive", 300),
        price("card:avatar", "printing:avatar-minimum-z", 250),
        price("card:avatar", "printing:avatar-minimum-a", 250),
    ];
    prices.extend(
        validation
            .deck
            .atlas
            .iter()
            .map(|row| price(&row.card_id, &format!("printing:{}", row.card_id), 10)),
    );
    prices.extend(validation.deck.spellbook.iter().map(|row| {
        let cents = if row.card_id == "card:spell-00" {
            25
        } else {
            20
        };
        price(&row.card_id, &format!("printing:{}", row.card_id), cents)
    }));
    let mut excluded = price("card:avatar", "printing:wrong-variant", 1);
    excluded.key.variant = "foil".to_owned();
    prices.push(excluded);
    let mut excluded = price("card:avatar", "printing:wrong-condition", 1);
    excluded.key.condition = "played".to_owned();
    prices.push(excluded);
    let mut excluded = price("card:avatar", "printing:wrong-currency", 1);
    excluded.key.currency = "CAD".to_owned();
    prices.push(excluded);
    let mut excluded = price("card:avatar", "printing:wrong-source", 1);
    excluded.key.source = "market:other".to_owned();
    prices.push(excluded);
    let snapshot = PriceSnapshot::new("snapshot:2026-08-31".to_owned(), prices)
        .expect("qualified price snapshot");

    let cost = price_deck(&validation, &cards, &snapshot, &scope()).expect("exact deck cost");

    let selected_printing = match &cost.lines[0].status {
        DeckCostStatus::Priced { key, .. } => Some(key.printing_id.as_str()),
        DeckCostStatus::AmbiguousMapping { .. } | DeckCostStatus::Unavailable { .. } => None,
    };
    assert_eq!(
        (cost.total_cents, selected_printing),
        (Some(1_770), Some("printing:avatar-minimum-a"))
    );
}

#[test]
fn pricing_reports_unavailable_and_ambiguous_official_mappings_without_a_total() {
    let (candidate, mut cards) = valid_fixture();
    cards
        .iter_mut()
        .find(|card| card.stable_id == "card:avatar")
        .expect("avatar catalog entry")
        .official_mapping = OfficialCardMapping::Ambiguous(vec![
        "official:z".to_owned(),
        "official:a".to_owned(),
        "official:z".to_owned(),
    ]);
    cards
        .iter_mut()
        .find(|card| card.stable_id == "card:site-00")
        .expect("site catalog entry")
        .official_mapping = OfficialCardMapping::Unavailable;
    let validation =
        validate_deck(candidate, &cards, FormatContext::constructed()).expect("valid deck");
    let snapshot =
        PriceSnapshot::new("snapshot:empty".to_owned(), Vec::new()).expect("empty snapshot");

    let cost = price_deck(&validation, &cards, &snapshot, &scope()).expect("explicit gaps");

    assert!(
        cost.total_cents.is_none()
            && matches!(
                cost.lines[0].status,
                DeckCostStatus::AmbiguousMapping { ref official_card_ids }
                    if official_card_ids == &["official:a", "official:z"]
            )
            && matches!(cost.lines[1].status, DeckCostStatus::Unavailable { .. }),
        "{:#?}",
        cost.lines
    );
}
