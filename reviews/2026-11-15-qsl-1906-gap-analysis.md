---
id: SR-599006
title: 'Gap analysis of PR #2714 against QSL-1906 acceptance criteria'
type: SpecReview
analysis: gap-analysis
scope: 'agent-ix/filament-spec-language@508b98459; git diff origin/main...508b98459 (PR #2714): fsl-bench/**, Makefile, spec/assurance/MP-007..MP-011; ticket QSL-1906 acceptance criteria'
review_set: subset
date: '2026-11-15'
---

## Summary

Ticket: QSL-1906. PR: filament-spec-language#2714, head 508b98459. The PR adds no FR or AC to this repository, so `quoin matrix` has no new criterion to trace. This gap analysis is a manual check of the ticket's three acceptance criteria and the eight listed axes against the code and the tests, scoped to the diff. Plan completion: not assessed.

## Scope

| Unit | Evidence | Role |
| --- | --- | --- |
| AC1: every axis runs by name under make bench / make bench-probe and is listed in BENCHMARKING.md with unit and generator | BENCH_AXES gains mc reach key en4; bench-probe lines for all 13 probe axes; BENCHMARKING.md table rows | examined |
| AC2: peak RSS prints a number on macOS and Linux | rss.rs cfg(linux) VmHWM and cfg(macos) vmmap; test peak_rss_is_a_number_on_linux_and_macos | examined |
| AC3: counter totals for the smallest size of each axis are asserted in a test | mc.rs counter_tests::smallest_size_counter_totals_are_fixed (run by ci-all-features) plus the per-axis smallest-size tests | examined |
| Axis mc/state_graph, with and without symmetry | state_graph_smallest_size_explores_the_ring_product (2x2), state_graph_symmetry_quotients_the_ring_product (3x3) | examined |
| Axis mc/safety (invariant, deadlock) | safety_smallest_size_holds_over_the_ring_product | examined |
| Axis mc/liveness | liveness_smallest_size_holds_under_weak_fairness | examined |
| Axis mc/frame_candidates | frame_candidates_smallest_size_generates_every_assignment | examined |
| Axis mc/observation | observation_smallest_size_keeps_the_state_count_fixed | examined |
| Axis reach/workers 1/2/4/all | reach_smallest_size_is_the_same_graph_at_every_worker_count | examined |
| Axis key/encode | encode_smallest_size_keys_one_state | examined |
| Axis en4/sample, 10^3..10^5 samples | sample_smallest_size_draws_the_okamoto_sample_count (1440 at eps 0.04) | examined |
| Reporting: states/s, edges/s, peak RSS, marginal bytes per state, samples/s | probe rates() and mc-marginal | examined |

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | For most axes, `canonical_encodes`, `expansions` and `generated_candidates` come from the bench's own breadth-first search over the plain `ModelSystem`, not from the engine run the bench times. For liveness, that search counts 4 states and 8 edges while `check_model` explores 12 product states and 28 edges. These counters fingerprint the generator, which is what the AC asks for. They cannot show an engine change, such as fewer canonical encodes, which is the before/after the ticket's Goal wants. BENCHMARKING.md names the seam but does not say the counters do not move when the engine changes. Say so, or keep the gap in a follow-up ticket. | fsl-bench/src/counters.rs:692-714, fsl-bench/BENCHMARKING.md:113-126 |

## Verdict

All three acceptance criteria are delivered, and all eight axes from the ticket's scope exist, run by name and have a smallest-size test with real values. The Linux branch of AC2 is cfg-gated, so this macOS run did not exercise it; the code is the unchanged VmHWM reader. The symmetric 3x3 count is questioned in SR-599005 FND-001.

## Dispositions

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | counters.rs module doc and BENCHMARKING.md now say the counters fingerprint the generator and do not move with engine changes, citing liveness 4/8 against 12/28. |

## String-edge gate-fix review

**PASS — no new findings.** Reviewed only commit 289ad6799ff4a608946d232c22ab018fafbe9ee7 against parent 46182f2cb28b28d69de3db39c29aa0c4392d5138 in the existing QSL-1906 worktree. The prior findings and dispositions above are preserved and are not reopened by this delta.

The new string_edge attributes mark actual CLI boundaries: run_axis dispatches the supplied axis word, and size_lists parses size words and the -- separator. Inspection of the installed ix-trace-rs marker confirms that an argument-free attribute returns the input item unchanged, so these annotations change no runtime behavior and do not bypass a semantic check.

The state-graph runner now reads the first checked claim instead of comparing a name string. The owning ring_state_graph generator emits exactly one Wraps claim. All current callers in the probe, Criterion bench and unit/counter tests construct their input through that generator, so this selects the same claim. The empty-claim path remains a typed AxisError. No duplicate implementation, new unsafe behavior, new requirement, changed spec or weakened gate was found.

Independent checks on this head: slot-wrapped target/debug/xtask string-edge passes with no unmarked/unlisted comparisons; cargo fmt --all -- --check and git diff --check HEAD^..HEAD pass; cargo test --locked -p fsl-bench --lib --all-features state_graph passes both focused tests (4/8 for 2x2; 27/81 plain and 18/54 symmetric for 3x3). Logs: /private/tmp/qsl-1906-gatefix-review-string-edge.log and /private/tmp/qsl-1906-gatefix-review-tests.log. No aggregate gate was run. GitHub reports MERGEABLE; final aggregate qualification remains the lead's responsibility.

Methods: code-review with rust-review/rust-style and a confined gap recheck of the changed claim selection against the existing generator tests. No new criterion is introduced; the repository-wide matrix was not rerun for this two-file fix. Spec-review is inapplicable because no spec or plan changed. Plan completion: not assessed.

No Linear finding comment was posted because this dispatch requested posting if findings were found. No repository file was edited or pushed. This scratch copy retains SR-599006; if persisted, replace the existing reviews/2026-11-15-qsl-1906-gap-analysis.md rather than creating a second artifact with this id.

## String-edge gate-fix scope

```yaml
{
  "clean": true,
  "scope": [
    {
      "id": "run_axis",
      "path": "fsl-bench/src/bin/fsl-bench-probe.rs",
      "role": "examined",
      "excerpt": "/// `#[string_edge]`: this CLI boundary selects a benchmark axis from its word.\n#[string_edge]\nfn run_axis(axis: &str, sizes: &[usize]) -> Result<(Work, f64), AxisError> {"
    },
    {
      "id": "size_lists",
      "path": "fsl-bench/src/bin/fsl-bench-probe.rs",
      "role": "examined",
      "excerpt": "/// `#[string_edge]`: this CLI boundary parses size words and the separator.\n#[string_edge]\nfn size_lists(arguments: &[String]) -> Result<(Vec<usize>, Vec<usize>), String> {"
    },
    {
      "id": "run_state_graph",
      "path": "fsl-bench/src/mc.rs",
      "role": "examined",
      "excerpt": "        // `ring_state_graph` builds exactly one `Wraps` claim, so use the\n        // model's first checked claim instead of selecting it by wire text.\n        .first()\n        .ok_or_else(|| fail(\"claim\", \"Wraps is not checked\"))?;"
    },
    {
      "id": "ring_state_graph",
      "path": "fsl-bench/src/mc.rs",
      "role": "examined",
      "excerpt": "    let claim = format!(\"possible Wraps using v over (c: M::Cell) {{ c.v = {top} }}\\n\");\n    Ok(World::build(&ring(objects, values, symmetric, &claim))?)"
    },
    {
      "id": "state_graph_smallest_size_explores_the_ring_product",
      "path": "fsl-bench/src/mc.rs",
      "role": "examined",
      "excerpt": "        let world = ring_state_graph(2, 2, false).expect(\"the ring builds\");\n        assert_eq!(run_state_graph(&world, false).expect(\"runs\"), work(4, 8));"
    },
    {
      "id": "state_graph_symmetry_quotients_the_ring_product",
      "path": "fsl-bench/src/mc.rs",
      "role": "examined",
      "excerpt": "        let world = ring_state_graph(3, 3, true).expect(\"the ring builds\");\n        assert_eq!(run_state_graph(&world, false).expect(\"runs\"), work(27, 81));\n        assert_eq!(run_state_graph(&world, true).expect(\"runs\"), work(18, 54));"
    },
    {
      "id": "state_graph",
      "path": "fsl-bench/benches/mc.rs",
      "role": "context_only",
      "excerpt": "            let world = mc::ring_state_graph(n, k, symmetric).expect(\"the ring builds\");\n            let work = mc::run_state_graph(&world, symmetric).expect(\"the graph completes\");"
    }
  ],
  "findings": [],
  "bindings": []
}
```
