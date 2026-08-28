# Sorcery Simulator

## Product constraints

- Build the engine, simulator, agents, and later browser client in TypeScript.
- Official rules and card rulings are authoritative. Never tune game balance by changing a real rule.
- The engine owns state and enumerates legal actions; clients and models may not submit arbitrary mutations.
- A run manifest plus seed must reproduce byte-identical deterministic-agent events.
- Unsupported exercised mechanics invalidate ranked results instead of becoming silent no-ops.

## Privacy and reuse boundary

- Keep official source bytes, locks, normalized snapshots, and built authority revisions under the ignored `.local/authority/` boundary. Never commit, package, share, host, upload, or redistribute them.
- Do not acquire official artwork. Later UI work may use only original/project-owned presentation art or user-supplied private local images.
- Treat external simulator code and data as behavioral reference unless its license and attribution obligations are explicitly accepted.
- Follow [the external reuse policy](docs/external-reuse-policy.md) for the complete boundary and permission trigger.

## Working rules

- Direct repository work is the default. Use a GSD workflow only when the user requests it or the task benefits from phase planning, specialist review, or persistent debugging state.
- Inspect and reuse existing code before adding abstractions or dependencies.
- Prefer MCP servers for authoritative external data and connected services.
- Use Podman, not Docker, when containers are necessary.
- Run `pnpm verify` after changes. Run `pnpm authority:verify-private` only for authority-release work that has the required ignored local inputs.
- Make one coherent verified commit per change. Delegate only independent work that benefits from parallel execution.
