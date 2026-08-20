# Card data authorization outreach

This packet is a practical contact checklist, not legal advice. The current boundary permits one fixed, user-run, one-shot private collector with explicit risk acceptance; it does not establish publisher permission. Record written replies before expanding that boundary in `external-reuse-policy.md`.

The current exception is limited to exactly seven fixed official sources, an independent private backup, and no artwork. The user accepts the unresolved conflict between the public API guidance, the publisher Terms, and the API host's `robots.txt`. The collector stops on 401, 403, 429, CAPTCHA/block evidence, or publisher objection, with no retry or evasion. Recurring, scheduled, unattended, or agent-run acquisition still requires written permission, as do sharing, publication, hosting, commercial use, bulk art, and private-CDN access.

## Erik's Curiosa / Sorcery TCG

**To:** community@sorcerytcg.com  
**Backup contact:** <https://sorcerytcg.com/contact>  
**Subject:** Permission request: private Sorcery card-data simulator and periodic updates

Hello Erik's Curiosa team,

I am building a private, local, noncommercial Sorcery: Contested Realm rules simulator for personal deck testing. It will not be released, shared, sold, or exposed as a public API. Card artwork is excluded.

I would like written permission to:

- retrieve gameplay card data, rules, FAQs, errata, and format updates from your official sites and card API;
- retain immutable local snapshots and normalized machine-readable derivatives for reproducible simulations;
- compare revisions and check for updates automatically, no more than weekly and normally every 14 days;
- use automated agents to validate completeness and consistency against the official rules, with human review before accepting a revision; and
- continue using previously retrieved snapshots privately if an endpoint changes or is retired.

Could you confirm which endpoints and fields may be used, the permitted request frequency, required attribution, retention requirements, and any other conditions? A single full-catalog request with conditional requests or checksums would be preferred. I will follow any rate limit or update mechanism you specify.

The public API page encourages intermittent polling, diffing, and self-hosting, while the site Terms restrict automated access and systematic database creation without written permission. Please confirm that this written permission controls for the private uses listed above.

This request excludes card images and any redistribution, public service, commercial use, or publication of your data. I would ask separately before changing that scope.

Thank you,

[Name]

### Reply checklist

Record the responder, date, permitted sources/fields, caching and derivative-data terms, request cadence, attribution, revocation/snapshot terms, and whether further approval is needed for a separate updater tool. Save the original message and reply in the private authority records; commit only a non-content receipt and the approved operating boundaries.

## Sad King Labs / sorcery-registry

**Channel:** <https://github.com/sadkinglabs/sorcery-registry/issues/new>  
**Title:** Permission/license clarification for registry-created metadata

Hello,

I am building a private, noncommercial Sorcery rules simulator and found `sorcery-registry` useful as a model for stable identifiers, schema design, checksums, and revision history. I understand that the repository's software license does not grant rights to Erik's Curiosa's underlying card content.

Would you be willing to identify and license the registry-created portions—such as original stable IDs, mappings, slug/name history, corrections, schema, checksum/manifest format, and export structure—under a data-appropriate license such as CC0, CC BY 4.0, or ODC-By? If some items are copied from or controlled by the publisher, could you distinguish those from original project metadata?

I will obtain publisher permission separately for the card content and will respect any attribution or share-alike requirements you specify. A written statement identifying the covered files/fields and license would be especially helpful.

Thank you,

[Name]

## Simulator versus separate updater

| Choice | Build now | Cost and boundary | Decision |
|---|---|---|---|
| Simulator-local import | One fixed user-run private snapshot, validation, normalization, pinned revision | Narrow one-shot risk-accepted exception; no recurring or agent-run acquisition | **Continue now** |
| Separate catalog updater | Authorized connectors, immutable raw snapshots, source/license registry, semantic diffs, agent validation, human approval, versioned export, scheduled run | Separate product and ongoing operations; cannot create rights to source data | **Start only after written permission, or when a permissive source and a second consumer exist** |

The future updater should publish a signed/versioned local snapshot for the simulator to import. It should not become a public HTTP API unless that scope is explicitly authorized. Default update cadence should be every 14 days; reduce to weekly only when permitted and useful.

## Decision gate

- **Written permission covers automated access and local derivatives:** plan the separate updater in its own project/thread.
- **Permission covers only private use:** keep the current fixed user-run one-shot path and do not add recurring or agent-run acquisition.
- **No response or ambiguous response:** treat it as no expanded permission; only the narrow private one-shot exception remains available without a legal conclusion.
- **Redistribution, artwork, public API, or commercial use becomes desirable:** request a separate explicit grant before implementation.

## Verified source links

- [Official Terms and contact clause](https://sorcerytcg.com/terms)
- [Official API guidance](https://api.sorcerytcg.com/) and [card endpoint](https://api.sorcerytcg.com/api/cards)
- [Official contact form](https://sorcerytcg.com/contact)
- [Sad King Labs repository](https://github.com/sadkinglabs/sorcery-registry), including its README, license, and contribution guidance
