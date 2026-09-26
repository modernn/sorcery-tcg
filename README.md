# Sorcery Simulator

A private local Rust simulation lab: deterministic self-play, paired deck experiments,
checkpoint search, and replay. TypeScript handles local data ingestion and the browser.
Results are **experimental and unranked**. Supported scenarios do not establish complete
Sorcery rules coverage or competitive deck strength.

## Run it

Use Rust 1.97+, Node 24.19.0 and pnpm 11.22.0. The authority collector tests also require
PowerShell 7 (`pwsh` on PATH). No container or API key is needed.

```sh
pnpm install --frozen-lockfile
pnpm game:demo
pnpm play
```

The browser is at `http://127.0.0.1:4174`. It uses locally bound real-card presets when
available, otherwise the synthetic demo. It is optional for simulation.

## Let an LLM compare deck variants

The command interface is local JSON over stdin/stdout. No model is required by the
engine. An LLM can prepare candidate compositions, invoke the command, inspect results,
and retain a separate seed set for its final comparison.

```sh
mkdir -p .local/experiments
pnpm --silent game:experiment-example > .local/experiments/request.json
pnpm --silent game:experiment < .local/experiments/request.json > .local/experiments/result.json
```

Edit only `candidate` and `opponent` deck compositions to try variations. The validated
`baseManifest` supplies immutable card facts. Deck zones accept either card-ID arrays or
`[{"cardId":"…","copies":4}]`. The engine prunes unused definitions, retains required
tokens, validates the new game, binds policies, and runs both seat orientations per seed.
Decks must differ. Use `game:demo` or `game:batch` for mirror self-play.

Requests contain `schemaVersion: 1`, `baseManifest`, `candidate`, `opponent`, `seeds`,
`workers`, and optionally `artifactsDir` for synthetic runs. Start with one worker and
one seed; the limits are 128 seeds, eight workers, and 16 MiB input. Identical inputs
produce identical reports regardless of worker count. Read `limitations` and deck
`diagnostics`: engine support and Constructed legality are distinct.

Results include stable experiment/deck/policy IDs, wins/draws/losses by deck and physical
seat, each game's hashes, replay verification, and explicit eligibility. A small sample
is a smoke test, not a statistical claim. The baseline policy favors sites and minions;
it does not use every supported spell or activated ability effectively.

For the existing private real-card presets:

```sh
pnpm game:experiment-private --output-id first-experiment
pnpm --silent game:experiment < .local/authority/experiments/first-experiment.json \
  > .local/authority/experiments/first-result.json
```

`--preset` selects one of the local lesson/starter presets. Only cards already bound in
that preset's manifest can be used. Starter presets initially contain identical decks;
change one composition before running a paired experiment. Arbitrary imported cards with unbound printed rules
are rejected; their abilities are never silently removed. Private experiment inputs,
results, and checkpoints must remain beneath `.local/authority/`. Private experiment
artifact-directory writes are disabled; use the local session protocol for checkpoints.

### Build decks across presets

The recovered presets currently bind 96 of the 1,100 cards. Export a private catalog
to discover those cards, their exact facts, rarity, printed rules, source/fact hashes,
and the presets supplying each binding:

```sh
pnpm game:experiment-private --catalog --output-id bound-cards
```

Read `.local/authority/experiments/bound-cards.catalog.json`. Cards without bindings have
`engineSupported: false` and `reason: "no-preset-binding"`; being in the source corpus
does not make a card playable. The catalog also includes the local Constructed format.

Create `.local/authority/experiments/decks.json` with `schemaVersion: 1`, `candidate`,
`opponent`, `seeds`, and `workers`. Decks have the same avatar/atlas/spellbook shape used
above; counted and expanded zones are accepted. You can start by copying an existing
experiment request and removing `baseManifest`. Select card IDs from any supported
preset, then prepare and run:

```sh
pnpm game:experiment-private --decks experiments/decks.json --output-id custom-decks
pnpm --silent game:experiment < .local/authority/experiments/custom-decks.json \
  > .local/authority/experiments/custom-result.json
```

`--decks` is relative to `.local/authority/` and accepts at most 1 MiB. Only deck
composition, seeds, and worker count may be supplied: facts come from the source-checked
presets. Conflicting authority/fact bindings and unsupported cards fail closed. Required
tokens are retained, and Rust validates the manifest before it is written. This does not
prove Constructed legality or improve the baseline policy. Generated files are private,
created with owner-only permissions, and never overwrite an existing file.

## Search, checkpoint, and replay

`pnpm --silent game:session` starts the existing persistent Rust JSON-lines service.
Every request has `schemaVersion: 1`, a numeric `id`, a `method`, and `params`.

```json
{"schemaVersion":1,"id":1,"method":"new","params":{"manifestJson":"<canonical manifest JSON>"}}
{"schemaVersion":1,"id":2,"method":"legalActions","params":{"seat":"north"}}
{"schemaVersion":1,"id":3,"method":"runCounterfactual","params":{"maxContinuationDecisions":4}}
{"schemaVersion":1,"id":4,"method":"runNoveltyFrontierSearch","params":{"maxActions":4,"maxBranches":2}}
{"schemaVersion":1,"id":5,"method":"checkpoint","params":{}}
{"schemaVersion":1,"id":6,"method":"verifyReplay","params":{}}
```

Send one JSON object per line and read the matching response ID. `new` takes the
serialized `baseManifest` from an experiment request. `step` accepts only an issued
`actionId`, `stateVersion`, and `seat`. `observe` gives that seat's redacted observation;
`selectPolicyAction` supplies a deterministic baseline action. `resume` takes
`{ "checkpoint": <checkpoint object> }`. Search does not mutate its root; unfinished
branches are horizons, not wins. `exportGameRecord` requires a finished game.

For synthetic disk replay:

```sh
pnpm game:engine demo 31 .local/experiments/replay-31
pnpm game:engine replay .local/experiments/replay-31
pnpm game:engine replay-steps .local/experiments/replay-31
```

## Verify

```sh
pnpm verify:all
pnpm game:selfplay-acceptance
pnpm game:selfplay-soak
```

`verify:all` runs locked Rust format/check/Clippy/tests with two build/test threads,
then TypeScript typecheck/lint/tests. Tests need child processes and local loopback ports.
The acceptance and soak commands are bounded synthetic campaign contracts; they do not
prove real-deck strategic strength. Private release verification is separate and requires
the historical ignored inputs described in the authority policy.

See [the release assessment](docs/local-release.md) for remaining work and
[the private authority policy](docs/external-reuse-policy.md) for data handling.
The old roadmap and overnight catalog handoff are historical, not the current release plan.
