# Overnight handoff

Branch: `cursor/phase3-drown-bury-artifacts-36d3`. Nothing pushed from this shipper turn.

## Catalog count

`data/rules/catalog.json`: **161 rust-supported / 0 typescript-supported** out of 161.

## Commits landed

| Hash | Change |
| --- | --- |
| `82c4dac` | Route synthetic demo rollouts through the Rust engine subprocess (prior parallel work). |
| _(this turn)_ | Rust `publicView` + TS `RustSessionClient`; harden demo hash types; session-json client proof. |

## Gate status

- `cargo fmt --all -- --check` — clean.
- `cargo clippy --workspace --all-targets --all-features --locked -- -D warnings` — clean.
- `cargo test --workspace --all-features --locked` — green (ignored: two release-only soak gates).
- `pnpm verify` — **404 tests, 0 fail** (was 403; added `tests/engine/rust-session-client.test.ts`).
- `pnpm game:demo 31` — matches expected seed-31 hashes (`finalStateHash` / `transcriptHash` unchanged).
- `pnpm play` — boots `http://127.0.0.1:4174` (still TypeScript legality for the HTTP prototype).

## Boundary cutover progress

Done:
- Demo path: `runGameDemo` → `runRustSyntheticDemo` → `sorcery-engine demo` (typed `sha256:` hashes, no casts on hashes).
- Batch path: already Rust `batch-json`.
- Interactive boundary: Rust `Game::public_view` + `session-json` method `publicView` (seat-scoped UI observation, opponent hands redacted to counts).
- TS client: `RustSessionClient` in `src/engine/rust-engine.ts` (prefers release `session-json` binary under `CARGO_TARGET_DIR` / `target/`).

Not done (honest remaining):
- `src/prototype/game-server.ts` / `pnpm play` still call `src/engine/game.ts` for `createGameSession` / `legalGameActions` / `stepGame` / `observeGame`.
- `src/engine/game.ts` still exists (~525KB) because public tests still exercise it (`game-setup.test.ts`, checkpoint/novelty/counterfactual tests) and parity fixture regeneration scripts still say `typescript-legality-engine`.
- Next exact step: rewrite `createGamePrototypeServer` onto `RustSessionClient` (`new` / `legalActions` / `step` / `publicView` / `checkpoint` / `resume` / `verifyReplay`), adapt `selectDeterministicGameAction` to Rust-issued actions + public view, then delete or quarantine superseded TS legality call sites once public coverage stays green fail-closed.

## Engine notes

- `publicView` derives unit attack/defense via `minion_current_stats` / avatar combat power, redacts opponent hands to counts, and exposes affinity / realm artifacts / auras / immobile areas for the play UI contract.
- Keep `observe` as the compact policy `SeatObservation`; do not overload it for UI.

## Do not

- Commit `.local/authority/` or other private authority bytes.
- Push unless explicitly asked.
