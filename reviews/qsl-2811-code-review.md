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
  - id: FR-237/Reading the wire
    path: spec/functional/FR-237-replay-a-timed-counterexample.md
    role: examined
    excerpt: "A step whose `type` lies outside `T`'s alphabet is not a shape fault. The reader returns it as an alphabet refusal, `invalid_runtime_input`/`invalid-value` at that step's `/transition/type` (FSpec FR-364 \"Step alphabet\")."
  - id: FR-237-AC-10
    path: spec/functional/FR-237-replay-a-timed-counterexample.md
    role: examined
    excerpt: "The FR-249-AC-9 refutation's counterexample (FSpec FR-364-AC-23's fixture `B`, with `<a>` the index of its `activate t1#0` step), written to JSON by FR-286's writer, reads back with `read_temporal_counterexample::<TaskProductTransition>`, and re-writing gives an equal JSON value."
  - id: FR-237-AC-11
    path: spec/functional/FR-237-replay-a-timed-counterexample.md
    role: examined
    excerpt: "FR-237-AC-1's `Rpc` counterexample with a `task-transition` step inserted refuses at that step's `/transition/type`. Passed to `replay_task_product_trace`, it refuses `TraceAlphabet`."
  - id: FR-128-AC-14
    path: spec/functional/FR-128-replay-a-model-counterexample.md
    role: context_only
    excerpt: "At the writer and reader (`fsl-replay/src/outcome/counterexample.rs`): a closure-only engine lasso with an empty prefix and one with a prefix of one `inc` each write the sole closure loop step with the entry key, and the reader reads each back to the same lasso for the model alphabet only."
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

## Fix-commit disposition — round 1

**CONDITIONAL — no remaining source defect found; final titan reruns are pending.** This is a confined disposition of `c9403b97..3c7626fa`, not a second full review. The source candidate is `3c7626faacd3fa6f8702e4cc81be0767ce91f4a0`. The original Findings table above is unchanged; its placeholder is not a defect requiring a disposition row.

- `bdf77515` fixes the missed frontier-evidence consumer at `fsl-analyze/src/analyze/task_product_item.rs:419`: the evaluator transition is cloned into the already-reviewed replay-owned type before calling `task_product_transition_wire`. The state replay, frontier indexing, edge deduplication and wire writer are unchanged. This is the same exhaustive field-preserving bridge used by the other task consumers.
- `3c7626fa` corrects the R14b baseline product regression recorded on QSL-2824. The FR-237 wire reader must validate the whole shape and then classify a valid foreign step at its `/transition/type`. The old selected-model-alphabet branch imposed origin position 0 even on a valid task payload whose actual first deadline miss was at position 1, misclassifying the payload as malformed.
- The origin-position branch now applies only to a parsed model lasso with no scheduler Task steps. Payloads containing scheduler steps retain the task alphabet's actual first-miss position, even when the caller requested ModelTransition. Therefore a correctly shaped task payload reaches the unchanged foreign-step check, while a wrong trace position remains a shape fault for both requested alphabets. Queue-growth position handling is untouched.
- Both complete shape-check loops and task-field parsing still precede alphabet classification. The new foreign-task test writes through the production task writer, reads and rewrites with its real alphabet, expects a foreign-alphabet refusal under the model reader, rejects a mutated position in both readers, and requires a later unknown member to take precedence. The native-model test writes and reads ordinary and closure-only lassos at origin 0, rejects position 1, and rejects a misplaced terminal stutter. Existing assertions, scanners and allow entries are unchanged.
- `git diff --check c9403b97..3c7626fa` exited 0. No builds, tests or repository executables were run by this reviewer. The lead reported earlier titan passes for eight TC-452 tests, evaluator renaming and the wrapper conversion test; other selected consumers were compile-blocked by the now-patched frontier call. Those earlier results do not establish acceptance of this final candidate. Final named reruns and compilation remain pending through the lead.

No new findings were added in this disposition. The initial-review execution limits above describe that earlier snapshot; this section records the later reported evidence without claiming independent execution or merge readiness.
