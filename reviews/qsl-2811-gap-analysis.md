---
id: SR-967175026302197768900474181941935084694
title: "Gap analysis of the QSL-2811 layering-repair change"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/filament-spec-language; repository static matrix and the assigned PR 3461 behavior delta"
review_set: subset
relationships:
  - { target: "ix://agent-ix/filament-spec-language", type: references }
---

## Summary

Read the repository's computed static matrix and coverage census, then traced the assigned layering-repair behavior delta back to its owning criteria and real tests. No introduced traceability, reverse-gap or stub defect was found in that delta. The repository-wide census has outstanding coverage gaps and diagnostics; this review does not claim a repository-wide assurance pass.

## Verdict

**CONDITIONAL** — the scoped change has requirement ownership and test binders, but no execution evidence was available, the evidence-backed matrix request was refused, and the repository-wide census is not clean. Runtime acceptance and an exhaustive repository-wide reverse inventory are not claimed.

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

## Matrix and evidence

- Reconciliation: `quire matrix` (quire 0.52.0, engine 0.62.2) — no run evidence read. The command passed with explicit repository scope; its result is retained in the reviewer scratchpad.
- Preferred `quoin matrix --repo <repository> --json` was attempted with `/home/peter/.npm-global/bin/quoin` 0.36.8 on terra. It exited 1 with `native runtime matrix.build failed: CORE_REFUSED: request exceeds the accepted size`. This failure was reported to the lead, without changing binaries or limits. The documented static fallback was used.
- The engine's repository rollup has 5020 tagged, 1081 untagged, 68 tagged-by-ignored-test, 107 method-without-symbol and 15 retired criteria. These figures describe the whole repository and are not a passing PR coverage percentage.
- FR-100-AC-8 is tagged by the unchanged TC-452 shipped-tree/probe tests and the new task conversion test. FR-062-AC-6 is tagged by the unchanged dependency/text scan and the real renamed-name evaluation test. FR-400-AC-9 is tagged by the unchanged shipped test-support isolation test. FR-087-AC-1 is method-without-symbol (Inspection), with the existing isolation and typestate tests also listed as binders.
- `quire coverage` exited 0. Its census reports 14441 candidate Rust test symbols, 13866 tagged and 13268 bound, and 6208 untracked symbols. Reported diagnostics include trace-id-outside-marker, uncatalogued-verification-method, untracked-id-has-minted-children, section/archetype matches-nothing and catch-all-universal. Source-symbol collision diagnostics are also present, including mutually exclusive cfg declarations. These are disclosed as limits; tagged status is not execution evidence and a bare mention in a module comment is not a trace binder.
- No acceptance criterion or existing trace attribute was removed by this PR. The only added test trace is FR-100-AC-8, and the engine binds it to the new test function.

## Reverse-gap and stub inspection

The assigned delta introduces seven public data carriers, eleven From bridges, two subject-transition implementations and one retained-result accessor. These serve FR-100-AC-8's owned facade boundary; they add no engine, external effect, command or configuration behavior. The task transition data/behavior remains governed by FR-249 and the existing task wire/replay requirements. Layer-3 enum exports and manifest routing implement FR-062/ADR-011; benchmark test-support removal implements the existing isolation rule and the explicit dev-only owner ruling.

Inspected all new module bodies and their production consumers. Private inner carriers retain real inputs/results, conversions execute rather than returning placeholders, and the new test calls the real production adapters. No new source/test stub, unconditional skip, mock replacement of the unit under test, or unowned semantic behavior was found in the assigned delta. This is a bounded PR reverse-gap inspection, not an exhaustive reverse inventory of every existing repository behavior.

## Coverage

- Plan completion: not assessed
- Evidence: no execution evidence read; no builds/tests/compiled repository scanners/titan jobs run or queued.
- Scoped criteria: FR-100-AC-8, FR-062-AC-6 and FR-400-AC-9 tagged; FR-087-AC-1 is Inspection/method-without-symbol. No changed-scope untagged criterion was found.
- Scoped reverse gaps/source stubs/test stubs: 0 observed.
- Semantic review: optional gap-analysis intent review was not requested and was skipped. Ordinary code-review requirement and caller checks are recorded in the separate code-review artifact.
- Repository-wide assurance: not established; global untagged/ignored-only criteria and extraction diagnostics remain outside this assigned PR's repair scope.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No findings in the assigned delta (placeholder) | - |

## Fix-commit disposition — round 1

**CONDITIONAL — no new scoped gap observed; final execution evidence remains pending.** Examined only the two fix commits in `c9403b97..3c7626fa` at candidate `3c7626faacd3fa6f8702e4cc81be0767ce91f4a0`. The original Findings table remains unchanged, and its placeholder is not a finding to dispose.

The frontier writer's explicit evaluator-to-replay conversion is owned by FR-100-AC-8 and uses the already-reviewed bridge; it introduces no new semantic surface. The R14b product correction is owned by FR-237's Reading the wire rule and AC-11, with AC-10 covering real writer/reader roundtrip and shape-fault classification. It prevents selection of the model alphabet from imposing the wrong position convention on a valid foreign task lasso. Shape validation, task first-miss validation, queue-growth handling and foreign-step classification remain real production paths.

The two added tests carry actual test-function trace attributes: `foreign_task_lasso_validates_its_miss_position_before_alphabet_refusal` names FR-237-AC-10/11; `model_lassos_validate_origin_position_with_and_without_closure_steps` names FR-237-AC-10 and FR-128-AC-14. Their assertions call the actual writer/readers and exercise valid payloads, wrong positions, later shape faults, closure-only origin handling and misplaced terminal stutters. No stub, skip or substituted semantic engine was introduced. These new tags were checked in source; the full matrix was not recomputed during this confined disposition, so the earlier matrix counts above remain explicitly the initial-review snapshot.

The lead reports earlier titan passes for the eight TC-452 tests, evaluator renaming and wrapper conversion, and a compiler failure at the frontier consumer now converted by the fix. This is reported evidence, not an independent reviewer run and not acceptance of the final source candidate. No builds/tests/repository executables ran here; no job was queued. Both the original evidence-matrix limitation and the pending final named reruns remain disclosed. No new findings were added.
