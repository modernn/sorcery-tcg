# Sorcery Simulator

## Product constraints

- Build the authoritative engine, simulator, deterministic agents, checkpoints, search, and replay in Rust. Keep TypeScript only as a thin boundary for authority ingestion, the server, and browser UI where useful.
- Do not maintain two permanent legality engines. Delete the superseded TypeScript implementation after parity and cutover.
- The official Sorcery Codex is primary for shared rules; retain exact entry/subentry IDs, URLs, and snapshot identity. Complete official card text, characteristics, updates, and card-specific rulings supply each binding. Follow [authority precedence](docs/authority-precedence.md); never tune balance by changing a real rule.
- The engine owns state and enumerates legal actions; clients and models may not submit arbitrary mutations.
- A run manifest plus seed must reproduce byte-identical deterministic-agent events.
- Unsupported exercised mechanics invalidate ranked results instead of becoming silent no-ops.
- Keep rule enforcement readable: bind cards to generic facts, route those facts through shared rule helpers, avoid card-name branches, and leave one direct scenario proof for each supported rule slice.
- Build scenario search from deterministic checkpoint branches and engine-issued legal actions. LLMs may explain results or compete optionally, but may not define legality or be required for rollouts.

## Privacy and reuse boundary

- Keep official source bytes, locks, normalized snapshots, and built authority revisions under the ignored `.local/authority/` boundary. Never commit, package, share, host, upload, or redistribute them.
- Do not acquire official artwork. Later UI work may use only original/project-owned presentation art or user-supplied private local images.
- Treat external simulator code and data as behavioral reference unless its license and attribution obligations are explicitly accepted.
- Follow [the external reuse policy](docs/external-reuse-policy.md) for the complete boundary and permission trigger.

## Rule and card batches

- Follow the [shared-rule batch loop](docs/shared-rule-behaviors.md#binding-and-validation-batch-loop). Review complete source semantics before grouping cards; reuse existing operations first and pair missing rules with their companion dependencies.
- Every implementation packet must name its approved representations, shared helpers, affected callers, timing owner, proof pattern, and compatibility consequences. Independent review checks both source semantics and architecture conformance before integration. New representations, operators, timing boundaries, or material hot-path ownership changes return to design before coding; ordinary approved reuse proceeds directly.
- Keep per-card source review, contextual requirements, shared-rule proofs, validation jobs, and admission distinct in the private catalog. A keyword match, source review, family count, or successful game does not establish complete support.
- After a native rule proof passes, recompute exact contextual dependencies and queue all newly eligible cards for bounded independent validation. Each card needs a passing source-guarded receipt before registry promotion; no arbitrary small-card cap or blanket group unlock.
- Keep catalog/workbook data under `.local/authority/`; SQLite tracks evidence and work, never legality. Refresh it after proof or binding changes. Get current counts from current inputs, not historical plans.
- TypeSafe Jev is available when useful for diagnosis, synthetic counterexamples, and play/probe proposals. Reuse the [existing adapters](docs/agent-evaluation.md) with configured private credentials; no repeated permission request is needed for this authorized use. Send only approved synthetic redacted packets, never private authority or its derivatives. Jev does not define legality and is not required for rollouts.

## Working rules

- Direct repository work is the default. Use a GSD workflow only when the user requests it or the task benefits from phase planning, specialist review, or persistent debugging state.
- Inspect and reuse existing code before adding abstractions or dependencies.
- Prefer MCP servers for authoritative external data and connected services.
- Use Podman, not Docker, when containers are necessary.
- Run `pnpm verify` after changes. Run `pnpm authority:verify-private` only for authority-release work that has the required ignored local inputs.
- For Rust changes, use locked dependencies and run `cargo fmt --all -- --check`, `cargo check --workspace --all-targets --all-features --locked`, `cargo clippy --workspace --all-targets --all-features --locked -- -D warnings`, and `cargo test --workspace --all-features --locked`.
- Make one coherent verified commit per change. Delegate only independent work that benefits from parallel execution.
