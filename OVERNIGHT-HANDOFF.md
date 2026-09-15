# Overnight handoff

Integration branch: `codex/rust-cutover`. Tip: `57989d8`. Shared-adapter foundation: `f95ff2c`. Native novelty: `4d44956`. Cascade strip: `ac884a6`. Native counterfactual/gauntlet: `6618d1a`. Native private novelty frontier: `57989d8`.

Started from later cutover work `9895d09` (`cursor/phase3-drown-bury-votes-36d3`). Do not reset to planning baseline `25424a8`. Local `master` remains at planning docs `504d576`.

Failed lane worktrees `C:\src\sorcery-wt-public` / `C:\src\sorcery-wt-private` were clean at `2c1eab2` and removed. Integration owner absorbed those lanes from `C:\src\sorcery-tcg`.

Audit checkout `codex/overnight-cutover-audit` at `.local/worktrees/overnight-cutover-audit` is not an implementation tree. Do not edit `docs/reports/overnight-cutover-audit.md` or that worktree.

## Frozen adapter

`SetupCtx` and `RustGameSessionHandle` are the shared boundary.

Frozen methods:

- `observe(seat)` is async and returns a validated Rust `publicView` as `GameObservation`.
- `stateHash()` is async and returns the Rust public-view state hash.
- `checkpoint()` / `resume()` root independent branches.
- `step` / `stepRequest` return rejections without mutating state. `verifyReplay()` is the replay check.
- `withSetup` and `RustGameSessionHandle.open` close child sessions on failure.

Proof: `tests/engine/rust-setup-session.test.ts`.
Boundary regression: `tests/engine/retired-ts-legality.test.ts` (retired TS legality/observe exports fail closed).

## Completed this cutover pass

- Migrated `game-setup-04` oversized-footprint proof to `SetupCtx`; deleted sync TS helpers from `game-setup-helpers.ts`.
- Fail-closed `createGameSession`, `legalGameActions`, `stepGame`, `replayGame`, `verifyGameReplay`, and `observeGame` in `src/engine/game.ts`.
- Stripped dead TypeScript helper bodies from `src/engine/game.ts`; file keeps DTOs, `createGameManifest` / `assertCanonicalGameManifest` / `hashGameState`, and fail-closed stubs.
- Routed public setup tests and `run-private-game-check.ts` observations through Rust `publicView` (`ctx.observe` / `observeAt`).
- Native novelty search via `noveltyRollout` (TS glue only).
- Native counterfactual search via `counterfactualRollout` (TS glue only).
- Native two-deck gauntlet via `sorcery-engine gauntlet-json` (TS glue only).
- Native private novelty frontier via `sorcery-engine novelty-gauntlet-json` (TS loads manifests / writes `.local/authority/` only).
- Fixed Rust `publicView` Avatar attack/defense (temporary + carried + nearby-ally power) and immobile flags (Avatar area immobilize; disabled minions ignore printed Immobile).
- Swept `src` / `tests` / `scripts` / `benchmarks`: no live TS legality callers remain (retired stubs + boundary test only). No `benchmarks/typescript-engine.ts`; historical JSON baselines retained.

## Remaining / follow-ups

- `hashGameState` remains identity hashing of exported state (serialization), not a rules engine.
- Interactive demo/browser still uses TS `selectDeterministicGameAction` over Rust-issued actions; batch/selfplay/counterfactual use Rust `PolicySnapshot`.
- Clean-reboot soak still pending (`pnpm game:selfplay-soak` / ignored Rust soak).
- Results stay unranked while supported mechanics remain incomplete. Do not claim ranked readiness.

Catalog stays 161 rust-supported / 0 typescript-supported. That label is not ranked readiness.

## Gates recorded on tip `57989d8`

- Release binaries rebuilt into this checkout's `target/release` (`session-json`, `sorcery-engine`) before JS checks that launch them.
- `pnpm verify` green (typecheck + lint + 407 public tests).
- `cargo fmt --check`, `cargo check --workspace --all-targets --all-features --locked`, `cargo clippy -D warnings`, `cargo test --workspace --all-features --locked` green.
- `pnpm game:verify-private` green (3 private scenarios).
- `pnpm game:novelty-private` green invoke; report `.local/authority/reports/official-2026-08-27-v4/novelty-gauntlet.json` with `failed: 3` / `completed: 1` job totals — stay unranked.
- Soak not run.
