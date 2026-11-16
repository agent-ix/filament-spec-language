---
id: SR-2026111610291804
title: "Code review of the FSL wave 6 spec readiness batch test stubs"
type: SpecReview
analysis: code-review
scope: "agent-ix/filament-spec-language; PR #2747 branch fsl-spec-w6 at 0ed71f1385f075422c2d149b4ba0f2fd4e3dc188: fsl-analyze/tests/it/{analyze_cli,analyze_coverage_cli,analyze_family_cli,analyze_reach_rulings,main}.rs, tests/it/{compile_package_output,control_node_lowered_type,record_sample_counterexample,main}.rs"
review_set: subset
evaluated_revision: "0ed71f1385f075422c2d149b4ba0f2fd4e3dc188"
---

## Summary

Ticket: QSL-2296 (wave 6 batch). The diff is test stubs only, and it includes the rust-review lane. Every new or edited test is `#[ignore = "<key>: ..."]` and carries `#[trace(...)]`. The edited tests `possible_claims_prove_and_refute`, `simulation_enumerates_the_counter_subject`, `always_possible_refutes_and_witnesses_repeat`, `digital_clocks_agree_and_the_clock_budget_stops` and `an_item_naming_a_registered_provider_settles_on_it` were already ignored, so no live test changes.

The diff adds no Python and no shell logic. These commands passed through `qsl-slot`:

- `cargo check --tests -p fsl-analyze -p filament-spec-language` exits 0.
- `cargo fmt --all -- --check` exits 0.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | The FR-621-AC-10 stub is ignored under the spec ticket `QSL-2296`. The code ticket for `backend` by `settled_by` is QSL-2304. Fix: `#[ignore = "QSL-2304: the analyze record does not write backend by settled_by yet"]`. | fsl-analyze/tests/it/analyze_reach_rulings.rs:299 |
| FND-002 | medium | The FR-641-AC-8 / FSpec FR-352-AC-19 stub is ignored under `QSL-2133`. The code ticket for the run result's `limits` member is QSL-2303. Fix: key the ignore to QSL-2303. | tests/it/record_sample_counterexample.rs:113 |
| FND-003 | medium | FR-810-AC-3 (`compile_outcome_with_overrides` with `RequestOverrides::output`, and the doubly supplied refusal) has no stub, although TC-2297 step 3 names `tests/it/compile_package_output.rs`. Fix: add an ignored stub keyed QSL-2299 traced `#[trace("TC-2297", "FR-810-AC-3")]`. If `RequestOverrides::output` does not exist yet, assert the observable effect through a request that carries both. | tests/it/compile_package_output.rs |
| FND-004 | medium | Two tests traced to FR-489 now contradict the amended ACs. `linked_engine_registrations_expose_the_owned_descriptors` is live, traced to FR-489-AC-5, and asserts the exact registration list without `en-7`. `production_snapshot_contains_only_linked_engine_registrations` (FR-489-AC-1/AC-5) is documented as EN-1, EN-4, EN-5 and EN-6 only. Today the first is a passing test bound to an AC it contradicts. Once QSL-2293 registers EN-7 it goes red. The new stub `the_linked_registrations_include_en7` (QSL-2293) also duplicates the existing `the_fsl_registry_holds_en7` (QSL-2131, FR-620-AC-8). Fix: record on QSL-2293 that its PR updates both live tests and retires one of the two EN-7 stubs. | fsl-analyze/src/analyze.rs:2184-2207; xtask/src/refinement/layering.rs:1216-1218; fsl-analyze/tests/it/analyze_reach_rulings.rs:318-343; fsl-analyze/tests/it/analyze_request_wire.rs:144-155 |
| FND-005 | low | The FR-635 stubs do not assert the decided design. The AC-4 stub leaves the misadvertising-plugin refusal to the coder (doc comment), and its remaining assertions pass today. The AC-1 stub still selects the provider through the `FSL_TEST_PROVIDER` env var and `counter_request`, not a workspace plugin passed to `analyze_with_providers`. `analyze_with_providers` does not exist yet, so the stub cannot call it and still compile. Fix: record on QSL-2132 that its PR rewrites both stubs against `analyze_with_providers` and the workspace test plugin, including the refusal half. | fsl-analyze/tests/it/analyze_reach_rulings.rs:258-293; fsl-analyze/tests/it/analyze_family_cli.rs:315-321 |
| FND-006 | low | The edited FR-628-AC-3 stub checks only that the record echoes `witness_samples` 3 and seed 7. It does not check the decided behaviour: exactly one sampled witness, whose source names seed 7 and a trace index below 3. Fix: assert the witness count and its source's seed and trace index. | fsl-analyze/tests/it/analyze_coverage_cli.rs:173-183 |

## Verdict

REQUEST CHANGES for FND-001 to FND-003, which are in this PR. FND-004 and FND-005 are recorded for the code tickets. The other stubs are keyed correctly and assert the decided behaviour for real:

- QSL-2129: the digital-clock route.
- QSL-2125: state-graph evidence, budget stops and instance order.
- QSL-2132: simulation traces.
- QSL-2299: the FR-810-AC-1 and AC-2 stubs.
- QSL-2300: the control-node hull.
- QSL-2133: record-sample counterexample steps.
