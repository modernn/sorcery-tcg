# Cursor Handoff — Rust engine cutover (2026-09-04)

Canonical handoff. Supersedes `OVERNIGHT-HANDOFF.md` (its uncommitted working-copy edits are stale; discard them, see "Working trees").

## Goal

One engine only (AGENTS.md). Rust (`crates/sorcery-engine`, via `target/release/session-json.exe`) is authoritative. The TypeScript legality engine in `src/engine/game.ts` (`createGameSession`, `legalGameActions`, `stepGame`, `verifyGameReplay`) is being deleted. Every test and command must run on Rust through `SetupCtx`/`withSetup` (`tests/engine/rust-setup-session.ts`) or `withRustSession` (`src/engine/rust-session-helpers.ts`).

## Working trees

| Path | Branch / HEAD | What to do |
|---|---|---|
| `C:\src\sorcery-tcg` | `cursor/phase3-drown-bury-artifacts-36d3` @ `72485b1` + 2 dirty files | **Work here. Start here.** |
| `C:\Users\Dad\.codex\worktrees\6fd5\sorcery-tcg` | detached @ `762bee9` (= `backlog/collection-photo-scanning` tip, unrelated, clean) | **Avoid.** Stale Codex worktree. Remove: `git worktree remove C:/Users/Dad/.codex/worktrees/6fd5/sorcery-tcg` then `git worktree prune`. |
| `.local/` (gitignored) | `.local/authority/` = private authority bytes; `.local/archive/` = archived scratch | **Never commit or quote `.local/authority/`.** Leave `.local/archive/` alone. |

Stashes, both safe to drop once you confirm nothing needed:
- `stash@{0}` "tmp2": 2-line delete in game-setup-07. Superseded. `git stash drop stash@{0}`.
- `stash@{1}` "wip-parallel": old whole-file rewrite of game-setup-04/06 from an earlier agent. Superseded by committed migrations. Drop.

Git position: branch is 87 commits ahead of `master` (`25424a8`) **and** 87 ahead of `origin/cursor/phase3-drown-bury-artifacts-36d3`. Nothing pushed since the lineout. Push at the end of every milestone below.

## Dirty state at handoff (2 files)

1. `crates/sorcery-engine/src/game.rs` (+37/-5, **not gated, not committed**). Fix for the 8th parity gap: `release_carried_artifacts` dropped a dying oversized bearer's Artifact at the bearer's anchor instead of its `bearer_cell` (documented in `72485b1` and in the `TODO(rust-cutover)` comment at `tests/engine/game-setup-04.test.ts:377`). Change: compute `cell = bearer_cell.unwrap_or(fell_at.cell)` per Artifact, place `Loose` there, emit that cell in the `artifact-dropped` payload. Inline proof `carried_artifacts_should_ride_one_named_cell_of_an_oversized_bearer` (bottom `mod tests` of game.rs) extended with a `release_carried_artifacts` section asserting Loose at C2 and payload cell C2. `cargo test --lib carried_artifacts_should_ride` passes. Full gates NOT yet run.
2. `OVERNIGHT-HANDOFF.md`: stale agent edits. `git checkout -- OVERNIGHT-HANDOFF.md`, then delete it or replace it with a pointer to this doc.

## Step-by-step

### 1. Land the release-path fix
```
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features --locked -- -D warnings
cargo test --workspace --all-features --locked
git commit --only crates/sorcery-engine/src/game.rs -m "fix(engine): release carried Artifacts on the cell they rode"
cargo build --release --locked -p sorcery-engine     # retry on Windows exe lock
```
The TS boundary (`RustSessionClient` in `src/engine/rust-engine.ts`) prefers the prebuilt exe. **Every engine change needs the rebuild or TS tests run stale code.**

### 2. Finish `tests/engine/game-setup-04.test.ts`
One test left on the legacy engine: `RULE-03 oversized minions occupy one canonical 2x2 footprint...` (line ~396). Unblocked by step 1. Migrate to `withSetup` (pattern: any neighbouring test in 04, or 08 which is fully done). Then remove the imports `createGameSession`, `legalGameActions`, `stepGame`, `verifyGameReplay` and the sync fixtures (`action`, `accept`, `keep`, `northSecondMain`, `northAttacksAtC2`, ...). Target: `grep -nE 'createGameSession|legalGameActions|stepGame|verifyGameReplay|TODO\(rust-cutover\)' tests/engine/game-setup-04.test.ts` returns nothing. Never weaken an assertion. Gate: `node --test tests/engine/game-setup-04.test.ts`, `pnpm typecheck`, `pnpm lint`. Commit.

### 3. Delete legacy sync fixtures from `tests/engine/game-setup-helpers.ts`
`action`, `accept`, `keep`, `northSecondMain`, `northAttacksAtC2` and anything else importing from the TS engine (line ~338 onward). Keep the Rust twins (`takeAction`, `toNorthSecondMain`, `withNorthSecondMain`, `withNumericGenesisSession`, `withNorthAttacksAtC2`, `withNorthAvatarAttacksSouthAtC2`, `withDevilsEggFixture`, `devilsEggManifest`, `peekOpening`). `pnpm typecheck` finds any straggler. Commit.

### 4. Gut the TS engine in `src/engine/game.ts`
Delete bodies and exports of `createGameSession` (~3550), `legalGameActions` (~5755), `stepGame` (~13389), `verifyGameReplay` (~13524) and everything only they reach. **Keep** the types, `createGameManifest` (~2933), `hashGameState` (~3578), `observeGame` (~3734). Expect a large dead-code cascade; delete, do not stub. Then:
```
pnpm verify                                  # typecheck + lint + test
pnpm game:check-private && pnpm game:verify-private
cargo fmt --all -- --check && cargo clippy --workspace --all-targets --all-features --locked -- -D warnings && cargo test --workspace --all-features --locked
```
Commit in slices if the cascade is big (each slice green).

### 5. Docs, master, push
- Replace `OVERNIGHT-HANDOFF.md` with a short "done" note or delete it. Record the one known open Rust gap: **site Genesis after Rubble replacement** is fail-closed in Rust (rejected, not mis-resolved). Not blocking.
- `git push origin cursor/phase3-drown-bury-artifacts-36d3`
- Fast-forward master: `git checkout master && git merge --ff-only cursor/phase3-drown-bury-artifacts-36d3 && git push origin master && git checkout cursor/phase3-drown-bury-artifacts-36d3`.

## Conventions
- Commit footer, exact:
  ```
  Co-Authored-By: Claude Fable 5.1 <noreply@anthropic.com>
  Claude-Session: https://claude.ai/code/session_01GVWAeopmFpfrZH3sGaqv1V
  ```
  (Cursor may substitute its own attribution; keep one verified commit per change.)
- Rust: edition 2024, clippy pedantic `-D warnings`. Long proofs use `#[expect(clippy::too_many_lines, reason = "...")]`. New standalone proofs go in `crates/sorcery-engine/tests/<family>_cutover_rules.rs`; direct `Position` construction goes in the inline `mod tests` at the bottom of game.rs (helpers: `selfplay_manifest_with`, `card_instance`, `test_minion`, `raise_dead_fixture`, `footprint_teleport_game`).
- Synthetic-state TS tests: replace with a Rust proof plus a pointer comment (precedent: end of `RULE-02 a player with no sites recovers at the closest available cell` in game-setup-01). Mark anything genuinely blocked `// TODO(rust-cutover): <repro>`.
- Official rules are authoritative. When Rust and TS disagree, check the rules; fix Rust, never hack the test.

## Hazards seen this session (watch for these)

1. **Stale exe.** TS tests silently run the old engine if `target/release/session-json.exe` predates the last engine commit. Rebuild after every `game.rs` change. Windows locks the exe while a test process holds it: kill stray `session-json.exe` / `node` or retry after a few seconds.
2. **Concurrent agents share one index.** Two `git add` races misattributed hunks in earlier commits. If you run parallel agents: `git commit --only <path>`, never `git add -A`, `git add .`, `git commit -a`, `git stash`, `git checkout <file>`, `git reset` in the shared tree. Retry on `index.lock`.
3. **`git stash push -u` swept a running agent's work** once. Do not stash in a tree with live agents.
4. **Replacing an inline Rust test deleted another agent's test** in the same `mod tests` block. Append, do not rewrite blocks.
5. **Stale Rust proofs contradict ported rules.** After a rules fix, a green test elsewhere may go red because it asserted the old (wrong) behaviour (happened for Stealth in `combat_rules.rs`, `damage_projectile_rules.rs`). Update the assertion with a comment; do not revert the rule.
6. **Line endings.** Repo is LF; PowerShell/Windows tooling emits CRLF (`warning: LF will be replaced by CRLF`). Write files with LF. Codex once mangled `…` to `?` in a diff.
7. **Clippy pedantic bites on tests too**: `let_and_return`, unused imports, `too_many_lines`. Run clippy with `--all-targets`.
8. **Fixture gotchas in inline proofs**: card rules are `Arc`, not mutable (use two cards); `Seat` is not `Ord` (sort by instance id); `SQUARE_AREAS` index off-by-one when hand-picking a footprint; `Cell::parse` fixture cards need at least one effect.
9. **Seed searches**: use `peekOpening(manifest)` (shared single Rust process, ~30 ms/call), never a fresh session per seed.
10. **Rate limits / model choice**: this work burns tokens. Batch mechanical migrations into cheaper model runs; keep engine fixes on a stronger model.
11. **Private authority**: `.local/authority/` bytes never go in commits, logs, prompts, or reports. `pnpm game:check-private` reads them locally; that is fine.

## Definition of done
- `grep -rnE '\b(createGameSession|legalGameActions|stepGame|verifyGameReplay)\b' src tests` → zero hits (or type-only leftovers you deliberately kept).
- `pnpm verify`, `pnpm game:check-private`, `pnpm game:verify-private`, and all four cargo gates green.
- Branch pushed; master fast-forwarded and pushed; stale worktree and stashes gone; `OVERNIGHT-HANDOFF.md` retired.

---

## Prompt for Cursor (paste as-is)

```
Work in C:\src\sorcery-tcg on branch cursor/phase3-drown-bury-artifacts-36d3 (HEAD 72485b1 plus an uncommitted fix in crates/sorcery-engine/src/game.rs). Read CURSOR-HANDOFF.md first, then AGENTS.md. Do NOT touch C:\Users\Dad\.codex\worktrees\6fd5\sorcery-tcg (stale detached Codex worktree; remove it with `git worktree remove` + `git worktree prune`). Never commit, print, or quote anything under .local/authority/.

Execute CURSOR-HANDOFF.md steps 1-5 in order:
1. Gate and commit the uncommitted release_carried_artifacts fix (cargo fmt --check, clippy -D warnings, full cargo test), then rebuild target/release/session-json.exe.
2. Migrate the last legacy test in tests/engine/game-setup-04.test.ts (RULE-03 oversized footprint) to withSetup; remove legacy imports; zero references to createGameSession/legalGameActions/stepGame/verifyGameReplay.
3. Delete the legacy sync fixtures from tests/engine/game-setup-helpers.ts.
4. Delete createGameSession/legalGameActions/stepGame/verifyGameReplay and their dead subtree from src/engine/game.ts; keep types, createGameManifest, hashGameState, observeGame. pnpm verify + private checks + cargo gates green.
5. Discard the stale working-copy edits to OVERNIGHT-HANDOFF.md, retire it, drop stash@{0} and stash@{1}, push the branch, fast-forward master and push.

Rules: one verified commit per step (or per green slice), never weaken an assertion, fix Rust to the official rules rather than hacking tests, rebuild the exe after any engine change, no `git add -A` / `git stash` / `git reset` if any other agent is running. Read the Hazards section before starting. Report per step: commit hash, gates run, anything left marked TODO(rust-cutover) with its reproduction.
```
