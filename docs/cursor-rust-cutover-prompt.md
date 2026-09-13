# Cursor implementation prompt

Copy everything below into Cursor at `C:\src\sorcery-tcg`.

---

Execute the Rust cutover plan in `docs/parallel-development-plan.md`. Carry the implementation through verification and coherent commits; do not stop after restating a plan. This task is the remaining engine migration, not new product features or a claim that all Sorcery rules are implemented.

## Read first

Read `AGENTS.md`, `docs/parallel-development-plan.md`, `OVERNIGHT-HANDOFF.md`, and `docs/external-reuse-policy.md`. Inspect the actual current code and Git state. The planning baseline is `25424a8`; do not reset to it if later work exists. The last verified baseline had 404 passing public tests and 340 passing Rust tests, with two ignored release gates.

The integration is already partly complete: demo and batch use Rust, browser play uses `RustSessionClient`, and all 161 public catalog scenarios are marked Rust-supported. The full TypeScript rules implementation remains in `src/engine/game.ts`; it still has live consumers. Planning docs are stale and must not override the Rust-first product constraints.

## Operating rules

- Act as the integration owner. Use up to three workers for independent implementation and a separate Git worktree/`codex/` branch for every writer. Never run multiple writing agents in one checkout. Cursor and another tool previously overwrote work here.
- Preserve unrelated changes and other agents' commits. Inspect worktrees before creating them. Only the integration owner integrates commits and changes shared interfaces.
- Reuse existing code, standard library facilities, and installed dependencies. Do not introduce speculative abstractions or a new orchestration framework. Use MCP tools when useful and Podman if containers are required.
- Use small rule-family or functional-slice commits with meaningful messages. Run focused checks, `pnpm verify`, and the required Rust gates where applicable before calling a commit verified.
- Keep official source bytes, snapshots, locks, and built authority revisions under ignored `.local/authority/`. Never commit, upload, package, or redistribute them. Acquire no official artwork. Worktrees do not automatically contain private inputs.
- Do not send external messages, deploy, publish, or create recurring automation as part of this task. Ordinary local implementation, fixes, tests, branches, and commits are authorized. Resolve routine implementation decisions without repeated approval requests.
- A separate Codex audit may inspect a fixed commit and write its own report branch. Do not edit `docs/reports/overnight-cutover-audit.md` or that audit checkout. Communicate implementation status through committed handoff text.

## Start with the shared boundary

Before spawning implementation writers, settle and verify the shared adapter on the integration branch. Workers may perform read-only planning meanwhile.

Reuse `src/engine/rust-engine.ts`, `src/engine/rust-session-helpers.ts`, and `tests/engine/rust-setup-session.ts`. Rust owns observations, state hashes, legal actions, transitions, checkpoints, and replay. `SetupCtx.observe()` currently still derives its result in TS. Route it through the existing Rust observation operation with appropriate boundary validation. Prove rejection without mutation, independent checkpoint branches, and exact replay. Close child sessions on failed setup as well as normal completion.

Freeze helper method signatures and report schemas before branching the workers. Keep only the temporary compatibility needed for not-yet-migrated callers. Allocate shared Rust RPC dispatch and any engine fixes to yourself; workers request narrowly scoped changes rather than modifying shared files.

Explicitly rebuild release binaries before JS tests. `sessionJsonLaunch()` runs an existing release binary without testing whether it matches source. Each worktree must use its own build artifacts; never reuse another branch's release executable as verification evidence.

## Delegate these lanes

### Worker A: public rule tests

Own `tests/engine/game-setup.test.ts` and any explicitly allocated dedicated Rust scenario test files. You are not alone in the repository; do not revert others' work or edit their files. Shared helpers belong to the integration owner.

Migrate the 161 tests in small rule-family batches using the agreed Rust adapter. Keep one owner of this large file; do not repeat the failed whole-file scripted rewrite or split the file merely to create more tasks.

Preserve all behavioral assertions, including private observations, legal-action filtering, rejection, ordered events, terminal behavior, checkpoint forks, and replay. Old immutable snapshots often represent independent branch roots: restore the same checkpoint or open separate contexts for each alternative. Some tests manufacture positions through direct snapshot edits; recreate those positions using legal actions or move the exact proof to a dedicated Rust scenario test using existing test-only facilities. Do not expose arbitrary-state injection. Replace JS reference-identity assertions with justified semantic equality across IPC. Record a mapping whenever a proof moves; do not delete a test because a related Rust test exists.

Deliver each verified rule-family commit with proof names, any justified assertion substitutions, and remaining work.

### Worker B: native simulator and novelty

Own `src/simulator/{novelty-rollout,counterfactual,gauntlet}.ts`, `src/commands/{run-game-demo,run-game-batch,run-private-novelty-gauntlet}.ts`, their associated public tests, `tests/private-authority/private-novelty-gauntlet.test.ts`, the obsolete executable TS benchmark, and explicitly allocated native simulator/policy/novelty modules and tests. You are not alone in the repository; preserve other workers' edits and request shared-boundary changes from the integration owner.

Reuse the existing Rust `Session`, simulator, policy, checkpoint, and gauntlet implementations. Rust must own deterministic policy execution, rollouts, counterfactual search, novelty selection, and frontier scheduling. A permanent TS search loop making Rust calls per transition does not satisfy this task.

Preserve current observable behavior: canonical ordering and tie breaks, bounded action/width/frontier limits, offered/probed/committed coverage distinctions, replay-verified branch predictions, resumable failure evidence, terminal/horizon classification, and explicit unranked/private output labels. Preserve the existing four private lesson/orientation jobs, 32-branch ceiling, pruning, and stop-on-failure behavior. Keep TS as local input/output and native invocation glue. Avoid per-probe process creation.

Agree schemas with the integration owner before changing entry points. Preserve manifest-builder exports and caller compatibility while other lanes proceed. Coordinate any policy or result-shape change with browser/scenario consumers rather than silently updating expected hashes. Remove obsolete executable TS benchmark code after its dependencies are retired, preserving historical JSON evidence.

Deliver deterministic synthetic proofs first, then command cutover and private gauntlet verification when local inputs are available.

### Worker C: private scenario command

Own all of `src/commands/run-private-game-check.ts`, `tests/private-authority/private-game-scenario.test.ts`, and an allocated synthetic smoke test. You are not alone in the repository; preserve other workers' changes and request shared helper fixes from the integration owner.

Keep the command's authority loading, card fact binding, assertions, and report composition. Preserve `loadPrivateStarterCatalog` because the browser, workload reporting, and novelty command consume it. Migrate authoritative legal-action, transition, observation, replay, and checkpoint operations to the agreed Rust adapter, one scenario family per commit. Do not split ownership by element: those paths share synchronous setup/action helpers and report aggregation.

Preserve independent branch roots, source-linked event assertions, rejected-action checks, and byte-exact replay. Add a small public synthetic proof for the migrated helper path. Exercise real-card scenarios only with required ignored local inputs. Report missing private verification as pending; do not skip it and claim the private cutover is verified.

## Integration owner responsibilities

You own shared `src/engine` boundary files, `tests/engine/rust-setup-session.ts`, checkpoint/client tests, shared Rust engine/session/RPC/module registration/CLI dispatch, browser server/tests, parity capture scripts, package scripts, and planning docs. Assign any unlisted file before a worker edits it.

Merge verified worker commits sequentially. Revalidate the resulting integration commit and communicate any interface change to all workers. Resolve root-cause engine failures in shared helpers with one direct scenario proof; never patch a rule by card name or relax a real rule to make tests pass.

When all migration lanes have landed, inspect every caller in `src`, `tests`, `scripts`, and `benchmarks`. Remove TS implementations of initialization/legality/stepping/replay, synchronous checkpoint reconstruction, derived rules/observation, simulator/search, and deterministic-agent logic. Trace wrappers, aliases, and selector fallbacks. Type-only imports and canonical serialization are not a second rules engine, but mechanical admission and rule enforcement must also have one authority. Do not retain the old engine under a new filename.

Preserve all 161 catalog proofs and the behavioral coverage of migrated tests. Add one small public boundary regression preventing retired TS authority paths from returning. Keep parity fixtures byte-identical; investigate any drift and document the justified fix rather than regenerating away failures.

Update the handoff, project/roadmap status, Phase 3 progress, and reliability docs to reflect actual verified state. Do not mark full official-game support or ranked readiness just because the migration is complete.

## Gates

For every change, run focused checks and `pnpm verify`. For Rust changes, run:

```powershell
cargo fmt --all -- --check
cargo check --workspace --all-targets --all-features --locked
cargo clippy --workspace --all-targets --all-features --locked -- -D warnings
cargo test --workspace --all-features --locked
cargo build --release --workspace --bins --locked
pnpm verify
```

Run full integration gates on the exact merged SHA using freshly built native binaries. Verify demo/batch, browser play and redaction, rejected/stale actions, checkpoint restore across process restart, replay, parity regeneration, and novelty boundaries. Use existing tests and add only missing meaningful proofs.

Run `pnpm game:verify-private` and `pnpm game:novelty-private` with required local inputs at private cutover acceptance. Use `pnpm authority:verify-private` only for authority-release work with its required inputs. Self-play changes also require the fast self-play suite and `pnpm game:selfplay-acceptance`. The clean-reboot `pnpm game:selfplay-soak` gate remains pending unless actually run in that environment. Bound expensive runs and avoid concurrent full builds that overload the host.

## Completion and handoff

Complete when the duplicate TS authority and permanent simulation/search logic are removed, callers use Rust, all catalog proofs and migrated behavioral assertions remain covered, and public/Rust verification passes on the integration SHA. Record private/soak checks separately with their real status; keep results unranked until all applicable authority and coverage gates pass.

At each handoff record: branch and commit, completed slices, remaining legacy callers, tests run and their exact outcomes, pending private/soak gates, worker ownership, and the next concrete step. If a lane is blocked, continue independent authorized work and report the precise dependency. Do not claim completion from test counts alone or leave a broken migration halfway through a commit.
