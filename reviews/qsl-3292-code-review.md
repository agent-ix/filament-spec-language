---
id: SR-967175026302197768900474181941935084855
title: "Code review — filament-spec-language QSL-3292 red-main fsl-analyze tests"
type: SpecReview
analysis: code-review
scope: "agent-ix/filament-spec-language; fsl-analyze/src/exact_probabilistic/build.rs, fsl-analyze/tests/it/component_certificate.rs, fsl-analyze/tests/it/exact_probabilistic.rs, fsl-analyze/tests/it/prob_world.rs; FR-196, FR-203-AC-14, FR-282-AC-6, FR-338 Rejection spellings, FR-339-AC-2, FR-125-backed FR-196-AC-13"
review_set: subset
relationships:
  - { target: "ix://agent-ix/filament-spec-language/FR-282", type: references }
  - { target: "ix://agent-ix/filament-spec-language/FR-338", type: references }
  - { target: "ix://agent-ix/filament-spec-language/FR-196", type: references }
---

## Summary

Ticket: QSL-3292. Reviewed PR #3907 at head `f168d5ef4c9442fcaca04fc0710e4a8946112785` (`git diff origin/main...HEAD`, four commits), with the rust-review lane folded in. The specs were read directly: FSL FR-282, FR-338, FR-339 and FR-196 in the worktree, and FSpec FR-331, FR-411 and `proposals/backend-provider-v1/schema.json` at FSpec origin/main `fd08823654`.

The ghost-instance fixture (commit 8f9e537858) is correct and weakens nothing. The `TargetState` split (f168d5ef4c) is a real FR-203-AC-14 improvement on the production builder. Commit 045487d709 changes the stale component test so that it expects `CertificateLocus::ProductEdge`. That locus serializes into the result cause as `{"source", "target"}` under `rule: backward-edge`, and FSL FR-338's Rejection spellings and FSpec FR-331 (schema `ProductStateLocus`, `additionalProperties: false`) both refuse that shape. The `build_product` relabel (cf7d2f0aa1) makes the FR-196 stop tests pass on a wrapper that production never calls.

## Verdict

**FAIL**: not mergeable at `f168d5ef4c9442fcaca04fc0710e4a8946112785`. FND-001 (high) makes a test assert a wire locus the spec forbids. FND-002 (medium) leaves the FR-196 and FR-203-AC-14 stop tests checking a test-only wrapper.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | The test now requires `BackwardEdge` at `CertificateLocus::ProductEdge{source,target}`. Production writes that as the result cause's `at` `{"source":…,"target":…}`, which FSL FR-338 Rejection spellings (ProductState only, "SHALL add none of its own") and FSpec FR-331 / schema `ProductStateLocus` refuse. The test blesses a non-conforming wire output from #3861; production must change. | fsl-analyze/tests/it/component_certificate.rs:226-273; fsl-replay/src/certificate/scc.rs:248-256,374-400; fsl-replay/src/certificate.rs:150-156,238-243; fsl-replay/src/outcome.rs:477-478,905-913 |
| FND-002 | medium | `build_product` relabels a finished `ProbBuild::Partial` as `Stopped(MaxStates)` after exploration has continued past the first denied key. It has no non-test caller, so the TC-631 FR-196 stop tests and the FR-203-AC-14 test check a wrapper and not production. It can also return `Undefined` or `MaxTransitions` where FR-196's breadth-first stop at the first denied key would return `MaxStates`. | fsl-analyze/src/exact_probabilistic/build.rs:35-57,612-630,903-913; fsl-analyze/tests/it/exact_probabilistic.rs:3545,3973,4010,4243 |
| FND-003 | low | The test name `swapping_counter_components_reports_the_source_of_a_backward_edge` no longer matches its assertion, which now requires both ends of the edge. | fsl-analyze/tests/it/component_certificate.rs:231 |

## Finding Details

### FND-001: ProductEdge locus is off-spec on the wire

How it reaches the wire: `scc::Checker::edges` → `K::edge_locus(source, target)` → for `ProductStateRef` this is `CertificateLocus::ProductEdge` (added in #3861, d10a05ba28). `check_components`, `check_components_over_stutter_closure` and `check_protocol_components` return the result. `settle.rs` `Row::of_check` turns it into `InconclusiveCause::CertificateRejected{rule, at}`. `outcome.rs` `inconclusive()` copies `at` into the item cause's `at` (`certificate_at`). The `Serialize` impl writes `{"source": <ProductStateRef>, "target": <ProductStateRef>}`.

Production callers: `analyze/items.rs:231,239,351`, `analyze/digital_item.rs:132`, `protocol_analysis/conformance.rs:1068` and `protocol_analysis/finite_replay.rs:1022`. Every EN-1 liveness `Holds` is checked in the qualified core, so any component-order defect in EN-1 output, which is exactly what this checker exists to catch, is written as an off-schema cause.

FSpec schema line 1109 pairs `backward-edge` with `ProductStateLocus` (`required: [model_state, automaton_state]`, `additionalProperties: false`), so a conforming reader refuses the cause. FSL FR-338 "Rejection spellings" has no ProductEdge row and forbids FSL-specific spellings. FR-338-AC-13's writer-side pair guard (`certificate-rejection-pair`) is not implemented anywhere in `fsl-*`, so nothing stops the write. `HyperProductEdge` (FR-163, same commit) has the same defect.

FR-282-AC-6's wording ("a cause naming the edge that runs backward") predates #3861 and #3861 did not change it. Of the two spec texts, FR-338 together with FSpec FR-331 is the binding wire contract.

Minimal conforming fix: make `edge_locus` return `ProductState(source)` (and `HyperProductState(source)`), which restores the pre-#3861 locus. Keep the original test expectation (source state). Update the `scc.rs` unit tests and the #3861 `an_altered_certificate_is_rejected_at_the_rule_the_alteration_breaks` assertion, which currently pins the `{source,target}` serialization. Reword FR-282-AC-6 and TC-764 step 5 to "naming the source product state of the edge that runs backward". The alternative is an FSpec FR-331 change adding a product-edge locus form, with schema and vectors, then FR-338 and FR-163 table rows and a pin bump. That is a cross-repo change that needs a release first, so it is not the minimal path.

### FND-002: build_product stop is a test-only relabel

`Builder::admit_partial_state` sets `partial = true` and exploration carries on through the whole queue. `run()` then returns `Partial`, and `build_product` maps that to a stop only after the build has finished. FSL FR-196 Behavior says "Reaching `max_states` … SHALL stop the build and name the limit". FR-203-AC-14 says the first new key "stops `MaxStates` … after only the work needed to establish the key". The production builder (`build_retaining_monitor`, used by `analyze/exact.rs`, `exact/measure.rs`, `exact/elapsed.rs` and `exact/long_run.rs`) does neither, by design, for FSpec FR-411 "Evidence on a partial product" and FR-1042-AC-18.

So the FR-196 stop tests now pass on code that production never runs. Traced, not run: after a denial, later expansions still read predicates and charge transitions, so `build_product` can return `Undefined(path)` or `Stopped(MaxTransitions)` where a stop at the first denied key would give `Stopped(MaxStates)`.

Resolution is spec text, not more code. FR-196 Outputs and Behavior need a `Partial` alternative scoped to FSpec FR-411, and the stop tests should then be retargeted to the production builder's `Partial`, or `build_product` documented as a test-support stop view. The `TargetState` and `observe_edge` changes are correct on the production path: a denied key builds no model clone or truths, and neither its target nor its edge is counted.

## Review Methods

- code-review with the rust-review lane: the diff, the production call graph of the changed functions, wire serialization of `CertificateLocus`, panic surface (`expect` in tests only; the `u64::try_from(...).unwrap_or(u64::MAX)` saturation is acceptable), trace tags (unchanged), fixture semantics in `prob_world.rs`.
- Tests: not run. The cargo run was refused by the brief's memory gate (MemAvailable 24 GB < 30 GB, checked inside the `lead-prob-cargo-C.lock` flock). The coder's reported results (67 passed, 0 failed on the name-filtered modules) were not reproduced.

## Verdict notes (clean units)

- Commit 8f9e537858 (FR-196-AC-13, FR-125, FSpec FR-361): `with_object_initials` with a first snapshot holding only `c` and a second holding both keys makes `ghost` an `over` instance by FR-125. The build still ends at initial 0, position 0, binding `ghost`. Every assertion (`steps.is_empty`, `position 0`, `initial 0`, probability `1/1`, the monitor, accumulator and pending shapes, `UndefinedRead::Atom`, the `GhostSteps` measure control) is unchanged. The `null`-filter in `prob_world.rs` only affects entries that are explicitly `null`, and only this test uses that. Nothing is weakened.
- Commit f168d5ef4c (FR-203-AC-14): the denied target carries `state: None`; `intern` takes the key and `TargetState` separately, and the omitted-edge path is unchanged.

## New findings (disposition pass 1)

Reviewed at head `0cbb54d8ce417fd9068d3c507a235fd475681d78`. This head also carries QSL-3315 (one work allowance per item), commit 7ef5bb5957 (the lower-completion edge order) and main merges including #3897 and #3917.

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-004 | medium | The FR-196 amendment adds a SHALL: a `Partial` build that settles nothing settles `Incomplete(LimitReached)` naming `exact_prob.max_states`. Three production paths still settle `Unsupported(not-yet-implemented)` on `ProbBuild::Partial`. Round 1's PR body listed this as remaining; it is now unlisted and has no ticket. | fsl-analyze/src/analyze/exact.rs:684-686; fsl-analyze/src/analyze/exact.rs:1119-1121; fsl-analyze/src/analyze/exact/elapsed.rs:385-387; spec/functional/FR-196-build-the-probabilistic-product.md:974-977 |
| FND-005 | low | `the_cyclic_check_past_the_rational_budget_spends_the_items_work_allowance` carries `FR-201-AC-65`, but it asserts a `WorkUnits` stop, not AC-65's `Incomplete(LimitReached{MaxRationalBits, 1, …})`. | fsl-analyze/tests/it/cyclic_state_certificate.rs:2108-2120 |

## Dispositions

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | 6451a8c028: the `ProductEdge` and `HyperProductEdge` variants are gone, and `BackwardEdge` is located at the edge's source. The `ProductState` and `HyperProductState` loci are now written in the FR-331 forms, with `{key}` State objects. FR-282-AC-6 and TC-764 step 5 now say "the source product state". The new test `a_backward_edge_rejection_is_written_in_the_fspec_locus_form` validates both written causes against FSpec's `CertificateRejected` and locus schemas, and it fails on a `{source,target}` cause. |
| FND-002 | fixed | 1d6f3dc94d: `build_product` returns what the production builder returns, `Partial` included. FR-196 Outputs gains `Partial` and a "The partial product" section, and Behavior splits the initial-key stop from the denial of a successor key. The stop tests now assert the prefix and `assert_completion`. |
| FND-003 | fixed | The test asserts `at == source`, so its name `…reports_the_source_of_a_backward_edge` is accurate again. It now carries `FR-282-AC-6`. |
