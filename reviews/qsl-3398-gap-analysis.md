---
id: SR-4574
title: "gap analysis of QSL-3398 FR-569-AC-5/6 (FSL PR #3963)"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/filament-spec-language@25173079b60e0031e1fe0c86e295ed0cf856a7de; diff only: FR-569-AC-5, FR-569-AC-6, TC-1408 steps 4-5, fsl-semantics/tests/it/source_objects.rs"
review_set: subset
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-569
    type: references
---
# SR-4574: gap analysis of QSL-3398 FR-569-AC-5/6 (FSL PR #3963)

## Summary

Ticket: QSL-3398. PR agent-ix/filament-spec-language#3963 at 25173079b6. Planless, limited to the
diff. `quire trace --id` resolves FR-569-AC-5 to two tagged tests
(`the_source_and_model_twins_link_equal_tables_and_check_equal_types`,
`a_redefinition_that_widens_the_frame_refuses_effect_escape_on_both_routes`) and FR-569-AC-6 to
eight tagged fsl-package emit tests. No criterion in scope is untagged. No production code changed,
so there is no reverse gap; no stubs in the diff. Semantic review was done for the two criteria
in scope only, by reading the tagged tests against the new AC-6 sentences and the author's
probe-emit log (uncommitted probe, head f9db25eff9, production code identical to the PR head).

Plan completion: not assessed

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | AC-6's new sentences have no tagged test: the dispatch row's `Order.size` handle naming `["Order","size","effective_precondition"]` (not `Qs`'s function), `Sub.size` with none in S, and Sub's own in S-R. `a_source_operation_declares_its_effective_precondition_function` only checks that a node with that name exists, is tagged `TC-1408` only, and does not read the candidate handle. FSpec FR-322-AC-544 (PR #848, SR-85913 FND-002 deferred here) needs the same: read the emitted candidate's `effective_precondition` handle, resolve it to the `[..,"effective_precondition"]` node, assert it is not the authored precondition node, and tag `FSpec-FR-322-AC-544` (and `FR-569-AC-6` if it covers AC-6's sentence). Test-only, inside the ruling's scope. | fsl-package/src/emit/tests/source_objects.rs:2357-2389, spec/functional/FR-569-declare-object-types-populations-and-operations-in-fsl-source.md:685 |
| FND-002 | medium | AC-6 (in the edited row) says `C`'s and `B`'s body functions carry "one `declaration` occurrence at the authoring declaration's span", and the FR-569 bullet says "for a body, the span of the `body` declaration". The probe-emit log shows `["Order","size","body"]` at span `n` (785-786) and `["Sub","size","body"]` at `m` (746-747): the body expression, not the `body` declaration. No test asserts it. Same class as SR-7932 FND-001; ticket with it. | fsl-semantics/src/check/dispatch_link.rs:529-540, spec/functional/FR-569-declare-object-types-populations-and-operations-in-fsl-source.md:544-547 |

## Coverage

- FR-569-AC-5: tagged; the twin test asserts the AC-5 rewrite (see SR-9476).
- FR-569-AC-6: tagged by eight tests; the sentences this PR adds and both occurrence claims are
  not asserted by any of them (FND-001, FND-002, SR-7932 FND-001).
- TC-1408 step 5 still reads "waits on QC-30" although tagged emit tests exist.
- Semantic review: done for FR-569-AC-5 and FR-569-AC-6 only.
- Plan completion: not assessed

## Verdict

CONDITIONAL. FND-001 is a test addition the coder can make in this PR. FND-002 is a pre-existing
spec/code mismatch that should be ticketed with SR-7932 FND-001.

## Dispositions

Round 1 reviewed head dd73650661eebaa7142ca31dedc717e95d524e1d.

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | `a_source_operation_declares_its_effective_precondition_function` now reads each candidate's emitted `effective_precondition` handle: Order's equals the `[Order, measure, effective_precondition]` node and differs from the authored `Qs` node, Sub's is null; tagged `TC-1408`, `FR-569-AC-6`, `FSpec-FR-322-AC-544`. |
| FND-002 | fixed | `clause_declaration_span` places a body at its `body` declaration; the new occurrence test asserts the literal text `body C using P on Order::measure(n) { n }`, `B`'s and an uncalled operation's body `T`. |
