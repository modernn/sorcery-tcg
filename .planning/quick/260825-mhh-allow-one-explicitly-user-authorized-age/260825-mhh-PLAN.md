---
phase: quick-260825-mhh-allow-one-explicitly-user-authorized-age
plan: "01"
type: execute
wave: 1
depends_on: []
files_modified:
  - scripts/collect-private-authority.ps1
  - scripts/verify-private-authority-boundary.ts
  - tests/authority/private-authority-collector.test.ts
  - tests/authority/private-authority-boundary.test.ts
  - docs/external-reuse-policy.md
  - .planning/phases/01-rules-and-data-authority/01-CONTEXT.md
  - .planning/phases/01-rules-and-data-authority/01-07-PLAN.md
  - .local/authority/authorizations/quick-260825-mhh.consumed.json
  - .local/authority/authorizations/quick-260825-mhh-retry-1.consumed.json
  - .local/authority/inputs/official-2026-08-20/primary/**
  - .local/authority/locks/official-2026-08-20/source-set-lock.json
  - .planning/quick/260825-mhh-allow-one-explicitly-user-authorized-age/260825-mhh-SUMMARY.md
autonomous: true
requirements: [DATA-01, DATA-02, DATA-03]
must_haves:
  truths:
    - "The user's two 2026-08-25 instructions permit exactly one attempted agent invocation apiece under the closed references `quick-260825-mhh` and `quick-260825-mhh-retry-1`; distinct atomic durable consumption records are created before transport and survive failure, cleanup, and deletion of collected outputs, so neither can become standing permission."
    - "The authorized run still requires the private-use risk acknowledgment and records `acquisitionMethod: user-authorized-agent-run-one-shot-powershell` plus the exact authorization reference in the ignored lock."
    - "The collector retains its fixed revision, seven-path/official-host allowlist, absent destinations, bounded sequential transport, no retry/evasion, no arbitrary URL, no scheduler/polling, no artwork, independent backup, verifier gates, and receipt-last publication."
    - "After the original authorization was consumed, the agent invokes the production collector exactly once under the fresh `quick-260825-mhh-retry-1` authorization, stops without another attempt on any failure, and independently verifies the completed primary, backup, metadata, rulebook evidence, and source-set root hash."
    - "Every official byte and private absolute locator remains outside Git and packages; committed artifacts contain only code, tests, policy, safe hashes/relative metadata, and summaries without source excerpts."
  artifacts:
    - path: "scripts/collect-private-authority.ps1"
      provides: "Atomic pre-transport consumption of either of the two closed one-run authorization references and truthful agent-run acquisition evidence without changing the collector manifest or transport"
      contains: "quick-260825-mhh"
    - path: "scripts/verify-private-authority-boundary.ts"
      provides: "Post-run Git index/HEAD/package-content leakage gate driven by the ignored lock and private source bytes"
    - path: ".local/authority/authorizations/quick-260825-mhh.consumed.json"
      provides: "Ignored durable evidence that the initial agent authorization was consumed before any publisher request, retained forever independently of collected outputs"
    - path: ".local/authority/authorizations/quick-260825-mhh-retry-1.consumed.json"
      provides: "Distinct ignored durable evidence that the fresh retry authorization was consumed before any publisher request"
    - path: "tests/authority/private-authority-collector.test.ts"
      provides: "Synthetic proof that only either exact fixed authorization reference selects its independent agent-run receipt and all prior collector boundaries remain enforced"
    - path: "docs/external-reuse-policy.md"
      provides: "Narrow historical authorization for this agent-run collection while preserving every private-use and permission gate"
    - path: ".local/authority/locks/official-2026-08-20/source-set-lock.json"
      provides: "Ignored seven-source lock with truthful acquisition method, authorization reference, acknowledgment, metadata, and source-set root hash"
    - path: ".planning/quick/260825-mhh-allow-one-explicitly-user-authorized-age/260825-mhh-SUMMARY.md"
      provides: "Safe completion receipt containing no official bytes, source excerpts, or private absolute locators"
  key_links:
    - from: "tests/authority/private-authority-collector.test.ts"
      to: "scripts/collect-private-authority.ps1"
      via: "loopback-only authorized-agent acquisition cases prove atomic consumption before transport plus unchanged fail-closed regression coverage"
      pattern: "user-authorized-agent-run-one-shot-powershell"
    - from: "scripts/collect-private-authority.ps1"
      to: ".local/authority/authorizations/quick-260825-mhh.consumed.json"
      via: "The selected fixed reference maps to its own FileMode.CreateNew record before transport on every authorized-agent entry path; cleanup never removes either record"
      pattern: "quick-260825-mhh"
    - from: "scripts/collect-private-authority.ps1"
      to: ".local/authority/locks/official-2026-08-20/source-set-lock.json"
      via: "exact per-run authorization reference copied into the receipt published after final verification"
      pattern: "quick-260825-mhh"
    - from: ".local/authority/locks/official-2026-08-20/source-set-lock.json"
      to: "src/authority/private-source-set.ts"
      via: "an independent post-run call to verifyPrivateSourceSet over the final primary and backup roots"
      pattern: "verifyPrivateSourceSet"
    - from: ".local/authority/inputs/official-2026-08-20/primary/"
      to: ".gitignore"
      via: "anchored ignore plus HEAD, index, and pnpm-pack blob/content leakage gates"
      pattern: ".local/authority/"
    - from: "scripts/verify-private-authority-boundary.ts"
      to: "Git HEAD/index and `pnpm pack --dry-run --json` contents"
      via: "exact private source hashes, source-derived byte markers, absolute locators, restricted signatures, artwork magic/extensions, and forbidden private paths"
      pattern: "pnpm pack --dry-run --json"
---

<objective>
Permit and execute the fresh fixed private authority retry explicitly authorized by the user on 2026-08-25 after the original authorization was durably consumed, then independently verify its source set without weakening any other acquisition, use, or distribution boundary.

Purpose: Finish the missing Phase 1 authority input safely and truthfully: the executor is the actor for this fresh fixed retry only, while recurring or otherwise agent-run collection remains blocked.
Output: A tested one-run authorization seam, reconciled policy/planning contracts, one ignored primary/independent-backup source set and lock, and a safe quick-task summary.
</objective>

<execution_context>
@C:/Users/Dad/.codex/get-shit-done/workflows/execute-plan.md
@C:/Users/Dad/.codex/get-shit-done/templates/summary.md
</execution_context>

<context>
@AGENTS.md
@.planning/STATE.md
@.planning/PROJECT.md
@.planning/REQUIREMENTS.md
@.planning/ROADMAP.md
@.planning/phases/01-rules-and-data-authority/01-CONTEXT.md
@.planning/phases/01-rules-and-data-authority/01-07-PLAN.md
@docs/external-reuse-policy.md
@docs/authority-precedence.md
@scripts/collect-private-authority.ps1
@src/authority/private-source-set.ts
@tests/authority/private-authority-collector.test.ts
@tests/authority/private-source-set.test.ts
@package.json

<interfaces>
Reuse the existing verifier without modification:

From `src/authority/private-source-set.ts`:

```typescript
export const PRIVATE_AUTHORITY_SOURCE_PATHS: readonly [
  'rulebook/rulebook-current.pdf',
  'formats/constructed-current.html',
  'codex/codex-current.html',
  'codex/faqs-current.html',
  'codex/changelog-current.html',
  'updates/card-updates-2025.html',
  'cards/cards.raw.json',
];

export type PrivateAuthoritySourceEntry = Readonly<{
  relativePath: (typeof PRIVATE_AUTHORITY_SOURCE_PATHS)[number];
  url: string;
  retrievedAt: string;
  effectiveDate: string | null;
  mediaType: 'application/pdf' | 'text/html' | 'application/json';
  byteLength: number;
  byteHash: `sha256:${string}`;
}>;

export async function verifyPrivateSourceSet(options: {
  primaryRoot: string;
  backupRoot: string;
  repositoryRoot: string;
  entries: readonly PrivateAuthoritySourceEntry[];
}): Promise<{
  entries: readonly PrivateAuthoritySourceEntry[];
  sourceSetRootHash: `sha256:${string}`;
}>;
```

`verifyPrivateSourceSet` already enforces exact paths and metadata, audited official HTTPS hosts, size limits, valid card JSON, ordinary independent roots/files, no symlink/junction/hardlink aliasing, primary/backup byte equality, deterministic sorting, and canonical source-set hashing. Do not duplicate or loosen it.
</interfaces>
</context>

<source_coverage>

| Source | ID | Feature / constraint | Task | Status | Notes |
|---|---|---|---|---|---|
| GOAL | — | Developers can identify and reproduce the exact official authority inputs governing later games | 3 | COVERED | One fixed ignored source set is collected and independently verified. |
| REQ | DATA-01 | Immutable pinned official rules/formats/Codex/update sources with provenance and hashes | 1-3 | COVERED | Existing seven-source contract is unchanged; the truthful acquisition actor is added and verified. |
| REQ | DATA-02 | Full card snapshot input remains available offline during games and experiments | 3 | COVERED | Exact bounded API JSON is private and validated from both roots. |
| REQ | DATA-03 | Stable provenance and content identity | 1-3 | COVERED | Existing sorted entries/root hash are retained with one authorization reference outside the identity document. |
| RESEARCH | — | No task-specific research artifact or new dependency/integration decision | — | COVERED | Current tested collector/verifier contracts supply sufficient Level 0 codebase evidence. |
| CONTEXT | D-01, D-02 | Official-only authority, precedence, effective dates, and fail-closed ambiguity | 1-3 | COVERED | The exact official manifest and existing precedence policy remain unchanged. |
| CONTEXT | D-03, D-04 | Offline selected bundle and immutable no-overwrite revisions | 1-3 | COVERED | The run creates only the fixed absent `official-2026-08-20` input and receipt. |
| CONTEXT | D-05, D-06, D-07 | Deterministic normalization/provenance/canonical hashing/exact diagnostics | 1-3 | COVERED | Existing TypeScript verifier and root identity are reused unchanged. |
| CONTEXT | D-08 | Fixed private one-shot seven-source acquisition, independent backup, metadata, and no content leakage | 1-3 | COVERED | Task 1 consumes the narrow authorization before transport; Task 2 records the actor amendment; every other D-08 clause remains locked. |
| CONTEXT | D-09 | Community code/data/assets remain behavioral-reference only | 1-3 | COVERED | No community source or content enters this task. |
| CONTEXT | D-10 | TypeScript/Node standard-library first | 1 | COVERED | Existing PowerShell/.NET and Node facilities are sufficient; no package is added. |
| CONTEXT | D-11 | Private/local/noncommercial scope and broader-use permission gate | 1-3 | COVERED | The one referenced agent run is the only amendment; recurring/other agent runs, redistribution, hosting, uploads, public APIs, commercialization, and artwork remain blocked. |

Deferred collection/deck import, rules execution, model competitors, browser play, recurring updates, public hosting/API, redistribution, commercial use, and artwork acquisition remain excluded.
</source_coverage>

<tasks>

<task type="auto" tdd="true">
  <name>Task 1: Consume closed one-run authorizations and add a private-boundary gate</name>
  <read_first>
    - AGENTS.md
    - .planning/phases/01-rules-and-data-authority/01-CONTEXT.md
    - docs/external-reuse-policy.md
    - scripts/collect-private-authority.ps1
    - src/authority/private-source-set.ts
    - tests/authority/private-authority-collector.test.ts
    - tests/authority/private-source-set.test.ts
  </read_first>
  <files>scripts/collect-private-authority.ps1, tests/authority/private-authority-collector.test.ts, scripts/verify-private-authority-boundary.ts, tests/authority/private-authority-boundary.test.ts</files>
  <behavior>
    - The current user-run invocation remains valid and continues to record `user-run-one-shot-powershell` without an authorization reference.
    - Supplying either exact authorization reference `quick-260825-mhh` or `quick-260825-mhh-retry-1` atomically creates its distinct `.local/authority/authorizations/<fixed-reference>.consumed.json` with `FileMode.CreateNew` before transport, selects `user-authorized-agent-run-one-shot-powershell`, and records that reference in the ignored lock after successful verification.
    - Each consumed record remains after success, controlled failure, collector cleanup, and deletion of primary/backup/lock outputs; every later use of its fixed reference, including a concurrent attempt, fails before a request or output mutation, while consumption of one reference neither creates nor resets the other record.
    - Any other nonempty authorization reference, including case variants, padded values, or fabricated retry numbers, fails before network or filesystem mutation; the direct wrapper has no generic authorization/config file, recurring mode, URL, manifest, artwork, retry, or scheduling input.
    - Agent-authorized loopback collection retains the exact seven sources, acknowledgment, staged/final/candidate verification, independent backup, and receipt-last behavior already covered by the suite.
    - A separate content-safe gate rejects private bytes or locators copied under any Git or package path by inspecting Git HEAD blobs, index blobs, and every path/content reported by `pnpm pack --dry-run --json`.
  </behavior>
  <action>Extend `tests/authority/private-authority-collector.test.ts` first. Keep the existing user-run cases unchanged. For each exact fixed reference `quick-260825-mhh` and `quick-260825-mhh-retry-1`, prove a controlled loopback stop immediately after authorization consumption observes its distinct durable record before the server receives any request; prove each strict schema contains only `schemaVersion: 1`, the fixed revision, `user-authorized-agent-run-one-shot-powershell`, the matching fixed reference, and a valid UTC consumed timestamp, with no private root or remote locator. Prove consuming the fresh retry leaves the original record byte-for-byte unchanged. Cover a successful run and a failure after transport begins, then perform the collector's normal cleanup and explicitly delete any primary, backup, and lock outputs; in both cases a second invocation with the same reference must fail before the request count or filesystem changes while the consumed record remains. Include a concurrent two-invocation case proving atomic create-new semantics allow at most one attempt to cross each guard. Prove an unknown, blank-padded, case-variant, fabricated retry, or otherwise noncanonical reference is rejected before any request, consumption record, or output mutation. Assert from the PowerShell AST/signatures that the direct script wrapper, exported production function, loopback seam, and collection core all forward the same closed authorization context and that no authorized-agent entry point bypasses the consumption guard. Retain the direct parameter-surface assertion forbidding production URL, descriptor, manifest, art, scheduler, polling, concurrency, retry, timeout, update, or consumption-record override inputs.

Update `scripts/collect-private-authority.ps1` so optional `UserAuthorizationReference` is accepted only as absent, exactly `quick-260825-mhh`, or exactly `quick-260825-mhh-retry-1`; keep `BackupRoot` and `AcknowledgePrivateUseRisk` mandatory. Route the direct wrapper, `Invoke-PrivateAuthorityCollection`, `Invoke-PrivateAuthorityCollectionForLoopbackTest`, and the core through one acquisition-context discriminator. With no reference, preserve the existing user-run method and do not create an authorization record. With either exact reference, atomically create its own `.local/authority/authorizations/<fixed-reference>.consumed.json` using `FileMode.CreateNew` after validating configuration/paths but before calling any transport; fail closed if the file or a same-named filesystem object already exists. Never remove, reset, overwrite, quarantine, or infer eligibility from primary/backup/lock existence. Keep both consumption records outside all staging/output cleanup paths. Pass the closed context into receipt creation so a successful ignored lock records `acquisitionMethod: user-authorized-agent-run-one-shot-powershell` and the selected exact reference; a failed attempt has no lock but remains consumed. Add only a loopback-controlled fault immediately after consumption to prove ordering; production still forbids fault injection. Do not change the revision ID, seven descriptors, URLs/hosts, transport, bounds, stop conditions, acknowledgment, backup/verifier gates, source-set identity document, or receipt-last publication.

Create `scripts/verify-private-authority-boundary.ts` with Node standard-library calls only. Given the ignored lock path and repository root, load the seven private files without printing their bytes, locators, or content-derived markers. Enumerate and read every committed HEAD blob, every current index blob (including a staged copy under a renamed path), and every worktree file listed by parsed `pnpm pack --dry-run --json`; reject any candidate path under `.local/authority`, any candidate whose complete SHA-256 equals a private entry hash, any candidate containing deterministic source-specific byte windows derived in memory from the private files, any textual occurrence of the private primary root, backup root, or non-normative rulebook locator in native or slash-normalized form, and any artwork/image file extension or binary magic signature. Allow safe relative source metadata and hash strings, and report only the candidate path plus failure category. Add `tests/authority/private-authority-boundary.test.ts` using a temporary Git/package fixture and synthetic private lock/files: prove a safe metadata-only file passes; exact bytes under another tracked or staged name, a private locator, a source-derived excerpt marker, an image/artwork blob, `.local/authority` package paths, and an untracked pack-included copy each fail. Prove the dry-run package list is parsed and every listed file's bytes are inspected. Do not add a dependency or package script.

Do not modify `src/authority/private-source-set.ts`, contact publisher endpoints, or run the production collector in this task. Commit the tested code slice as `feat(quick-260825-mhh): consume one agent authorization` after the collector, boundary, and source-set suites plus `pnpm verify` pass.</action>
  <verify>
    <automated>node --test tests/authority/private-authority-collector.test.ts &amp;&amp; node --test tests/authority/private-authority-boundary.test.ts &amp;&amp; node --test tests/authority/private-source-set.test.ts &amp;&amp; pnpm verify</automated>
  </verify>
  <done>Each exact fixed authorization can independently cross its pre-transport guard once at most, remains consumed after every success/failure/cleanup/output-deletion path, every entry point shares the guard, and the tested Git/index/package gate detects private bytes, excerpts, locators, paths, and artwork without adding a dependency.</done>
</task>

<task type="auto">
  <name>Task 2: Reconcile the two fixed exceptions across policy and Phase 1 contracts</name>
  <read_first>
    - .planning/PROJECT.md
    - .planning/REQUIREMENTS.md
    - .planning/phases/01-rules-and-data-authority/01-CONTEXT.md
    - .planning/phases/01-rules-and-data-authority/01-07-PLAN.md
    - docs/external-reuse-policy.md
    - docs/authority-precedence.md
    - scripts/collect-private-authority.ps1
    - tests/authority/private-authority-collector.test.ts
  </read_first>
  <files>.planning/phases/01-rules-and-data-authority/01-CONTEXT.md, docs/external-reuse-policy.md, .planning/phases/01-rules-and-data-authority/01-07-PLAN.md</files>
  <action>Amend D-08 and D-11 in `01-CONTEXT.md` with both 2026-08-25 decisions: authorization reference `quick-260825-mhh` permits the consumed initial attempted invocation, while the user's later `Authorize new run` instruction after the PDF and changelog parser fixes permits one fresh attempted invocation under `quick-260825-mhh-retry-1`. Each has its own durable ignored `.local/authority/authorizations/<fixed-reference>.consumed.json` created before transport, and each record independently exhausts its permission regardless of success, failure, cleanup, or later deletion of collected primary/backup/lock outputs. State that these supersede only the prior actor restriction for their respective consumed references and create no standing permission. Encode exactly three allowed evidence pairs—`user-run-one-shot-powershell` with `authorizationReference` absent, or `user-authorized-agent-run-one-shot-powershell` with either exact fixed authorization reference and its matching distinct consumed record—and state no other method/reference pair is valid. Retain every other locked clause verbatim in meaning: private/local/noncommercial storage, exact seven sources and independent backup, no overwrite, no recurring/scheduled/unattended collection, no retry/evasion after any stop, no arbitrary URL/public API, no artwork, no redistribution/release/hosting/upload/package/commercial use, no legal-permission conclusion, and written-permission/separate-plan gates for broader use. Do not introduce any deferred Phase 6, rules, model, or GUI work.

Update `docs/external-reuse-policy.md` with exactly three allowed evidence pairs: `user-run-one-shot-powershell` with `authorizationReference` absent, and `user-authorized-agent-run-one-shot-powershell` with either `authorizationReference: quick-260825-mhh` or `authorizationReference: quick-260825-mhh-retry-1` plus its matching distinct durable consumed record; state that no other method/reference pair is valid. Show the existing personal user-run command and both fixed explicitly authorized agent-run commands. Identify each fixed agent authorization as exhausted when its pre-transport consumption record is created, not when primary or lock publication succeeds; failure, cleanup, or output deletion does not renew it, and neither record may be removed, reset, changed, or reused. Keep absolute private locators out of policy. Preserve the fixed source table, risk acknowledgment, stop/no-retry/no-evasion rules, backup requirements, storage/Git/package boundary, community clean-room limits, permission gate, and artwork prohibition. Qualify every broad actor restriction as applying to all other agent-run acquisition; remove unqualified statements that the user is the only possible actor or that agent-run acquisition is categorically blocked.

Update `01-07-PLAN.md` so its independent validation accepts exactly the same three evidence pairs, never a mix, a missing or mismatched consumed record for either exact agent pair, or an unknown method/reference. Because this quick plan owns the two fixed authorized live invocations, keep frontmatter `autonomous: true`, no `user_setup`, and the validation-only `auto` task with no human command, pause, or resume signal. Keep 01-07 focused on independently re-running `verifyPrivateSourceSet`, checking exact seven-source metadata, rulebook evidence/acknowledgment, durable matching consumption evidence for the selected agent pair, and the Git/index/package leakage gate before creating its safe summary. Preserve the fail-closed no-summary/no-Plan-08 rule. In all three files use an explicit statement that all other agent-run acquisition remains blocked pending written publisher permission and a separate plan, so the narrow exceptions cannot be read as broader permission. Validate the revised phase plan structure; commit only the tracked operating policy with the code/test retry gate while the orchestrator retains ownership of PLAN, CONTEXT, STATE, ROADMAP, and SUMMARY metadata commits.</action>
  <verify>
    <automated>node --test tests/authority/private-authority-collector.test.ts &amp;&amp; node --input-type=module -e "import fs from 'node:fs';const paths=['.planning/phases/01-rules-and-data-authority/01-CONTEXT.md','docs/external-reuse-policy.md','.planning/phases/01-rules-and-data-authority/01-07-PLAN.md'],texts=paths.map(p=&gt;fs.readFileSync(p,'utf8')),lower=texts.map(s=&gt;s.toLowerCase());const required=['quick-260825-mhh.consumed.json','quick-260825-mhh-retry-1.consumed.json','user-run-one-shot-powershell','user-authorized-agent-run-one-shot-powershell','authorizationreference','all other agent-run acquisition remains blocked','failure','cleanup','deletion','private','local','noncommercial','no redistribution','no artwork','no retry','no evasion','written publisher permission'];for(const term of required)for(let i=0;i&lt;texts.length;i++)if(!lower[i].includes(term.toLowerCase()))throw Error(paths[i]+' missing '+term);const stale=['user personally invokes','only the user may invoke','agent-run acquisition remains blocked pending','executor never invokes the production','personally run exactly','after the user reports completion','reply `collector complete`','only the user\'s fixed one-shot collector','user-run one-shot collector is the only accepted acquisition exception'];for(const term of stale)for(let i=0;i&lt;texts.length;i++)if(lower[i].includes(term))throw Error(paths[i]+' retains stale actor restriction: '+term);for(let i=0;i&lt;texts.length;i++){const s=lower[i];if(!/user-run-one-shot-powershell[^\n]{0,240}authorizationreference[^\n]{0,120}(absent|omitted|no authorization reference)/i.test(s))throw Error(paths[i]+' lacks exact user-run pair');if(!/user-authorized-agent-run-one-shot-powershell[^\n]{0,240}authorizationreference[^\n]{0,120}quick-260825-mhh/i.test(s))throw Error(paths[i]+' lacks initial agent-run pair');if(!/quick-260825-mhh-retry-1/i.test(s))throw Error(paths[i]+' lacks fresh retry pair');if(!/(no other|only these three|exactly three)[^\n]{0,160}(pair|method)/i.test(s))throw Error(paths[i]+' does not close evidence pairs');for(const match of s.matchAll(/agent-run acquisition/gi)){const start=Math.max(s.lastIndexOf('.',match.index),s.lastIndexOf('\n',match.index))+1,nextDot=s.indexOf('.',match.index),nextLine=s.indexOf('\n',match.index),ends=[nextDot,nextLine].filter(x=&gt;x&gt;=0),end=ends.length?Math.min(...ends):s.length,clause=s.slice(start,end);if(!/(all other|except|exception|quick-260825-mhh)/i.test(clause))throw Error(paths[i]+' has unqualified agent-run restriction: '+clause.trim());}}const plan=texts[2];if(!/^autonomous: true\s*$/m.test(plan)||/^user_setup:/m.test(plan)||/&lt;task type=\"checkpoint:|<task type=\"checkpoint:|&lt;resume-signal&gt;|<resume-signal>/i.test(plan))throw Error('01-07 remains non-autonomous or human-gated');" &amp;&amp; gsd-sdk query frontmatter.validate .planning/phases/01-rules-and-data-authority/01-07-PLAN.md --schema plan &amp;&amp; gsd-sdk query verify.plan-structure .planning/phases/01-rules-and-data-authority/01-07-PLAN.md</automated>
  </verify>
  <done>The context, operating policy, and Plan 01-07 encode the same two independently consumed fixed agent exceptions and retain every other private-use, acquisition, distribution, artwork, and permission boundary.</done>
</task>

<task type="auto">
  <name>Task 3: Execute once and independently verify the private source set</name>
  <read_first>
    - AGENTS.md
    - .gitignore
    - .planning/phases/01-rules-and-data-authority/01-CONTEXT.md
    - .planning/phases/01-rules-and-data-authority/01-07-PLAN.md
    - docs/external-reuse-policy.md
    - scripts/collect-private-authority.ps1
    - scripts/verify-private-authority-boundary.ts
    - src/authority/private-source-set.ts
    - tests/authority/private-authority-collector.test.ts
    - tests/authority/private-authority-boundary.test.ts
    - tests/authority/private-source-set.test.ts
  </read_first>
  <files>.local/authority/authorizations/quick-260825-mhh.consumed.json (private, Git-ignored durable original consumption record), .local/authority/authorizations/quick-260825-mhh-retry-1.consumed.json (private, Git-ignored durable fresh consumption record), .local/authority/inputs/official-2026-08-20/primary/** (private, Git-ignored), .local/authority/locks/official-2026-08-20/source-set-lock.json (private, Git-ignored), one runtime-selected absent durable backup root outside the repository (private locator; never committed), .planning/quick/260825-mhh-allow-one-explicitly-user-authorized-age/260825-mhh-SUMMARY.md</files>
  <action>First run the collector, boundary, and source-set focused suites plus `pnpm verify`. Prove the canonical primary and lock do not exist, the exact original `.local/authority/authorizations/quick-260825-mhh.consumed.json` remains present and valid without modification, and the fresh `.local/authority/authorizations/quick-260825-mhh-retry-1.consumed.json` does not exist. Require `git check-ignore -q .local/authority/inputs/official-2026-08-20/primary/probe` to succeed, and prove `git ls-files -- .local/authority` plus the staged-path scan are empty. Select one absent absolute durable backup root outside and not containing the repository; keep its absolute locator only in a PowerShell variable and the ignored lock, never in chat, a commit, command transcript copied into a summary, or another tracked file. Confirm the selected target is absent before any request.

Invoke the production collector exactly once with `pwsh -NoProfile -NonInteractive -File scripts/collect-private-authority.ps1 -BackupRoot $privateBackupRoot -AcknowledgePrivateUseRisk -UserAuthorizationReference quick-260825-mhh-retry-1`. The user's later exact instruction `Authorize new run` is the authorization for this one fresh invocation after the PDF and changelog parser fixes. Do not invoke again on any success or failure. On HTTP 401/403/429, CAPTCHA/block evidence, publisher objection, redirect/content/size/timeout error, or filesystem/verifier failure, stop with no summary and report the terminal failure without retry, fallback, alternate endpoint, manual repair, or evasion.

For this fresh continuation, every Task 3 post-run reference to the selected consumption record and lock authorization means exactly `quick-260825-mhh-retry-1` and `.local/authority/authorizations/quick-260825-mhh-retry-1.consumed.json`. The prior `quick-260825-mhh` record remains immutable historical evidence and is never a valid substitute for the fresh record.

After success, independently load the durable consumption record and ignored lock in a fresh Node process. Require the consumed record to have exactly `schemaVersion`, `revisionId`, `acquisitionMethod`, `authorizationReference`, and `consumedAt`; require values `1`, `official-2026-08-20`, `user-authorized-agent-run-one-shot-powershell`, `quick-260825-mhh`, and a valid UTC timestamp, with no root or locator fields. Require lock `schemaVersion === 1`, the same acquisition method/reference, and the exact existing structured operating acknowledgment. Before opening source bytes compare all seven lock rows against this exact relative-path contract: `rulebook/rulebook-current.pdf` -> `https://sorcerytcg.com/news/sorcery-contested-realm-december-2025-rulebook-update`, `application/pdf`, fixed `2025-12-19`; `formats/constructed-current.html` -> `https://sorcerytcg.com/constructed`, `text/html`, required null date; `codex/codex-current.html` -> `https://curiosa.io/codex`, `text/html`, required null date; `codex/faqs-current.html` -> `https://curiosa.io/faqs`, `text/html`, required null date; `codex/changelog-current.html` -> `https://curiosa.io/codex/changelog`, `text/html`, required non-null `YYYY-MM-DD` dynamic date; `updates/card-updates-2025.html` -> `https://sorcerytcg.com/news/sorcery-contested-realm-card-updates-2025`, `text/html`, fixed `2025-11-25`; `cards/cards.raw.json` -> `https://api.sorcerytcg.com/api/cards`, `application/json`, required null date. Reject missing/extra/duplicate paths, any URL/media mismatch, wrong fixed/null rule, invalid dynamic date, invalid UTC retrieval timestamp, non-lowercase SHA-256, or out-of-bound/non-positive length. Then require the existing rulebook evidence contract: source URL is the exact release page; relative path/hash/retrievedAt match its row; observed filename is `SorceryRulebook.pdf`; `privateLocatorIsNormative === false`. Call `verifyPrivateSourceSet` with the final roots and repository root, then require returned sorted entries and `sourceSetRootHash` to equal the lock exactly. This independent call, not collector checks or authorization, establishes byte-set acceptance; it establishes neither publisher authenticity nor legal permission.

Run `node scripts/verify-private-authority-boundary.ts --repository-root . --lock .local/authority/locks/official-2026-08-20/source-set-lock.json` before creating a summary. This gate must inspect committed HEAD blobs, index/staged blobs, and every current `pnpm pack --dry-run --json` path/content against all seven private source hashes, in-memory source-derived byte markers, the private absolute primary/backup/rulebook locators, forbidden `.local/authority` candidate paths, and artwork/image extensions and magic signatures. Create the quick-task summary only after the metadata, byte-set, ignore, and leakage gates pass. The summary may record the acquisition method/reference, fixed repository-relative primary label, seven relative paths plus safe metadata/hashes, source-set root hash, non-sensitive backup label/fingerprint, private-lock hash, exact operating acknowledgment, and no-legal-conclusion caveat; it must contain no consumption timestamp, private absolute locator, official source bytes/excerpts, normalized corpus, or artwork. Stage only the intended tracked code/tests/policy/planning/summary files, rerun the boundary gate so the staged summary and actual pack list/content are inspected, and only then commit as `chore(quick-260825-mhh): verify authorized private source set`. Never stage `.local/authority` or the outside backup. If either leakage pass fails, unstage/remove only the unsafe tracked candidate or summary, retain the consumed record, create no summary/commit, and do not rerun collection.</action>
  <verify>
    <automated>node --input-type=module -e "import fs from 'node:fs';import cp from 'node:child_process';import {verifyPrivateSourceSet} from './src/authority/private-source-set.ts';const keys=['acquisitionMethod','authorizationReference','consumedAt','revisionId','schemaVersion'],original=JSON.parse(fs.readFileSync('.local/authority/authorizations/quick-260825-mhh.consumed.json','utf8')),consumed=JSON.parse(fs.readFileSync('.local/authority/authorizations/quick-260825-mhh-retry-1.consumed.json','utf8'));if(JSON.stringify(Object.keys(original).sort())!==JSON.stringify(keys)||original.schemaVersion!==1||original.revisionId!=='official-2026-08-20'||original.acquisitionMethod!=='user-authorized-agent-run-one-shot-powershell'||original.authorizationReference!=='quick-260825-mhh'||JSON.stringify(Object.keys(consumed).sort())!==JSON.stringify(keys)||consumed.schemaVersion!==1||consumed.revisionId!=='official-2026-08-20'||consumed.acquisitionMethod!=='user-authorized-agent-run-one-shot-powershell'||consumed.authorizationReference!=='quick-260825-mhh-retry-1'||!/^\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}\.\d{3}Z$/.test(consumed.consumedAt))process.exit(1);const lock=JSON.parse(fs.readFileSync('.local/authority/locks/official-2026-08-20/source-set-lock.json','utf8')),ack={scope:'private-local-noncommercial',noRedistributionReleaseHostingUploadOrArtwork:true,apiTermsRobotsConflictAndPrivateUseRiskAccepted:true,establishesLegalPermission:false,stopOnBlockedStatusCaptchaOrPublisherObjection:true,retryOrEvasion:false};if(lock.schemaVersion!==1||lock.acquisitionMethod!==consumed.acquisitionMethod||lock.authorizationReference!==consumed.authorizationReference||JSON.stringify(lock.operatingAcknowledgment)!==JSON.stringify(ack))process.exit(1);const expected=new Map([['rulebook/rulebook-current.pdf',['https://sorcerytcg.com/news/sorcery-contested-realm-december-2025-rulebook-update','application/pdf','2025-12-19']],['formats/constructed-current.html',['https://sorcerytcg.com/constructed','text/html',null]],['codex/codex-current.html',['https://curiosa.io/codex','text/html',null]],['codex/faqs-current.html',['https://curiosa.io/faqs','text/html',null]],['codex/changelog-current.html',['https://curiosa.io/codex/changelog','text/html','dynamic']],['updates/card-updates-2025.html',['https://sorcerytcg.com/news/sorcery-contested-realm-card-updates-2025','text/html','2025-11-25']],['cards/cards.raw.json',['https://api.sorcerytcg.com/api/cards','application/json',null]]]);if(lock.entries.length!==expected.size||new Set(lock.entries.map(e=&gt;e.relativePath)).size!==expected.size)process.exit(1);for(const e of lock.entries){const x=expected.get(e.relativePath);if(!x||e.url!==x[0]||e.mediaType!==x[1]||(x[2]==='dynamic'?!/^\d{4}-\d{2}-\d{2}$/.test(e.effectiveDate):e.effectiveDate!==x[2])||!/^\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}\.\d{3}Z$/.test(e.retrievedAt)||!/^sha256:[0-9a-f]{64}$/.test(e.byteHash)||!Number.isSafeInteger(e.byteLength)||e.byteLength&lt;=0)process.exit(1);}const u=expected.get('rulebook/rulebook-current.pdf')[0],rule=lock.entries.find(e=&gt;e.relativePath==='rulebook/rulebook-current.pdf'),a=lock.rulebookAcquisitionEvidence;if(!a||a.sourceUrl!==u||a.relativePath!==rule.relativePath||a.byteHash!==rule.byteHash||a.retrievedAt!==rule.retrievedAt||a.observedFilename!=='SorceryRulebook.pdf'||a.privateLocatorIsNormative!==false)process.exit(1);const v=await verifyPrivateSourceSet({primaryRoot:lock.primaryRoot,backupRoot:lock.backupRoot,repositoryRoot:'.',entries:lock.entries});if(v.sourceSetRootHash!==lock.sourceSetRootHash||JSON.stringify(v.entries)!==JSON.stringify(lock.entries))process.exit(1);if(cp.execFileSync('git',['ls-files','--','.local/authority'],{encoding:'utf8'}).trim())process.exit(1);if(cp.execFileSync('git',['diff','--cached','--name-only'],{encoding:'utf8'}).split(/\r?\n/).some(x=&gt;x.startsWith('.local/authority/')))process.exit(1);cp.execFileSync('git',['check-ignore','-q','.local/authority/inputs/official-2026-08-20/primary/probe']);" &amp;&amp; node scripts/verify-private-authority-boundary.ts --repository-root . --lock .local/authority/locks/official-2026-08-20/source-set-lock.json &amp;&amp; pnpm verify</automated>
  </verify>
  <done>The one authorized agent run produced the exact private seven-source primary and independent backup, a truthful ignored lock, and an independently verified root hash while no private bytes or locators entered Git or a package.</done>
</task>

</tasks>

<threat_model>
## Trust Boundaries

| Boundary | Description |
|---|---|
| user authorization -> agent CLI | The quick plan authorizes exactly the independently consumed references `quick-260825-mhh` and `quick-260825-mhh-retry-1`; each reference is audit evidence, not cryptographic proof against a repository owner changing code. |
| official HTTP -> private staging | Status, redirects, headers, and bodies are untrusted, potentially blocked, misleading, malformed, oversized, or truncated. |
| collector -> primary/backup/lock | Partial writes, path aliases, pre-existing destinations, or a false acquisition actor could corrupt or misstate immutable evidence. |
| ignored lock -> independent verifier | Receipt metadata and byte hashes remain untrusted until `verifyPrivateSourceSet` accepts both final roots. |
| private roots -> Git/package boundary | Official corpus bytes, absolute locators, normalized derivatives, and artwork must not escape private storage. |

## STRIDE Threat Register

| Threat ID | Category | Component | Disposition | Mitigation Plan |
|---|---|---|---|---|
| T-QM-01 | Spoofing / Repudiation | authorization actor and receipt | mitigate | Atomically create the durable fixed-reference consumption record before transport on every entry path; preserve it after success/failure/cleanup/output deletion; distinct lock method/reference, concurrency/regression tests, policy traceability, and independent post-run checks; no standing configuration, wildcard reference, or reset input. |
| T-QM-02 | Tampering / DoS | remote responses | mitigate | Preserve the fixed seven-source allowlist, manual redirect validation, bounds/timeouts, strict content checks, terminal stop statuses, and no retry/evasion. |
| T-QM-03 | Tampering / Elevation | primary and backup filesystem | mitigate | Preserve absent destinations, staging, realpath/lstat/stat/open identity, no symlink/junction/hardlink, outside-repository backup, and receipt-last verification. |
| T-QM-04 | Information disclosure | private publisher content | mitigate | Anchored ignore plus a tested gate over Git HEAD/index and `pnpm pack --dry-run --json` path/content using exact private hashes, source-derived markers, private locators, restricted signatures, and artwork magic/extensions; outside-repository backup, safe summary fields, and no packaging/artwork/API publication. |
| T-QM-05 | Repudiation / legal | private-use acknowledgment | mitigate | Require the existing risk switch and exact structured acknowledgment; state that authorization and hashes establish neither authenticity nor legal permission. |
| T-QM-SC | Tampering | package installs | accept | No install occurs; existing PowerShell/.NET, Node standard library, and verifier are sufficient. |
</threat_model>

<verification>
1. Run the collector, private-boundary, and source-set focused suites plus `pnpm verify`; the collector suite proves atomic pre-transport consumption survives success, failure, cleanup, output deletion, and concurrent attempts.
2. Structurally assert the three policy/plan contracts encode exactly the three allowed method/reference pairs, 01-07 is autonomous with no `user_setup` or human checkpoint, every stale actor-only statement is removed/qualified, and all other agent acquisition remains blocked.
3. Prove primary, lock, and the fresh retry consumption record and privately selected backup destinations are initially absent while the original consumed record remains valid and unchanged; prove `.local/authority` is ignored/untracked/unstaged, then invoke production exactly once with reference `quick-260825-mhh-retry-1`; never retry.
4. In a fresh Node process, validate the durable consumption record, lock method/reference/acknowledgment, all seven exact URL/media/fixed-null-dynamic-date contracts, bounded metadata/hashes, and rulebook evidence before calling `verifyPrivateSourceSet` and matching its entries/root hash to the lock.
5. Before summary creation and again after staging it, run the tested Git HEAD/index/`pnpm pack --dry-run --json` content gate against private source hashes, source-derived markers, absolute locators, `.local/authority` candidate paths, restricted signatures, and artwork; commit only after the second pass succeeds.
</verification>

<success_criteria>
- Only authorization references `quick-260825-mhh` and `quick-260825-mhh-retry-1` can label an agent-run receipt; their distinct atomic durable records each permit one attempted invocation even after failure, cleanup, or output deletion, with no renewed or recurring permission.
- The exact seven official sources exist in the ignored primary and an independent outside-repository backup with complete metadata and byte-identical content accepted by `verifyPrivateSourceSet`.
- The existing private/local/noncommercial, no-redistribution/release/hosting/upload/package, no-artwork, stop/no-retry/no-evasion, and no-legal-conclusion boundaries remain enforceable and documented.
- The collector/verifier focused suites and full repository gate pass; Git HEAD, index, and actual dry-run package contents contain no private official byte/hash match, derivative source marker/excerpt, artwork/image payload, forbidden private path, or absolute locator.
- Code/tests, policy/planning, live verification receipt, and safe summary are committed in small descriptive slices while private artifacts remain uncommitted.
</success_criteria>

<output>
Create `.planning/quick/260825-mhh-allow-one-explicitly-user-authorized-age/260825-mhh-SUMMARY.md` only after all three tasks and every verification criterion pass. If the single live invocation or any independent metadata/byte/leakage verification fails, retain the consumed record, create no summary, do not retry collection, do not create `01-07-SUMMARY.md`, do not execute Plan 01-08, and leave DATA-01/02/03 pending.
</output>
