# OVERNIGHT QUEUE — Rust cutover / Phase 3 core rules

_Read-mostly gap scout snapshot. No GSD. Engine source untouched except this file._
_Generated for the primary Opus shipper. Verify counts before trusting; do not treat as authoritative rules coverage._

## 1. Current state

- **Branch:** `rust-engine-cutover` (up to date with `origin/rust-engine-cutover`).
- **HEAD:** `93278e6 wip(engine): land Leap Attack Rust path and proofs`.
- **Uncommitted working tree** (4 files, this is the live WIP):
  - `crates/sorcery-engine/src/game.rs` — moves `LeapAttackAlly` out of the unsupported-magic reject list into the supported set (+ unit-test wiring).
  - `data/rules/catalog.json` — flips the two Leap Attack rules (`RULE-0021`, `RULE-0022`) from `typescript-supported` → `rust-supported` and re-points `scenarioProof` to `crates/sorcery-engine/tests/magic_rules.rs`.
  - `docs/self-play-reliability.md` — Rust direct-proof count `103 → 105`.
  - `tests/engine/leap-attack-action-parity.test.ts` — local `CastMagicDescriptor` type so `allyDestination` typechecks.
- **Leap Attack WIP status:** essentially COMPLETE but UNCOMMITTED. The committed Rust proofs already exist (`magic_rules.rs::rule_catalog_0021_…` and `…_0022_…`, plus `assert_oversized_leap_attack_focuses_one_occupied_cell`). Fixture `tests/engine/fixtures/leap-attack-action-v1.json` is present.
- **Typecheck:** GREEN **only with the working-tree changes applied** (`pnpm typecheck` → exit 0, 6.5s). At the bare `93278e6` commit the committed parity test references `allyDestination` on the narrowed `Extract<…, {kind:'cast-magic'}>` union, so **HEAD-as-committed leaves `pnpm typecheck` (and therefore `pnpm verify`) broken** until the uncommitted fix lands. First job tonight = commit the WIP.
- **Cutover ledger (working tree `data/rules/catalog.json`):** 100 `rust-supported`, 56 `typescript-supported`. The 56 TS-only rules are the cutover backlog below. (`docs/self-play-reliability.md` tracks its own "105 of 161" figure.)
- **Parity harness assets present:** capture scripts + `v1.json` fixtures for cast-magic, combat-response, deathrite-order, duel, shoot-projectile, shoot-damage-projectile, sparkmage, site-destruction, random-card-discard-summon, sacrifice-summon, leap-attack. `crates/sorcery-engine/tests/action_parity.rs` consumes all of them except leap-attack (which got a native `magic_rules.rs` proof instead).
- **Stale-status finding:** `sparkmage` is still `typescript-supported` in the catalog (line ~2995) **but already has a full Rust proof** (`sparkmage_rules.rs::air_thresholds_should_select_a_hidden_candidate_deterministically_damage_it_and_reset`). The catalog status ledger is drifting — audit it (item P1-A) before porting anything, or you will re-port work that is already done.

---

## 2. Prioritized top-15 overnight queue

Priority key: **P0** = unblock everything / restore green, **P1** = cheap high-leverage cutover, **P2** = leverage but heavier, **P3** = opportunistic.

### P0-1 — Commit the Leap Attack WIP and restore a green tree
- **Why:** HEAD-as-committed is red on `pnpm typecheck`/`verify`; the fix is already sitting uncommitted. Nothing else should be built on a red tree, and a half-landed WIP invites drift. This is a 5-minute win that unblocks the whole night.
- **Touch points:** the 4 uncommitted files only. Run `pnpm typecheck && pnpm lint && pnpm test` (or `pnpm verify`) and `cargo test -p sorcery-engine --test magic_rules --locked` (targeted) + `cargo fmt --all -- --check`.
- **Evidence:** working-tree diff already flips catalog to `rust-supported`; `magic_rules.rs` proofs exist; typecheck passes with the tree applied.
- **Commit subject:** `fix(engine): finish Leap Attack cutover and restore green typecheck`

### P0-2 — Verify the full Rust suite is green post-WIP (targeted, not the soak)
- **Why:** The +557-line `game.rs` change from `93278e6` touched the shared magic-resolution path; confirm no regression in `magic_rules`, `action_parity`, `deathrite_rules`, `combat_rules` before layering more ports. Skip the ignored selfplay soak.
- **Touch points:** `cargo test -p sorcery-engine --tests --locked` limited to the four suites above; `cargo clippy --workspace --all-targets --all-features --locked -- -D warnings`.
- **Evidence:** `game.rs` diff in `93278e6` reworked `unsupported_magic_effect` + leap-attack resume path shared with movement Deathrites.
- **Commit subject:** _(no commit — verification gate; fold fixes into P0-1 if any surface)_

### P1-A — Reconcile catalog `implementationStatus` against real Rust proofs
- **Why:** `sparkmage` (and likely others) is proven in Rust but still marked `typescript-supported`, so the "56 remaining" backlog is overstated and the shipper cannot trust the ledger to pick work. A one-pass audit flips already-ported rules and prevents duplicate effort — highest leverage per minute tonight.
- **Touch points:** `data/rules/catalog.json` (`scenarioProof` + `implementationStatus`); cross-check every `typescript-supported` rule against `crates/sorcery-engine/tests/*.rs` test names; update `docs/self-play-reliability.md` count.
- **Evidence:** `sparkmage_rules.rs` full proof vs catalog line ~2995 `typescript-supported`; fixtures exist for several already-consumed mechanics.
- **Commit subject:** `chore(catalog): reconcile rule implementation status with Rust proofs`

### P1-B — Port drag projectile (Pudge Butcher) to Rust
- **Why:** Fixture `shoot-projectile-action-v1.json` + capture script already exist and are consumed by `action_parity.rs`; the remaining gap is the two TS-only rules "drag projectile stops at first visible unit / resumes after ordered movement Deathrites" (catalog ~1858, ~1876). Cheapest projectile port because the parity harness and movement-Deathrite serialization already landed for Leap Attack — reuse that resume machinery.
- **Touch points:** `crates/sorcery-engine/src/game.rs` (projectile drag + Deathrite resume), `crates/sorcery-engine/tests/ranged_projectile_rules.rs` or new `drag_projectile_rules.rs`, `data/rules/catalog.json`.
- **Evidence:** TS proof `tests/engine` Pudge branches; `ranged_projectile_rules.rs` present; drag serialization pattern mirrors landed Leap Attack resume.
- **Commit subject:** `feat(engine): support Pudge drag projectile and Deathrite resume`

### P1-C — Port targeted-relocation Magic: Teleport + Lure
- **Why:** Both are self-contained targeted spells (no new region model), each proven in TS teaching decks (11-action Teleport, 17-action Lure). They unlock real Air/Water cards and share the existing cast-magic + movement path. `cast-magic-action-v1.json` fixture already covers the enumeration contract.
- **Touch points:** `crates/sorcery-engine/src/game.rs` (magic effects `Teleport`, `LureEnemyMinionOneStepCloser`), `crates/sorcery-engine/tests/magic_rules.rs`, `data/rules/catalog.json` (~736, ~751).
- **Evidence:** TS proofs in `03-PROGRESS.md` lines 31 (Lure), 80 (Teleport); cast-magic parity fixture present.
- **Commit subject:** `feat(engine): support Teleport and Lure targeted magic`

### P1-D — Port Fatality (kill wounded minion) + Mesmerism (control transfer)
- **Why:** Fatality is a small, already-TS-proven non-damage kill routed through the shared owner-cemetery pipeline; Mesmerism is the one supported control-change and unlocks the Kettletop Deathrite transfer scenario. Both are single-effect magic ports with existing TS receipts.
- **Touch points:** `game.rs` (`KillTargetWoundedMinion`, `GainControlOfTargetNearbyMinion`), `magic_rules.rs`, `catalog.json` (~2959, ~2942).
- **Evidence:** TS proofs `03-PROGRESS.md` line 100 (Fatality), line 139 (Mesmerism); both currently in `unsupported_magic_effect` reject list in `game.rs`.
- **Commit subject:** `feat(engine): support Fatality kill and Mesmerism control transfer`

### P1-E — Port Bury + Drown forced-terrain Magic
- **Why:** Both force a target underground/underwater and immediately settle region survival through the shared Deathrite pipeline — the settlement half already exists in Rust; only the forced-move magic effects are TS-only (catalog ~834 Bury-artifact, ~875 Drown). Unlocks Earth Bury and Water Drown teaching decks.
- **Touch points:** `game.rs` (`SubmergeTargetMinion` + Bury artifact detach), `magic_rules.rs`, `catalog.json`.
- **Evidence:** TS proofs `03-PROGRESS.md` lines 57-58; `SubmergeTargetMinion` still in reject list per `game.rs` diff context.
- **Commit subject:** `feat(engine): support Bury and Drown forced-terrain magic`

### P2-F — Port the subsurface/void region model (Burrowing, Submerge, Voidwalk)
- **Why:** Single biggest teaching-deck unlock — Cave Trolls, Entombed, Sea Witch, Coral-Reef Kelpie, Spectral Stalker, Forsaken, Pirate Ship, etc. all gate on it. But it is the deepest `game.rs` change (region graph, summon locations, movement, region settlement) and currently blanket-rejected at manifest admission. Do it as one focused wave after the cheap magic ports, not interleaved.
- **Touch points:** `game.rs` region graph + settlement + summon legality; `crates/sorcery-engine/tests/{summon_rules,combat_rules,new region_rules}.rs`; manifest admission; `catalog.json` (~2268, ~2291, ~2389, ~982, ~1002).
- **Evidence:** extensive TS proofs `03-PROGRESS.md` lines 67-72, 110-115, 134; region settlement already partially in Rust.
- **Commit subject:** `feat(engine): support subsurface and void region movement`

### P2-G — Port cast/placement restrictions that ride on the region model
- **Why:** must-be-burrowed / must-be-submerged / outer-column / Water-site / Secret Tunnel / Planar Gate restrictions (~2313, ~2332, ~2352, ~2429, ~2412, ~2372) are cheap once P2-F lands and directly gate more real-card summon legality. Batch after F to avoid re-touching summon enumeration twice.
- **Touch points:** `game.rs` cast-location enumeration; `summon_rules.rs`; `catalog.json`.
- **Evidence:** TS proofs `03-PROGRESS.md` lines 73-76, 66-67.
- **Commit subject:** `feat(engine): support region-gated casting restrictions`

### P2-H — Port Waterbound derived-Disable + subsurface-Disable settlement
- **Why:** Waterbound (~960) and "disabling a subsurface minion immediately settles its region" (~510) are continuous-ability-loss interactions that depend on P2-F terrain facts; porting them closes the Pirate Ship / Waterbound Stealth-loss teaching deck. Fail-closed combinations already specified in TS.
- **Touch points:** `game.rs` derived-Disable + state-based settlement; `readiness_affinity_rules.rs` / new test; `catalog.json`.
- **Evidence:** TS proof `03-PROGRESS.md` line 71.
- **Commit subject:** `feat(engine): support Waterbound derived Disable and settlement`

### P2-I — Port the Artifact interaction layer (Pick Up / Drop / carried Lethal)
- **Why:** Pick Up/Drop (~2819), Drop-kills-wounded-bearer (~2839), carried Lethal Artifact (~2859) unlock Sword and Shield / Poisonous Dagger / Devil's Egg decks and are a prerequisite for the siege-artifact family (P3). Self-contained action layer over units, low region coupling.
- **Touch points:** `game.rs` artifact carry/drop actions + power derivation; new `artifact_rules.rs`; `catalog.json`.
- **Evidence:** TS proofs `03-PROGRESS.md` lines 85, 119-120, 182.
- **Commit subject:** `feat(engine): support artifact pick up, drop, and carried Lethal`

### P2-J — Port oversized 2x2 footprint + timed 2x2 Auras (Mountain Giant, Quagmire, Entangle)
- **Why:** The generic fixed-2x2 fact (~1321) plus Quagmire (~1267), Entangle (~1285), and end-turn Aura random damage (~1302) share one footprint/area primitive; porting the primitive once unlocks several retail Earth cards. Heavy geometry work but isolated to footprint/aura code.
- **Touch points:** `game.rs` footprint enumeration + aura timers (see `footprint_rules.rs` already present); `catalog.json`.
- **Evidence:** TS proofs `03-PROGRESS.md` lines 190-192; `footprint_rules.rs` scaffold exists.
- **Commit subject:** `feat(engine): support oversized footprints and timed 2x2 auras`

### P3-K — Port global/aura power effects (House Arn, King of the Realm) + aura-loss Deathrite
- **Why:** Nearby-allies power (~656), controlled-Mortal power (~675), and aura-loss Deathrite ordering (~695, ~716) are derived-power settlement effects; medium leverage (Earth boxed lesson) but they retouch the same survival-settlement code as P2-F/H, so sequence them after those to avoid churn.
- **Touch points:** `game.rs` derived-power + death settlement; `catalog.json`.
- **Evidence:** TS proofs `03-PROGRESS.md` lines 188-189.
- **Commit subject:** `feat(engine): support global aura power and aura-loss deaths`

### P3-L — Port random-selection Magic (Lightning Bolt location, Lucky Charm, Raise Dead, start-turn teleport)
- **Why:** These (~530, ~549, ~3193, ~3174) all exercise the seeded-random receipt path; batch them because they share the PRNG-evidence contract and each is a small effect. Lower leverage than region work but cheap and improves selfplay novelty coverage.
- **Touch points:** `game.rs` random-draw receipts; `magic_rules.rs`; `catalog.json`.
- **Evidence:** TS proofs `03-PROGRESS.md` lines 102, 186; `shoot-damage-projectile`/`sparkmage` fixtures show the random-receipt pattern.
- **Commit subject:** `feat(engine): support seeded-random selection magic`

### P3-M — Port siege-artifact activations (Siege Ballista, Payload Trebuchet, Rolling Boulder) + end-turn Artifact life loss
- **Why:** ~2883, ~2904, ~2924, ~3111/3130/3151 depend on P2-I artifact carry; they are the retail Earth siege payload set and Devil's Egg. Lowest urgency (few teaching decks, heavy) — do last, only if the region and artifact waves finished cleanly.
- **Touch points:** `game.rs` carried-artifact activation + measured-range damage; new `artifact_siege_rules.rs`; `catalog.json`.
- **Evidence:** TS proofs `03-PROGRESS.md` lines 196-198, 182.
- **Commit subject:** `feat(engine): support siege artifact activations`

---

## 3. Do NOT touch tonight

- **No GSD skills/commands or `.planning/` phase machinery.** Read `03-PROGRESS.md` only as an inventory input.
- **Phases 4–9** (anything beyond Phase 3 core rules): search/scenario tooling expansion, ranked-match overlays, tournament endings, LLM explainers/competitors, browser UI polish beyond parity.
- **Private authority packaging / release:** do not run `pnpm authority:verify-private` or `authority:release` work, do not touch `.local/authority/`, do not commit/normalize official bytes. No `authority:import`.
- **Do not delete the superseded TypeScript legality engine yet.** AGENTS.md mandates deletion *after* parity + cutover; parity is still ~56 rules short. Deleting `src/engine/game.ts` now would destroy the fixtures/parity source. Deletion is a post-cutover task, not tonight.
- **Do not run the long/ignored suites** (`game:selfplay-acceptance`, `game:selfplay-soak`, `game:novelty-private`) unless a specific regression needs them; they cost minutes each.
- **Do not tune game balance / rewrite real rules** to make a port pass — an unsupported mechanic must fail closed at manifest admission, never become a silent no-op.
- **No new dependencies or abstractions** without reusing the existing parity-harness pattern first.

---

## 4. Parallelization notes

**The bottleneck is `crates/sorcery-engine/src/game.rs`.** Almost every port edits its magic/movement/settlement paths, so the actual implementation commits must be **serialized** (one coherent verified commit each, per AGENTS.md). Sequence: `P0-1 → P0-2 → P1-A → P1-B → P1-C → P1-D → P1-E → P2-F → P2-G/H → P2-I → P2-J → P3-*`.

**What CAN be prepped in parallel without touching `game.rs`** (safe to fan out to helpers/fixtures first):
- Capture scripts + `tests/engine/fixtures/*-v1.json` for any not-yet-captured mechanic (Teleport, Lure, Fatality, Mesmerism, Bury, Drown) — mirror `scripts/capture-leap-attack-action-parity.ts`. These are additive files; no `game.rs` contention.
- Rust test scaffolds in **separate** `crates/sorcery-engine/tests/*.rs` files (e.g. a new `drag_projectile_rules.rs`) — the test file is independent even though the impl it calls lands later.
- **P1-A catalog reconciliation** is pure `catalog.json`/docs and conflicts with nobody except other catalog edits — do it first so subsequent ports flip status in a clean ledger.
- `docs/self-play-reliability.md` count bumps.

**Hard conflicts (never run two of these against `game.rs` concurrently):** P1-B/C/D/E (shared `unsupported_magic_effect` + cast-magic dispatch), and the entire P2 region wave (F/G/H share region-graph + settlement). If using worktrees/subagents, give each its own branch and rebase serially; do not let two agents edit `game.rs` in the same tree.

**Cheapest ordering rationale:** fixture-backed magic ports (P1-B…E) reuse the just-landed Leap Attack Deathrite-resume machinery and the existing parity fixtures, so they carry the least new risk. The region model (P2-F) is the highest teaching-deck unlock but the deepest `game.rs` surgery — do it as a dedicated wave after the cheap wins bank progress and keep the tree green between every commit.
