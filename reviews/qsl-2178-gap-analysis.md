---
id: SR-17952655000002
title: "QSL-2178 gap-analysis of frozen PR 3303"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/filament-spec-language; QSL-2178 QSL-2717; frozen PR 3303"
review_set: subset
---

## Summary

Ticket: QSL-2178; linked QSL-2717. The engine-backed static matrix exposes repository assurance backlog, and the two new snapshot AC bindings provide incomplete behavioral evidence. No global or scoped unconditional acceptance is claimed.

## Verdict

FAIL

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | The only test bound to FR-106-AC-72 stores one owner c1, one repeated place ID, a present optional and a one-element sequence. It never admits the specified c1/c2 distinct identities, [c2.x,c1.x] sequence, or absent optional repeat. It can pass if admission merges owners, reverses sequences, or rejects absent ClockRefs. Implement TC-464 step 7 and assert both presence cases and exact ordered identities. | fsl-eval/tests/it/refinement_row_clock_places.rs:331-380 |
| FND-002 | medium | The only test bound to FR-106-AC-73 keeps x declared and substitutes a missing owner or the model node as a fake template; it never compiles the required x-removed/y-retained model. It asserts only DanglingReference, so wrong cause, document, owner, field, nested path, or target details still pass. Add TC-464 step 8 using the real formerly declared c1.x identity and exact refusal records for r, maybe and refs. | fsl-eval/tests/it/refinement_row_clock_places.rs:387-443 |
| FND-003 | high | The repository-wide static matrix is not clean: 1135 criteria are untagged and 81 are tagged only by ignored tests; FR-001-AC-7 is one live untagged example. These are repository assurance backlog, not a claim that this PR introduced all gaps. The complete machine output is retained at /private/tmp/qsl-third-matrix.json. A global gap PASS is unavailable. | spec/functional/FR-001-read-exact-source.md:Acceptance Criteria |

## Coverage

Plan completion: not assessed

Reconciliation: quire matrix (quire 0.52.0, engine 0.62.2), exit 0; static only, no evidence store read. 6136 criteria: tagged 4803, untagged 1135, tagged-by-ignored-test 81, method-without-symbol 104, retired 13. FR-106-AC-72/73 and FR-1040-AC-3/8 are tagged, but tags alone do not establish intent coverage. quire coverage exit 0 reports 5760 untracked symbols, 6608 unmatched tags, 22666 diagnostics; census 13467 Rust candidates, 12897 tagged, 12391 bound. Diagnostics distinguish unmarked-trace-id and symbol-id-collision; extraction is nonzero, not unavailable. Whole-tree public-surface and stub scans retained at /private/tmp/qsl-third-public-surface.txt and /private/tmp/qsl-third-stub-scan.txt. Changed behavior reverse mapping: codec/intake/evaluation/domain/timed data writes -> FR-1040/FR-106/FR-120; protocol sorting -> FSpec FR-464; hyper slots/source -> FSpec FR-402/FR-322; timed refinement -> FR-430. No newly introduced source stub, skipped test or lowered gate found. Full repository public-surface semantic ownership reconciliation was not exhausted; broader reverse audit remains incomplete and cannot establish global PASS. Findings preserve specific new AC evidence omissions separately from repository backlog. Existing baseline retirement prose can still appear untagged; the selected live FR-001-AC-7 sample is not retired. Semantic agreement for the dispatched changed behavior was explicitly requested and examined in this combined review; no unrelated optional expansion or subagents.

## Examined Scope

```yaml
scope:
  - id: FR-106
    path: spec/functional/FR-106-admit-snapshots-and-invocations.md
    role: examined
    excerpt: |-
      A statically `Reference<Clock>`-typed value
      does not name a population member and adds no required population through
      its target; its declared-place validation is independent of population
      completeness.
  - id: FR-106
    path: spec/functional/FR-106-admit-snapshots-and-invocations.md
    role: examined
    excerpt: |-
      `Reference<Clock>` target is checked against declared clock places
             under the same rule as an object field.
  - id: FR-531
    path: spec/functional/FR-531-evaluate-from-a-checked-package-read-from-its-bytes-alone.md
    role: examined
    excerpt: >-
      The bare route SHALL give the same result, the same refusal and the same
      charge
        log as the in-process route for one package and one request.
  - id: FR-1040
    path: spec/functional/FR-1040-check-and-lower-clock-place-references-and-presence-in-population-forms.md
    role: examined
    excerpt: |-
      `ref` of a place under a `Map` value refuses `ill_typed`/`type-mismatch`
          at the `ref` (FR-230 "Place identity");
  - id: FR-1040-AC-3
    path: spec/functional/FR-1040-check-and-lower-clock-place-references-and-presence-in-population-forms.md
    role: examined
    excerpt: "In a refinement row over `calls = {c1, c2}`, `only(c in M::calls where
      c.n = 2: ref(c.x))` and `count(c in M::calls where ref(c.x) = self.r)`
      check, lower with the binder `c` itself (typed `Reference<Call>`) as owner
      term, and evaluate as FSpec FR-829-AC-2 states; the same `count` in a
      state invariant (FR-391) checks and lowers."
  - id: FR-1040-AC-8
    path: spec/functional/FR-1040-check-and-lower-clock-place-references-and-presence-in-population-forms.md
    role: examined
    excerpt: Over a domain package `acme/rpc` whose object type `Call` has clock
      fields `x` and `y` and a field `r` typed `ix://acme/rpc/ClockRef`, a type
      definition of `kind` `reference` with `target`
      `ix://filament/native/Clock`, intake gives `r` the type `Reference<Clock>`
      as a data member, and the postcondition `self.r = ref(self.x)` of an
      operation `point` whose frame names only `r` checks with no `clock-write`
      refusal and lowers the write to a `filament.op.clock_reference.ref`
      application typed by the `scalar_type`/`clock_reference` node, which also
      types the field; after `point` the state's `r` hold
  - id: FR-106-AC-72
    path: spec/functional/FR-106-admit-snapshots-and-invocations.md
    role: examined
    excerpt: "Over [TC-464](../test-cases/TC-464-snapshots-and-invocations-admit.md)
      step 7's dense-time `Call` fixture, a raw snapshot storing the actual
      per-instance place identities of `c1.x` and `c2.x` in `r:
      Reference<Clock>`, `maybe: Option<Reference<Clock>>` and `refs:
      Sequence<Reference<Clock>>` admits, retaining those identities as data,
      the optional presence and the sequence order. Neither place identity is an
      object key, and neither target is resolved as a population member. Reading
      the admitted fields distinguishes `c1.x` from `c2.x`; the required
      evaluation of the stored `self.r` under FR-1040-"
  - id: FR-106-AC-73
    path: spec/functional/FR-106-admit-snapshots-and-invocations.md
    role: examined
    excerpt: Over [TC-464](../test-cases/TC-464-snapshots-and-invocations-admit.md)
      step 8's model variant without `x`, storing the formerly declared `c1.x`
      identity in `r`, in the present value of `maybe`, or at element 0 of
      `refs` refuses
      `dangling_reference`/`absent-target-in-complete-population`, naming the
      snapshot, owner population `calls`, object `c1`, the affected field and
      nested value path, and that undeclared target identity, with no admitted
      observation or truth value. A target's clock-reference static type or
      native Clock target alone does not make it declared.
  - id: FR-001-AC-7
    path: spec/functional/FR-001-read-exact-source.md
    role: examined
    excerpt: "Package declarations holding one record `Point` with field `x: Int[0,
      9]`, checked under the source reference of bytes `b` admitted as authority
      `a`, identity `u`, under the reference of bytes `b'` admitted as `a`, `u`,
      under the reference of `b` admitted as authority `c`, identity `u`, and
      under the reference of `b` admitted as authority `a`, identity `v`, give
      `Point` the same node key `{package, path: [\"Point\"], ordinal: 0}` each
      time: the source reference is no part of a node key."
```
