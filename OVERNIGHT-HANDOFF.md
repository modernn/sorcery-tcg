# Overnight handoff

Integration branch: `codex/rust-cutover`. Shared-adapter foundation: `f95ff2c`. Started from later cutover work `9895d09` (`cursor/phase3-drown-bury-artifacts-36d3`). Do not reset to planning baseline `25424a8`. Local `master` remains at planning docs `504d576`. `origin/master` has 50 diverged feature commits from the same baseline; that line is preserved and is not this cutover.

Audit checkout `codex/overnight-cutover-audit` at `.local/worktrees/overnight-cutover-audit` is not an implementation tree. Do not edit `docs/reports/overnight-cutover-audit.md` or that worktree.

## Frozen adapter

`SetupCtx` and `RustGameSessionHandle` are the shared boundary. Workers must not edit `src/engine/rust-session-helpers.ts`, `src/engine/rust-engine.ts`, `src/engine/game.ts`, `tests/engine/rust-setup-session.ts`, shared Rust session/RPC files, or this handoff.

Frozen methods:

- `observe(seat)` is async and returns a validated Rust `publicView` as `GameObservation`. Opponent hands are counts; the viewer's hands are card lists. This is not the compact policy `observe` RPC.
- `stateHash()` is async and returns the Rust public-view state hash.
- `checkpoint()` returns a Rust checkpoint that can root an independent branch. Restore it with `resume()` before each alternative. Do not treat a mutable handle as a sibling snapshot.
- `step` / `stepRequest` return rejections without mutating state. `verifyReplay()` is the replay check.
- `withSetup` closes the child session on callback failure. `RustGameSessionHandle.open` also closes the spawned process if session creation fails.

Proof: `tests/engine/rust-setup-session.test.ts`.

## Frozen native novelty contract

Worker B must not change RPC dispatch. Request a dispatch addition against this shape; do not invent a second result schema.

Input: manifest JSON, seed, `maxActions` 0–500, width ceiling 128, policy `one-step-novelty-v1`. Output keeps current `NoveltyRolloutResult`: status `completed` | `horizon` | `failed`; offered/probed/committed coverage; frontier candidates with checkpoint id, predicted events, and predicted state hash; explicit failure evidence; `rulesCoverage: unranked_partial_rules`; classification `authority-private` only for private runs. Repeated synthetic runs must be byte-identical. Selected branches must replay. Do not start a process per probe.

Private gauntlet remains four orientation jobs, 32-branch ceiling, pruning, and stop-on-failure. Keep `loadPrivateStarterCatalog` and demo-manifest exports.

## Remaining legacy callers

- `tests/engine/game-setup-04.test.ts` still calls `createGameSession` / `legalGameActions` / `stepGame` / `verifyGameReplay` for the oversized-footprint proof.
- `tests/engine/game-setup-helpers.ts` still has sync TS fixtures (`action`, `keep`, `createGameSession`).
- `src/engine/game.ts` still implements those legality exports.
- `src/commands/run-private-game-check.ts` and setup files still call TS `observeGame` on exported state. Play paths otherwise use `SetupCtx`.
- Novelty selection now runs in Rust (`noveltyRollout`). Counterfactual search and gauntlet scheduling in `src/simulator/{counterfactual,gauntlet}.ts` still live in TypeScript and call Rust per transition. That remaining search is not yet native.
- `benchmarks/typescript-engine.ts` remains until its imports are retired.

Catalog stays 161 rust-supported / 0 typescript-supported. That label is not ranked readiness.

## Worker ownership

| Lane | Owns | Next step |
| --- | --- | --- |
| Integration | shared engine boundary, RPC, browser, parity scripts, this handoff | native novelty dispatch after Worker B requests it against the frozen schema |
| A | `tests/engine/game-setup-04.test.ts` and allocated Rust proof files | migrate the remaining oversized-footprint legality calls; do not edit helpers except by request |
| B | simulator, novelty commands, their tests, TS benchmark | deterministic native proof first; request RPC, do not edit `session_json.rs` |
| C | `src/commands/run-private-game-check.ts` and its private test | one scenario family per commit through the frozen adapter; private inputs stay pending if absent |

## Pending gates

Private `pnpm game:verify-private` and `pnpm game:novelty-private` are not claimed. Soak is not claimed. Results stay unranked.
