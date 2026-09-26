# Local release assessment

Assessed 2026-09-25 on `master`, starting at `a0391e0bb`. Scope agreed with the user:
reliable local self-play, search, replay, and an LLM-usable deck variation interface.

## Assessment

The project has a real, substantial Rust engine and usable deterministic simulation,
checkpoint, search, and replay primitives. Its main obstacle is integration and evidence
quality. Continuing to grow the rule catalog would not finish the product.

The previous completion signals were unreliable. Planning documents disagreed about
the language and phase. The 2,590-row catalog contained duplicate descriptions and proof
references; hundreds of rows restated the same unranked batch property at different
sizes. Catalog size is not an estimate of official-card coverage.

## Release blockers addressed

- A public fixture authority hash, or an arbitrary local hash list, could make synthetic
  facts count as verified official authority. These trust paths are removed. Manifest
  declarations alone cannot produce ranked results.
- Candidate imports could strip nonblank abilities from Avatars, sites, and minions.
  Unbound printed text now yields explicit unsupported diagnostics before simulation.
  Unsupported imports no longer throw while constructing an incomplete manifest.
- The TypeScript session adapter launched any existing release executable without
  checking its source. It now launches through locked Cargo so edits cannot be hidden
  by an old binary.
- Fresh builds exposed a Leap Attack continuation that struck surviving enemies again
  after resolving Deathrites. The continuation now defers only Magic completion after
  the strike. A checkpoint regression proves surviving targets are struck once.
- The ordinary eligibility suite scheduled at least 66,532 complete games. Redundant
  size-only tests and their catalog rows were removed; the 12 distinct cases passed in
  about 23 seconds with two threads. Retained catalog IDs are not renumbered.
- ESLint traversed old worktrees and produced 5,153 errors. Its scope and parser root
  now target this checkout. Rust test lint defects and broken proof links were repaired.
- A synthetic format fixture had LF bytes but a CRLF hash. Its provenance and lock now
  bind the actual tracked bytes; all 27 bundle tests passed after this correction.
- The seven private source files were found in a sibling backup. The rebuilt v4 input,
  bundle, and full revision hashes match the historical receipt exactly. All 1,100 cards
  and six local presets are available again; reconstructed scenario seeds are recorded
  separately from the historical authority identity.

The new `experiment-json` command accepts two compositions over a validated immutable
card pool, binds deterministic policies, swaps seats per seed, and emits stable IDs,
outcome counts, replay verification, and limitations. It reuses the existing Rust
gauntlet and rule validation. Existing JSON-lines sessions expose position search,
engine-issued actions, checkpoint branching, and replay without an LLM dependency.

## Definition of this release

The local release is complete only when the locked Rust gates and `pnpm verify` pass,
a fresh-process synthetic experiment agrees across worker counts, a recovered private
preset completes both seat orientations with verified replay, and checkpoint search
preserves its root. Commands and constraints are documented in the root README.

This is an experimental local simulator. Ranked competitive evaluation is not part of
this release. Unknown mechanics must remain blocked; real rules must never be weakened
to make a run succeed.

## Verified outcome

- Locked Rust formatting, compilation, and Clippy passed. The ordinary workspace suite
  passed 2,151 tests; its two release-only tests were run separately and passed.
- `pnpm verify` passed typecheck, lint, and all 479 public tests, including browser API
  checkpoint/replay and Rust fixture parity. The corrected Leap Attack fixture matches
  the existing expected events without changing its expected outcome.
- The production-sized synthetic campaign contract passed in 154 seconds. The repeated
  self-play/search soak passed in 8.32 seconds with identical evidence across both runs:
  two candidates, two opponent policies, four seeds, and eight seat orientations.
- A recovered private lesson preset completed both seat orientations. Worker counts one
  and two produced byte-identical reports. Changing one candidate card copy produced a
  different deck identity, retained the opponent identity, and verified both replays.
- Counterfactual and frontier search left the root checkpoint unchanged; checkpoint
  resume round-tripped exactly. Saved synthetic artifacts replayed with matching hashes.

These checks establish the bounded local workflow above. They do not establish complete
official-card coverage, competitive policy strength, or a private authority release.

## Cross-preset deck construction

The private experiment command now exports a card-binding catalog and accepts deck-only
requests across the six existing presets. Their union contains 96 distinct bindings
under one authority identity, with no conflicting facts. The catalog records the supplying
presets and exact source/fact hashes; the other 1,004 cards are explicitly unbound.

Preparation accepts card IDs and quantities, rejects supplied fact overrides, keeps token
dependencies, and requires Rust to admit the resulting manifest before saving it. Inputs
and outputs stay inside the private authority boundary. No new rule interpretation or
ranked eligibility is introduced.

A Fire-versus-Water cross-preset smoke test completed four games over two seeds and both
seat orientations. Every replay verified, and reports were byte-identical with one and
two workers. Synthetic regressions cover conflicting bindings, authority mismatches,
token dependencies, canonical deck encodings, unsupported cards, and input bounds.
The follow-up `pnpm verify` passed typecheck, lint, and all 484 public tests.

## Real-deck benchmark intake (2026-09-26)

A private research snapshot inspected 434 published lists from SorceryCard and
Curiosa, with additional discovery and corroboration from SorceryRec and Sorcerers
Summit. It selects four distinct compositions for each of the 34 Avatars and a
12-deck budget shortlist. Seventy selected lists have a source-reported tournament
finish; the other selections are provisional community benchmarks, not established
top-four competitive decks. Element diversity uses spell composition rather than
only the site's primary-element badge.

The snapshot, linked review index, counted-card requests, admission diagnostics, and
offline reproduction scripts live under the ignored
`.local/authority/sorcerycard-research/2026-09-26/` directory. External deck lists are
examples to test, never authority for card behavior. Obvious incomplete lists and
copy-limit violations were excluded; the intake screen does not replace engine
legality. Prices are source estimates, not complete purchase quotes.

All selected card names resolve to the recovered authority. The 136 selections use
881 distinct main-deck cards, including 805 without current engine bindings. All 136
correctly fail preparation; none has a simulated win rate. A separate supported
control completed four games with every replay verified. This proves the control
workflow and rejection boundary, not support for the researched decks.

Grow support against these fixed requests: reuse shared Rust mechanics, add one
direct proof for each missing rule slice, bind matching cards, and rerun admission.
The first narrow candidate has 15 missing bindings and an already bound Avatar.
No per-deck rules engine, automatic text interpreter, or replacement preset is needed.

## Shared behavior expansion, cycle 1 (2026-09-26)

The private corpus matches all 1,100 unique card names and their set memberships in
the live official API. No names are missing locally or present only locally. This
checks identity completeness, not current rules wording: gameplay still uses the
pinned private authority revision. Unreleased expansion cards are outside this scope.

The usable pool grew from 96 to 190 bindings: 59 from existing source-checked scenario
facts and 35 reviewed private entries. Private bindings require exact source identity,
matching base stats, complete-text review, and references to direct scenario proofs.
The engine still owns legality. Review metadata does not confer ranked eligibility.
There are 910 unbound cards; 732 occur in the selected benchmark field. All 136
researched decks remain blocked, and the closest initial candidate now lacks 13 cards.

One typed shared site-count query now handles scope, controller, same-card identity,
and enemy surface occupancy. Existing conditional mana and adjacent-copy draw paths
reuse it, alongside Genesis mana per matching site. The direct scenario checks timing
after site placement, duplicate occupants, excluded regions, deterministic transitions,
strict query validation, and checkpoint resume. Other new bindings reuse existing
keyword and effect implementations; no new per-card dispatcher was added.

Four private deck compositions (a control and one, two, or three card replacements)
completed 32 paired games with verified replay. Repeating those cases at one and 16
workers produced byte-identical reports to eight workers. Three separate guided
probes actually summoned the new cards, branched search without changing the root,
resumed checkpoints exactly, and replayed to completion. Guided openings establish
coverage, not unbiased deck strength; all four small paired samples finished 5–3.

Rust batches now distribute independent jobs through an atomic work queue and retain
canonical output order. A release benchmark on this 24-logical-CPU machine measured
3.53 games/second with one worker, 38.59 with 16, and 37.43 with 24 (medians of three
runs after warmup, 48 fixed games per run). Every worker count produced the same
result hash. Peak process RSS was approximately 94 MiB. These are local workload
measurements, not a promise for other machines or deck workloads. Single-position
search remains serial; independent games are the parallel unit.

Final validation passed formatting, locked compilation, Clippy, 2,153 ordinary Rust
tests, and all 486 public tests with typecheck and lint. The two optional release-only
Rust tests remain ignored by this ordinary run; their earlier release results are
recorded above. Private coverage, guided records, and experiment receipts are under
`.local/authority/binding-cycles/cycle-01/`; verification and performance logs are under
`.local/recovery/`. The earlier intake counts above describe the pre-expansion snapshot.

## Remaining work, in order

1. Expand the 190-card binding pool through shared behavior slices and complete-card
   review. Use the whole released corpus to identify reusable behavior families, then
   prioritize slices that unblock the fixed benchmark decks. Repeat small deck changes,
   guided mechanic exercises, and deterministic replay checks after each expansion.
2. Improve the baseline policy using observed tactical state and engine-issued actions.
   Its observation currently contains deck counts, enemy Avatar position, and temporary
   power identities; it needs more public tactical information to evaluate spell effects.
   It favors sites/minions and underuses many spells, Artifacts, Auras, and activations.
   Policy strength limits what deck win rates mean even when rules and
   replay are correct.
3. Bind authoritative card facts to independently verified private inputs before
   reintroducing ranked eligibility. Exact historical bytes are necessary evidence,
   but a caller-supplied authority label is not sufficient proof of its facts.
4. Expose persistent policy-training campaigns only when they are needed. Promotion,
   held-out seeds, final audits, and checkpoint APIs already exist in Rust; the current
   command surface focuses on deterministic deck experiments and position search.

Do not spend this release window splitting the 30,000-line game module, adding a
database, acquiring art, building a model-provider layer, or redesigning the browser.
Those tasks do not close the current simulation acceptance gates.

## App interruption

Codex exited at 23:03:29 Alaska time and relaunched at 23:03:37, then restarted again
around 23:15:56. The first incident had no recorded core dump or OOM event. After the
second, orphaned Vitest workers from another project exhausted the workstation's
30 GiB RAM and drove roughly 20 GiB of swap use. One worker's core dump confirms
V8 `FatalProcessOutOfMemory`; this establishes a test-worker heap failure and severe
system memory pressure, but does not prove the cause of the desktop app's own exit.

Terminating only the verified orphan workers relieved that pressure. This project's
build/test concurrency and tool output are capped, with logs and an ignored recovery
checkpoint saved locally. Check available RAM, swap activity, and memory pressure
before increasing concurrency. Additional swap is a buffer, not a replacement for
bounded worker counts or a fix for per-process heap exhaustion.
