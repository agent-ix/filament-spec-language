---
id: SR-967192
title: "base review of PR 4064 spec changes"
type: SpecReview
analysis: base
scope: "agent-ix/filament-spec-language; spec/functional/FR-189, FR-192, FR-203, FR-315, FR-896; spec/test-cases/TC-624, TC-900, TC-3241"
review_set: subset
---

## Summary

Ticket: QSL-3497. PR 4064 at a8b116f24c243dbacdb05ec6ae6559c00900359c. 2 finding(s).

## Verdict

Ids are unique (TC-3241 appears on this branch only), new criteria carry test cases, `quire validate` on the changed files passes with one unrelated module warning, no hash or pin record is added, and body text adds no spec links outside frontmatter.

## Findings

| ID | Severity | Summary | Refs | Escape Cause |
| --- | --- | --- | --- | --- |
| FND-001 | low | RequiresBound/conflicting-identity order and exact NoInitialState-before-form have no criterion | spec/functional/FR-189-decide-a-probabilistic-claim-by-statistical-model-checking.md:262-265 | correct-requirement-no-evidence |
| FND-002 | low | Scope line lost the comma between two AC ranges | spec/test-cases/TC-900-the-smt-lib-encoding-is-canonical.md:16 | wrong-requirement |

## Detail

- FND-001: Parts of the stated order have no criterion. No statistical AC or TC-624 step puts RequiresBound before NoInitialState or places conflicting-identity; no exact AC places conflicting-identity; and exact NoInitialState before form admission is untested (only AC-34 has the outside-form case, TC-3241 step 3). An implementation that returns NoInitialState with a universe missing on the statistical route, or unsupported for an outside-form claim over an empty initial list, passes every criterion.
- FND-002: The edited scope line lost a separator: "FR-315-AC-15 to FR-315-AC-32 FR-315-AC-53 to FR-315-AC-57" reads as one run-on range.

## Examined scope

- examined: FR-189 (spec/functional/FR-189-decide-a-probabilistic-claim-by-statistical-model-checking.md)

```text
- EN-4 SHALL take a claim's instances, and its instancing outcomes in their
  order, from `ModelSystem::claim_instances` (FR-125 "Instancing"):
  `RequiresBound`, then `NoInitialState`, then a conflicting identity, then
  `NoInstance`. A claim with no `over` has one instance, with no bound object.
```

- examined: FR-189-AC-11 (spec/functional/FR-189-decide-a-probabilistic-claim-by-statistical-model-checking.md)

```text
A request whose subject has an empty `initial` list is refused `NoInitialState`, and FR-192 settles it `inconclusive` with the cause `no-initial-state`; it is never Accepted.
```

- examined: FR-189-AC-17 (spec/functional/FR-189-decide-a-probabilistic-claim-by-statistical-model-checking.md)

```text
Over AC-6's `Coin` model and claim `over (c: M::Coin)`, with the universe of `coins` `[k]` and one initial state that holds no object, the request is refused `NoInstance` before any sample is drawn, and FR-192 settles it `inconclusive` with the cause `no-instance`, never Accepted. With the same universe and an empty `initial` list it is refused `NoInitialState`, not `NoInstance`.
```

- examined: FR-192-AC-1 (spec/functional/FR-192-settle-a-statistical-result-on-the-measured-axis.md)

```text
Each outcome row maps exactly as the table states: an Accepted, a Rejected and an Undecided (`IndifferenceRegion`) `Completed` outcome; a `Stopped` naming `max_samples`, settling `Incomplete(LimitReached{limit, value, setting})` with setting `statistical.max_samples`; an `Unsupported` outcome with cause `NotMarkov`; a `NoInitialState` refusal settling `inconclusive`, `no-initial-state`; a `NoInstance` refusal settling `inconclusive`, `no-instance`; a `ConfidenceBeyondPrecision` refusal settling `unsupported`, `confidence-beyond-precision`; an `Internal` refusal settling `failed`; a `Stopped` b
```

- examined: FR-203-AC-20 (spec/functional/FR-203-bound-an-exact-run-and-settle-its-verdict.md)

```text
An exact item whose subject has an empty `initial` list and bounded universes settles `inconclusive` with cause `no-initial-state`, `Inconclusive(NoInitialState)`, for an `on origin` claim, a claim with an `over` parameter and an `on each` claim, and when those universes hold no key. It has no runtime diagnostic (never `failed` or `invalid_runtime_input/invalid-value`), verdict, certificate, counterexample or numeric entry. With a universe missing as well, the item is `RequiresBound` instead. An initial-state value rejected earlier by model admission or reading keeps that typed cause and locus
```

- examined: FR-203-AC-21 (spec/functional/FR-203-bound-an-exact-run-and-settle-its-verdict.md)

```text
When exact EN-5 cannot build a selected claim binding, other than for lack of any admitted key (AC-34), the item settles `unsupported` with `unsupported_construct/not-yet-implemented` at that claim's binding-form locus and names the unsupported binding form (QSL-3143); no proof, counterexample, or numeric entry is emitted.
```

- examined: FR-203-AC-34 (spec/functional/FR-203-bound-an-exact-run-and-settle-its-verdict.md)

```text
An exact item whose subject has a non-empty `initial` list and no admitted key for the claim's `over` parameter, or none for its `on each` activation type, settles `inconclusive` with cause `no-instance`, `Inconclusive(NoInstance)`, never `proved`, `unsupported` or `failed`, with no certificate, counterexample or numeric entry. This holds also when the claim's form is outside what EN-5 admits.
```

- examined: FR-203-AC-35 (spec/functional/FR-203-bound-an-exact-run-and-settle-its-verdict.md)

```text
FR-201's certificate checker, called directly with a well-formed document of the claim's identity, returns `NoInitialState` for AC-20's subjects and `NoInstance` for AC-34's, for an `on each` claim and for a claim that is not, and neither accepts the document nor reports it rejected, malformed or unsupported. The test states each expected cause itself and takes none from an engine result.
```

- examined: FR-315-AC-62 (spec/functional/FR-315-encode-the-smt-lib-transition-relation.md)

```text
Instancing outcomes are refused in FR-125's order. Over FR-315-AC-1's `Counter` with no universe for the population of `c` and an empty `initial` list, `encode_smt_query` refuses `requires-bound` naming the population, not `no-initial-state`. Over FR-125-AC-13's records, an `Animal` under `c` in `animals` and a `Dog` under `c` in `dogs`, a claim over `Animal` refuses `subject-admission` carrying `invalid_runtime_input`/`conflicting-identity`. Over `Counter` with the universe `[c]`, one initial state that holds no object and an operation `setTo(n: Integer)`, it refuses `no-instance`, not `objec
```

- examined: FR-896-AC-7 (spec/functional/FR-896-decide-a-global-conformance-claim-over-the-protocol-system-or-a-recording.md)

```text
The keyed order fixture is the FR-636 order fixture with the constraint of `send Placed`, the first control of `Main`, written `{ k.r1 = 0 }`, over the universe `cores` `[p, q]` with an initial state holding `p.r1 = 0` and `q.r1 = 1`. With no `subject.protocol`, `global-conformance` settles `refuted`, `p` being proved and `q` blocked at `Placed` with wait cause `Constraint`: the counterexample is the deadlock with the empty prefix, its protocol subject's `over` names `q`, it replays, and `closure.state` is `open`. With `q.r1 = 0` too, the item settles `proved` with `closure.state` `closed`.
```

- examined: TC-900 (spec/test-cases/TC-900-the-smt-lib-encoding-is-canonical.md)

```text
Scope: FR-315-AC-1 to FR-315-AC-6, FR-315-AC-15 to FR-315-AC-32 FR-315-AC-53 to FR-315-AC-57 and FR-315-AC-62.
```

- examined: TC-624 (spec/test-cases/TC-624-statistical-model-checking-decides-probabilistic-claims.md)

```text
Scope: FR-189-AC-1 to FR-189-AC-5, FR-189-AC-7 to FR-189-AC-12, FR-189-AC-13 to FR-189-AC-17.
```

- examined: TC-3241 (spec/test-cases/TC-3241-an-exact-item-with-no-initial-state-or-no-instance-settles-inconclusive.md)

```text
1. Take three claims over one subject: one `on origin` with no parameter, one
   with an `over` parameter and one `on each`. Run each through `fsl analyze`
   with exact evidence over an empty `initial` list and bounded universes,
   then with those universes also empty.
2. Run them with an empty `initial` list and one universe removed.
3. Over a non-empty `initial` list whose states admit no key of the
   parameter's type, run the `over` claim and the `on each` claim, then a
   variant of each whose form EN-5 does not admit.
4. For the subjects of steps 1 and 3, call the certificate checker d
```

## New findings (disposition pass 1)

| ID | Severity | Summary | Refs | Escape Cause |
| --- | --- | --- | --- | --- |
| FND-003 | low | No criterion for the checker's RequiresBound and conflicting-identity refusals or their order | spec/functional/FR-201-check-a-probability-certificate.md:803-811 | correct-requirement-no-evidence |

- FND-003: The checker is now required to decide all of FR-125's outcomes in order and to refuse RequiresBound and conflicting-identity, but FR-201-AC-85, FR-203-AC-35 and TC-3241 steps 4 and 7 exercise only NoInitialState and NoInstance. A checker that returns NoInitialState over an empty initial list with a universe missing, or that accepts or rejects a document over two-typed records, passes every criterion. Add a criterion for the checker on those two subjects.

## Dispositions

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | - |
| FND-002 | fixed | - |
