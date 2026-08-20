<!-- GSD:project-start source:PROJECT.md -->
## Project

**Sorcery Simulator**

Sorcery Simulator is a new TypeScript application for playing and simulating Sorcery: Contested Realm with an authoritative, deterministic rules engine. It lets automated and AI competitors play legal games, evaluates owned and commonly played online decks, supports deck optimization, and later provides a browser GUI for human play over the exact same engine.

**Core Value:** Simulation results must be reproducible and rules-correct enough that deck and model comparisons are trustworthy.

### Constraints

- **Language**: TypeScript for the engine, simulator, agents, and GUI — user preference and potential reuse of existing TypeScript implementations.
- **Rules fidelity**: Official rules and card rulings are authoritative — simulator balance must never be tuned by changing a real rule.
- **Determinism**: A run manifest plus seed must reproduce byte-identical engine events for deterministic agents.
- **Action safety**: The engine enumerates legal actions and is the sole state owner — clients and models cannot submit arbitrary mutations.
- **Coverage honesty**: Ranked results require complete coverage of every exercised mechanic — unsupported behavior invalidates the game instead of becoming a silent no-op.
- **Licensing**: External source may be copied only when its license is compatible and attribution/derivative obligations are accepted; otherwise it is behavioral reference only.
- **Containers**: Use Podman rather than Docker when containers become necessary.
- **Delivery**: Commit small verified slices with tests throughout development.
<!-- GSD:project-end -->

<!-- GSD:stack-start source:research/STACK.md -->
## Technology Stack

## Recommendation
## Recommended Stack
### Core runtime and package tooling
| Technology | Exact version | Purpose | Why |
|---|---:|---|---|
| Node.js | 24.19.0 LTS | Runtime, scripts, tests, file I/O, hashing, later worker pool | Node 24 is maintained LTS through April 2028. Native TypeScript type stripping is stable in Node 24.12+, so this project does not need a runtime transpiler. **Confidence: HIGH.** |
| TypeScript | 6.0.3 | Static types and type checking | Use the current stable compiler with a complete programmatic API. TypeScript 7.0.2 exists, but its official release notes say the programmatic API is not yet available and recommend a TS 6 compatibility package for tools that require it. Avoid that dual-toolchain complexity. **Confidence: HIGH.** |
| pnpm | 11.22.0 | Package manager and lockfile | Fast, strict dependency layout and an explicit `packageManager` pin. Workspaces remain available if the later GUI creates a real package boundary. **Confidence: HIGH.** |
| `node:test` + `node:assert/strict` | bundled with Node 24 | Unit, property-style loop, integration, replay, and determinism tests | Node runs `.test.ts` files through native type stripping and isolates test files in child processes. This is sufficient for a headless engine and removes a test-runner dependency. **Confidence: HIGH.** |
| `node:crypto` | bundled with Node 24 | Snapshot hashes, seed derivation, run identity | SHA-256 can bind every result to exact card/rules/agent inputs. **Confidence: HIGH.** |
| `node:fs` / `node:readline` | bundled with Node 24 | Versioned snapshots and append-only JSONL logs | Simple, durable, inspectable output is preferable to introducing a database before query needs are known. **Confidence: HIGH.** |
| `node:worker_threads` | bundled with Node 24 | Parallel match execution after correctness is established | Official Node guidance identifies workers as appropriate for CPU-intensive JavaScript and recommends pooling rather than spawning one worker per task. **Confidence: HIGH.** |
### Production dependencies
| Library | Exact version | Purpose | When to use |
|---|---:|---|---|
| `pure-rand` | 8.4.2 | Versioned, injectable pseudo-random number generators and unbiased integer distributions | All shuffle, random choice, tie-break, and procedural test generation. Pin the package and algorithm name in run manifests. **Confidence: HIGH.** |
| `zod` | 4.4.3 | Runtime validation at untrusted boundaries | Parse pinned card snapshots, deck files, run manifests, saved games, and model-provider responses. Do not repeatedly validate already-typed internal state in hot simulation loops. **Confidence: HIGH.** |
### Development dependencies
| Library | Exact version | Purpose | Why |
|---|---:|---|---|
| `@types/node` | 24.1.0 | Node API types | Keep the declaration major aligned with the Node 24 runtime. **Confidence: HIGH.** |
| ESLint | 10.8.1 | Static linting | Catch unsafe or inconsistent code beyond the type checker. **Confidence: HIGH.** |
| `typescript-eslint` | 8.67.0 | TypeScript-aware ESLint support | Official TypeScript lint integration; use a small flat config rather than a large shared preset. **Confidence: HIGH.** |
### Later browser GUI — do not install in the engine phase
| Technology | Verified version | Purpose | Why / timing |
|---|---:|---|---|
| React / React DOM | 19.2.8 | Human play and analysis UI | Add only after the authoritative engine/action protocol is stable. **Confidence: HIGH for current version, MEDIUM for eventual selected version.** |
| Vite | 8.2.1 | Browser dev server and production build | A static client needs neither SSR nor a full-stack framework. **Confidence: HIGH for current version, MEDIUM for eventual selected version.** |
| `@vitejs/plugin-react` | 6.0.5 | React integration for Vite | Standard minimal Vite integration. **Confidence: HIGH for current version, MEDIUM for eventual selected version.** |
| `@types/react` / `@types/react-dom` | 19.2.18 / 19.2.4 | React TypeScript declarations | Install with the GUI and reverify versions then. **Confidence: HIGH for current versions.** |
## Suggested Project Shape
### Native TypeScript constraints
## Determinism Contract
## Test Strategy
- state transition tests for every implemented rule and card effect;
- illegal-action rejection and unsupported-interaction fail-closed tests;
- legal-action enumeration checks proving every accepted action was enumerated;
- same-manifest-twice byte comparison of events and final state;
- replay tests that rebuild the final state solely from the initial manifest and event/action stream;
- randomized invariant loops over many fixed seeds, printing the failing seed for exact reproduction;
- seat-alternated paired matches for deck/agent comparisons;
- adapter contract tests showing scripted, heuristic, search, model, and later human clients all select from the same action schema.
## Parallel Simulation Strategy
- create one reusable `worker_threads` pool, not a worker per game;
- default pool size to `Math.max(1, os.availableParallelism() - 1)` and allow an explicit override;
- send immutable, self-contained jobs containing game ordinal, seed, seat assignment, deck IDs, snapshot hashes, and agent configuration;
- keep engine state worker-local and avoid shared mutable state or `SharedArrayBuffer`;
- return structured results keyed by game ordinal, then sort by ordinal before aggregation and output;
- make crash/retry preserve the exact original job and seed;
- keep model/API calls out of CPU workers; they are I/O-bound and need separate rate/concurrency limits.
## Storage and Model Integration
## Reference Implementation Audit
### `realms-cards/contested-realms`
- separate candidate generation from evaluation/search;
- phase-aware policies and explicit evaluation-score breakdowns;
- machine-readable decision telemetry;
- state transition and replay-oriented tests;
- mapping rules to focused modules and coverage status.
- arbitrary client state patches or client-authoritative combat;
- fail-open validation/cost behavior;
- inferred intent from state diffs instead of typed actions;
- card-text regex as the primary executable rules model;
- wall-clock search limits for reproducible ranked comparisons;
- optimistic local repair of authoritative state.
### `JollyGrin/sorcery-tcg-playtest` / spells.bar
- clear spatial presentation of sites, aura intersections, stacked cards, hand, deck, atlas, and cemetery;
- direct deck import/share and reconnect flows;
- touch-friendly manipulation and visible opponent state.
- full-state client synchronization as the game protocol;
- index/array position as durable card identity;
- shallow object mutation and global random shuffle;
- drag gestures as the only action input;
- a generic relay server as a substitute for an authoritative engine.
## Reuse Boundary
| Source | May study/recreate as independent concepts | Should not copy into this project |
|---|---|---|
| `contested-realms` | Candidate/evaluator separation, evaluation telemetry, phase policies, rule coverage documentation, replay-focused workflow | Source code or close translations without consciously accepting GPL-3.0; patch-based authority; fail-open validators |
| spells.bar | Board layout and zone affordances, import/share/reconnect user flows, touch interaction lessons | Source code without permission; full-state relay protocol; manual-only rules model |
| Official rules/card data | Encode factual game behavior with provenance, pinned source/version, and coverage tests | Untracked live fetches or copyrighted artwork/assets without a confirmed right to redistribute |
## Alternatives Considered
| Category | Recommended | Alternative | Why not now |
|---|---|---|---|
| Runtime | Node 24 LTS native TS stripping | `tsx`, `ts-node`, Bun, Deno | Another execution layer or runtime is unnecessary for erasable TypeScript. Node has the required test, crypto, file, and worker APIs. |
| Compiler | TypeScript 6.0.3 | TypeScript 7.0.2 | TS 7 currently lacks the programmatic API, creating compatibility/tooling complexity with little engine benefit. Reassess at 7.1+. |
| Test runner | `node:test` | Vitest/Jest | Headless pure logic does not need DOM mocking or runner plugins. Add Vitest only with demonstrated GUI-test value. |
| Randomness | `pure-rand` | `Math.random`, custom Mulberry32, `seedrandom` | Global randomness cannot be injected/replayed; custom PRNG/distributions add avoidable algorithm and bias risk. |
| Results store | JSON/JSONL + hashes | PostgreSQL/Prisma, Redis, SQLite immediately | These add operational/schema complexity before the query workload exists. Node SQLite is not yet stable in the selected runtime line. |
| Concurrency | Built-in worker pool after profiling | Piscina or one worker per match | The initial pool needs only a small fixed protocol; per-match worker creation wastes startup cost. |
| Browser app | React + Vite later | Next.js | No SSR, routing platform, or server-rendered public product is required for a local simulator GUI. |
| Repository | One package initially | Nx/Turborepo monorepo | Premature orchestration adds configuration without an actual multi-package release boundary. |
| Model access | `fetch` adapters | Provider SDK in core | Keeps engine/provider boundaries strict and dependencies small; add SDKs only for concrete missing capabilities. |
## Explicit Non-Choices
- No human-play CLI. Commands are for tests, batches, replay, and automation; human play belongs in the later GUI.
- No Next.js, Socket.IO, database server, Redis, Prisma, Three.js, Redux, or Zustand in the rules core.
- No LLM referee, free-form model mutation, or model-generated rules. All clients choose from engine-enumerated legal actions.
- No custom rules DSL until repeated implementations prove ordinary typed functions and explicit effect registries are insufficient.
- No ML/GPU framework for the first agents; random, scripted, heuristic, and bounded search baselines establish the evaluation harness first.
- No container requirement for local execution. If a reproducible service/deployment image later becomes necessary, use Podman-compatible OCI files and commands, per project instruction.
## Installation
## Confidence Assessment
| Area | Confidence | Basis / remaining uncertainty |
|---|---|---|
| Node/TypeScript/package tooling | HIGH | Official runtime/compiler docs, official release pages, and npm package metadata checked on the research date. |
| Deterministic PRNG approach | HIGH | Official package repository/npm metadata plus Node crypto APIs; canonical serialization details remain a project design decision. |
| Test and worker strategy | HIGH | Official Node documentation supports native TypeScript tests, process isolation, CPU workers, pooling, and `availableParallelism`. |
| Minimal persistence/model adapters | HIGH | Meets stated local-first requirements with built-in APIs; database need should be evidence-driven. |
| `contested-realms` audit | HIGH | Current pinned source, repository docs, bot code, tests, and GPL license inspected. |
| spells.bar audit | HIGH | Current pinned client source and linked Go relay inspected; absence of a license is verifiable at the audited revision but could change later. |
| Future GUI exact versions | MEDIUM | Versions are current and official now, but the GUI phase is later and must re-pin against its actual start date. |
| Legal reuse implications | MEDIUM | Licenses/absence are factual; consequences depend on distribution and jurisdiction, so obtain legal advice before material code reuse. |
## Sources
### Official runtime and package sources
- [Node.js releases and LTS schedule](https://nodejs.org/en/about/previous-releases)
- [Node.js TypeScript execution](https://nodejs.org/api/typescript.html)
- [Node.js test runner](https://nodejs.org/download/release/latest-v24.x/docs/api/test.html)
- [Node.js worker threads](https://nodejs.org/api/worker_threads.html)
- [Node.js `os.availableParallelism`](https://nodejs.org/api/os.html#osavailableparallelism)
- [Node.js SQLite stability](https://nodejs.org/download/release/latest-v24.x/docs/api/sqlite.html)
- [TypeScript 6.0.3 release](https://github.com/microsoft/TypeScript/releases/tag/v6.0.3)
- [TypeScript 7.0 release and programmatic API status](https://devblogs.microsoft.com/typescript/announcing-typescript-7-0/)
- [`pnpm` package](https://www.npmjs.com/package/pnpm)
- [`pure-rand` repository](https://github.com/dubzzz/pure-rand) and [`pure-rand` package](https://www.npmjs.com/package/pure-rand)
- [`zod` package](https://www.npmjs.com/package/zod)
- [React package](https://www.npmjs.com/package/react), [React DOM package](https://www.npmjs.com/package/react-dom), and [Vite package](https://www.npmjs.com/package/vite)
### Reference repositories
- [`realms-cards/contested-realms`](https://github.com/realms-cards/contested-realms), its [package manifest](https://github.com/realms-cards/contested-realms/blob/main/package.json), [GPL-3.0 license](https://github.com/realms-cards/contested-realms/blob/main/LICENSE), and [rules-engine analysis](https://github.com/realms-cards/contested-realms/blob/main/docs/rules-engine-analysis.md)
- `contested-realms` [rules validation](https://github.com/realms-cards/contested-realms/blob/main/server/modules/rules-validation.ts), [movement module](https://github.com/realms-cards/contested-realms/blob/main/server/modules/rules-movement.ts), [client combat state](https://github.com/realms-cards/contested-realms/blob/main/src/lib/game/store/combatState.ts), [bot engine](https://github.com/realms-cards/contested-realms/blob/main/bots/engine/index.js), [bot limitations](https://github.com/realms-cards/contested-realms/blob/main/bots/engine/README.md), [headless client](https://github.com/realms-cards/contested-realms/blob/main/bots/headless-bot-client.js), and [bot validation test](https://github.com/realms-cards/contested-realms/blob/main/tests/bot/bot-rules-validation.js)
- [`JollyGrin/sorcery-tcg-playtest`](https://github.com/JollyGrin/sorcery-tcg-playtest), its [package manifest](https://github.com/JollyGrin/sorcery-tcg-playtest/blob/main/package.json), [state representation](https://github.com/JollyGrin/sorcery-tcg-playtest/blob/main/src/types/card.ts), [client action utilities](https://github.com/JollyGrin/sorcery-tcg-playtest/tree/main/src/utils/actions), [WebGameProvider](https://github.com/JollyGrin/sorcery-tcg-playtest/blob/main/src/lib/contexts/WebGameProvider.tsx), and [socket client](https://github.com/JollyGrin/sorcery-tcg-playtest/blob/main/src/lib/gamesocket/socket.ts)
- [`JollyGrin/unbrewed-p2p` Go relay](https://github.com/JollyGrin/unbrewed-p2p/blob/main/gameserver/gameserver/server.go)
<!-- GSD:stack-end -->

<!-- GSD:conventions-start source:CONVENTIONS.md -->
## Conventions

Conventions not yet established. Will populate as patterns emerge during development.
<!-- GSD:conventions-end -->

<!-- GSD:architecture-start source:ARCHITECTURE.md -->
## Architecture

Architecture not yet mapped. Follow existing patterns found in the codebase.
<!-- GSD:architecture-end -->

<!-- GSD:skills-start source:skills/ -->
## Project Skills

No project skills found. Add skills to any of: `.claude/skills/`, `.agents/skills/`, `.cursor/skills/`, `.github/skills/`, or `.codex/skills/` with a `SKILL.md` index file.
<!-- GSD:skills-end -->

<!-- GSD:workflow-start source:GSD defaults -->
## GSD Workflow Enforcement

Before using Edit, Write, or other file-changing tools, start work through a GSD command so planning artifacts and execution context stay in sync.

Use these entry points:
- `/gsd-quick` for small fixes, doc updates, and ad-hoc tasks
- `/gsd-debug` for investigation and bug fixing
- `/gsd-execute-phase` for planned phase work

Do not make direct repo edits outside a GSD workflow unless the user explicitly asks to bypass it.
<!-- GSD:workflow-end -->



<!-- GSD:profile-start -->
## Developer Profile

> Profile not yet configured. Run `/gsd-profile-user` to generate your developer profile.
> This section is managed by `generate-claude-profile` -- do not edit manually.
<!-- GSD:profile-end -->

## Local Working Rules

- Use Podman rather than Docker.
- Prefer available MCP servers for authoritative external data and connected services.
- Use multiple agents for independent work whenever practical.
- Commit small verified changes frequently with descriptive messages.
- Include the smallest meaningful automated test in every implementation plan.
