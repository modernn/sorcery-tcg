# Overnight handoff

Branch: `cursor/phase3-drown-bury-artifacts-36d3` is the integration line. `master` is fast-forwarded to it. Superseded branches and stashes live only as `archive/*` tags (`git tag -l 'archive/*'`).

## Catalog count

`data/rules/catalog.json`: **161 rust-supported / 0 typescript-supported** out of 161.

## Commits landed (boundary cutover line)

| Hash | Change |
| --- | --- |
| `82c4dac` | Route synthetic demo rollouts through the Rust engine subprocess. |
| `2d6270f` | Add session-json RPC bridge and migrate parity capture to Rust. |
| `ca6bb4c` | Route playable-core game server through Rust session-json. |
| `397bdf5` / `8dc140e` | Migrate action parity capture scripts to Rust session helpers. |
| `3d30a81` | Route checkpoint resume test through Rust session-json. |
| `2d4363f` | Record handoff + harden `RustSessionClient` launch/types (`publicView` path). |
| `a4a81e5` | Attempted scripted migration of `game-setup.test.ts` to `SetupCtx`; left the file unparseable. |
| `a1d0e5b` | Restored `game-setup.test.ts` from `eecfa00`; kept `tests/engine/rust-setup-session.ts` bridge. |
| `167c6e3` | Aligned Rust `action_parity` and `drag_projectile_rules` proofs with Rust-captured fixtures (checkpoint hash is now of the object, not the serialized string). |

## Gate status at tip

- `cargo fmt` / `clippy -D warnings` / `cargo test --workspace --all-features --locked` — green (2 ignored release soak gates).
- `pnpm verify` — **404 tests, 0 fail**.

## Boundary cutover status

Done:
- Demo → Rust `sorcery-engine demo`.
- Batch → Rust `batch-json`.
- Play prototype → Rust `session-json` (`new` / `legalActions` / `step` / `publicView` / checkpoint / resume / verifyReplay).
- Parity fixture regeneration → Rust session helpers.
- Interactive UI observation → Rust `Game::public_view`.

Still present:
- `src/engine/game.ts` (~525KB) still exports `createGameSession` / `legalGameActions` / `stepGame`. Callers: `tests/engine/game-setup.test.ts`, `game-server.test.ts`, `game-novelty-rollout.test.ts`, `src/simulator/novelty-rollout.ts`, `src/engine/checkpoint.ts`, `src/commands/run-game-demo.ts`, `run-private-game-check.ts`, `run-private-novelty-gauntlet.ts`, `benchmarks/typescript-engine.ts`.
- `tests/engine/game-setup.test.ts` is the pre-migration TypeScript version (161 tests). The `SetupCtx` / `withSetup` / `withPreview` bridge in `tests/engine/rust-setup-session.ts` is ready but unused.

## Next exact step

1. Migrate `game-setup.test.ts` to `SetupCtx` in small reviewed chunks (one rule family per commit, `pnpm verify` green each time). Do not repeat the one-shot scripted rewrite; its scripts are archived under `.local/archive/2026-09-02-game-setup-migration/`.
2. Then gut the TypeScript legality bodies in `src/engine/game.ts` once every caller above routes through Rust session-json fail-closed.
3. Keep byte-identical Rust regeneration fixtures; do not reintroduce TypeScript as a second legality engine.
4. Phase 4+ product surfaces stay out of scope until that deletion is complete.

## Do not

- Commit anything under `.local/` (whole directory is ignored; `.local/authority/` holds private authority bytes).
- Run more than one agent against this working tree at a time. A Cursor agent and a Claude session overwrote each other here on 2026-09-02.
