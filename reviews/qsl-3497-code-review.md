---
id: SR-967190
title: "code-review review of PR 4064 spec changes"
type: SpecReview
analysis: code-review
scope: "agent-ix/filament-spec-language; spec/functional/FR-189, FR-192, FR-203, FR-315, FR-896; spec/test-cases/TC-624, TC-900, TC-3241"
review_set: subset
---

## Summary

Ticket: QSL-3497. PR 4064 at a8b116f24c243dbacdb05ec6ae6559c00900359c. 1 finding(s).

## Verdict

FR-896-AC-7 is realizable as read: FR-218 and FSpec FR-464 do not bar model state or `over` in a send constraint, the TC-2832 test checks a send constraint that reads model state, `k` is the `over (k: M::Core)` parameter of the order protocol, and the hand values hold (p sends and completes; q has no step from the empty prefix, FR-211 cause Constraint). FR-315 renumbering is consistent: the refinement paragraph's `4 to 15 and 17` and `row 16` match the old `4 to 14 and 16` and `row 15`; no spec or test text outside it cites a table row. Code numbering is now stale (fsl-eval/src/smt/refusal.rs row(), comments in smt/query.rs) and follows in the code change; that table already lacks the `several-instances` refusal refusal.rs numbers 14, which predates this PR. No `NoTest` text remains in spec. Choice of deciding both causes before form admission: accepted; FR-125 orders only the instancing outcomes among themselves and ahead of exploration, and FR-875 already puts no-instance ahead of the profile check. Nothing was built or run.

## Findings

| ID | Severity | Summary | Refs | Escape Cause |
| --- | --- | --- | --- | --- |
| FND-001 | medium | FR-189 names `ModelSystem::claim_instances`, which no spec defines and which as built cannot produce the stated four-outcome order | spec/functional/FR-189-decide-a-probabilistic-claim-by-statistical-model-checking.md:262-265 | wrong-requirement |

## Detail

- FND-001: FR-189 requires the four outcomes, in order, from `ModelSystem::claim_instances` (FR-125 "Instancing"). No spec defines claim_instances: FR-125 Outputs names only ModelSystem::instances and excluded_keys, and the function is not on main. On origin/titan/qsl-3507-claim-instances (cd923f76) it returns only NotSimulated, NoInitialState or NoInstance, and ModelSystem::new has already refused a conflicting identity, so a conflicting identity is reported before RequiresBound, not third. An implementer using the named call cannot give the stated order. Define the call in FR-125 Outputs with what it decides and what admission decides, or drop the name and state the order against FR-125 alone.

## Examined scope

- examined: FR-189 (spec/functional/FR-189-decide-a-probabilistic-claim-by-statistical-model-checking.md)

```text
- EN-4 SHALL take a claim's instances, and its instancing outcomes in their
  order, from `ModelSystem::claim_instances` (FR-125 "Instancing"):
  `RequiresBound`, then `NoInitialState`, then a conflicting identity, then
  `NoInstance`. A claim with no `over` has one instance, with no bound object.
```

- examined: FR-315 (spec/functional/FR-315-encode-the-smt-lib-transition-relation.md)

```text
| 4 | `subject-admission` | FR-120's admission of the subject refuses, carrying that refusal, other than for a key with two types (row 7) | the subject |
| 5 | `requires-bound` | A population that admits the clause's `over` type has no universe (FR-125 `RequiresBound`) | the population |
| 6 | `no-initial-state` | The subject's `initial` list is empty (FR-125), so every query would be unsatisfiable | the subject |
| 7 | `subject-admission` | Two member records of the subject give one key two types (FR-125), carrying `invalid_runtime_input`/`conflicting-identity` | the subject |
| 8 | `no-insta
```

- examined: FR-315 (spec/functional/FR-315-encode-the-smt-lib-transition-relation.md)

```text
**Refusals.** `encode_refinement_query` SHALL check the refusals 1 to 5 below
first, in this order, then the table's rows 4 to 15 and 17 as they apply to the
concrete subject, with row 16's `out-of-fragment` check (refusal 6) read over the
```

- examined: FR-896-AC-7 (spec/functional/FR-896-decide-a-global-conformance-claim-over-the-protocol-system-or-a-recording.md)

```text
The keyed order fixture is the FR-636 order fixture with the constraint of `send Placed`, the first control of `Main`, written `{ k.r1 = 0 }`, over the universe `cores` `[p, q]` with an initial state holding `p.r1 = 0` and `q.r1 = 1`. With no `subject.protocol`, `global-conformance` settles `refuted`, `p` being proved and `q` blocked at `Placed` with wait cause `Constraint`: the counterexample is the deadlock with the empty prefix, its protocol subject's `over` names `q`, it replays, and `closure.state` is `open`. With `q.r1 = 0` too, the item settles `proved` with `closure.state` `closed`.
```

- context_only: FR-896-AC-8 (spec/functional/FR-896-decide-a-global-conformance-claim-over-the-protocol-system-or-a-recording.md)

```text
Over AC-7's first state, with `subject.protocol` naming `Flow` and binding both roles, `over` `p` settles `proved`, and `over` `q` settles `refuted` with AC-7's counterexample.
```

- examined: FR-189-AC-17 (spec/functional/FR-189-decide-a-probabilistic-claim-by-statistical-model-checking.md)

```text
Over AC-6's `Coin` model and claim `over (c: M::Coin)`, with the universe of `coins` `[k]` and one initial state that holds no object, the request is refused `NoInstance` before any sample is drawn, and FR-192 settles it `inconclusive` with the cause `no-instance`, never Accepted. With the same universe and an empty `initial` list it is refused `NoInitialState`, not `NoInstance`.
```

- examined: FR-203-AC-20 (spec/functional/FR-203-bound-an-exact-run-and-settle-its-verdict.md)

```text
An exact item whose subject has an empty `initial` list and bounded universes settles `inconclusive` with cause `no-initial-state`, `Inconclusive(NoInitialState)`, for an `on origin` claim, a claim with an `over` parameter and an `on each` claim, and when those universes hold no key. It has no runtime diagnostic (never `failed` or `invalid_runtime_input/invalid-value`), verdict, certificate, counterexample or numeric entry. With a universe missing as well, the item is `RequiresBound` instead. An initial-state value rejected earlier by model admission or reading keeps that typed cause and locus
```

- examined: FR-203-AC-34 (spec/functional/FR-203-bound-an-exact-run-and-settle-its-verdict.md)

```text
An exact item whose subject has a non-empty `initial` list and no admitted key for the claim's `over` parameter, or none for its `on each` activation type, settles `inconclusive` with cause `no-instance`, `Inconclusive(NoInstance)`, never `proved`, `unsupported` or `failed`, with no certificate, counterexample or numeric entry. This holds also when the claim's form is outside what EN-5 admits.
```

