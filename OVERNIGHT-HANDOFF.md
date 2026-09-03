# Overnight handoff

Branch: `cursor/phase3-drown-bury-artifacts-36d3` (ahead of origin by local tip). Nothing pushed from this shipper turn after `2d4363f`.

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

Tip: `2d4363f599d55a8c6ab14d5069b814a6ec737f5f`.

## Gate status at tip

- `cargo fmt` / `clippy -D warnings` / `cargo test --workspace --all-features --locked` — green when last run on the publicView work (ignored: two release-only soak gates).
- `pnpm verify` — **404 tests, 0 fail** (includes `tests/engine/rust-session-client.test.ts`).
- `pnpm game:demo 31` — canonical seed-31 report (`finalStateHash` `sha256:be86c59b…`, `transcriptHash` `sha256:fbdad70e…`, 230 actions, south wins).
- `pnpm play` — boots `http://127.0.0.1:4174` via Rust `session-json` (`RustSessionClient` in `game-server.ts`).

## Boundary cutover status

Done:
- Demo → Rust `sorcery-engine demo`.
- Batch → Rust `batch-json`.
- Play prototype → Rust `session-json` (`new` / `legalActions` / `step` / `publicView` / checkpoint / resume / verifyReplay).
- Parity fixture regeneration → Rust session helpers (`source: rust-legality-engine` for the main parity fixture).
- Interactive UI observation → Rust `Game::public_view` (opponent hands redacted to counts).

Still present:
- `src/engine/game.ts` still exists (~525KB). It remains for types, manifest helpers, `hashGameState` on exported Rust state, and any leftover TS-backed tests/scripts not fully migrated.
- Uncommitted WIP in the tree (do not treat as landed): edits under `tests/engine/game-setup.test.ts` (large), `game-demo.test.ts`, `game-counterfactual.test.ts`, `counterfactual.ts`, `run-private-game-check.ts`, `benchmarks/typescript-engine.ts`, plus untracked `tests/engine/rust-setup-session.ts`. Inspect/finish or discard before the next commit.

## Next exact step

1. Finish or discard the in-flight `game-setup` / counterfactual / private-check WIP so `pnpm verify` stays green on a clean tree.
2. Delete or gut superseded TS legality (`legalGameActions` / `stepGame` / `createGameSession` bodies) only after every public test routes through Rust session-json fail-closed.
3. Keep byte-identical Rust regeneration fixtures; do not reintroduce TypeScript as a second legality engine.
4. Phase 4+ product surfaces stay out of scope until that deletion is complete.

## Do not

- Commit `.local/authority/` or other private authority bytes.
- Push unless explicitly asked.
