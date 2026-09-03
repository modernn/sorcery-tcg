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
| `167c6e3` | Aligned Rust `action_parity` and `drag_projectile_rules` proofs with Rust-captured fixtures. |
| `6276f41` | Fail-closed demo agent (issued actions required); remove unused TS `runDeterministicGame`; migrate first RULE-01/TEST-03 setup proofs + playable-core opening-hand peek to Rust `SetupCtx` / `withRustSession`. |

Tip: `6276f41`.

## Gate status at tip

- `pnpm verify` — **404 tests, 0 fail**.
- `pnpm game:demo 31` — canonical seed-31 report (`finalStateHash` `sha256:be86c59b…`, `transcriptHash` `sha256:fbdad70e…`, 230 actions, south wins).
- `pnpm play` / `node src/prototype/game-server.ts` — boots `http://127.0.0.1:4174` via Rust `session-json` (HTTP 200).
- Rust workspace gates not re-run this turn (no crate changes after `167c6e3`).

## Boundary cutover status

Done:
- Demo → Rust `sorcery-engine demo` (`pnpm game:demo` is the cargo binary).
- Batch → Rust `batch-json`.
- Play prototype → Rust `session-json`.
- Parity fixture regeneration → Rust session helpers.
- Interactive UI observation → Rust `Game::public_view`.
- Demo agent selection is fail-closed: `selectDeterministicGameAction(session, issuedActions)` requires engine-issued actions (no TS `legalGameActions` fallback).
- First setup proofs on Rust: RULE-01 setup / mulligan / first-player domain, plus TEST-03 observation parity, via `withSetup`.

Still present — `src/engine/game.ts` (~525KB):
- Still exports `createGameSession` / `legalGameActions` / `stepGame` because most of `game-setup.test.ts` and several other callers still use them.
- Keep types / `hashGameState` / `createGameManifest` / `observeGame` as the thin TS boundary over exported Rust state.

Remaining deletion surface (TS legality callers):
- `tests/engine/game-setup.test.ts` — majority still on TS helpers (`action` / `accept` / `keep` / `createGameSession`); only the first RULE-01/TEST-03 chunk uses `SetupCtx`.
- `tests/engine/game-novelty-rollout.test.ts` + `src/simulator/novelty-rollout.ts`
- Sync `resumeGameCheckpoint` in `src/engine/checkpoint.ts` (async Rust path already exists)
- Private: `run-private-game-check.ts`, `run-private-novelty-gauntlet.ts`
- `benchmarks/typescript-engine.ts` (intentional TS baseline until deletion)

## Next exact step

1. Continue migrating `game-setup.test.ts` to `SetupCtx` in small reviewed chunks (next: remaining RULE-01 helpers / RULE-02 site expansion). One rule family per commit; `pnpm verify` green each time. Do **not** re-run the archived one-shot rewrite scripts under `.local/archive/`.
2. After public tests no longer call TS legality, gut `createGameSession` / `legalGameActions` / `stepGame` bodies in `game.ts` (keep types + hash/manifest/observe).
3. Keep byte-identical Rust regeneration fixtures; do not reintroduce TypeScript as a second legality engine.
4. Phase 4+ product surfaces stay out of scope until that deletion is complete.

## Do not

- Commit anything under `.local/` (whole directory is ignored; `.local/authority/` holds private authority bytes).
- Run more than one agent against this working tree at a time. Parallel migration scripts corrupted `game-setup.test.ts` on 2026-09-02.
- Push unless explicitly asked.
