---
id: TC-3040
title: "Optional operation parameters are option-typed, omissible and admitted present or absent"
type: TC
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-056
    type: verifies
  - target: ix://agent-ix/filament-spec-language/FR-103
    type: verifies
  - target: ix://agent-ix/filament-spec-language/FR-106
    type: verifies
---
# TC-3040: Optional operation parameters are option-typed, omissible and admitted present or absent

## Description

Verify that FSL intake reads an operation parameter's `presence`, types an
`optional` parameter `Option<X>`, checks it under the option rules, lets a
call omit it in a trailing position, and admits an invocation that passes it
present, absent or not at all. Scope: FR-056-AC-65, FR-103-AC-12,
FR-106-AC-71.

## Test Procedure

Fixture: FSpec TC-2054's golden `test/refunds` domain package and unit (object
types `Staff`, `Order`, `Desk`; query operation `Order.note(firstApprover:
Staff required, secondApprover: Staff optional): Boolean`; clauses `NoteBy`,
`NoteVal` and `NoteSame` on `R::Order::note`), selected as `R`. The staged
example `shop/staged-optional-parameter` uses the same shape.

1. Admit the package and its `"maybe"`, absent-presence, `OptStaff`
   `optional` and `OptStaff` `required` variants
   (`fsl-semantics/tests/it/operation_clauses.rs`,
   `#[trace("TC-3040", "FR-056-AC-65")]`).
2. Check the clauses and the `Desk` call forms of FR-103-AC-12
   (`operation_clauses.rs::present_over_an_optional_parameter_checks`,
   `a_call_may_omit_an_optional_parameter`,
   `a_call_may_not_omit_a_required_parameter`,
   `value_of_an_optional_parameter_without_a_presence_guard_refuses_unproved_presence`,
   `deref_over_an_optional_parameter_follows_the_option_rules`; tag
   `FR-103-AC-12`, with `FSpec-FR-151-AC-20` and `FSpec-FR-322-AC-476` on the
   call tests).
3. Run the five invocations of FR-106-AC-71
   (`fsl-eval/tests/it/model_operations.rs::a_run_passes_present_or_absent_for_an_optional_parameter`,
   tag `FR-106-AC-71`).

## Expected Results

- Step 1: as FR-056-AC-65 states, with the refusals at
  `Operation.params[1].presence`.
- Step 2: as FR-103-AC-12 states; the omitted and explicit-`absent` calls give
  equal checked forms.
- Step 3: the absent, present and omitted invocations are admitted and
  `NoteBy` evaluates false, true and false; the bare-reference invocation
  refuses `invalid_runtime_input`/`wrong-value-kind` at
  `parameters.secondApprover`; omitting `firstApprover` refuses
  `invalid_runtime_input`/`missing-member` at `parameters.firstApprover`.
