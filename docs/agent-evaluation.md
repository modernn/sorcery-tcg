# Optional semantic review for development agents

The first Jev experiment evaluates 16 project-owned synthetic proposals against
explicit software contracts. It asks whether the proposal contradicts its supplied
evidence, is consistent with the limited claim, or needs more evidence. These are
agent-reviewed development examples, not a held-out quality benchmark.

Run the dependency-free offline adapter check:

```sh
pnpm agents:review-example
```

This exercises request construction, response validation, egress guards, and
abstention accounting without network calls. Mock responses prove only adapter
behavior; the report labels actual model quality as unmeasured.

For one live evaluation, configure `TYPESAFE_API_KEY` locally and run:

```sh
node scripts/jev-semantic-review.mjs --live > .local/recovery/jev-live-report.json
```

The fault-localization packet asks where a synthetic failure originates and which
local check best separates the possible causes:

```sh
pnpm agents:diagnose-example
node scripts/jev-semantic-review.mjs --live --diagnose > .local/recovery/jev-diagnostics-live.json
```

It contains ten cases with twenty independent questions. Similar missing-action
symptoms have different causes: resource setup, hand preparation, insufficient
evidence, or a reversed legality comparison. Other cases cover stale client actions,
effect suspension, transaction rollback, checkpoint history, experiment identity,
and incorrect search assertions. Labels were reviewed with expectations visible;
these are development examples, not a blind evaluation.

The adapter sends only its hash-pinned synthetic packet. It cannot load arbitrary
diffs, private authority, decks, or game observations. It uses the official endpoint,
a pinned model, one request, a timeout, a bounded response, and no retries. See the
[TypeSafe API contract](https://docs.typesafe.ai/api). Missing credentials or invalid
responses fail the experiment without changing engine state or result eligibility.

## Development experiment

Use semantic feedback to choose what to inspect or repair. Source review, direct
expected-outcome proofs, replay, and the required Rust/project checks decide whether
a change is accepted. A model confidence score cannot replace these requirements.

Before relying on Jev, independently label more realistic synthetic examples and
reserve a disjoint holdout. Compare the coding agent alone, the same agent with the
checklist, and the agent with Jev feedback. Measure missed defects, false alarms,
verified acceptance, rework, wall time, and total model cost. Record request/response
hashes, resolved model, token usage, latency, abstentions, and disagreements. The
current exploratory confidence threshold is not calibrated.

Two initial calls on 2026-09-29 returned model `jev-1.13.0`. The semantic packet
agreed with all sixteen author labels in 372 ms. The diagnostic packet agreed on
nine of ten areas and all ten next checks in 319 ms. Its area mismatch had low
confidence and was treated as an abstention. These are single-call observations;
they establish neither typical latency nor faster development. Responses and usage
receipts remain local. Extend the examples without selecting them to favor a model,
then measure failure-to-verified-fix time on held-out failures.

## Playing-agent experiment

An optional exploration prefix now ranks engine-issued actions for bounded native
search. Rust validates the exact session hash, state version, and every action ID
before branching. It preserves canonical action indices for replay, appends unlisted
actions in their original order, and leaves ordinary search unchanged when advice is
absent. Models cannot create actions or participate in deterministic continuations.

Run the fixed synthetic integrations without credentials or network calls:

```sh
pnpm agents:probe-example
pnpm agents:play-example
```

With `TYPESAFE_API_KEY` configured, the corresponding live commands are:

```sh
pnpm agents:probe-live
pnpm agents:play-live
```

The Node entry point supplies only HTTPS and process transport. Rust creates the
synthetic failure, executes the fixed probes, binds advice, runs search, and verifies
replay. The transport permits only the reviewed hash-pinned packets. Offline play
uses an explicitly synthetic answer to test the connection, not model quality.

In the first native development pilot, Jev requested a stored-history comparison
and withheld a diagnosis before that evidence was supplied. After the fixed probe,
it identified the deliberately truncated checkpoint producer. In the first live
play pilot, Jev chose `insufficient_evidence`; both search arms retained canonical
order and replayed exactly. An offline noncanonical preference also replays correctly.
These narrow observations establish functioning integrations, not stronger play or
measured development acceleration. Preserve model decisions locally; fresh hosted
inference is not assumed deterministic.

Compare policy, search, model ranking, and ranked search using equal node budgets
and separate equal wall-time budgets. Keep both seat orientations and fixed seeds.
Unfinished search branches remain unknown. Private official derivatives remain
local under the [reuse policy](external-reuse-policy.md); a player-visible observation
is not authorization to upload it.
