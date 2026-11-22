---
id: SR-967175026302197768900474181941935084693
title: "Code review of the QSL-2811 layering repairs"
type: SpecReview
analysis: code-review
scope: "agent-ix/filament-spec-language; QSL-2811 and QSL-2792; PR 3461; the 24 changed Rust and manifest files"
review_set: subset
---

## Summary

Reviewed the frozen layering-repair diff, its public consumers, the task counterexample writer/reader/replay path, and its governing requirements. No source defect was found in the assigned change; compilation and runtime acceptance remain unmeasured under the titan reservation.

## Verdict

**CONDITIONAL** — no source findings. This is not merge readiness: the named titan checks and the required workspace all-target compilation check have not run.

## Scope

```yaml
scope:
  - id: FR-100-AC-8
    path: spec/functional/FR-100-run-a-named-function-through-the-spine.md
    role: examined
    excerpt: "No public item of `fsl_replay::spine`, and no `fsl_replay` re-export, names a `fsl_eval` path; a `pub use` of a `fsl_eval` item from `fsl_replay`, or a `fsl_eval` type in `spine::run`'s signature, fails the check."
  - id: FR-062-AC-6
    path: spec/functional/FR-062-implement-checked-family-contract.md
    role: examined
    excerpt: "Every other family's evaluation hook, invoked on a checked node built only from checked input, returns without reading any CST, token or display string."
  - id: FR-087-AC-1
    path: spec/functional/FR-087-typestate-and-cross-package-node-key.md
    role: context_only
    excerpt: "Each of `CheckedGraph` (in `check`), `CheckedPackage` (in `package`), `EmittedPackage` (in `package`), `ImportView` (in `library`) has exactly one defining location, with every field and every constructor (the implicit struct-literal constructor, or an explicit `new`) private to that owning module, and every external read of the type's state goes through a named accessor."
  - id: FR-400-AC-9
    path: spec/functional/FR-400-separate-checked-terms-from-the-checker.md
    role: examined
    excerpt: "`filament-checked` builds for `thumbv7em-none-eabi` with default features off, and its manifest forbids unsafe code; no normal or build dependency in FSL enables its `test-support` feature."
  - id: FR-249-AC-3
    path: spec/functional/FR-249-lower-a-task-automaton-to-a-timed-subject.md
    role: context_only
    excerpt: "The lowered EN-6 subject preserves FSpec FR-419's task-product state and FR-364's derived-step identity across a preemption/resumption sequence; FR-237 replays the resulting existing `ModelTrace` arm against the rebuilt product."
```

Source paths examined in the diff:

- `fsl-analyze/src/analyze/task_product_item.rs`, `fsl-analyze/src/zone_check/task_symbolic.rs`, `fsl-analyze/tests/it/task_product.rs`.
- `fsl-bench/Cargo.toml`, `fsl-bench/src/prop/unit.rs`.
- `fsl-command/src/command/package_run.rs`, `fsl-command/src/command/tl/package_observed.rs`.
- `fsl-eval/Cargo.toml`, `fsl-eval/src/simulation/model/input.rs`, `fsl-eval/src/simulation/model/reference_bindings.rs`, `fsl-eval/src/simulation/protocol/package_program.rs`.
- `fsl-replay/src/certificate/zone_certificate/task_check.rs`, `fsl-replay/src/lib.rs`, `fsl-replay/src/model_check/replay.rs`, `fsl-replay/src/outcome/counterexample.rs`, `fsl-replay/src/replay_model_input.rs`, `fsl-replay/src/spine/clause/model_trace.rs`, `fsl-replay/src/spine/clause/temporal.rs`, `fsl-replay/src/spine/clause/temporal/package_selection.rs`, `fsl-replay/src/spine/clause/temporal/package_window.rs`, `fsl-replay/src/spine_eval_api.rs`, `fsl-replay/src/task_transition.rs`.
- `fsl-semantics/src/check/mod.rs`, `tests/it/request_surface_s1_16.rs`.

Context included ADR-011 section 6.1; the unchanged TC-452 scanner and its probe tests; `refinement_query.rs`; the evaluator-owned task alphabet and lasso-position hook; `outcome.rs` task wire adapters; the typed counterexample readback and replay; command valuation consumers; and the benchmark generator test.

## Review evidence

- Applied Rust API, implementation, safety, wire and testing guidance, plus repository guidance. No applicable AssuranceProfile was found under `spec/`.
- The selected evaluator remains in a borrowed private carrier, and the complete ModelCheckInput is moved or borrowed through its carrier. The request does not substitute a new package evaluator or rebuild its model world.
- The observed valuation conversion moves all four fields. The retained attribution result is kept intact behind its carrier.
- The replay-owned task alphabet converts all eight actions exhaustively in both directions, moves every identity field, and preserves all three product transition arms. Its subject-transition lasso hook agrees with the evaluator's existing hook. The task writer, typed reader, TypeId guard and replay conversion consistently select the new replay type; conversion back to the evaluator alphabet occurs before product replay.
- Existing command/analyze callers and task certificate-template conversions were updated together. The new conversion test is a real production-path test with assertions on serialized identity fields, distinct actions, round-trip equality and non-task arms; the existing wire and task consumers remain the runtime regression checks.
- StateClauseKind and ProtocolNodeKind join the existing AccessOrdering export at layer 3. Production evaluator uses resolve there; the remaining forms references are test-only or documentation. The forms dependency is now dev-only.
- The benchmark feature enables only proptest. Grammar derivation support is a dev dependency, and the existing required-feature generator test still directly calls the real derivation producer. Removing the unused shipped grammar_units helper follows the explicit owner ruling; no replacement shipped generator API or compatibility shim was added.
- No workflow, scanner, allow entry or existing layering assertion changed. `git diff --check origin/main...HEAD` exited 0.

## Validation limits

No builds, tests, compiled repository scanners, cargo metadata/check/clippy invocations or titan jobs were run or queued. The PR body lists the required named TC-452, dependency, generator, task conversion/wire/certificate and command temporal-consumer checks, plus the required workspace and benchmark library compilation checks. All remain pending.

The review used source inspection and static Quire output, not runtime evidence. Exact runtime model identity is not exposed by this session and is recorded as unavailable in the tracker marker.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No findings (placeholder) | - |
