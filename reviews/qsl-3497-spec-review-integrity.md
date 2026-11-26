---
id: SR-967191
title: "integrity review of PR 4064 spec changes"
type: SpecReview
analysis: integrity
scope: "agent-ix/filament-spec-language; spec/functional/FR-189, FR-192, FR-203, FR-315, FR-896; spec/test-cases/TC-624, TC-900, TC-3241"
review_set: subset
---

## Summary

Ticket: QSL-3497. PR 4064 at a8b116f24c243dbacdb05ec6ae6559c00900359c. 4 finding(s).

## Verdict

The cause and order changes agree across FR-125, FR-189, FR-192, FR-203 and FR-315 for the cases their criteria test. The ruled amendments are present, including the checker deriving the causes by its own code.

## Findings

| ID | Severity | Summary | Refs | Escape Cause |
| --- | --- | --- | --- | --- |
| FND-001 | medium | The checker must return NoInitialState/NoInstance, but FR-201's return type has no such member and FR-201 is not amended | spec/functional/FR-203-bound-an-exact-run-and-settle-its-verdict.md:346 | missing-requirement |
| FND-002 | low | "The causes every other route reports" is false: FR-315 checks form before instancing and FR-189 leaves the order unstated | spec/functional/FR-203-bound-an-exact-run-and-settle-its-verdict.md:148-151 | wrong-requirement |
| FND-003 | low | "Whatever document is offered" conflicts with the retained rule for malformed and identity-mismatched certificates | spec/functional/FR-203-bound-an-exact-run-and-settle-its-verdict.md:153-157 | wrong-requirement |
| FND-004 | low | Row 7 says "one key" where FR-125 says one universe and key, overlapping row 10 | spec/functional/FR-315-encode-the-smt-lib-transition-relation.md:146 | wrong-requirement |

## Detail

- FND-001: AC-35 requires FR-201's checker to return NoInitialState or NoInstance, and the new Behavior bullet calls it a typed cause that is not acceptance, rejection, malformed or unsupported. FR-201, which owns that checker, is unchanged: its return is CertificateCheck {Accepted, Rejected, Stopped}, and FR-921 lists Accepted or Rejected or ProbabilityDocumentError. There is no member the checker can return, so the criterion cannot be implemented against the published interface. Amend FR-201 (and FR-921 where it applies) with the member and its position relative to document reading.
- FND-002: "These are the causes every other route reports for the same subject" is false as written. FR-315 applies item-form, profile and query-kind (rows 1 to 3) before its instancing rows, so an empty-initial subject with a TP-4 clause gives item-form there and no-initial-state here. FR-189 does not order the instancing outcomes against EveryScheduler, MissingConfidence or ExactOnlyForm, so the statistical route may report either. Deciding before form admission is consistent with FR-125 and FR-875; drop or narrow the sentence, and say in FR-189 where its form refusals fall.
- FND-003: The bullet says that for these subjects the checker returns the typed cause for whatever document is offered and never reports it malformed. The unchanged Settlement text says other malformed certificates, identity mismatches and refusals SHALL retain FR-201's existing typed rule and locus. For a malformed or wrong-identity document over an empty-initial subject the two require different results (cancellation and byte limits are also unstated). AC-35 tests only a well-formed document of the claim's identity. State the precedence.
- FND-004: Row 7 refuses member records that give "one key" two types. FR-125 refuses only records with one universe and key, over the populations that overlap the over type; a key in two unrelated universes names two objects, which is row 10's constant-name case ("two objects under one key"). Read literally, row 7 takes that case first as conflicting-identity. Write "one universe and key".

## Examined scope

- examined: FR-203-AC-35 (spec/functional/FR-203-bound-an-exact-run-and-settle-its-verdict.md)

```text
FR-201's certificate checker, called directly with a well-formed document of the claim's identity, returns `NoInitialState` for AC-20's subjects and `NoInstance` for AC-34's, for an `on each` claim and for a claim that is not, and neither accepts the document nor reports it rejected, malformed or unsupported. The test states each expected cause itself and takes none from an engine result.
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

- examined: FR-203 (spec/functional/FR-203-bound-an-exact-run-and-settle-its-verdict.md)

```text
### Initial states and instances

- Before it admits the claim's form, builds a product, writes a certificate
  or reads one, EN-5 SHALL decide FR-125's instancing outcomes in FR-125's
  order: `RequiresBound`, `NoInitialState`, `conflicting-identity`,
  `NoInstance`. This holds for every claim form and activation.
- When the subject's `initial` list is empty and no universe is missing,
  EN-5 SHALL report `NoInitialState`, whatever the universes hold.
- When the `initial` list is non-empty and the claim has no instance, since
  its `over` parameter or its `on each` activation type has no admi
```

- examined: FR-315 (spec/functional/FR-315-encode-the-smt-lib-transition-relation.md)

```text
| 4 | `subject-admission` | FR-120's admission of the subject refuses, carrying that refusal, other than for a key with two types (row 7) | the subject |
| 5 | `requires-bound` | A population that admits the clause's `over` type has no universe (FR-125 `RequiresBound`) | the population |
| 6 | `no-initial-state` | The subject's `initial` list is empty (FR-125), so every query would be unsatisfiable | the subject |
| 7 | `subject-admission` | Two member records of the subject give one key two types (FR-125), carrying `invalid_runtime_input`/`conflicting-identity` | the subject |
| 8 | `no-insta
```

- examined: FR-315-AC-62 (spec/functional/FR-315-encode-the-smt-lib-transition-relation.md)

```text
Instancing outcomes are refused in FR-125's order. Over FR-315-AC-1's `Counter` with no universe for the population of `c` and an empty `initial` list, `encode_smt_query` refuses `requires-bound` naming the population, not `no-initial-state`. Over FR-125-AC-13's records, an `Animal` under `c` in `animals` and a `Dog` under `c` in `dogs`, a claim over `Animal` refuses `subject-admission` carrying `invalid_runtime_input`/`conflicting-identity`. Over `Counter` with the universe `[c]`, one initial state that holds no object and an operation `setTo(n: Integer)`, it refuses `no-instance`, not `objec
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

- context_only: FR-201 (spec/functional/FR-201-check-a-probability-certificate.md)

```text
pub enum CertificateCheck { Accepted { bound_side: Bound }, Rejected(CertificateRejection), Stopped(ExactProbLimit) }
```

- context_only: FR-125 (spec/functional/FR-125-read-a-model-subject-s-behaviours-as-temporal-traces.md)

```text
- FSL SHALL decide the item's instancing outcomes in this order, each before
  it explores any state:
  1. When a population that overlaps `T` has no universe, FSL SHALL return
     `NotSimulated::RequiresBound` naming it (FR-120).
  2. When the subject's `initial` list is empty, FSL SHALL report
     `NoInitialState`, whatever the universes hold.
  3. The `conflicting-identity` check above.
  4. When the claim has no admitted key, because no population overlaps `T`,
     every overlapping population has an empty universe, or no member record
     of an initial state admits a key of their univ
```

