---
id: SR-566
title: Native probability finite default gap-analysis
type: SpecReview
analysis: gap-analysis
scope: agent-ix/filament-spec-language; QSL-2158; fsl-analyze/src/analyze/request.rs;
  fsl-analyze/tests/it/probability_from_source.rs; scoped native probability finite
  local default
review_set: subset
relationships:
- target: ix://agent-ix/filament-spec-language/FR-741
  type: references
---

## Summary

Independent Codex review of PR #3368, HEAD `81604cfb7237a6cce626bd62d699a376878932be`, direct parent `de41000de355cfb9cc462f724d18043cdb2c5105`. The two-file change supplies the finite local probability expansion default and tests preserved caller accounting. This is a scoped precursor review, not cumulative WorkScope completion or ticket acceptance.

## Verdict

**CONDITIONAL** — one medium evidence gap requires a regression that detects loss or mutation of the new default. No production correctness defect was found by inspection.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | No regression observes the new native probability candidate ceiling: B=0/1 stops at accounting before 1000000 candidates, so deleting the override or making it unlimited preserves every new assertion. | fsl-analyze/tests/it/probability_from_source.rs:1043; fsl-analyze/src/analyze/request.rs:1136 |

The test exercises real source intake and both engines, but its tiny accounting bounds dominate the local ceiling. Add a focused regression observing the configured 1000000 ceiling on the actual probability adapter path for both evidence kinds (a narrow assertion over the real constructed request can avoid materializing a million candidates); retain caller-accounting checks. The existing small-bound settlement tests do not establish this adapter default. Direct default-exhaustion coverage remains absent. The same gap is recorded by both methods; one correction resolves both.

## Coverage

Plan completion: not assessed

Only this PR and its owning requirement slices were audited. This is not an end-to-end repository assurance verdict. Scope excerpts and roles are recorded below. The cumulative clauses read as context remain unimplemented by this precursor; they are not reported as new PR defects or passed criteria.

The adapter creates one request, overrides only expansion.max_candidates, and moves it into exclusive exact/statistical branches. `simulation()` at request.rs:636-657 retains the caller meter and the shared admission at request.rs:494-496 keeps its original helper policy. Statistical model execution remains serial; workers=4 is retained in the emitted request without a parallel-execution claim. Existing local settlement paths retain configured expansion and accounting values.

Reverse inventory: three relevant behaviors (probability finite expansion default; unchanged caller accounting propagation; unchanged common preflight), zero unowned behaviors, zero confirmed source stubs, zero test stubs. The added test has real, mandatory behavioral assertions and no doubles. Source/test stub scans found no live TODO/unimplemented branch in these two files. No new public/wire surface, dependency, unsafe code, concurrency, CI change, or gate removal. No applicable AssuranceProfile was found under spec/. Existing clause_item::DEFAULT_MAX_CANDIDATES was inspected: it documents EN-1 fixture policy; no second probability policy implementation was introduced.

Reconciliation: quire matrix (quire 0.52.0, engine 0.62.2) — no run evidence read. Commands used explicit repository scope; matrix and coverage both exited 0. Changed test criterion slice: tagged 4, untagged 0, tagged-by-ignored-test 0. These are presence counts, not full criterion satisfaction. All six contextual FR documents together contain tagged 70, untagged 24 and tagged-by-ignored-test 1; those broader gaps were not audited as PR regressions.

Coverage diagnostics: repository-wide coverage emits 22835 diagnostics and 5850 untracked symbols; binding census has 13715 Rust candidates, 13144 tagged, 12623 bound. The current added test has no diagnostic or untracked entry and is bound to all four claimed criterion IDs. Widespread older grammar/collision diagnostics do not invalidate these observed binders, but no repository-wide PASS is claimed. No grep fallback or old binary was used. Raw outputs are retained alongside this review as matrix.json and coverage.json. A second matrix run from the actual worktree cwd with `quire matrix --scope . --format json` also exited 0 and produced an identical coverage_matrix; diagnostics are therefore not caused by parent-cwd/absolute-scope invocation. Its stderr is retained in matrix-worktree.stderr (8724252 bytes); first example is the duplicate Msg declaration at fsl-conformance/cases/link-unit.native:10 and :11. No independent new tool defect is established by those pre-existing global diagnostics.

Trace strength: the new test gives partial evidence for each of FR-255-AC-23 / FR-741-AC-6 (caller B preservation only), FR-192-AC-1 (one accounting stop form only), and FR-203-AC-7 (terminal accounting stop only, without independently asserting ProductRefusal). These bindings are relevant, not wrong; none proves all clauses. It does not demonstrate cumulative charging, first unpaid cumulative work, separate-run freshness, omitted-versus-explicit-default equivalence, local default exhaustion, every refusal variant, or internal refusal origin.

Semantic review: limited requirement/test/code faithfulness requested in the dispatch and performed inline by Codex; no external judge or delegated semantic campaign.

Validation evidence supplied by lead for this exact frozen head: seven-package check rc0; focused analyze integration tests rc0/7 passed; analyze library settlement map rc0/1 passed; replay library statistical settlement map rc0/1 passed (9 expected tests total). These are supplied actual one-line remote results, not locally rerun tests. No local Cargo/check/test/clippy, broad gate, or baseline was run under the dispatch restrictions; formatter/static diff cleanliness is supplied by lead. quoin 0.36.8 and installed spec-artifacts-process@v0.36.0 supplied the current schema.

No applicable AssuranceProfile was found. Rust review loaded repository guidance and the rust-style implementation, testing, and safety references plus rust-review detailed checks. Spec/plan methods are inapplicable because no spec/plan file changed.

## Reviewed scope

```yaml
scope:
- id: FR-192-AC-1
  path: spec/functional/FR-192-settle-a-statistical-result-on-the-measured-axis.md
  role: examined
  excerpt: 'Each outcome row maps exactly as the table states: an Accepted, a Rejected
    and an Undecided (`IndifferenceRegion`) `Completed` outcome; a `Stopped` naming
    `max_samples`, settling `Incomplete(LimitReached{limit, value, setting})` with
    setting `statistical.max_samples`; an `Unsupported` outcome with cause `NotMarkov`;
    a `NoTest` refusal settling `inconclusive`, `no-initial-state`; a `ConfidenceBeyondPrecision`
    refusal settling `unsupported`, `confidence-beyond-precision`; an `Internal` refusal
    settling `failed`; a `Stopped` by an expansion naming `EvaluationMeter` with exhausted
    counter `work_un'
- id: FR-203-AC-7
  path: spec/functional/FR-203-bound-an-exact-run-and-settle-its-verdict.md
  role: examined
  excerpt: The build-result map is exhaustive over `ProductRefusal` and `ClauseOutcome`
    with no `_` arm. A build whose expansion stops at `max_candidates` 2 settles `incomplete`,
    `Incomplete(LimitReached{max_candidates, 2, expansion.max_candidates})`. A build
    whose `ProductRefusal`, asserted before settling, is `Expansion(stop)` naming
    `EvaluationMeter` (a subject whose successor expansion charges the meter before
    the claim reads any predicate, under an accounting bound `B` of `work_units` its
    first expansion exceeds) settles `Incomplete(LimitReached{work_units, B, accounting.work_units})`,
    naming the ex
- id: FR-255-AC-23
  path: spec/functional/FR-255-name-the-setting-that-raises-a-reached-limit.md
  role: examined
  excerpt: Native analyze probability items with omitted candidate and accounting
    settings use local `expansion.max_candidates` 1000000 and cumulative `accounting.work_units`
    1000000 for either engine. Supplying a different accounting bound B preserves
    B. The local ceiling stops as `max_candidates` with `expansion.max_candidates`;
    the cumulative bound stops as `work_units` with `accounting.work_units`. Neither
    default is `u64::MAX`, and starting an expansion or clause does not renew cumulative
    work.
- id: FR-741-AC-6
  path: spec/functional/FR-741-request-and-report-probability-items-through-analyze.md
  role: examined
  excerpt: For each engine, a source-model probability item requiring more than 1000000
    cumulative candidate and clause work units under otherwise sufficient limits settles
    `incomplete`, `LimitReached{work_units, 1000000, accounting.work_units}` before
    the first unpaid charge, even though each expansion fits the local ceiling. Omitting
    the two limits and explicitly selecting their defaults through caller limits give
    equal item results. A caller accounting bound B is preserved and reported as B
    when reached; a local candidate denial instead reports `max_candidates`, its local
    bound and `expansion.max_cand
- id: FR-120-AC-35
  path: spec/functional/FR-120-simulate-a-checked-package-s-state-family.md
  role: context_only
  excerpt: 'In one EN-4 or EN-5 model-subject run with local `max_candidates` above
    each expansion''s charge, two expansions that each fit `accounting.work_units
    = B` alone but whose combined candidate and clause work exceeds B stop at the
    first unpaid cumulative charge with `EvaluationMeter { counter: work_units }`;
    starting the second expansion does not reset the allowance. The same holds for
    two clause evaluations that fit separately but exceed B together, and for an expansion
    followed by a claim predicate. A separate run starts at zero; other accounting
    counters retain their existing scopes.'
- id: FR-192-AC-8
  path: spec/functional/FR-192-settle-a-statistical-result-on-the-measured-axis.md
  role: context_only
  excerpt: FR-120-AC-35's cumulative EN-4 work stop settles `Incomplete(LimitReached{work_units,
    B, accounting.work_units})`, equal to FR-203's settlement of the same typed stop
    and configured bound. A local `MaxCandidates` stop retains `max_candidates`, its
    local bound and `expansion.max_candidates`; cancellation retains its existing
    cancelled cause. No stop carries truth, a measured decision, or a replacement
    bare `meter` setting.
- id: FR-203-AC-13
  path: spec/functional/FR-203-bound-an-exact-run-and-settle-its-verdict.md
  role: context_only
  excerpt: In a model-subject build, two expansions, two claim reads, and an expansion
    followed by a claim read each fit the caller's `accounting.work_units = B` individually
    but exceed B cumulatively. Each run stops at its first unpaid cumulative charge
    and settles `Incomplete(LimitReached{work_units, B, accounting.work_units})` with
    no truth. A separate run has a fresh allowance. A local candidate denial continues
    to settle `Incomplete(LimitReached{max_candidates, C, expansion.max_candidates})`
    at its configured C; cancellation retains `Incomplete(Cancelled{source})`.
- id: FR-741
  path: spec/functional/FR-741-request-and-report-probability-items-through-analyze.md
  role: examined
  excerpt: "For either engine's model subject, the native adapter SHALL use the\n\
    \  finite local `expansion.max_candidates` default of 1,000,000 and the\n  cumulative\
    \ `accounting.work_units` default of 1,000,000 under"
```

## Publication

Linear reads and writes were prohibited by the user hold; no comments posted, dedup/readback not performed, has-review known present from dispatch only. The marked method comment is a local draft pending lead publication once the hold and coordination permit. Review artifacts remain in scratch; the lead owns final repository custody and merge.

## Dispositions

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | Round 1: the new admitted-Link unit test observes both real production request payloads and asserts the finite 1000000 local candidate ceiling; production directly consumes that same builder's variants. |

## Disposition pass 1

**Current source verdict: PASS. Runtime gate for the rebased head: pending.** All original findings have latest outcome fixed; no new grounded defect found. This supersedes the historical review verdict for source assessment only. The initial Findings section remains unchanged.

Reviewed frozen HEAD `93ce07b20d9f65e06dd93975a33e4929e854371b`, base `8ba8b1bdd5fc4df61caf7ddd9ac1422203c30278`. Before appending this disposition, both scratch SRs were byte-identical to the copies on the frozen branch. The source fix is within the approved finite-local-default scope.

`ProbabilityWork` and `probability_work` at request.rs:694-751 are private, with inline owned request variants, no added Box or public/wire/test-only inspection surface. The builder is the sole probability-request construction policy: one simulation request receives the 1000000 override and is moved into exactly the selected evidence branch. Every previous payload member, claim lookup, sampler clone, method/seed selection, optional witness default, symmetry and timed sampling is preserved. The real work arm at request.rs:1244-1258 calls this builder and directly passes each resulting payload into the existing engine item constructor; it does not reconstruct or override it afterward. No unused helper or mocked engine provides the evidence.

The new test at request.rs:1606-1723 uses the real spine parse/select/check path and existing Link model/snapshot fixtures, checks actual ModelSystem admission, and tests both evidence kinds through the same production builder. A removed override or a value other than 1000000 fails the explicit payload bound assertion. A missing branch fails expect; a swapped request kind fails the evidence/variant match. Claim occurrence and retained meter equality are also asserted. The unchanged common simulation helper remains unlimited for preflight, which the test explicitly checks before admission. The original B=0/1 integration test is preserved. No test was executed locally and no mutation run is claimed: this falsification assessment follows the mandatory assertions and direct source wiring.

New unit trace tags are relevant partial bindings for FR-741-AC-6 and FR-255-AC-23. They now establish the local configured default, alongside the integration test's caller-B preservation. They still do not establish cumulative charging, first unpaid cumulative admission, omitted/explicit-default equivalence, complete AC satisfaction, or direct million-candidate exhaustion. Those broader claims remain outside this precursor's acceptance. No claim of statistical parallel execution is added; requested workers remain untouched.

Runtime provenance: the lead supplied a seven-package check rc0, the new unit 1 PASS and seven focused integration tests 7 PASS for the pre-rebase final fix head identified as `1ed3` in the dispatch. The original two library settlement-map passes remain attributed to 81604cfb7237a6cce626bd62d699a376878932be. These are 10 distinct tests across those heads; none is relabeled as execution of 93ce07b20d9f65e06dd93975a33e4929e854371b. The required same commands for the rebased head are pending with the lead. No manifest or lock diff exists versus the stated new base. Static disposition can pass independently; merge runtime readiness requires the actual new-head result.

Publication remains local-only under the user Linear hold and root coordination. No tracker reads, dedup, posting, labels or readback were performed. A round-1 marked disposition draft accompanies each SR; the lead must publish original findings and dispositions once each with dedup and readback when permitted.

## Publication read-back

The user hold was released by root coordination for this publication batch. Initial review comment `73dd267d-d36d-4fcc-958e-a07ee36973bc` and round-1 disposition comment `57dfbb1b-edc7-4ef2-886c-34bdf064ea95` were each posted once to QSL-2158 after one latest-100 comment dedup read. One final batch readback found both comments; exact body, YAML parsing, finding-ID parity, initial scope equality, and fixed after_excerpt checks passed. No labels, issue state or GitHub findings were changed. Known has-review was not queried.

Current runtime status supplied by the lead: the actual 93ce07b20d9f65e06dd93975a33e4929e854371b new unit passed (1 PASS); compile and focused integration execution are blocked by outside-PR main initializer fields pending an architecture repair. Source verdict remains PASS; final-head gates and merge are owned by the lead and are not reported complete here.
