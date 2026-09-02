# Reliable self-play gates

Self-play remains `unranked_partial_rules_unverified_authority` until the Rust engine covers every
exercised mechanic and the private authority checks pass. A policy win is not evidence of a correct
game when a manifest fact was accepted but ignored.

## Required invariants

- A manifest, seed, deck pair, and policy pair reproduce the same ordered engine-issued actions,
  terminal result, authoritative replay, state hash, and transcript hash.
- Callers select only from engine-issued legal actions. Unsupported facts fail closed before a game
  contributes to training, promotion, or audit results.
- Every evaluation seed runs both physical seat orientations. Training, promotion, and final-audit
  seeds are disjoint, and the opponent/scenario portfolio cannot drift between generations.
- Promotion requires at least 20 paired held-out seeds, a 5% normalized gain, an exact one-sided
  sign-test result under a campaign-wide 1% error budget, and no physical-seat or subgroup
  regression. A campaign has at most ten promotion attempts.
- The per-game action bound is immutable for the campaign. A promoted champion must significantly
  outperform the root policy again on a fresh replay-verified audit before the campaign is sealed.
- An unattended campaign must persist its canonical self-hashed checkpoint before process-restart
  recovery can be called reliable. The checkpoint ID provides content integrity, not a signature.

## Gate cadence

| Cadence | Gate | Purpose |
| --- | --- | --- |
| Every Rust change | locked workspace format, check, Clippy, and tests | Legality, replay, policy, and statistical regressions |
| Every self-play change | `cargo test --locked -p sorcery-engine --test selfplay` | Fast contract and failure-path coverage |
| Before accepting a self-play slice | `pnpm game:selfplay-acceptance` | Two independent 20-pair promotions and fresh 20-pair audits must agree exactly |
| After a clean reboot | `pnpm game:selfplay-soak` | Bounded termination, byte determinism, seat symmetry, and authoritative selected replay |
| Before ranked claims | all catalog scenarios proved in Rust plus private authority verification | No silent partial-rule results |

The release acceptance gate currently takes about three minutes on this host. It is ignored by the
ordinary debug suite because running the same workload there takes several minutes.

## Iteration loop

1. Freeze the authority revision, candidate deck, opponent portfolio, scenario identities, and
   three non-overlapping seed partitions.
2. Generate deterministic one-step policy neighbors and score them with lightweight Rust rollouts.
3. Nominate one child from training results; compare it with the champion on the locked held-out
   portfolio using paired seat swaps.
4. Promote only through the statistical and subgroup gates. Record failed candidates as results,
   not new policy lineage, and stop after the precommitted attempt budget.
5. Replay both the root and selected champion on fresh audit seeds; seal only when the gain repeats,
   then persist the campaign result.
6. Change one axis for the next experiment: policy or deck composition, never both in one attribution
   step. Price belongs in deck-search scoring, not in legality or transition state.

## Current blockers to strong unattended self-play

- Rust has direct proofs for 151 of 161 cataloged scenarios. The remaining mechanics must be ported
  or rejected at manifest admission before their games can affect training.
- Realm Artifacts are admitted only as power Artifacts, Lethal Artifacts, end-turn site-controller
  life-loss Artifacts, the Siege Ballista, the Payload Trebuchet, and the Rolling Boulder. The
  remaining ability Artifacts, plus any manifest that pairs an Artifact with burrow Magic or an
  oversized minion, still fail closed
  at admission.
- A Siege Ballista shoots three damage at one unit within two measured steps of the cell its bearer
  stands on, paid for by tapping that bearer and one other ready ally standing with it. The
  Artifact is the source, so the shot carries neither the bearer's power nor its Lethal, is not
  stopped by prevention keyed to unit power, and costs the bearer no Stealth.
- A Payload Trebuchet pays those same two taps plus one Atlas or Spellbook card discarded from hand,
  then deals that card's mana cost to every unit at one location within three measured steps of its
  bearer's cell; a discarded site throws nothing. The location is targeted rather than its
  occupants, so a Stealthed occupant is included and a layer below the target is not, and the
  Artifact is again the source, so no bearer power or Lethal rides along.
- A Rolling Boulder is pushed by tapping any one ready unit standing with it, needs no bearer, and
  rolls as far as one cardinal direction reaches without leaving its own region. Every other unit in
  that region takes four damage per passed cell it stands on, all at once, so no early death shields
  a later target; the pusher is spent rather than run over. The Artifact is the source, so no pusher
  power or Lethal rides along and prevention keyed to unit power does not stop it. A roll the realm
  blocks passes no cell, so it spends the pusher and damages nobody, and a carried Boulder leaves
  its bearer to finish loose at the endpoint.
- A tapped area-damage minion blankets one adjacent location in its own region with its current
  power and its carried Lethal. The blanket is not a strike, so it never draws a return strike.
- The selector neighborhood is intentionally small and `seat-observation-v1` does not expose enough
  state for strong tactical play; `powered-movement` is therefore inactive.
- Campaign state round-trips through a strict checkpoint with the seed set, lineage, fixed action
  bound, attempt budget, portfolio, audit state, and exact pending-suite commitment. An unattended
  caller must use `reserve_generation` or `reserve_final_audit`, publish that checkpoint atomically,
  then use the matching `complete_` method. The one-call wrappers are for supervised execution.
- Deck mutation, opponent-league rotation, cost-aware objectives, and post-reboot all-core soak
  reports come after the authoritative rule boundary is complete.
