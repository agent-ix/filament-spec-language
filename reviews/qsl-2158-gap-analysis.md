---
id: SR-202611210022442313144108509
title: "QSL-2158 PR 3285 FR-1043 finite-horizon measure profile gap analysis"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/filament-spec-language@986e56a8e157cc8cf694ff4daed7cd2dcdc144e0 (merge-base f3aa681d7ed753a2cdd998bc24842ef38dda3c86); criteria FR-1043-AC-1..AC-11, FR-202-AC-27..AC-30, FR-203-AC-10..AC-11, FR-1042-AC-12..AC-14 (added in this PR); computed quire matrix; tests fsl-analyze/tests/it/exact_probabilistic_item/measure_profile.rs, tools/fsl-probability-check/tests/it/certificate_kinds.rs, fsl-eval/src/probabilistic.rs tests; production diff of PR 3285"
review_set: subset
relationships:
  - {target: "ix://agent-ix/filament-spec-language/FR-1043", type: references}
  - {target: "ix://agent-ix/filament-spec-language/FR-1042", type: references}
  - {target: "ix://agent-ix/filament-spec-language/FR-202", type: references}
  - {target: "ix://agent-ix/filament-spec-language/FR-203", type: references}
---

## Summary

Ticket: QSL-2158, PR 3285. This is a planless gap analysis scoped to the
criteria this PR implements or adds. It does not cover the global backlog.
The matrix was computed with `quire matrix --scope . --format json`
(quire 0.52.0, engine 0.62.2) at frozen head `986e56a8e1`.

The test audit was required by the dispatch brief, so a semantic review was
performed. It judged assertion strength, adverse cases, independent oracles
and real physical replay. The code-review artifact
SR-202611210022442313144108508 records the test-strength findings that belong
with code. This artifact records traceability and AC-oracle gaps.

Matrix result: FR-1043-AC-1 to AC-5 and AC-7 to AC-11, FR-202-AC-27 to AC-30,
and FR-203-AC-10 and AC-11 are tagged by real, non-ignored tests.
FR-1043-AC-6 is untagged. FR-1042-AC-12 to AC-14, which this PR adds, are
untagged. AC-12 is inside this PR's narrow upper-minimum and lower-maximum
scope.

## Verdict

**FAIL**. The verdict rule makes any untagged in-scope criterion a FAIL
(FR-1043-AC-6 and FR-1042-AC-12). There are no high findings. Every finding
is fixable within this PR.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | FR-1043-AC-6 is untagged in the computed matrix. `a_finite_horizon_quantile_certificate_is_accepted` asserts it (shifted `11/135`, separate-process acceptance, level 0.5 refuted) but carries only `TC-2065`/`FR-201-AC-51`. | tools/fsl-probability-check/tests/it/certificate_kinds.rs:120 |
| FND-002 | medium | FR-1042-AC-12, added by this PR and implemented by the narrow fraction-base `bounds.rs` path, has no tagged test. Its scenario is not exercised: an initial terminal state, a false predicate, window 2, an all-zero `upper` minimum and `lower` maximum with rederived ranks, accepted with no policy rows. | spec/functional/FR-1042-certify-value-bounds-with-lower-and-upper-certificates.md:238 |
| FND-003 | medium | FR-1043-AC-8's stated every-scheduler extrema (minimum `11/48`, maximum `5/12`) and the checker-selected policies (`slow` for the upper on the minimum, `fast` for the lower on the maximum) are never asserted. The tagged tests check only `Refuted`, accept hand-edited bound values `1/4`/`2/5`, and check `fast` only in the FR-202-AC-30 slack fixture. | fsl-analyze/tests/it/exact_probabilistic_item/measure_profile.rs:1076 |
| FND-004 | low | FR-1042-AC-13 and AC-14, added by this PR, have no tagged test, while TC-3151's amended scope claims "FR-1042-AC-7 to FR-1042-AC-14". Under the brief they belong to the general FR-1042 PR, but no `Remaining work` record marks them as not delivered here. | spec/test-cases/TC-3151-certify-value-bounds.md:21 |

## Coverage

- Plan completion: not assessed
- Matrix (scoped): 20 of 24 examined criteria are tagged. The untagged ones
  are FR-1043-AC-6, FR-1042-AC-12, FR-1042-AC-13 and FR-1042-AC-14.
  FR-1042-AC-1 to AC-11 are also untagged, but they are general FR-1042
  backlog outside this PR and are not counted. No criterion is tagged only by
  an ignored test.
- Reverse gap: the new production code has owning requirements: FR-1043,
  FR-196 (key and control ownership), FR-201, FR-202, FR-203 and FR-1042. The
  speculative `ProbabilityCertificatePart::Post`/`Reset` variants are recorded
  in the code review (FND-018).
- Stubs and inflation: no source stubs. The `#[ignore]` stubs that remain in
  `certificate_kinds.rs` (FR-1041, general FR-1042) predate this PR and are
  keyed to QSL-2158. Two `#[ignore]` markers were removed for delivered
  behavior (P95, TC-2065 quantile), and no test was deleted.
- Semantic review: AC-11 replays the actual `drop(0)` step and its zero
  reward. It verifies `c` is really absent, and rejects forged locus,
  position, control, sums and cause. AC-10 validates the FSpec `Result`
  schema. AC-4 uses the emitted package and alters one numerator value. The
  defined `Always` unsupported dispositions are preserved by exact regression
  tests 5 to 7 in the parent's focused list.
- Runtime: the 33 exact focused tests are pending on draco. This analysis
  does not claim them.

## Disposition pass 1

Latest reviewed source: `1bef7ca58bebe9972e3fe74189d578e060034722`; actual merge-base `529282e9dc28cc6be8b382ae634636e3b4fe4a14`. One independent Codex reviewer (GPT-6 family as supplied by harness; no more specific model version exposed), user-directed replacement of initial reviewer. Initial findings/history above are retained verbatim.

**Latest verdict: CONDITIONAL.** GA001/003/004 fixed; GA002 deferred only for architecture-allocated generic proof scope. All delivered criterion rows are tagged; runtime is compile-blocked with zero tests executed. Code review FND-019/020 separately block merge.

## Dispositions

Round 1. `fixed` records inspected source resolution; execution evidence remains separately qualified below.

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | Standalone TC2065 checker test now carries FR1043AC6; matrix binds the real nonignored test. |
| FND-002 | deferred | Architecture narrows this PR to mean-refutation. Its terminal false fraction Upper(Minimum)=0 is accepted with inferred TerminalStutter; Lower(Maximum)=0 reaches exactly ThresholdMissed at <=0 after local checks. Generic standalone Lower proof acceptance and full FR1042AC12 remain on QSL2158, comment d59dedf8-adff-4b9c-a5fc-3509ecb5b930. AC12 remains untagged deliberately; no invalid negative threshold/public shim or full-coverage claim. |
| FND-003 | fixed | Actual exact initial extrema11/48 and5/12 plus selected slow/fast identities are asserted; bound tests reuse those policies and independent replay parity. |
| FND-004 | fixed | Named narrow hint tests bind AC13/14; private Remaining work comment d59dedf8 explicitly retains general FR1042 AC12/13/14 obligations and ticket stays In Progress. Fixed means truthful allocation, not full generic coverage. |

### Round 1 source excerpts

FND-001 after_excerpt:

```rust
#[trace("TC-2065", "FR-201-AC-51", "FR-1043-AC-6")]
#[test]
fn a_finite_horizon_quantile_certificate_is_accepted() {
```

FND-003 after_excerpt:

```rust
    };
    assert_ne!(fast.identity.operation, slow.identity.operation);
    for (name, expected, action) in [
        ("MinNo", ratio(11, 48), &slow.identity),
        ("MaxNo", ratio(5, 12), &fast.identity),
    ] {
        let record = settle(&world, name, ModelCheckLimits::default());
        assert_eq!(record.terminal().unwrap().value(), &TerminalValue::Refuted);
        let token = record.probabilistic_counterexample.as_ref().unwrap();
        let witness = token.evidence();
        let document: Value =
```

FND-004 after_excerpt:

```rust
#[trace(
    "FR-1043-AC-8",
    "FR-1043-AC-9",
    "FR-202-AC-28",
    "FR-1042-AC-13",
    "FR-1042-AC-14"
)]
#[test]
fn mean_bound_policy_hints_constrain_search_and_absence_reconstructs_internal_choices() {
```

### Round 1 scope and verification

No cargo, local tests, clippy, deny, baseline, Kani or sweep was run. Parent reports exact1bef7ca58bebe9972e3fe74189d578e060034722 first29-profile command COMPILE101/ZERO tests: E0425 document.rs229 missing ExactProbLimits import, plus unused MeasureError at document/measure.rs7. Raw /private/tmp/qsl-3285-1bef-1.log. Commands2–14 held/not run; no42-test acceptance. Original986e33 never ran/HOLD and is superseded. Upstream monitor_root23 is reader-only evidence, not acceptance. Remote10970 admission failure is only prior post-rebase compile/probe evidence.

The actual current PR diff preserves main exports, boxed task transitions, refinement guards/time invariants, protocol replay, call_failure propagation and unavailability removals. IR20a4dfbf/core-data424ba49 remain pinned; only authorized FSpecd6d066 replaces its prior pin. No workflow lane changes; git diff --check passes. No applicable AssuranceProfile was found. No new source edits, test deletion, vendoring or speculative owner API retained. FractionValue replays the actual post-step state at h+1 with PRE undecided control/accumulator before outside-window predicate access.

Plan completion: not assessed

Reconciliation: quire matrix (quire0.52.0 engine0.62.2), static only, no run evidence read. Scoped21 rows:20 tagged,1 untagged (FR1042AC12 explicitly allocated later); FR1042AC13/14 tags establish narrow hint behavior only. All owning delivered rows have real nonignored binders. Coverage command also exits0; engine diagnostics contain documentation ids outside markers, cfg/native symbol collisions and unrelated unminted headings. Compared saved initial matrix.err before classification; documentation warnings do not erase inspected real test binders, and no affected owning test binder or new dangling owning tag was found. This is no global traceability-cleanliness claim. No source/test stubs or new unowned production behavior found: control/key FR196, finite arithmetic FR197/1043, checking FR201/1042, witness FR202, settlement FR203, reward admission FR185/187. Semantic assertion audit is the authorized disposition recheck, not an optional new analysis. New code acceptance defect is owned in SR508 FND019 rather than duplicated here.

Scope entries below record criteria examined against implementation; AC12 is context_only at full generic scope.

```yaml
scope:
- id: FR-1042-AC-12
  path: spec/functional/FR-1042-certify-value-bounds-with-lower-and-upper-certificates.md
  role: context_only
  excerpt: For a bounded fraction mean over an initial terminal state with a false
    fraction predicate and window 2, the base's only continuation is terminal stutter
    and its value is 0. An `upper` minimum document with every value 0 and no policy
    rows, and a `lower` maximum document with every value 0, the rederived remaining-horizon
    ranks and no policy rows, are accepted. The checker retains the existing stutter
    action at each undecided key; no fabricated operation identity or new wire form
    is required.
- id: FR-1042-AC-13
  path: spec/functional/FR-1042-certify-value-bounds-with-lower-and-upper-certificates.md
  role: examined
  excerpt: At an intermediate state with two enabled post-state choices `a` and `b`,
    their successors reach event success with probability `1/4` and `3/4`, respectively.
    With correct successor values and ranks, a minimum `upper` initial bound `1/2`
    with no hint is accepted selecting `a`, and a maximum `lower` bound `1/2` with
    no hint is accepted selecting `b`. Initial bounds `1/8` for the upper and `7/8`
    for the lower reject `InequalityFails` there because no action satisfies. Post-state
    selection needs no fabricated TransitionIdentity row.
- id: FR-1042-AC-14
  path: spec/functional/FR-1042-certify-value-bounds-with-lower-and-upper-certificates.md
  role: examined
  excerpt: In AC-3, minimum `upper` `s` `1/4` with hint `go` rejects `InequalityFails`
    although `wait` satisfies, and the same document without that hint accepts selecting
    `wait`. A nonexistent hint rejects `InequalityFails` even when an enabled action
    satisfies. Maximum `lower` `s` 1 with hint `wait` rejects `RankBroken`; removing
    the hint does not accept it, since no action satisfies both inequality and progress.
    When several actions satisfy every condition, an omitted hint selects the first
    in canonical full-product-action order.
- id: FR-1043-AC-1
  path: spec/functional/FR-1043-certify-finite-horizon-measure-verdicts.md
  role: examined
  excerpt: '`Retry`''s undelivered state `u` has `try`, delivering with `1/2` and
    otherwise staying; delivered is terminal. Under its workload, `quantile 0.5 of
    (steps from holds(true) until holds(delivered) within 4) <= 2` settles `proved`,
    `ExactValue{3/4}`, with a `conditional` `ratio` certificate whose numerator (payoff
    `satisfied`, `comparison` `"le"`, `threshold` 2) has values `3/4`, `1/2`, 0, 0
    at `u` with active amounts 0, 1, 2 and above the bound and ranks 4, 3, 2, 1, and
    whose denominator (payoff `activated`) is 1 there; `fsl-probability-check` accepts
    it, and rejects it `RankBroken` with the ran'
- id: FR-1043-AC-2
  path: spec/functional/FR-1043-certify-finite-horizon-measure-verdicts.md
  role: examined
  excerpt: With `try` split into `fast` (`1/2`) and `slow` (`1/4`), under every scheduler
    `quantile 0.4 of (…) <= 2` settles `proved` with an `exact` `shifted` document
    (`level` `2/5`) whose values are `3/80`, `−3/20`, `−2/5`, `−2/5`; `quantile 0.5
    of (…) <= 2` settles `refuted` (minimum shifted value `−1/16`, `Pr(M <= 2) = 7/16`
    under `slow`); the level-0.4 document with `3/80` replaced by `fast`'s `9/40`
    is rejected `InequalityFails` at the initial key.
- id: FR-1043-AC-3
  path: spec/functional/FR-1043-certify-finite-horizon-measure-verdicts.md
  role: examined
  excerpt: 'Over AC-2''s model under every scheduler, `quantile 0.6 of (…) >= 2` (claim
    `Pr(M < 2 | act) < 3/5`) settles `proved`: the `shifted` document (`comparison`
    `"lt"`) has maximum initial value `−1/10` (through `fast`) and `−3/5` at every
    other undecided state; `quantile 0.5 of (…) >= 2` settles `refuted`, its maximum
    0 attained by `fast` with an activated state reachable.'
- id: FR-1043-AC-4
  path: spec/functional/FR-1043-certify-finite-horizon-measure-verdicts.md
  role: examined
  excerpt: '`P95` over FR-185-AC-1''s `Service` under `Steady` at `5 ms` (FR-741-AC-2,
    FR-203-AC-1) settles `proved`, `ExactValue{24233/25000}`, with a `conditional`
    `ratio` certificate whose parts are documents of this profile (`kind` `"accumulate"`,
    `reward` the checked `duration`, `threshold` 5 in its unit) and whose denominator
    is 1 at the initial key; `fsl-probability-check` accepts it from the emitted package
    and rejects it with one numerator value altered. At `2 ms` the item settles `refuted`
    with FR-202-AC-1''s one-path set.'
- id: FR-1043-AC-5
  path: spec/functional/FR-1043-certify-finite-horizon-measure-verdicts.md
  role: examined
  excerpt: Under `Retry`'s workload, `mean of fraction holds(delivered) over [0, 2]
    >= rational(2,5)` settles `proved`, `ExactValue{5/12}`, with a `fraction` document;
    at `rational(1,2)` it settles `refuted` with FSpec FR-413 subsystem evidence containing
    an independently accepted `exact` `fraction` certificate of `5/12 < 1/2`, checked
    for refutation rather than the proof threshold. The comparison event `probability
    >= 0.7 [M <= 2]` over AC-1's measure settles `proved`, `ExactValue{3/4}`, with
    an `event` document.
- id: FR-1043-AC-6
  path: spec/functional/FR-1043-certify-finite-horizon-measure-verdicts.md
  role: examined
  excerpt: FR-201-AC-51's finite-horizon quantile item over the cyclic `link` model
    under every scheduler (`Pr(M <= 3) = 13/27` for `steps from holds(true) until
    holds(m.delivered) within 4`), written at level 0.4, settles `proved` with shifted
    initial value `11/135` and a document `fsl-probability-check` accepts in a separate
    process; written at level 0.5 it settles `refuted`.
- id: FR-1043-AC-7
  path: spec/functional/FR-1043-certify-finite-horizon-measure-verdicts.md
  role: examined
  excerpt: AC-1's numerator with `window`, `threshold`, `comparison`, `reward`, `unit`,
    `level` or `payoff` changed refuses objective mismatch; a key whose accumulator
    amount or phase is altered is not an alias and fails coverage; a `shifted` value
    below `−level` is rejected `Malformed` at `values`. Pre-cancellation and exhaustion
    of each caller limit during expansion, payoff evaluation or exact sums settle
    `incomplete` with the actual limit and no truth value.
- id: FR-1043-AC-8
  path: spec/functional/FR-1043-certify-finite-horizon-measure-verdicts.md
  role: examined
  excerpt: With AC-1's `try` split into `fast` (`1/2`) and `slow` (`1/4`), `mean of
    fraction holds(delivered) over [0, 2]` has minimum `11/48` and maximum `5/12`.
    Under every scheduler, `>= 1/2` is refuted by an accepted exact minimum or FR-1042
    `upper` on the minimum with initial bound `1/4` and policy `slow`; `<= 1/3` is
    refuted by an accepted exact maximum or `lower` on the maximum with initial bound
    `2/5` and policy `fast`. Both reuse subsystem evidence and are independently accepted
    by FR-202's checked-subject mean-certificate replay entry.
- id: FR-1043-AC-9
  path: spec/functional/FR-1043-certify-finite-horizon-measure-verdicts.md
  role: examined
  excerpt: AC-5's exact workload mean `5/12` at `>= 5/12` and `<= 5/12`, a valid workload
    `upper` bound `1/2` at `>= 1/2`, and a valid `lower` bound `2/5` at `<= 2/5` each
    reject `ThresholdMissed` at the initial key and settle `inconclusive`, `CertificateRejected`.
    In AC-8, a valid `lower` minimum document with initial bound `1/5` at `>= 1/2`
    and a valid `upper` maximum document with bound `1/2` at `<= 1/3` each reject
    `Malformed` at `kind` with the same settlement. Identity/base mismatch retains
    its typed refusal; an invalid value rejects `InequalityFails` with `CertificateRejected`;
    a disagreeing witne
- id: FR-1043-AC-10
  path: spec/functional/FR-1043-certify-finite-horizon-measure-verdicts.md
  role: examined
  excerpt: FR-203-AC-10's no-activation quantile under a resolved workload and under
    every scheduler, with and without an admitted fairness set, settles certified
    `Exhaustive` with an independently accepted conditional `vacuous` certificate,
    no `ExactValue`, shifted-value proof or numeric entry. Offering an all-zero `exact`
    `shifted` document under every scheduler, or a workload `ratio` with denominator
    0, for that no-activation claim rejects `InequalityFails` at the initial complete
    product key and settles `inconclusive`, `CertificateRejected{rule, at}`, without
    numeric acceptance or `ExactValue`. A var
- id: FR-1043-AC-11
  path: spec/functional/FR-1043-certify-finite-horizon-measure-verdicts.md
  role: examined
  excerpt: '`Coin` initially has bound object `c` with `c.v = 0`; its only operation
    `drop(b)` draws `b` uniformly from `{0,1}`, deleting `c` when `b = 0`, and declares
    reward `R = 0` for both supported draws. Under its resolved workload, `mean of
    fraction holds(c.v = 0) over [0,0] weighted by R >= 1/2` reaches undefined `division-by-zero`
    on the actual `drop(0)` step of probability `1/2`: the final tentative weighted
    sums are `(0,0)`, and the `FractionValue` location is position 1 with the actual
    post-state lacking `c`, pre-step undecided control `remaining = 1` and pre-step
    sums `(0,0)`. Native replay r'
- id: FR-202-AC-27
  path: spec/functional/FR-202-replay-a-probabilistic-witness.md
  role: examined
  excerpt: Through the checked-subject mean-certificate replay entry, FR-1043-AC-5's
    workload mean `5/12` refutes `>= 1/2` using subsystem evidence containing the
    independently accepted `exact` `fraction` certificate; the same document checks
    in its refutation role despite missing the proof threshold. An unchecked document,
    an unreplayed engine value or a path set yields no accepted mean-refutation evidence.
- id: FR-202-AC-28
  path: spec/functional/FR-202-replay-a-probabilistic-witness.md
  role: examined
  excerpt: FR-1043-AC-8's minimum `11/48` with an accepted `upper` bound `1/4` refutes
    every-scheduler `>= 1/2`; its maximum `5/12` with an accepted `lower` bound `2/5`
    refutes every-scheduler `<= 1/3`. A valid `lower` minimum document with initial
    bound `1/5` at `>= 1/2`, and a valid `upper` maximum document with initial bound
    `1/2` at `<= 1/3`, each reject `Malformed` at `kind` and settle `inconclusive`,
    `CertificateRejected`; their conditions hold but their refutation roles are wrong.
- id: FR-202-AC-29
  path: spec/functional/FR-202-replay-a-probabilistic-witness.md
  role: examined
  excerpt: AC-27's exact `5/12` at `>= 5/12` or `<= 5/12`, and a valid workload `upper`
    bound `1/2` at `>= 1/2`, each reject `ThresholdMissed` at the initial key and
    settle `inconclusive`, `CertificateRejected`. Changing identity or base payoff/optimization
    retains the typed refusal; invalid certificate values settle `inconclusive`, `CertificateRejected{InequalityFails,
    at}`; a disagreeing witness policy settles `inconclusive`, `ReplayParity`; cancellation
    or a caller limit preserves its typed incomplete result without accepted evidence.
- id: FR-202-AC-30
  path: spec/functional/FR-202-replay-a-probabilistic-witness.md
  role: examined
  excerpt: 'A mean minimum refutation with a slack `upper < θ` for which both the
    optimizer''s attaining action and a different canonical-first action satisfy every
    FR-1042 condition emits a witness matching the independently checked policy: with
    no hint it uses the canonical-first satisfying full-product action; with a representable
    valid hint it uses that hinted action. Both replay as refuted. An old optimizer
    witness disagreeing with the checked selection settles `inconclusive`, `ReplayParity`;
    implicit stutter or post-state selection uses the same rule without a fabricated
    policy identity.'
- id: FR-203-AC-10
  path: spec/functional/FR-203-bound-an-exact-run-and-settle-its-verdict.md
  role: examined
  excerpt: 'The untimed claim `quantile 0.5 of (steps from holds(false) until holds(true)
    within 4) <= 2`, checked separately under a resolved workload and under every
    scheduler (with and without an admitted fairness set) has no activating state
    in the complete reachable product. Native checking independently accepts its `Conditional`
    `Vacuous` certificate and settlement returns `proved`, `closed-scope`, `Certified`,
    `Exhaustive`, category success, carrying that certificate with empty `values`
    and `conditional` exactly `{"mode": "vacuous"}`. The emitted result passes FR-331''s
    contract with basis `{"type":'
- id: FR-203-AC-11
  path: spec/functional/FR-203-bound-an-exact-run-and-settle-its-verdict.md
  role: examined
  excerpt: Changing AC-10's activation predicate to `holds(true)` makes the offered
    vacuous certificate reject `InequalityFails` and settle `inconclusive`, `CertificateRejected`,
    without vacuity acceptance. Offering an otherwise well-shaped `exact`, `ratio`
    or `optimum` certificate on `exhaustive`, or generic `evidence` with or without
    its permitted vacuous certificate, refuses the FR-331 result contract. An `exact-value`
    basis missing its value, using scalar zero or denominator zero refuses; AC-1's
    ordinary exact result with a valid `Rational` value remains admitted. An engine-only
    assertion, partial ex
- id: FR-203-AC-12
  path: spec/functional/FR-203-bound-an-exact-run-and-settle-its-verdict.md
  role: examined
  excerpt: '`Retry` has `Msg.delivered`, terminal when delivered, and the resolved
    `Steady` workload whose `fast` step delivers with probability `1/2` and whose
    `slow` weight is 0. With request initial states ordered as index 0 `{delivered:
    true}` and index 1 `{delivered: false}`, the same admitted binding `m`, and `quantile
    0.5 of (steps from holds(not m.delivered) until holds(m.delivered) within 4) <=
    2`, the independent conditional certificate check accounts for both pairs: index
    0 has unreachable activation with numerator and denominator 0, while index 1 activates
    immediately with numerator `3/4`, den'
```

## Disposition pass 2

Final normal repair round; same independent Codex reviewer. Frozen source `10564ea0ef37a7f47113ad44e3ec13ff3cac4c43`, actual merge-base `645bef0e0e810a5c551b7894ee98d7c6288b7467`. Earlier findings and dispositions above are retained verbatim. Existing closed outcomes remain: CR001/003–018 fixed, CR002 rejected on bounded admission evidence; GA001/003/004 fixed, generic GA002 deferred per architecture/QSL2158 commentd59dedf8. This round does not waive any new defect.

**Latest verdict: CONDITIONAL.** No new criterion/tag, reverse-ownership or stub gap was found in this round. Prior generic GA002 remains deferred outside the allocated mean-refutation scope. Final runtime fails compilation before any test; SR508 CR021/022 separately block merge. No new gap finding or disposition row is needed.

### Round 2 implementation and evidence

Inspected the full831-line inverse and every changed provenance/anchor consumer, control decoder, reservation bridge, pending-row validator and regression. Model restoration validates declared populations, member types, field values, references, universes, extents and object closure; rejects nonempty unsupported auxiliary maps; re-encodes through the existing owner and requires byte equality. Structural errors map to existing Malformed member(values); retained caller denials and cancellation map through the actual admission scope. The resource defect CR021 qualifies the claim that all validation is bounded. No default arithmetic meter was introduced.

RestoredFromKey has no anchor or incoming effect; actual successors keep absent anchor and record genuine new effects. Required-anchor consumers refuse absence; sampler position checks before cache lookup. Declared pre-state construction remains separate. Existing initial/witness parity comparisons still compare against actual admitted anchored initials. Named pending rows rebuild enabled actions and positive draws, match owner pending bytes and require multiple actual posts; no invented history, external pending parser or wire format.

The new profile test exercises genuinely detached terminal, undecided and pending rows, retains an actual PostState selection, and rejects forged universe, horizon/control and draw. The owner unit exercises actual noninitial restoration, successor transition/probability/key equality, absent provenance after real effects, sampler/anchor refusal, typed integer-range error, storage denial and precancellation. These are real assertions, not proof that the final batch has run. No new source/test stub or unsupported speculative API was found.

Parent reports final exact10564 first30-profile command COMPILE101/ZERO tests, two E0499 at restore.rs656/657: self.push holds mutable self while self.reference(source) and self.reference(target) also borrow self. Commands2–15 held/not run. Raw /private/tmp/qsl-3285-1056-1.log. No44-test acceptance. Owner-only400617 exact restoration test passed1/0/0/272filtered, log /private/tmp/qsl-3285-4006-inverse.log. Later buffer-admission/import/main-merge changes mean that owner result is not final profile acceptance. Old1bef first29-profile command COMPILE101/ZERO tests and commands2–14 held; never green. No local cargo, tests, clippy, deny, broad baseline or sweep was run.

Read-only git diff --check passes. Actual final PR scope is645bef...HEAD; unrelated main merges are excluded. Main certificate/hyper/refinement/protocol APIs are preserved; authorized FSpecd6d066 and IR20a4dfbf/core-data424 pins remain. Accepted normative SR507 is unchanged and not re-reviewed. No applicable AssuranceProfile.

Plan completion: not assessed

Reconciliation: quire matrix0.52.0(engine0.62.2), static only; exit0. Twenty of21 scoped criterion rows are tagged, with only generic FR1042AC12 deliberately untagged. No delivered owning row is ignored-only. New owner and detached profile tests bind actual FR1043AC9/FR202AC28. quire coverage also exits0; no new dangling tags in the changed owner/control/profile tests. The10 FSpec-prefixed untracked tags in the now-inspected probabilistic owner test file are byte-identical to D1 evidence; they are not new claims from this fix. Documentation-id-outside-marker and global cfg/native-collision diagnostics remain visible; inspected real binders are counted, documentation alone is not. No global traceability-cleanliness claim. New inverse is owned by FR196/FR1043 with FR120 provenance context; checker/error/settlement remain FR201/202/203. Resource defect is recorded once as SR508 CR021 rather than duplicated as a gap finding.

### Round 2 examined scope

```yaml
scope:
- id: FR-1042-AC-12
  path: spec/functional/FR-1042-certify-value-bounds-with-lower-and-upper-certificates.md
  role: context_only
  excerpt: For a bounded fraction mean over an initial terminal state with a false fraction predicate and window
    2, the base's only continuation is terminal stutter and its value is 0. An `upper` minimum document with every
    value 0 and no policy rows, and a `lower` maximum document with every value 0, the rederived remaining-horizon
    ranks and no policy rows, are accepted. The checker retains the existing stutter action at each undecided key;
    no fabricated operation identity or new wire form is required.
- id: FR-1042-AC-13
  path: spec/functional/FR-1042-certify-value-bounds-with-lower-and-upper-certificates.md
  role: examined
  excerpt: At an intermediate state with two enabled post-state choices `a` and `b`, their successors reach event
    success with probability `1/4` and `3/4`, respectively. With correct successor values and ranks, a minimum `upper`
    initial bound `1/2` with no hint is accepted selecting `a`, and a maximum `lower` bound `1/2` with no hint is
    accepted selecting `b`. Initial bounds `1/8` for the upper and `7/8` for the lower reject `InequalityFails`
    there because no action satisfies. Post-state selection needs no fabricated TransitionIdentity row.
- id: FR-1042-AC-14
  path: spec/functional/FR-1042-certify-value-bounds-with-lower-and-upper-certificates.md
  role: examined
  excerpt: In AC-3, minimum `upper` `s` `1/4` with hint `go` rejects `InequalityFails` although `wait` satisfies,
    and the same document without that hint accepts selecting `wait`. A nonexistent hint rejects `InequalityFails`
    even when an enabled action satisfies. Maximum `lower` `s` 1 with hint `wait` rejects `RankBroken`; removing
    the hint does not accept it, since no action satisfies both inequality and progress. When several actions satisfy
    every condition, an omitted hint selects the first in canonical full-product-action order.
- id: FR-1043-AC-1
  path: spec/functional/FR-1043-certify-finite-horizon-measure-verdicts.md
  role: examined
  excerpt: '`Retry`''s undelivered state `u` has `try`, delivering with `1/2` and otherwise staying; delivered is
    terminal. Under its workload, `quantile 0.5 of (steps from holds(true) until holds(delivered) within 4) <= 2`
    settles `proved`, `ExactValue{3/4}`, with a `conditional` `ratio` certificate whose numerator (payoff `satisfied`,
    `comparison` `"le"`, `threshold` 2) has values `3/4`, `1/2`, 0, 0 at `u` with active amounts 0, 1, 2 and above
    the bound and ranks 4, 3, 2, 1, and whose denominator (payoff `activated`) is 1 there; `fsl-probability-check`
    accepts it, and rejects it `RankBroken` with the ran'
- id: FR-1043-AC-2
  path: spec/functional/FR-1043-certify-finite-horizon-measure-verdicts.md
  role: examined
  excerpt: With `try` split into `fast` (`1/2`) and `slow` (`1/4`), under every scheduler `quantile 0.4 of (…) <=
    2` settles `proved` with an `exact` `shifted` document (`level` `2/5`) whose values are `3/80`, `−3/20`, `−2/5`,
    `−2/5`; `quantile 0.5 of (…) <= 2` settles `refuted` (minimum shifted value `−1/16`, `Pr(M <= 2) = 7/16` under
    `slow`); the level-0.4 document with `3/80` replaced by `fast`'s `9/40` is rejected `InequalityFails` at the
    initial key.
- id: FR-1043-AC-3
  path: spec/functional/FR-1043-certify-finite-horizon-measure-verdicts.md
  role: examined
  excerpt: 'Over AC-2''s model under every scheduler, `quantile 0.6 of (…) >= 2` (claim `Pr(M < 2 | act) < 3/5`)
    settles `proved`: the `shifted` document (`comparison` `"lt"`) has maximum initial value `−1/10` (through `fast`)
    and `−3/5` at every other undecided state; `quantile 0.5 of (…) >= 2` settles `refuted`, its maximum 0 attained
    by `fast` with an activated state reachable.'
- id: FR-1043-AC-4
  path: spec/functional/FR-1043-certify-finite-horizon-measure-verdicts.md
  role: examined
  excerpt: '`P95` over FR-185-AC-1''s `Service` under `Steady` at `5 ms` (FR-741-AC-2, FR-203-AC-1) settles `proved`,
    `ExactValue{24233/25000}`, with a `conditional` `ratio` certificate whose parts are documents of this profile
    (`kind` `"accumulate"`, `reward` the checked `duration`, `threshold` 5 in its unit) and whose denominator is
    1 at the initial key; `fsl-probability-check` accepts it from the emitted package and rejects it with one numerator
    value altered. At `2 ms` the item settles `refuted` with FR-202-AC-1''s one-path set.'
- id: FR-1043-AC-5
  path: spec/functional/FR-1043-certify-finite-horizon-measure-verdicts.md
  role: examined
  excerpt: Under `Retry`'s workload, `mean of fraction holds(delivered) over [0, 2] >= rational(2,5)` settles `proved`,
    `ExactValue{5/12}`, with a `fraction` document; at `rational(1,2)` it settles `refuted` with FSpec FR-413 subsystem
    evidence containing an independently accepted `exact` `fraction` certificate of `5/12 < 1/2`, checked for refutation
    rather than the proof threshold. The comparison event `probability >= 0.7 [M <= 2]` over AC-1's measure settles
    `proved`, `ExactValue{3/4}`, with an `event` document.
- id: FR-1043-AC-6
  path: spec/functional/FR-1043-certify-finite-horizon-measure-verdicts.md
  role: examined
  excerpt: FR-201-AC-51's finite-horizon quantile item over the cyclic `link` model under every scheduler (`Pr(M
    <= 3) = 13/27` for `steps from holds(true) until holds(m.delivered) within 4`), written at level 0.4, settles
    `proved` with shifted initial value `11/135` and a document `fsl-probability-check` accepts in a separate process;
    written at level 0.5 it settles `refuted`.
- id: FR-1043-AC-7
  path: spec/functional/FR-1043-certify-finite-horizon-measure-verdicts.md
  role: examined
  excerpt: AC-1's numerator with `window`, `threshold`, `comparison`, `reward`, `unit`, `level` or `payoff` changed
    refuses objective mismatch; a key whose accumulator amount or phase is altered is not an alias and fails coverage;
    a `shifted` value below `−level` is rejected `Malformed` at `values`. Pre-cancellation and exhaustion of each
    caller limit during expansion, payoff evaluation or exact sums settle `incomplete` with the actual limit and
    no truth value.
- id: FR-1043-AC-8
  path: spec/functional/FR-1043-certify-finite-horizon-measure-verdicts.md
  role: examined
  excerpt: With AC-1's `try` split into `fast` (`1/2`) and `slow` (`1/4`), `mean of fraction holds(delivered) over
    [0, 2]` has minimum `11/48` and maximum `5/12`. Under every scheduler, `>= 1/2` is refuted by an accepted exact
    minimum or FR-1042 `upper` on the minimum with initial bound `1/4` and policy `slow`; `<= 1/3` is refuted by
    an accepted exact maximum or `lower` on the maximum with initial bound `2/5` and policy `fast`. Both reuse subsystem
    evidence and are independently accepted by FR-202's checked-subject mean-certificate replay entry.
- id: FR-1043-AC-9
  path: spec/functional/FR-1043-certify-finite-horizon-measure-verdicts.md
  role: examined
  excerpt: AC-5's exact workload mean `5/12` at `>= 5/12` and `<= 5/12`, a valid workload `upper` bound `1/2` at
    `>= 1/2`, and a valid `lower` bound `2/5` at `<= 2/5` each reject `ThresholdMissed` at the initial key and settle
    `inconclusive`, `CertificateRejected`. In AC-8, a valid `lower` minimum document with initial bound `1/5` at
    `>= 1/2` and a valid `upper` maximum document with bound `1/2` at `<= 1/3` each reject `Malformed` at `kind`
    with the same settlement. Identity/base mismatch retains its typed refusal; an invalid value rejects `InequalityFails`
    with `CertificateRejected`; a disagreeing witne
- id: FR-1043-AC-10
  path: spec/functional/FR-1043-certify-finite-horizon-measure-verdicts.md
  role: examined
  excerpt: FR-203-AC-10's no-activation quantile under a resolved workload and under every scheduler, with and without
    an admitted fairness set, settles certified `Exhaustive` with an independently accepted conditional `vacuous`
    certificate, no `ExactValue`, shifted-value proof or numeric entry. Offering an all-zero `exact` `shifted` document
    under every scheduler, or a workload `ratio` with denominator 0, for that no-activation claim rejects `InequalityFails`
    at the initial complete product key and settles `inconclusive`, `CertificateRejected{rule, at}`, without numeric
    acceptance or `ExactValue`. A var
- id: FR-1043-AC-11
  path: spec/functional/FR-1043-certify-finite-horizon-measure-verdicts.md
  role: examined
  excerpt: '`Coin` initially has bound object `c` with `c.v = 0`; its only operation `drop(b)` draws `b` uniformly
    from `{0,1}`, deleting `c` when `b = 0`, and declares reward `R = 0` for both supported draws. Under its resolved
    workload, `mean of fraction holds(c.v = 0) over [0,0] weighted by R >= 1/2` reaches undefined `division-by-zero`
    on the actual `drop(0)` step of probability `1/2`: the final tentative weighted sums are `(0,0)`, and the `FractionValue`
    location is position 1 with the actual post-state lacking `c`, pre-step undecided control `remaining = 1` and
    pre-step sums `(0,0)`. Native replay r'
- id: FR-202-AC-27
  path: spec/functional/FR-202-replay-a-probabilistic-witness.md
  role: examined
  excerpt: Through the checked-subject mean-certificate replay entry, FR-1043-AC-5's workload mean `5/12` refutes
    `>= 1/2` using subsystem evidence containing the independently accepted `exact` `fraction` certificate; the
    same document checks in its refutation role despite missing the proof threshold. An unchecked document, an unreplayed
    engine value or a path set yields no accepted mean-refutation evidence.
- id: FR-202-AC-28
  path: spec/functional/FR-202-replay-a-probabilistic-witness.md
  role: examined
  excerpt: FR-1043-AC-8's minimum `11/48` with an accepted `upper` bound `1/4` refutes every-scheduler `>= 1/2`;
    its maximum `5/12` with an accepted `lower` bound `2/5` refutes every-scheduler `<= 1/3`. A valid `lower` minimum
    document with initial bound `1/5` at `>= 1/2`, and a valid `upper` maximum document with initial bound `1/2`
    at `<= 1/3`, each reject `Malformed` at `kind` and settle `inconclusive`, `CertificateRejected`; their conditions
    hold but their refutation roles are wrong.
- id: FR-202-AC-29
  path: spec/functional/FR-202-replay-a-probabilistic-witness.md
  role: examined
  excerpt: AC-27's exact `5/12` at `>= 5/12` or `<= 5/12`, and a valid workload `upper` bound `1/2` at `>= 1/2`,
    each reject `ThresholdMissed` at the initial key and settle `inconclusive`, `CertificateRejected`. Changing
    identity or base payoff/optimization retains the typed refusal; invalid certificate values settle `inconclusive`,
    `CertificateRejected{InequalityFails, at}`; a disagreeing witness policy settles `inconclusive`, `ReplayParity`;
    cancellation or a caller limit preserves its typed incomplete result without accepted evidence.
- id: FR-202-AC-30
  path: spec/functional/FR-202-replay-a-probabilistic-witness.md
  role: examined
  excerpt: 'A mean minimum refutation with a slack `upper < θ` for which both the optimizer''s attaining action
    and a different canonical-first action satisfy every FR-1042 condition emits a witness matching the independently
    checked policy: with no hint it uses the canonical-first satisfying full-product action; with a representable
    valid hint it uses that hinted action. Both replay as refuted. An old optimizer witness disagreeing with the
    checked selection settles `inconclusive`, `ReplayParity`; implicit stutter or post-state selection uses the
    same rule without a fabricated policy identity.'
- id: FR-203-AC-10
  path: spec/functional/FR-203-bound-an-exact-run-and-settle-its-verdict.md
  role: examined
  excerpt: 'The untimed claim `quantile 0.5 of (steps from holds(false) until holds(true) within 4) <= 2`, checked
    separately under a resolved workload and under every scheduler (with and without an admitted fairness set) has
    no activating state in the complete reachable product. Native checking independently accepts its `Conditional`
    `Vacuous` certificate and settlement returns `proved`, `closed-scope`, `Certified`, `Exhaustive`, category success,
    carrying that certificate with empty `values` and `conditional` exactly `{"mode": "vacuous"}`. The emitted result
    passes FR-331''s contract with basis `{"type":'
- id: FR-203-AC-11
  path: spec/functional/FR-203-bound-an-exact-run-and-settle-its-verdict.md
  role: examined
  excerpt: Changing AC-10's activation predicate to `holds(true)` makes the offered vacuous certificate reject `InequalityFails`
    and settle `inconclusive`, `CertificateRejected`, without vacuity acceptance. Offering an otherwise well-shaped
    `exact`, `ratio` or `optimum` certificate on `exhaustive`, or generic `evidence` with or without its permitted
    vacuous certificate, refuses the FR-331 result contract. An `exact-value` basis missing its value, using scalar
    zero or denominator zero refuses; AC-1's ordinary exact result with a valid `Rational` value remains admitted.
    An engine-only assertion, partial ex
- id: FR-203-AC-12
  path: spec/functional/FR-203-bound-an-exact-run-and-settle-its-verdict.md
  role: examined
  excerpt: '`Retry` has `Msg.delivered`, terminal when delivered, and the resolved `Steady` workload whose `fast`
    step delivers with probability `1/2` and whose `slow` weight is 0. With request initial states ordered as index
    0 `{delivered: true}` and index 1 `{delivered: false}`, the same admitted binding `m`, and `quantile 0.5 of
    (steps from holds(not m.delivered) until holds(m.delivered) within 4) <= 2`, the independent conditional certificate
    check accounts for both pairs: index 0 has unreachable activation with numerator and denominator 0, while index
    1 activates immediately with numerator `3/4`, den'
- id: FR-1043
  path: spec/functional/FR-1043-certify-finite-horizon-measure-verdicts.md
  role: examined
  excerpt: |-
    Expansion, payoff evaluation, unit conversion and exact
      sums SHALL run under the caller's byte, state, edge, work, storage,
      representation and arithmetic limits and cancellation; a stop SHALL
      settle `incomplete` with its limit and no truth value.
- id: FR-120
  path: spec/functional/FR-120-simulate-a-checked-package-s-state-family.md
  role: context_only
  excerpt: |-
    A model state is one complete, closed population state: for each universe
    population, its objects, each with its key, its most-specific type and a
    value for each declared field, where every reference value names an object
    of the state. It also carries its anchor, which its key omits.
```

## Targeted verification round 3

Frozen source `9e0a042d5f3f967e32caa2f9d4cd57d373379b82`; actual base `645bef0e0e810a5c551b7894ee98d7c6288b7467`. Same independent Codex reviewer. All earlier history above is retained verbatim.

**Latest qualification: CONDITIONAL with final runtime compile-blocked/zero tests.** Prior gap outcomes remain unchanged (GA001/003/004 fixed; GA002 generic proof scope deferred). No new gap analysis or finding.

Architecture-authorized targeted exception only: CR021 resource boundary and CR022 mechanical borrow error, not a full third review. The superseding ruling (QSL2158 commenta36a5342) accepts one conservatively prepaid bounded atomic clone/closure/extent interval, cancellation polled before and after; an inner callback is explicitly outside completion scope. The older CR021 wording requiring inner cancellability is therefore not a remaining requirement. No new upstream callback API, pin or validator was demanded.

CR021 source verification: declaration fields and universe members are indexed once, with caller admission before index construction and per-lookup work. No per-row field or universe vector rescan remains at the reported sites. observation_work_size traverses selected package records with caller callback charges, at least one per view including empty views, plus population-member visits; returned cardinalities include records absent from the state key. Its traversal performs sizing without validation or retained allocation. It accounts relationship/population declaration-key byte sizes. Restoration combines canonical source bytes, decoded value/task count, actual objects/extents, indexed fields/universes and selected declarations; checked arithmetic computes conservative cross-product work and clone storage. All prepayment uses the existing actual caller budget/meter and can stop before the owner call. tick/retain poll before the interval, tick(0) polls after the call even if observation returns a structural error, so cancellation retains precedence. Owner validation remains the existing observation call. No fresh default budget or duplicated validator was introduced.

CR022 source verification: source and target references are evaluated in separate locals before self.push, removing the two overlapping mutable self borrows with the same evaluation order and same retained pair.

The new owning66-field test constructs a genuinely admitted Link model with64 extra Boolean fields, obtains its actual canonical key, and supplies positive caller work bytes.len()+33. The initial byte charge and reservation-backed canonical read precede declaration indexing; the66-field charge must be the denied next charge. Assertions inspect actual retained Incomplete WorkUnits, consumed input bytes, next_charge66 and exact charge sequence. It is a meaningful nonzero caller-denial assertion by inspection; runtime is still pending. Existing owner/provenance/detached tests were not changed by this correction.

No remaining CR021/022 defect found within the authorized target; newly supplied compiler evidence is recorded separately as CR023. Every earlier fixed/rejected/deferred outcome is preserved; CR019/020 are not reopened, and generic GA002 remains architecture-deferred. No new full gap, BASE, normative or global audit ran. Accepted SR507 remains intact. Only the newly supplied actual compiler block is added as CR023; no expanded source-review hunt ran. Source stayed frozen and clean. Read-only diff --check passes; this correction changes only model restoration, its approved sizing accessor and one owning resource test (plus prior review custody). Pins, wire, default scopes and main behavior are unchanged.

Parent reports exact frozen9e0a first30-profile command COMPILE101/ZERO tests: measure.rs475/504/521 pass ProbabilityKeyAdmission where quire_canonical::Reservation is required, but it implements filament_exact::Reservation; measure.rs552 calls nonexistent enabled_actions_with_budget (actual API actions_with_budget). Commands2–16 held. Raw /private/tmp/qsl-3285-9e0a-1.log. No45-test acceptance or executed66-field test. Prior10564 first30-profile command COMPILE101/ZERO tests (two E0499); commands2–15 held. Older400617 owner-unit1pass0fail0ignore272filtered is older-source evidence only. No local cargo/tests/clippy/baseline/Kani/sweep/devcompile; no45-test acceptance claim.

Plan completion: not assessed

Existing D2 static matrix qualifications and diagnostic limitations are retained; no matrix/coverage command was rerun for this narrow exception.

### Targeted examined scope

```yaml
scope:
- id: FR-1043-AC-7
  path: spec/functional/FR-1043-certify-finite-horizon-measure-verdicts.md
  role: examined
  excerpt: AC-1's numerator with `window`, `threshold`, `comparison`, `reward`, `unit`, `level` or `payoff` changed
    refuses objective mismatch; a key whose accumulator amount or phase is altered is not an alias and fails coverage;
    a `shifted` value below `−level` is rejected `Malformed` at `values`. Pre-cancellation and exhaustion of each
    caller limit during expansion, payoff evaluation or exact sums settle `incomplete` with the actual limit and
    no truth value.
- id: FR-1043-AC-9
  path: spec/functional/FR-1043-certify-finite-horizon-measure-verdicts.md
  role: examined
  excerpt: AC-5's exact workload mean `5/12` at `>= 5/12` and `<= 5/12`, a valid workload `upper` bound `1/2` at
    `>= 1/2`, and a valid `lower` bound `2/5` at `<= 2/5` each reject `ThresholdMissed` at the initial key and settle
    `inconclusive`, `CertificateRejected`. In AC-8, a valid `lower` minimum document with initial bound `1/5` at
    `>= 1/2` and a valid `upper` maximum document with bound `1/2` at `<= 1/3` each reject `Malformed` at `kind`
    with the same settlement. Identity/base mismatch retains its typed refusal; an invalid value rejects `InequalityFails`
    with `CertificateRejected`; a disagreeing witne
- id: FR-1043
  path: spec/functional/FR-1043-certify-finite-horizon-measure-verdicts.md
  role: examined
  excerpt: |-
    Expansion, payoff evaluation, unit conversion and exact
      sums SHALL run under the caller's byte, state, edge, work, storage,
      representation and arithmetic limits and cancellation; a stop SHALL
      settle `incomplete` with its limit and no truth value.
```

## Targeted mechanical verification round 4

Frozen source `bd54ba0f8b40d960acb39086268fffd4269f3a13`; same independent Codex reviewer. Earlier history remains verbatim.

**Latest qualification: CONDITIONAL pending final exact-head runtime.** Existing gap outcomes unchanged, no new gap finding or full gap analysis.

Targeted CR023 mechanical verification only, continuing the same architecture-authorized correction. Inspected the eight production lines and their actual called APIs/error mappings. The quire_canonical::Reservation implementation explicitly forwards to the existing filament_exact::Reservation implementation; its poll, cumulative storage ceiling, checked size conversion and original first retained stop remain unchanged. Canonical denial handling resolves ModelKeyError::Denied through admission.refusal, preserving cancellation/storage/accounting causes instead of replacing them with Malformed.

restore_named now calls the real public ModelSystem::actions_with_budget and maps its ModelKeyExpansionError through admission.expansion_refusal. The owner method uses the existing KeyScope and actual caller budget/cancel. Its difference from enabled_actions is filtering actions with no physical post for any draw; such an action cannot contain a matching pending draw, which independently requires positive probability and at least two actual posts. Draw selection, pending identity equality and retained rewards/posts remain unchanged. No error-semantic change or new defect found in this narrow correction.

CR023 is fixed by inspected source. CR021/022 remain fixed under the accepted prepaid atomic-interval ruling; no deeper callback or full additional review was demanded. All prior histories/dispositions, including rejected CR002 and architecture-deferred generic GA002, remain preserved. Accepted normative SR507, pins, scope, defaults and wire are unchanged. No new full gap/BASE/matrix/global analysis was run. Source remains clean/frozen; read-only diff --check passes.

Final exactbd54ba0f8b40d960acb39086268fffd4269f3a13 same45tests/16commands is being requested remotely; no results supplied yet. No45-test acceptance claim. Previous9e0a first30 command COMPILE101/ZERO tests, commands2–16 held, remains historical failure. No local cargo, tests, baseline, clippy, Kani, sweeps or devcompile.

Plan completion: not assessed

Earlier scoped static matrix/diagnostic limits remain; no recomputation was needed for this targeted mechanical correction.

## Subsequent exact-head runtime evidence

Parent reports exact bd54ba0f8b40d960acb39086268fffd4269f3a13 first30-profile command COMPILE101/ZERO tests; commands2–16 held. Earlier CR023 compiler errors are gone and CR023 remains fixed. The newly observed blocker is three E0609 in existing liveness test code: liveness.rs1065 reads anchors[0].kind on Option<&SelectedAnchor>;1067 reads anchor.kind/name on &Option, following anchor().clone() captures at1048/1062. Raw /private/tmp/qsl-3285-bd54-1.log. Overall execution remains compile-blocked; no45-test acceptance or merge clearance. Parent has authorized the coder to require the actual anchor before cloning, preserving every assertion, with no production change. This addendum records supplied runtime evidence only; no new source inspection/full review was performed. Await the frozen correction and targeted test diff before verification. All previous source clearances, including CR021/022/023, remain preserved.

## Targeted liveness compiler verification round 5

Frozen source `df2460d71671b173a71a9b0fc7e38ace62030c43`; same independent Codex reviewer.

**Latest targeted source verdict: PASS. Overall CONDITIONAL pending compile-only and focused runtime.** No remaining source finding or new defect identified within this targeted test correction.

Targeted inspection only of the test compiler correction in commit5762aa5e5859e2550919316a5a760f46bb718178 (df246 is artifact custody). The actual source uses required_anchor().expect(...).clone() at the two existing initial/handler captures. required_anchor returns the real admitted anchor or a typed refusal; expect makes missing provenance fail the test explicitly. This captures SelectedAnchor values rather than Option<&SelectedAnchor>, resolving the reported E0609 member accesses without fabricating a default anchor.

All existing assertions are preserved: one initial and one successor, values0/1/2/3, initialization kind, handler kinds/names, shared handler identity, terminal stutter, repeated state/value/key and distinct stutter-versus-inc identities. No production change, weakened test, changed fixture, removed trace or broadened review. No new defect found in the authorized correction. The late bd54 liveness compiler blocker is corrected by source inspection; this is not evidence that df246 compiles or the test runs.

CR023 and all earlier source clearances remain fixed; rejected CR002 and architecture-deferred generic GA002 remain unchanged. CR021/022 preserve the accepted prepaid atomic validation boundary. All prior finding/history text remains verbatim. No new full code/gap/BASE/matrix/global audit or finding id. Source clean/frozen; read-only diff --check passes.

Exact df2460d71671b173a71a9b0fc7e38ace62030c43 compile-only all seven touched packages with --tests has been requested under new architecture ruling before focused46tests/17commands; both compilation and runtime results pending. No acceptance or merge clearance claimed. Prior bd54 first30 command COMPILE101/ZERO tests, commands2–16 held, due this existing liveness test; CR023 compiler errors were gone. Raw /private/tmp/qsl-3285-bd54-1.log. No local cargo/tests or other gates run.

Plan completion: not assessed

No new gap analysis or matrix run; earlier scoped qualifications remain.

## Targeted Binding compiler verification round 6

Frozen source `e7787487c512f4f40662d291d0bc8422d7bffe9b`; same independent Codex reviewer.

**Latest targeted source verdict: PASS. Overall CONDITIONAL pending compile-only and focused runtime.**

Targeted test compiler correction only. The entire source delta is four added `..` rest patterns in existing Binding destructures in fsl-replay/src/spine/clause/tests.rs. Each continues to bind exactly the same name/value or context/frame values; it now permits the additional binder field without attempting to match it. All existing pattern alternatives, literal checks, names, reference targets, panic paths and assertions remain verbatim. The fifth Binding pattern already had a rest pattern. No production change, fixture change, weakened assertion, unrelated warning cleanup or expanded review. Source inspection clears the four reported E0027 sites; it does not establish compilation/runtime acceptance.

Every prior source clearance and fixed/rejected/deferred finding outcome remains preserved, including CR023 fixed, CR021/022 fixed under the accepted prepaid interval, CR002 rejected and generic GA002 architecture-deferred. No new finding, full code/gap/BASE/matrix/global analysis or source mutation. Earlier artifacts remain verbatim prefixes. Source remains clean/frozen; read-only diff --check passes.

Parent reports prior df246 architecture-directed seven-package --tests check exit101 with exactly four E0027 at spine/clause/tests.rs3841/3951/4452/5114, Binding patterns omit binder; raw /private/tmp/qsl-3285-df24-compile.log. Same seven-package compile-only queued at exacte778; focused46 runtime is HOLD. Compile/runtime results pending, no46-test acceptance or merge clearance. No local cargo/tests/gates run.

Plan completion: not assessed

No new gap analysis; existing scoped qualifications remain unchanged.

## Actual e778 compile and runtime evidence

**Latest overall verdict: FAIL, runtime acceptance blocked.** Earlier targeted static compiler/source clearances remain historical source findings outcomes; they are not runtime acceptance.

Exact e7787487c512f4f40662d291d0bc8422d7bffe9b seven-touched-package --tests compile-only EXIT0/13.09s, remote /private/tmp/qsl-3285-e778-compile.log. First focused measure_profile module:30 selected,1 passed,29 failed,0 ignored,2578 filtered,0.25s,exit101; commands2–17 held/not run. Verified raw runtime /tmp/qsl-3285-e778-runtime-1.log. No46-test acceptance or merge clearance.

Observed failure clusters are S1 `expected (` in mean/fraction fixtures; `delivered` wrong-value-kind admissions/producer Declined InvalidInput; residual Undefined expected Refuted but actual Failed runtime_invariant; vacuity control expectation failure; and P95 emitted-package independent checker StorageBytes bound8000000 actual8000015. Only `an_undefined_measure_after_merged_draws_replays_a_real_positive_draw` passed. These are observed failures, not established common root causes. The raw log was read for this evidence update.

Per parent direction, no new source review, finding hunt, test rerun or source mutation was performed. No failing test was bent/deleted and no budget was raised. Coder is diagnosing the clusters read-only; parent requested an architecture ruling before corrections because29 failures exceed the standing single-test scope. Await later bounded authorization/frozen head before targeted verification. All prior fixed/rejected/deferred source dispositions remain preserved; no mergeable claim.

## Targeted runtime bundle verification round 7

Frozen source `790ffd226b950c075584ead59d9cb76c57c04648`; actual current main base `7d9dd0ee52e92712f9e74cc4922fcd73d4eb0d9f`; same independent Codex reviewer.

**Latest targeted source verdict: PASS. Overall CONDITIONAL pending final compile and focused46 runtime.** No new finding within the authorized bundle. Earlier failing runtime remains preserved as history.

Architecture-authorized runtime bundle only, reviewed against nondiagnostic e778 and directly affected current context; no full additional review or unrelated main audit. The ratio checker independently reconstructs one graph from the checked package/claim. Both parts still require their own identity/payoff roles and independently derived complete objectives, range checks, exact sorted values/ranking coverage, rederived ranks and Bellman equality. Shared topology/control/key derivation follows the same admitted measure, including the claim threshold retained by both controls. Numerator values remain retained while the graph terminal payoff switches to the independently evaluated and retained shared MeasurePayoff::Activated values; denominator values are checked separately and both maps feed total actual-index/binding coverage and ratio/vacuity checks. No second payoff-rule table, graph/counter reset, budget raise or model shrink. The added activation payoff evaluation and retention are charged. Wrong denominator objective/role/rank/value does not inherit numerator acceptance.

P95 still asserts exact24233/25000, initial denominator1, checked threshold/unit, emitted-package acceptance, altered numerator InequalityFails at Numerator, independent altered denominator InequalityFails at Denominator, and two-ms refutation. Existing bad key/objective/rank controls remain. These are preserved meaningful assertions, not executed corrected-head evidence.

Bounded pending identity now reads ScheduledIdentity.operation as the owner declaration text {node,package}, checks exactly two string members and canonical byte equality through DeclarationRecord, then emits via existing project_declaration. Reservation/cancel/byte checks and retained owner refusal are preserved. Receiver/arguments/actual draw are unchanged. Actual residual Undefined replay still uses positive physical draw, resolved post, pending policy and PRE control/sums; no fabricated anchor/evidence or runtime_invariant relabeling was introduced.

Test repair follows explicit rulings: actual Boolean fixture carriers and parenthesized mean/fraction syntax; partial Lower <= independently checked full optimizer value with scheduler rows restricted to the named subset, while full/equality/adverse controls remain; interior FR186 thresholds with valid Lower0 reaching exactly ThresholdMissed; canonical fraction [n,d] expectations with the same zeros and existing cause/position/control/sums forgeries. No extra frontier product-parity rule was imposed.

Vacuity exercises real resolve_items/settle_run Result output with actual owning-model string, exact wrapper, accepted checker, exhaustive basis and unchanged conditional/vacuous document. Schema exception controls use FSpec's published positive carrier and verify an admitted ordinary numeric baseline before mutations. The broader preexisting nested structuralNodeKey/exportNodeId schema gap is architecture-deferred and is not claimed fixed or raised as this PR's blocker. No schema weakening/stringification was introduced.

All QSL2158-DIAG searches in production trees are empty. The three diagnostic-only source files outside the two changed production owners have no diff from nondiagnostic e778. No diagnostic patch was blindly applied. All earlier source dispositions remain preserved: CR001/003–023 fixed; CR002 rejected; GA001/003/004 fixed, generic GA002 deferred. CR021 retains accepted prepaid bounded atomic validation with pre/post cancellation; no withdrawn inner callback demand. No new finding within the authorized bundle. Accepted normative SR507, pins and unrelated main changes remain intact. Source stayed clean/frozen; read-only diff --check passes.

Historical actual evidence: e778 compile-only seven packages --tests EXIT0/13.09s, then module30 selected1passed29failed0ignored2578filtered/0.25s/exit101, commands2–17held. Fixture correction add87 reached19passed11failed0ignored2578filtered/1.17s/exit101, raw /tmp/qsl-3285-add8-module.log. Diagnostic877 seven-package compile passed, exactP95 failed0pass1fail0ignore2607filtered/0.23s/exit101; raw /tmp/qsl-3285-877d-p95.log shows denominator SECOND graph expansion used7999759,next256,bound8000000,actual8000015. These are diagnosis/failure history, not corrected790ffd acceptance.

Final790ffd seven-package --tests compile is requested but delayed in shared TEST-RUN backlog; final compile and focused46tests/17commands have no supplied results yet. Overall CONDITIONAL; no mergeable/acceptance claim. No local cargo, tests, broad baseline, clippy, Kani, sweeps or devcompile.

Plan completion: not assessed

No full new gap analysis or matrix/coverage recomputation; directly affected test assertions were checked as part of this targeted bundle. Earlier scoped traceability and diagnostic limitations remain.

## Targeted runtime test-failure round 1 verification

Frozen source `22884bbd1f29bdd249e5ec1f3293e24cc77b9282`; actual base `1ed4808a5abe4c093c32267a2d59716123bf1000`; same independent Codex reviewer.

**Latest targeted source verdict: PASS. Overall CONDITIONAL pending exact22884 compile and focused46 runtime.**

Targeted test-failure round1of2 only: the private residual payload callback, its ownership/accounting and the genuine residual fixture expansion, plus necessary integration import preservation. No full additional review or unrelated main audit.

The private FnMut callback is invoked only for PostState in WitnessWork::Action/Step; all existing identity, terminal-stutter, scalar/draw/reward/probability and slot charges remain. The nonmeasure caller still explicitly refuses PostState as BrokenInvariant. The measure callback visits/polls each owned product state through existing witness_charge(0), compares full ProbKey identity, requires a matching nonpending resolved target and an existing qualified canonical certificate key, and charges fixed ProbKey size plus canonical key length before the residual action is cloned. No reencoding, new codec/public API, reset, budget raise, search/replay/settlement change.

Actual ownership was traced: measure builder creates PostState edge-target keys with certificate_state=None; canonical certificate keys are subsequently installed only in product.states. Cloning the edge action therefore does not clone a second certificate-key Vec. The callback borrows the matched state's qualified canonical key as a conservative size bound for the actual edge target's model/binding/control/accumulator payload, plus fixed ProbKey storage. Native action/step/path retention calls remain before ownership changes. No ungrounded duplicate-certificate allocation finding is asserted.

The residual fixture variant now removes the delivered terminal and not-delivered preconditions after replacing the postcondition with true, enabling both actual post states at later positions. This expands transitions so genuine positive accumulated pending rows exist. Initial state, claims, window, random support, operation bodies and all meaningful positive/adverse checks remain; the original terminal-stutter variant is unchanged. Actual PostState, canonical tie, no-satisfying positive residual, bad/nonexistent hint and no-fallback assertions remain. No test weakened, deleted or ignored.

All temporary QSL2158_DIAG/force_capture code is absent from the two production files. The initial terminal/search hypothesis was disproved by the probe and no such logic change was applied. The actual correction is35 changed formatted production lines, plus7added/1removed fixture lines.

Integration preservation checked: five owned correction/test/review SHA256s match premerge afd038 exactly; all76 range pairs are unchanged. Sole manual frames.rs import resolution retains main ModelSystem and removes obsolete ModelSimState, preserving reached() provenance. Actual base is main1ed4808. Authorized IR1cbd86a48f02e4623e558a3d3b23c066c7f280bf, FSpecd6d066 and core-data424 pins are retained. No unrelated main audit. Source remains clean/frozen and read-only diff --check passes.

No new defect found within the authorized target. Prior finding history/outcomes remain preserved, including CR002 rejected, CR021 accepted prepaid interval, CR023 fixed and generic GA002 architecture-deferred. Accepted BASE SR507 unchanged; no new full gap/matrix/global analysis or finding id.

Actual790ffd full46 completed44pass2fail0ignore: module28/30, commands2–17 each1pass. Failures were policy-hint fixture missing a genuine positive residual row and native residual Quantile Failed/runtime_invariant. Raw /home/peter/.cache/handoff/3285-cmd-1-790ffd.log read. Probe4533 compiled seven-package --tests EXIT0/24.30s, exact residual test0pass1fail0ignore2612filtered/0.10s; full81-line /home/peter/.cache/handoff/3285-probe-4533e1.log read: Event ThresholdMissed then witness produced/replay REPRODUCED, positive draw/forged-post checks passed; subsequent Quantile ThresholdMissed had no produced marker and source traces its controlled PostState payload refusal. This is old diagnosis, not22884 acceptance.

Exact integrated22884 seven-package --tests compile and same46tests/17commands are queued through parent/architecture; no supplied new-head results yet. Overall CONDITIONAL, no acceptance/merge clearance. No local cargo, tests, baseline, clippy, Kani, sweeps, external models or extra agents.

Plan completion: not assessed

This is the directly affected fixture/assertion/runtime qualification only; no new full gap or matrix run.

## Final exact-head runtime qualification

**Latest overall verdict: PASS — READY for merge at source22884bbd1f29bdd249e5ec1f3293e24cc77b9282.**

Architecture supplied actual acceptance on exact22884bbd1f29bdd249e5ec1f3293e24cc77b9282: seven-package --tests compile exit0; all17 focused commands complete,46 selected/46passed/0failed/0ignored. Module command1 passed30/30; commands2–17 each passed1/1. Remote logs dracoscratch3285-22884bbd.../cmd-N.log. This meets the pending runtime condition; architecture explicitly accepts the result and directs exact-head merge.

Evidence-only qualification of the already reviewed immutable source; no new source inspection, review round, method, finding or disposition change. All prior histories remain preserved. Earlier source PASS plus actual compile/focused acceptance now clears the merge condition. Generic GA002 remains architecture-deferred; this is scoped PR acceptance, not a claim that all QSL2158 work is complete. Source freeze unchanged; parent owns artifact-only custody and exact-head admin merge.
