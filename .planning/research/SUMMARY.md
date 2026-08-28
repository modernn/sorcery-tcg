# Architecture and Stack Decisions

This is the retained project-level research summary. Revisit versions and unresolved questions only when the phase that needs them begins.

## Current stack

- Node.js 24.19.0 and pnpm 11.22.0
- TypeScript 6.0.3 with native Node type stripping
- `node:test` and `node:assert/strict`
- `node:crypto`, `node:fs`, and JSON/JSONL for identity and storage
- Zod 4.4.3 at untrusted runtime boundaries
- ESLint 10.8.1 with typescript-eslint 8.67.0
- React/Vite are deferred until the browser phase and must be re-pinned then

The package manifest and lockfile are authoritative for installed versions.

## Architecture boundaries

1. The engine is a pure authoritative state machine.
2. Clients receive seat-scoped observations and engine-issued legal actions.
3. Every random decision uses injected, versioned, serializable PRNG state.
4. Accepted actions emit semantic events and canonical replay receipts.
5. Rules and card effects use typed functions and engine-owned primitives.
6. Unsupported exercised behavior fails closed and makes a result ineligible for ranking.
7. Runs bind exact authority, deck, behavior, agent, and seed inputs by hash.
8. Simulation workers are deferred until profiling and must return results in deterministic job order.
9. Model providers remain adapters outside the rules core.
10. The browser consumes the same observation/action protocol and never implements rules.

## Deliberate non-choices

No custom rules DSL, database server, Redis, Prisma, Next.js, Socket.IO, Redux, ML/GPU framework, provider SDK in core, worker-per-game design, or container requirement without a demonstrated need. If a container becomes useful, use Podman-compatible OCI tooling.

## Testing contract

Use `pnpm verify` as the ordinary repository gate. Add the smallest test that would fail for each non-trivial behavior. Determinism work must compare canonical bytes across fresh runs; hidden-information work must vary private state and prove observations/actions remain unchanged.

## Reuse boundary

- Official rules and card data are factual authority inputs with pinned provenance, not live runtime dependencies.
- Contested Realms and spells.bar may inform behavior and UX, but source/data/assets are not copied without accepting their licenses and obligations.
- Private official inputs and derivatives stay under the ignored local boundary described in [the external reuse policy](../../docs/external-reuse-policy.md).

## Phase-specific research triggers

- Phase 3–4: official timing, spatial, combat, Storyline, keyword, and interaction rulings.
- Phase 6: attributable deck sources, terms, benchmark strata, and evidence weights.
- Phase 7: current provider identifiers, schemas, retry semantics, budgets, and costs.
- Phase 8: candidate budget, blocked evaluation design, and holdout retirement.
- Phase 9: current React/Vite versions and accessibility/usability validation.

Do not research these early; versions, providers, and product needs can change before implementation.
