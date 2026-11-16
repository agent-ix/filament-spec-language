---
id: SR-2026111605245700000001
title: "Spec review of the FR-766 count kernel ruling"
type: SpecReview
analysis: base
scope: "agent-ix/filament-spec-language; branch fsl-spec-count-kernel 45fd33275, git diff origin/main...HEAD (FR-766, FR-495, TC-2240, check_mode.rs, QSL-2003 ignore notes)"
review_set: subset
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-766
    type: reviews
  - target: ix://agent-ix/filament-spec-language/FR-495
    type: reviews
---

## Summary

Ticket: QSL-2003. This branch settles the conflict between FR-766 and FR-495.
The ruling is that an unproved `count<N>` in a declared invariant, pre or post
refuses at check under both modes. The Kernel run-time refusal applies only to
temporal clause atoms and to function bodies evaluated at run.

Verdict: REQUEST CHANGES (one high finding).

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | FR-268-AC-3 still requires a variant of the invariant `AllBelow` whose domain evaluation is refused at run: "a `count` into `Int[0, 0]` of a one-element collection, in a package checked under `CheckMode::Kernel`". Under the new FR-766 and FR-495 that invariant refuses at check in both modes, so the AC can no longer be met and contradicts FR-766. The FR-766 Notes and the QSL-2003 ignore notes tell that fixture to expect the check refusal instead. That drops FR-268's refused-domain step-2 case. The Notes' other option, moving the atom to a temporal clause, does not apply: a separation domain is an invariant's. Fix: amend FR-268-AC-3 so the domain calls a function whose body holds the `count<Int[0, 0]>`, for example `function z using v(x: Integer): Int[0, 0] pure { count<Int[0, 0]>(m in ints(sequence[x]): m > 0) }`, in a package checked under Kernel. The count is then outside a declared clause, and the refused-domain case is kept. Update the FR-766 Notes (line 138) and the witness.rs part of the three ignore notes to match. | spec/functional/FR-268-check-a-state-clause-witness-on-replay.md:98; spec/functional/FR-766-prove-the-result-range-of-count-at-check.md:130-142; fsl-eval/tests/it/collection_queries.rs:2093 |

## Scope and evidence

- The ruling is stated the same way in FR-766, in the Kernel bullets and Notes,
  and in FR-495, in the declared-clause bullet. The relationship edges were
  added in both directions.
- `count<N>` is not narrowed. Its typing, the Linked obligation and the
  function-body and temporal-atom Kernel paths are unchanged. AC-1 to AC-7 are
  untouched and still consistent: AC-5 and AC-6 already refuse a declared
  invariant at compile.
- The FR-766-AC-8 / TC-2240 stub is ignored with key QSL-2003 and tagged
  `#[trace("TC-2240", "FR-766-AC-8")]`. It reuses the existing
  `admit_and_assemble_with_body`, `tc465_document` and `kernel_drops` helpers,
  and its imports are present. It makes a real `[Unproved(Range)]` assertion
  under both modes, plus the Kernel function-body contrast.
- The ignore notes are correct for the temporal fixtures:
  fsl-replay/src/spine/clause/tests/temporal.rs:35-141 holds `temporal D0..`
  atoms, and fsl-analyze/tests/it/monitor_inputs.rs:45-55 holds
  `temporal Refuse`. They are also correct that witness.rs:84 is the
  `RefusedDomain` invariant (test `tc_743_the_separation_check_names_its_failing_step`
  at witness.rs:1684). That is the fixture FND-001 concerns.
- IDs: TC-2240 is absent from origin/main, qsl-2121-code and qsl-1964-code.
  FR-766-AC-8 follows AC-7.

## Verdict

REQUEST CHANGES for FND-001. The rest of the change is consistent and ready.

## Dispositions

| ID | Disposition | Resolution |
| --- | --- | --- |
| FND-001 | fixed | FR-268-AC-3 keeps the refused-domain case. The domain now calls `function z using v(x: Integer): Int[0, 0] pure { count<Int[0, 0]>(m in ints(sequence[x]): m > 0) }` in a Kernel-checked package, so the count sits in a function body outside the declared clause. The FR-766 Notes say a called function's body is checked under its own declaration's mode and its unproved range belongs to that body, not to the clause, which sees `Int[0, 0]` from the signature. So it composes with FR-495. The FR-766 behavior bullet names called function bodies. TC-743 and the four QSL-2003 ignore notes say the witness.rs fixture moves its count into `z` and keeps its step-2 refusal record. |
