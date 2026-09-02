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

- Rust has direct proofs for 108 of 161 cataloged scenarios. The remaining mechanics must be ported
  or rejected at manifest admission before their games can affect training.
- The selector neighborhood is intentionally small and `seat-observation-v1` does not expose enough
  state for strong tactical play; `powered-movement` is therefore inactive.
- Campaign state round-trips through a strict checkpoint with the seed set, lineage, fixed action
  bound, attempt budget, portfolio, audit state, and exact pending-suite commitment. An unattended
  caller must use `reserve_generation` or `reserve_final_audit`, publish that checkpoint atomically,
  then use the matching `complete_` method. The one-call wrappers are for supervised execution.
- Deck mutation, opponent-league rotation, cost-aware objectives, and post-reboot all-core soak
  reports come after the authoritative rule boundary is complete.
