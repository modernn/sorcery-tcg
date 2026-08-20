# Technology Stack

**Project:** Sorcery: Contested Realm deterministic simulator and deck laboratory  
**Researched:** 2026-08-20  
**Overall confidence:** HIGH for the core runtime/tooling recommendation; MEDIUM for later GUI versions because they should be reverified when that phase begins

## Recommendation

Build the simulator as a small Node.js TypeScript application before adding a browser application. Keep the rules engine pure and synchronous: given immutable state, a validated semantic action, and an injected deterministic random source, it returns the next state plus ordered domain events. Use Node's built-in test runner, file APIs, hashing, and worker threads rather than adding framework equivalents.

Begin as one `pnpm` package. A monorepo, database, server framework, and UI state library do not solve an immediate project problem. When the human GUI phase starts, add a Vite/React browser application that consumes the same public engine protocol; do not turn the GUI into a second rules implementation.

The two existing tools are useful behavioral references, not engine foundations. `contested-realms` is GPL-3.0 and its mixed client/server implementation is intentionally permissive in important places. `sorcery-tcg-playtest` has no repository license and is a manual virtual tabletop backed by opaque full-state synchronization. Study concepts and user flows, but implement the new engine clean-room unless the project deliberately accepts the corresponding licensing consequences.

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

The GUI package versions above are a direction, not a future lockfile. Recheck them at GUI-phase planning because that phase is intentionally deferred.

## Suggested Project Shape

Start with one package and enforce dependency direction in code review and tests:

```text
src/
  engine/       # state, actions, legality, resolution, events; no Node/browser I/O
  cards/        # pinned card schema, normalized data, effect implementations, coverage
  simulator/    # match loop, replay, manifests, tournament/gauntlet orchestration
  agents/       # random, heuristic, search, and model adapters using legal actions only
  commands/     # automation entry points; not a human-play CLI
tests/
  rules/
  cards/
  determinism/
  replay/
  agents/
fixtures/
  cards/
  decks/
```

The engine must not import `node:fs`, worker code, model-provider code, React, or transport code. Automation commands may depend inward on all headless modules. When the GUI arrives, add `web/` or convert to a two-package pnpm workspace only if that creates a useful enforced boundary.

### Native TypeScript constraints

Node's native TypeScript execution strips erasable syntax; it does not type-check, transform every TypeScript feature, or honor `tsconfig.json` at runtime. Run `tsc --noEmit` separately and deliberately stay inside erasable syntax. Use string unions and `as const` objects instead of enums, and avoid parameter properties and runtime namespaces.

Recommended compiler baseline:

```json
{
  "compilerOptions": {
    "target": "ESNext",
    "module": "NodeNext",
    "moduleResolution": "NodeNext",
    "strict": true,
    "noEmit": true,
    "erasableSyntaxOnly": true,
    "verbatimModuleSyntax": true,
    "rewriteRelativeImportExtensions": true,
    "noUncheckedIndexedAccess": true,
    "exactOptionalPropertyTypes": true,
    "types": ["node"]
  }
}
```

## Determinism Contract

Determinism is an interface requirement, not just a seed option.

1. Represent the root seed as a canonical string or fixed-width integer in the run manifest.
2. Derive each game's seed from `SHA-256(rootSeed + canonical game ordinal + seat assignment)` using `node:crypto`. Scheduling order must never select seeds.
3. Construct a `pure-rand` generator per game and inject it. Record both `pure-rand@8.4.2` and the chosen algorithm (recommended: `xoroshiro128plus`) in the manifest.
4. Route every shuffle, random policy choice, randomized tie-break, and procedural generator through that object. Ban `Math.random()` in engine, simulator, and agents with linting or a repository check.
5. Define deterministic ordering for legal actions, map/set materialization, simultaneous triggers, tie-break candidates, and serialized events. Never rely on worker completion order.
6. Use fixed node, ply, rollout, or action budgets for ranked agents. Wall-clock time is telemetry only; a `Date.now()` cutoff can choose a different move under different machine load.
7. Bind a run to hashes of the normalized card snapshot, rules implementation/version, deck lists, agent versions/configurations, and output schema.
8. Make canonical event serialization part of the contract. The same manifest must produce byte-identical event JSONL and an identical final-state hash.

`pure-rand` is preferred over a hand-written Mulberry32 or `seedrandom`: it has no runtime dependencies, exposes pure generator state, provides unbiased distributions, and supports jumpable generators for independent streams. Still pin its version and algorithm; seeded reproducibility is not a promise that every future library release preserves a bitstream.

## Test Strategy

Use `node:test` initially. The core test matrix should include:

- state transition tests for every implemented rule and card effect;
- illegal-action rejection and unsupported-interaction fail-closed tests;
- legal-action enumeration checks proving every accepted action was enumerated;
- same-manifest-twice byte comparison of events and final state;
- replay tests that rebuild the final state solely from the initial manifest and event/action stream;
- randomized invariant loops over many fixed seeds, printing the failing seed for exact reproduction;
- seat-alternated paired matches for deck/agent comparisons;
- adapter contract tests showing scripted, heuristic, search, model, and later human clients all select from the same action schema.

Add Vitest only if the React phase demonstrates a real need for its DOM/component ecosystem. Do not run the headless engine under `jsdom`; keep its tests in the Node environment.

## Parallel Simulation Strategy

Prove single-thread determinism first, then profile. If CPU time justifies parallelism:

- create one reusable `worker_threads` pool, not a worker per game;
- default pool size to `Math.max(1, os.availableParallelism() - 1)` and allow an explicit override;
- send immutable, self-contained jobs containing game ordinal, seed, seat assignment, deck IDs, snapshot hashes, and agent configuration;
- keep engine state worker-local and avoid shared mutable state or `SharedArrayBuffer`;
- return structured results keyed by game ordinal, then sort by ordinal before aggregation and output;
- make crash/retry preserve the exact original job and seed;
- keep model/API calls out of CPU workers; they are I/O-bound and need separate rate/concurrency limits.

This design produces the same logical result at worker counts 1, 2, or N. Add that equivalence as a determinism test when the pool is introduced.

## Storage and Model Integration

Use versioned JSON for card/deck/manifests and append-only JSONL for action, event, decision, and telemetry streams. Produce aggregate JSON/CSV reports. This is enough for initial gauntlets and is easy to diff, replay, and archive.

Do not adopt a database initially. Node 24's built-in `node:sqlite` is still marked release-candidate stability in the official documentation. Reconsider SQLite when real result sets make ad hoc queries or indexes painful; do not add Redis, PostgreSQL, or Prisma for local deterministic runs.

Use Node's global `fetch` plus `AbortController` for the first model adapters and validate responses with Zod. Keep adapters outside the engine and persist the exact provider/model identifier, sampling controls, prompt/template hash, raw response or hash, parsed action, latency, and token/cost data. Add a provider SDK only when a required feature such as provider-specific streaming or authentication makes the extra dependency worthwhile. A model proposes an enumerated action; it never adjudicates rules.

## Reference Implementation Audit

### `realms-cards/contested-realms`

**Audited revision:** [`bfde327a90e96f6281ef8f6876712665954d8317`](https://github.com/realms-cards/contested-realms/commit/bfde327a90e96f6281ef8f6876712665954d8317) (2026-08-18)  
**License:** GPL-3.0  
**Confidence:** HIGH, from repository source, documentation, tests, and license

Its current application stack is substantially heavier than this project needs: Next 15, React 19, Zustand, Socket.IO, Prisma, Redis, Three.js, Zod, and Vitest. Its server accepts client-authored `MatchPatch` objects and validates/merges portions of them. Combat state and resolution also live in the client Zustand store. The repository's rules-engine analysis characterizes the engine as suitable for casual/basic play rather than strict competitive enforcement, and source paths still contain fail-open exception handling and an unimplemented movement validator.

The bot engine has useful ideas: phase-specific candidate generation, a separate evaluation function with a score breakdown, search modes, deterministic configuration, and decision telemetry. However, its candidates are whole-state patches rather than authoritative semantic actions. Its documented limitations include incomplete regions, instants, abilities, stack, and graveyard interactions. Search uses a wall-clock budget, while IDs/delays and some client paths use `Math.random()`. Its headless client also preserves or repairs local zones around server snapshots, evidence that the synchronization model is not appropriate as a simulator authority. A bot rules-validation test constructs mock states and checks hand-written conditions rather than importing the bot engine, so it does not demonstrate end-to-end engine correctness.

**Clean-room concepts worth adopting:**

- separate candidate generation from evaluation/search;
- phase-aware policies and explicit evaluation-score breakdowns;
- machine-readable decision telemetry;
- state transition and replay-oriented tests;
- mapping rules to focused modules and coverage status.

**Do not carry forward:**

- arbitrary client state patches or client-authoritative combat;
- fail-open validation/cost behavior;
- inferred intent from state diffs instead of typed actions;
- card-text regex as the primary executable rules model;
- wall-clock search limits for reproducible ranked comparisons;
- optimistic local repair of authoritative state.

Because the repository is GPL-3.0, copying or adapting its implementation can impose GPL obligations on a distributed derivative work. The conservative choice is behavior-level study followed by independent implementation unless the whole project intentionally adopts GPL-3.0-compatible distribution. This is a project-risk recommendation, not legal advice.

### `JollyGrin/sorcery-tcg-playtest` / spells.bar

**Audited revision:** [`a570ff48e435abdc0eaa777a8bd1d2cc6a69a7b5`](https://github.com/JollyGrin/sorcery-tcg-playtest/commit/a570ff48e435abdc0eaa777a8bd1d2cc6a69a7b5) (2026-07-27)  
**License:** no license file or package license declaration found  
**Confidence:** HIGH, from repository source and its linked Go server

This is a browser virtual tabletop, not a rules engine. The client uses Next 14, React 18, dnd-kit, React Query, and Tailwind. A game board is represented largely as indexed arrays; client utility functions directly move/draw/shuffle cards, including a `Math.random()` Fisher-Yates shuffle. WebSocket synchronization sends full player state as JSON. Its linked Go server stores each player's payload as opaque `json.RawMessage` and broadcasts combined state; it does not validate Sorcery actions or rules. No automated test files or test script were found in the audited tree.

**Behavior and UX concepts worth independently recreating later:**

- clear spatial presentation of sites, aura intersections, stacked cards, hand, deck, atlas, and cemetery;
- direct deck import/share and reconnect flows;
- touch-friendly manipulation and visible opponent state.

**Do not carry forward:**

- full-state client synchronization as the game protocol;
- index/array position as durable card identity;
- shallow object mutation and global random shuffle;
- drag gestures as the only action input;
- a generic relay server as a substitute for an authoritative engine.

No license grant means the default safe assumption is that its source cannot be copied or adapted without permission, despite being publicly visible. Reimplement only observed behavior and generic ideas; ask the owner for a license if code reuse becomes desirable. Its later GUI can inspire ergonomics, but the new UI should also provide accessible click-select/target-select controls and submit typed engine actions.

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

```bash
corepack enable
corepack prepare pnpm@11.22.0 --activate
pnpm add pure-rand@8.4.2 zod@4.4.3
pnpm add -D typescript@6.0.3 @types/node@24.1.0 eslint@10.8.1 typescript-eslint@8.67.0
```

Set `"packageManager": "pnpm@11.22.0"` and pin the Node version to `24.19.0` in the repository's chosen version file and CI. Commit `pnpm-lock.yaml`.

Later, after re-verifying versions at the GUI phase:

```bash
pnpm add react@19.2.8 react-dom@19.2.8
pnpm add -D vite@8.2.1 @vitejs/plugin-react@6.0.5 @types/react@19.2.18 @types/react-dom@19.2.4
```

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
