---
id: SR-599005
title: 'Code review of PR #2714: fsl-bench model-check, sampling and key-encoding axes, code-review + rust-review'
type: SpecReview
analysis: code-review
scope: 'agent-ix/filament-spec-language@508b98459; git diff origin/main...508b98459 (PR #2714, branch qsl-1906-bench): Cargo.lock, Makefile, fsl-bench/BENCHMARKING.md, fsl-bench/Cargo.toml, fsl-bench/benches/{mc,reach,key,en4}.rs, fsl-bench/src/bin/fsl-bench-probe.rs, fsl-bench/src/{counters,lib,mc,rss,world}.rs; ticket QSL-1906'
review_set: subset
date: '2026-11-15'
---

## Summary

Ticket: QSL-1906. PR: filament-spec-language#2714, head 508b98459. Method: code-review with the rust-review lane folded in. The gap analysis is SR-599006 and the spec review of MP-007 to MP-011 is SR-599007.

The harness is sound. Generators are deterministic (fixed keys, fixed initial values, fixed seed 7). Worlds are built outside the timed region. Every run checks that the outcome is `Holds`, `Completed` or a fully expanded graph, so a bench cannot quietly time a refusal. No `unsafe`, no `allow`, no Python. Cargo.lock adds only fsl-bench's edges on fsl-analyze and fsl-route. The macOS `vmmap` reader is honest about what it measures. rustix 1.1.4 in the lock has no `getrusage`, so the claim in the deviation note holds.

## Scope

| Unit | Path | Role |
| --- | --- | --- |
| generators ring/wide/observation/protocol/keys/service, run_* functions, FieldLetters | fsl-bench/src/mc.rs | examined |
| Model, World::build, document/snapshot builders | fsl-bench/src/world.rs | examined |
| fingerprint, Counters | fsl-bench/src/counters.rs | examined |
| peak_rss_kib (Linux VmHWM, macOS vmmap), footprint_kib | fsl-bench/src/rss.rs | examined |
| run_axis, size_lists, probe_axis, probe_marginal, rates | fsl-bench/src/bin/fsl-bench-probe.rs | examined |
| criterion groups | fsl-bench/benches/{mc,reach,key,en4}.rs | examined |
| bench and bench-probe targets | Makefile | examined |
| features, deps, bench entries | fsl-bench/Cargo.toml, Cargo.lock | examined |
| ModelSystem::canonicalise (sort canonicaliser) | fsl-eval/src/simulation/model/symmetry.rs | context_only |
| ReachedState::steps | fsl-eval/src/simulation/protocol/reach.rs | context_only |

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | The symmetric state-graph test pins 18 states and 54 edges for 3 rings of 3 values under one symmetry class of all 3 keys. The orbits of S3 acting on 3^3 states are the multisets of size 3 over 3 values, which is 10. fsl-eval's sort canonicaliser (symmetry.rs:336-399) sorts every key of the class by its object bytes, so it should reach 10. The test enshrines an unexplained count: either the reduction is incomplete (an engine defect the symmetry tickets will measure against), or the bench measures something other than the quotient. Explain 18 in the test or file the engine defect. | fsl-bench/src/mc.rs:745-749, fsl-bench/src/mc.rs:838-842 |
| FND-002 | low | The en4 bench's module doc says `eps` is in thousandths with values 50, 15 and 5. The constant is in ten-thousandths with values 400, 120 and 48, as the item doc on the next line says. | fsl-bench/benches/en4.rs:3-4 |
| FND-003 | low | `footprint_kib` splits the text at byte `len - 1`. When the last character of vmmap's text is multi-byte, `split_at` is not on a char boundary and panics instead of returning `None`. Use `char_indices().next_back()` or `strip_suffix` per unit. | fsl-bench/src/rss.rs:837 |
| FND-004 | low | `size_lists` drops a size word that does not parse (`filter_map`), so `mc state_graph 3 x 4` runs as `[3, 4]` instead of failing. `mc-marginal` does not check that the large sizes come after the small ones, although its result is only meaningful in that order (BENCHMARKING.md says "the pair must ascend"). | fsl-bench/src/bin/fsl-bench-probe.rs:533-539, fsl-bench/src/bin/fsl-bench-probe.rs:563-580 |

## Verdict

Mergeable after FND-001 is explained or ticketed. FND-002 to FND-004 are low. The macOS reader spawns `/usr/bin/vmmap` on its own pid (about 0.4 s, outside any timed region), does not check the exit status, but any failure parses to `None` and the probe prints `unavailable`. Its figure is the peak physical footprint, which counts compressed pages and differs from `ru_maxrss`. The rss.rs doc, BENCHMARKING.md and MP-011 all say so. The Linux path is unchanged apart from its `cfg` gate. Counter tests assert real engine totals (for example 4/8/9/18/4 for safety and 4/16/17/0/4 for frame candidates). `generated_candidates == edges` is stated in counters.rs, BENCHMARKING.md and the PR body. Protocol `edges` sums `ReachedState::steps`, which are the outgoing successor edges, so the count is correct.

## New findings (disposition pass 1)

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-005 | low | The fix commit adds an empty file `target_gate_dummy` at the repository root. It is unrelated to the ticket and looks like a leftover from a gate run. Remove it before merge. | target_gate_dummy |

## Dispositions

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | The diagnosis is confirmed. `instances` keeps one orbit representative (c0) as the claim's instance, and `common_stabiliser` (fsl-analyze/src/model_check/state_graph.rs:365-386) quotients by the subgroup that fixes it: S2 on {c1, c2}. That gives k * C(k+n-2, n-1) = 3 * 6 = 18 states. The engine's own test (fsl-analyze/tests/it/state_graph.rs:388-424, TC-592 / FR-167-AC-6) pins 27/18/54. This is intended label-preserving behaviour, not a defect, so no engine ticket is needed. The test doc comment and BENCHMARKING.md now explain it. |
| FND-002 | fixed | en4.rs doc now says ten-thousandths 400, 120 and 48. |
| FND-003 | fixed | footprint_kib uses char_indices().next_back(). Tests cover a trailing multi-byte char, a lone multi-byte char and empty input. |
| FND-004 | fixed | size_lists returns Err for a word that does not parse. `mc` rejects a stray `--`. mc-marginal fails when the large run reaches no more states than the small one. |
| FND-005 | fixed | `target_gate_dummy` is absent from the current tree at 714846902aa4e45034b9359e2d9ed6a5a03ca00a (`git ls-tree HEAD -- target_gate_dummy` returns no entry). |

## String-edge gate-fix review

**PASS — no new findings.** Reviewed only commit 289ad6799ff4a608946d232c22ab018fafbe9ee7 against parent 46182f2cb28b28d69de3db39c29aa0c4392d5138 in the existing QSL-1906 worktree. The prior findings and dispositions above are preserved and are not reopened by this delta.

The new string_edge attributes mark actual CLI boundaries: run_axis dispatches the supplied axis word, and size_lists parses size words and the -- separator. Inspection of the installed ix-trace-rs marker confirms that an argument-free attribute returns the input item unchanged, so these annotations change no runtime behavior and do not bypass a semantic check.

The state-graph runner now reads the first checked claim instead of comparing a name string. The owning ring_state_graph generator emits exactly one Wraps claim. All current callers in the probe, Criterion bench and unit/counter tests construct their input through that generator, so this selects the same claim. The empty-claim path remains a typed AxisError. No duplicate implementation, new unsafe behavior, new requirement, changed spec or weakened gate was found.

Independent checks on this head: slot-wrapped target/debug/xtask string-edge passes with no unmarked/unlisted comparisons; cargo fmt --all -- --check and git diff --check HEAD^..HEAD pass; cargo test --locked -p fsl-bench --lib --all-features state_graph passes both focused tests (4/8 for 2x2; 27/81 plain and 18/54 symmetric for 3x3). Logs: /private/tmp/qsl-1906-gatefix-review-string-edge.log and /private/tmp/qsl-1906-gatefix-review-tests.log. No aggregate gate was run. GitHub reports MERGEABLE; final aggregate qualification remains the lead's responsibility.

Methods: code-review with rust-review/rust-style and a confined gap recheck of the changed claim selection against the existing generator tests. No new criterion is introduced; the repository-wide matrix was not rerun for this two-file fix. Spec-review is inapplicable because no spec or plan changed. Plan completion: not assessed.

No Linear finding comment was posted because this dispatch requested posting if findings were found. No repository file was edited or pushed. This scratch copy retains SR-599005; if persisted, replace the existing reviews/2026-11-15-qsl-1906-code-review.md rather than creating a second artifact with this id.

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
