---
id: SR-2026111612172400000051
title: "Gap analysis — QSL-2252 reward evaluator port"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/filament-spec-language; fsl-semantics/src/check/reward_evaluator.rs; fsl-semantics/src/check/reward_validation.rs; fsl-eval/src/value/expression/mod.rs; fsl-replay/src/spine/lifecycle.rs; fsl-semantics/tests/it/probabilistic_declarations.rs; FR-185-AC-13; TC-3027"
review_set: subset
relationships:
  - { target: "ix://agent-ix/filament-spec-language/FR-185", type: reviews }
  - { target: "ix://agent-ix/filament-spec-language/TC-3027", type: reviews }
---

## Summary

Ticket: QSL-2252. Compared the changed S3/S6a reward path and tests with FR-185-AC-13 and TC-3027. The code establishes the required port, adapter and spine call path. TC-3027 step 2 has a coverage gap: the two S3 recording-evaluator tests use a fixture with a pre-existing reward and assert on that reward rather than on the `refund` reward named in the test procedure.

## Verdict

**CONDITIONAL** — one medium coverage finding for TC-3027 step 2.

## Scope examined

- FR-185's “The reward evaluator port” text and FR-185-AC-13
- TC-3027 test procedure steps 1-2 and their expected results
- `fsl-semantics/src/check/reward_evaluator.rs`
- `fsl-semantics/src/check/reward_validation.rs`
- `fsl-eval/src/value/expression/mod.rs`
- `fsl-replay/src/spine/lifecycle.rs`
- `fsl-semantics/tests/it/probabilistic_declarations.rs:3505-3576`
- `fsl-analyze/tests/it/reward_evaluator_port.rs:1-40` (S6a-backed test for step 1)

## Findings

| ID      | Severity | Summary | Refs |
| ------- | -------- | ------- | ---- |
| FND-001 | medium | TC-3027's recording-evaluator tests do not exercise the specified `refund` reward. | fsl-semantics/tests/it/probabilistic_declarations.rs:3509-3532, 3559-3575; TC-3027 step 2 |

## Coverage analysis

The S6a-backed analyzer test covers TC-3027 step 1's actual expression result for `refund` over the random support. The two S3 tests cover canonical-vector stopping and preservation of an undefined diagnostic in general, but `service(None, extra)` includes the baseline `duration` reward before the extra declaration. The negative test confirms this by expecting `duration` in the refusal. Consequently neither recording test proves the specified result is attached to `refund`; an evaluator assertion on the reward identity or a fixture replacing `DURATION` is needed to bind the port behavior to the requirement's case.

The `quoin matrix --repo . --json` static matrix lists the analyzer test and both S3 tests as binders for FR-185-AC-13. This manual check refines that static result: tags exist, but the two S3 binders do not exercise the reward identity prescribed by TC-3027 step 2. Optional semantic review was not run because the dispatch did not request it.

## Verification

- Read FR-185-AC-13 and TC-3027, then followed both specified test steps to their tagged tests.
- `quoin matrix --repo . --json` — completed; inspected the FR-185-AC-13 static binder record.
- At the fix head, `cargo test -p fsl-semantics --test it probabilistic_declarations::s3_` through `~/dev/worktrees/locked-build.sh` with debuginfo and incremental compilation disabled — passed, 2 tests.
- No `fsl-analyze` tests were run.

## Dispositions

Round 1 checked against PR head `cfa4a3ccd845e60b3e2c01d8cf4008fbc1fb3e81` and its fix commit.

| FND | outcome | reason | after_excerpt |
| --- | --- | --- | --- |
| FND-001 | fixed | The tests now replace the baseline duration reward with refund, and assert evaluator calls/refusal provenance for refund. | `assert_eq!(\n    evaluator.0.into_inner(),\n    ["refund"],\n    "the recording evaluator sees the intended reward"\n);` |

