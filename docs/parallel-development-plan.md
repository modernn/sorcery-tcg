# Parallel development plan

## Objective and baseline

Finish the Rust authority cutover before adding later product features. Rust owns legality, transitions, deterministic agents, simulation, search, checkpoints, and replay. TypeScript remains an ingestion, transport, private scenario assertion, and UI boundary.

Planning baseline: `25424a8` on `master`. Inspection found 161 Rust-supported catalog scenarios, but remaining TypeScript authority callers. Fresh verification in this planning session passed `pnpm verify` (404 tests, typecheck, lint) and locked Rust workspace tests (340 passed, two ignored release gates). These counts are a baseline, not evidence of complete official-game coverage or current private-authority verification.

## Team and isolation

Cursor coordinates implementation: one integration owner and up to three independent workers. Each writer gets a separate Git worktree and `codex/` branch. No two tools or agents write the same checkout. The September 2 shared-checkout collision documented in `OVERNIGHT-HANDOFF.md` is the reason for this boundary.

| Owner | Exclusive files/responsibility | First deliverable |
| --- | --- | --- |
| Integration owner | `src/engine/{game,checkpoint,rust-engine,rust-session-helpers}.ts`, `tests/engine/rust-setup-session.ts`, Rust RPC/module registration and shared engine fixes, browser server and its tests, parity capture scripts, package scripts, planning docs | Verified common adapter and frozen interfaces |
| A: Public scenario migration | `tests/engine/game-setup.test.ts`; dedicated Rust proof files allocated when a scenario cannot be set up through public actions | First small rule-family migration with assertions preserved |
| B: Native simulation migration | `src/simulator/{novelty-rollout,counterfactual,gauntlet}.ts`, `src/commands/{run-game-demo,run-game-batch,run-private-novelty-gauntlet}.ts`, their engine/private novelty tests, `benchmarks/typescript-engine.ts`, native novelty module/tests and existing Rust simulator/policy/gauntlet changes as allocated | Deterministic native novelty proof and thin public wrapper |
| C: Private scenario migration | Entire `src/commands/run-private-game-check.ts` and `tests/private-authority/private-game-scenario.test.ts`; dedicated synthetic adapter test | First migrated scenario family with exact replay |

Worker B owns `tests/engine/{game-demo,game-batch,game-gauntlet,game-counterfactual,game-novelty-rollout}.test.ts` and `tests/private-authority/private-novelty-gauntlet.test.ts`. The integration owner owns checkpoint/client tests and resolves any unassigned file before it is edited. Shared Rust `game.rs`, `session.rs`, `session_json.rs`, `lib.rs`, and CLI dispatch remain with the integration owner. Workers request a small interface or rule fix there instead of editing those files concurrently.

Private inputs remain under ignored `.local/authority/`. Worktrees do not inherit those inputs automatically. Run private verification in the designated local integration checkout with available inputs; never copy them into tracked files, public fixtures, handoffs, or remote jobs. Give every worktree its own build output and local test output. Do not share a release binary between branches.

## Wave 0: Settle shared interfaces

The integration owner does this before implementation workers fork their common starting commit. Other agents may inspect their assigned files read-only meanwhile.

1. Record HEAD, clean/dirty state, existing worktrees, and current baseline checks. Create an integration branch; preserve unrelated work.
2. Reuse `RustSessionClient`, `RustGameSessionHandle`, and `SetupCtx`. Inventory remaining runtime authority imports separately from type-only imports.
3. Make the common helper usable for seat observations, hashes, checkpoints, independent branches, replay, and rejected actions using Rust authority. `SetupCtx.observe()` currently still invokes TypeScript observation logic. Validate the Rust response at the boundary and close spawned sessions on failure.
4. Freeze the methods and result shapes used by all workers. Specify how async helpers replace immutable TS snapshot branches. Keep temporary legacy calls only until their assigned consumers migrate.
5. Agree Worker B's native operation inputs/outputs before changing RPC dispatch. Include explicit classification, limits, failure evidence, and privacy boundary. Preserve `loadPrivateStarterCatalog` and shared demo-manifest exports consumed by other lanes.
6. Prove the adapter with one meaningful synthetic scenario covering independent checkpoint branches, exact replay, and rejection without mutation. Commit the green foundation and give its exact SHA to all workers.

## Wave 1: Three parallel implementation lanes

### A: Public scenarios

- Keep one owner for the roughly 21,000-line, 161-test file. Migrate one rule family per verified commit; avoid another whole-file scripted rewrite. No preparatory test-file split is needed for three independent lanes.
- Suggested batches: setup/privacy/mulligans; sites/resources/summoning; movement/regions; combat/projectiles; Magic; Artifacts/auras/lifecycle/ordered continuations. Follow real helper dependencies when choosing batch order.
- Preserve every behavioral assertion. A Rust IPC response cannot preserve JavaScript array reference identity: replace such implementation-specific assertions with semantic equality and explain the substitution.
- Immutable TS snapshots currently serve as independent branch roots. Use separate sessions or restore the same checkpoint before each alternative; a mutable handle must not turn sibling branches into sequential actions.
- Some tests manufacture state by editing a snapshot. Rebuild those positions with engine-issued actions, or move the precise scenario proof into a dedicated Rust test using existing test-only facilities. Never add arbitrary-state mutation to the public API.
- Keep a compact original-test-to-replacement mapping for any relocated proof. Test count alone is not assertion coverage.

### B: Simulation and novelty

- Move novelty selection, deterministic policy execution, counterfactual rollout loops, and frontier scheduling into Rust, reusing existing simulator, policy, session, checkpoint, and gauntlet code. Replacing TS `stepGame` calls with RPC calls while leaving permanent search in TS does not meet the product constraint.
- Preserve the current novelty semantics: canonical tie breaks; offered/probed/committed signals; width 128; action bounds 0–500; predicted branch evidence; resumable failures; terminal and horizon distinctions. Preserve the private gauntlet's four orientation jobs, 32-branch bound, pruning, and stop-on-failure behavior.
- Keep TS responsible for private input loading, invoking native operations, and writing local outputs. Do not start a process for every probe. Do not add a generalized worker framework.
- Preserve shared exports while other lanes migrate. Coordinate any signature change with the integration owner and all consumers before landing it.
- Retire the executable TS benchmark once its imports are obsolete; retain historical benchmark JSON as historical evidence. Preserve deterministic fixture bytes; investigate changes rather than blindly regenerating expectations.
- Acceptance: repeated synthetic runs yield byte-identical reports and checkpoints; selected branches replay exactly; limits and failures remain explicit; serial/parallel ordered results match where existing parallel execution applies.

### C: Private scenarios

- Give the entire roughly 24,000-line command to one owner. Splitting by element would still collide on common setup/action helpers and summary aggregation.
- Preserve authority loading, card fact binding, scenario assertions, and `loadPrivateStarterCatalog`. Move authoritative action selection, stepping, observation, replay, and resume through the common Rust adapter.
- Convert one scenario family at a time. Preserve independent snapshot branches, exact causal events, expected rejected actions, and replay checks.
- Add a small public synthetic proof for the migrated helper path. Run existing real-card scenarios only with required ignored local inputs. Missing private inputs mean private verification is pending, not passed.

## Wave 2: Integration and deletion

The integration owner merges verified worker commits one at a time, resolving shared changes deliberately and rerunning the appropriate gates on the resulting integration commit. Workers stay on their owned files; interface changes return to the integration owner.

After all three lanes have landed:

1. Sweep `src`, `tests`, `scripts`, and `benchmarks` for every legacy authority caller, including `createGameSession`, `legalGameActions`, `stepGame`, `replayGame`, `verifyGameReplay`, synchronous checkpoint reconstruction, TS observation/derived-rule helpers, and deterministic-selector fallbacks. Trace wrappers and aliases, not just names.
2. Delete the superseded TypeScript legality/transition/simulation implementation. Retain only needed DTOs, canonical serialization, and boundary validation; do not preserve a second mechanical admission/legality engine in a renamed file.
3. Add one runnable boundary regression in the existing public checks that fails if retired authority routes return. Keep the existing direct Rust scenario proofs and rejection tests.
4. Verify browser play, redaction, checkpoint save/resume, demo, batch, parity capture, novelty, and private scenarios through Rust.
5. Refresh `OVERNIGHT-HANDOFF.md`, `.planning/PROJECT.md`, `.planning/ROADMAP.md`, Phase 3 progress, and reliability documentation to match evidence. Migration completion does not imply full rules or ranked readiness.

## Verification and commits

Each change gets focused checks and one coherent verified commit. Run `pnpm verify` after changes. When Rust changes, use locked dependencies and run:

```powershell
cargo fmt --all -- --check
cargo check --workspace --all-targets --all-features --locked
cargo clippy --workspace --all-targets --all-features --locked -- -D warnings
cargo test --workspace --all-features --locked
cargo build --release --workspace --bins --locked
pnpm verify
```

The release build must precede JS tests that launch native code: `sessionJsonLaunch()` prefers an existing release executable and does not check its freshness. Record the checkout SHA and target directory with test results. Limit simultaneous full builds to host capacity; independent coding does not require competing full-suite runs.

At final integration, run `pnpm game:verify-private` and `pnpm game:novelty-private` with the required local inputs. Run `pnpm authority:verify-private` only for authority-release work with its required inputs. Self-play changes require the fast self-play tests and release acceptance gate; the clean-reboot soak is a separate gate and must not be described as completed without that environment.

Completion requires zero remaining TS authority/simulation implementations, all 161 catalog scenario proofs intact, preserved migrated test assertions, green public/Rust gates on the merged SHA, and explicit private verification status. Keep results unranked until every applicable coverage and authority gate passes.

## Proposed Codex overnight assignment

This is a prepared assignment, not a scheduled or started automation. Cursor owns implementation; Codex independently verifies a pinned commit in a separate worktree. This avoids concurrent edits and gives useful evidence while the migration proceeds.

Paste-ready assignment:

> Work overnight on an independent verification audit of the Sorcery Simulator Rust cutover. Resolve and record the current integration commit, then inspect and test that fixed commit in a separate `codex/overnight-cutover-audit` worktree. If Cursor has supplied a verified integration SHA, use it; otherwise use the current committed baseline and state that limitation. Do not edit implementation, fixtures, shared planning files, or Cursor's checkout. Check all 161 catalog entries against actual Rust proof locations; trace remaining TS authority and simulation callers; assess branch isolation, hidden-state redaction, replay determinism, stale-action rejection, unsupported-mechanic handling, and release-binary freshness. Run the locked Rust gates, explicitly rebuild release binaries, and run `pnpm verify`; report any failure with a reproducible command and file reference. Use synthetic data and preserve the ignored private authority boundary. Do not run private release checks without their required local inputs. Do not infer correctness from catalog labels or test counts alone. Write one report at `docs/reports/overnight-cutover-audit.md` with the audited SHA, commands and results, prioritized findings, and concrete follow-up tasks for Cursor. Commit only that report on your audit branch. If time remains, investigate the highest-impact finding read-only; do not repeat unchanged tests, invent new features, or schedule recurring runs. Stop when the bounded audit is complete and report what remains unverified.

After cutover, choose the next milestone from measured mechanic/owned-deck coverage gaps. Collection/deck optimization, model competitors, and broader UI work should get their own plans after that evidence is available.
