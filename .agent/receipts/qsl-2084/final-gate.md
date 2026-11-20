# QSL-2084 final affected-runtime gate

Independent reviews SR-1485/SR-1486 passed without findings at feature source head `e6d7dfdfc70a0e75f3459b4c96229527a460b17f`. The latest validated review artifacts were committed unchanged at `06d70724550d3f45462f55b4b86e17ceb15accf3`; this was the gate head. No source, main refresh, lint retry, review sweep or feature-mode rebuild accompanied the gate.

With 210G free, one logical complete affected-runtime gate executed these nonoverlapping integration-test segments:

| Segment | Passed | Failed | Ignored | Exit |
| --- | ---: | ---: | ---: | ---: |
| Exact top-level checked-expression argument refusal case | 1 | 0 | 0 | 0 |
| Exact top-level closed/protocol/kernel charge case | 1 | 0 | 0 | 0 |
| All `model_simulation::` tests | 68 | 0 | 1 | 0 |
| All `named_population::` tests | 14 | 0 | 0 | 0 |
| All `refinement_populations::` tests | 12 | 0 | 0 | 0 |
| All `type_invariants::` tests | 16 | 0 | 0 | 0 |
| **Total** | **112** | **0** | **1** | **0** |

The 112 passes include the twelve new cases. Each actual case executed once in this final gate. Top-level leaf names used `--exact`, preventing the nested evaluator modules from being run twice. Every segment used `cargo test --locked -p fsl-eval --test it`, the owned warm target, jobs 2, debug 0, incremental 0, one test thread and both user skips. No discovery/list run or zero-test segment occurred. The model-simulation segment completed in 1360.06 seconds; all actual processes exited before this receipt was written.

The inherited ignored case is `model_simulation::a_float_field_model_explores_by_its_ieee_bits`, explicitly pending QSL-2176. It is not counted as coverage and was not changed or reprobed.

The previously accepted foundation `thumbv7em-none-eabi` no-default build and package host no-default check both retain their exit-0 receipts on unchanged inputs; neither was rerun for this gate. They prove only their current NFR-015 rows. Independent no_std checked-term evaluation, its real consumer and std retirement remain pending under IR-590 and IR-593, not covered by these std cases.

Private per-segment complete logs, exact exits and aggregate exit are under `/home/peter/dev/.agent/qsl-2084/logs/`, named `final-top-admission`, `final-top-charges`, `final-model-simulation`, `final-named-population`, `final-refinement-populations`, `final-type-invariants` and `final-aggregate.exit`. The scoped lint limitation remains recorded in the authoring validation receipt; no green lint claim is added.
