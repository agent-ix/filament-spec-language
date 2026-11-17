---
id: TC-2195
title: "A linked count proves its result range"
type: TC
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-766
    type: verifies
---
# TC-2195: A linked count proves its result range

## Description

A `count<N>` checked under `CheckMode::Linked` proves `[0, size maximum]`
inside `N` or is refused `unproved-range`; under `CheckMode::Kernel` the
final count is refused `integer_out_of_domain` at run.

Scope: FR-766-AC-1, FR-766-AC-2, FR-766-AC-3, FR-766-AC-4, FR-766-AC-5, FR-766-AC-6, FR-766-AC-7.

## Test Procedure

1. Check `count<Int[0, 3]>`, `count<Int[0, 5]>` and `count<Integer>` over sequence, bag and set sources bounded `[0, 5]`. Tests: `fsl-eval/tests/it/collection_queries.rs` `q14_count_refuses_unproved_range_at_check` and `q14_count_proves_when_the_bound_fits`.
2. Check the count under a `size(q) <= 3` and a `size(q) <= 4` guard. Test: `q14_count_under_a_size_guard_proves`.
3. Check `count<Int[1, 3]>` over `[0, 3]` and `count<Int[0, 5]>` over an unbounded source. Test: `q14_count_refuses_unproved_range_at_check`.
4. Check and evaluate the kernel-mode count over five `true` occurrences. Test: `coerce_and_count_refusals_carry_the_declared_domain`.
5. Compile the `AtMostOne` invariant unit with `One = Int[0, 1]` and `Int[0, 2]`. Test: `fsl-replay/src/spine/clause/tests/clause_paths.rs` `count_out_of_domain_names_its_cause`.
6. Run `AtMostOne` of the `Int[0, 1]` unit as a clause run and build its run item. Test: `clause_paths.rs` `a_count_refused_at_compile_is_a_declined_run_item_with_its_cause`.
7. Project a clause-run report whose disposition is a kernel `integer_out_of_domain`/`outside-domain` record. Test: `clause_paths.rs` `a_kernel_refusal_record_names_its_cause_on_the_run_item`.

Tag the tests `#[trace("TC-2195", "FR-766-AC-N")]` with the criterion each asserts.

## Expected Results

- Step 1: `Int[0, 3]` refuses `undefined_expression`/`unproved-range`, required `[0, 3]`, proved `[0, 5]`; the others check.
- Step 2: the `<= 3` guard checks; the `<= 4` guard refuses with proved `[0, 4]`.
- Step 3: required `[1, 3]` proved `[0, 3]`; proved interval with no upper bound.
- Step 4: `integer_out_of_domain`/`outside-domain`, `expected` `Int[0, 3]`.
- Step 5: `Int[0, 1]` refuses at compile `undefined_expression`/`unproved-range`, exit 20; `Int[0, 2]` compiles.
- Step 6: a compile disposition; the item is `refusal`, `declined`, cause `{code: undefined_expression, kind: unproved-range}`.
- Step 7: the item is `refusal`, `declined`, cause `{code: integer_out_of_domain, kind: outside-domain}`, never `null`.
