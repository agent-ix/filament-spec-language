---
id: SR-2026111414542500000008
title: "QSL-1918 code-review of evaluator kernel benchmark"
type: SpecReview
analysis: code-review
scope: "agent-ix/filament-spec-language; QSL-1918; Makefile; fsl-bench/Cargo.toml; fsl-bench/benches/evaluator_kernel.rs; fsl-bench/src/kernel.rs; fsl-bench/src/bin/fsl-bench-kernel-report.rs; fsl-bench/src/lib.rs; fsl-bench/BENCHMARKING.md; spec/assurance/MP-012-evaluator-kernel-wall-time.md"
review_set: subset
relationships:
  - target: ix://agent-ix/filament-spec-language/MP-012
    type: references
---

## Summary

The three kernels, reusable checked entry and derived ratios are implemented, but the report duplicates the benchmark population, adds two lint violations, and breaks filtered invocations with no paired result.

No applicable AssuranceProfile was found under spec/. Direct review is permitted; no enforced review or acknowledgement is required. Methods run: code-review with rust-review/rust-style, gap-analysis, spec-review/base and spec-review/integrity. Other language lanes and unrelated requirement lenses were not applicable. MP-012 changes no requirement statement or domain object. Its measures relationship names existing FR-068; no new dependency ordering is introduced. Jev/criterion-strength was not selected.

## Verdict

**FAIL** — The three kernels, reusable checked entry and derived ratios are implemented, but the report duplicates the benchmark population, adds two lint violations, and breaks filtered invocations with no paired result.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | The reporter duplicates the kernel population and fixed sizes instead of reusing one catalog. Searching fsl-bench found SIZES in src/kernel.rs:22 and KERNELS in benches/evaluator_kernel.rs:9-13, while report() repeats both. A size added to SIZES is benchmarked but silently omitted from the report. Remove the copies and have bench, report and generator test consume the shared population; high severity follows code-review's mandatory duplication rule. | fsl-bench/src/bin/fsl-bench-kernel-report.rs:40 |
| FND-002 | medium | The new report CLI dispatch is missing #[string_edge]. Confined make string-edge fails with two new branch-gating occurrences at lines 21 and 22. Mark the actual CLI boundary using the existing ix_trace_rs marker, as the neighboring probe does. Fifteen additional occurrences are unchanged base findings and are not attributed to this PR. | fsl-bench/src/bin/fsl-bench-kernel-report.rs:13 |
| FND-003 | medium | A filter selecting no kernel pair becomes a Make failure after Criterion succeeds. For example make bench BENCH_ARGS=parser reaches the unconditional kernel report with no fresh pair, and report exits 1; filtering to only interpreter also fails after valid measurements. Preserve successful filtered runs while still refusing stale estimates, and distinguish no selected pair from an expected pair with missing output. | fsl-bench/src/bin/fsl-bench-kernel-report.rs:69 |

## Scope

Every examined and contextual unit is recorded below, including the clean parts.

```yaml
{
  "scope": [
    {
      "id": "QSL-1918",
      "path": "Linear/QSL-1918",
      "role": "examined",
      "excerpt": "The axis runs by name under `make bench` and is listed in BENCHMARKING.md.\nOutput states ns per frame, ns per operation and the ratio to the Rust reference."
    },
    {
      "id": "QSL-1918",
      "path": "Linear/QSL-1918",
      "role": "examined",
      "excerpt": "An `evaluator/kernel` axis: arithmetic recursion, a comparison-heavy kernel and a collection fold, each at two sizes, with the Rust reference printed beside it.\nThe same kernel entry is what the JIT benchmark will run, so the two are comparable."
    },
    {
      "id": "Makefile::bench-evaluator_kernel",
      "path": "Makefile",
      "role": "examined",
      "excerpt": "bench-evaluator_kernel:\n\tcargo run --locked -p fsl-bench --bin fsl-bench-kernel-report -- start\n\tcargo bench --locked -p fsl-bench --bench evaluator_kernel -- $(BENCH_ARGS)\n\tcargo run --locked -p fsl-bench --bin fsl-bench-kernel-report -- report"
    },
    {
      "id": "fsl-bench::evaluator_kernel",
      "path": "fsl-bench/benches/evaluator_kernel.rs",
      "role": "examined",
      "excerpt": "fn evaluator_kernel(c: &mut Criterion) {\n    let objects = ObjectEnvironment::default();\n    let mut group = c.benchmark_group(\"evaluator/kernel\");\n    for kind in KERNELS {\n        for size in SIZES {"
    },
    {
      "id": "KernelCase",
      "path": "fsl-bench/src/kernel.rs",
      "role": "examined",
      "excerpt": "/// The checked package, call inputs and operation counts for one kernel size.\n///\n/// The same checked package and arguments can be passed to another evaluator\n/// backend, including the later JIT benchmark."
    },
    {
      "id": "KernelKind",
      "path": "fsl-bench/src/kernel.rs",
      "role": "examined",
      "excerpt": "/// The two fixed sizes included in the evaluator-kernel benchmark.\npub const SIZES: [usize; 2] = [5, 10];"
    },
    {
      "id": "fsl-bench-kernel-report::report",
      "path": "fsl-bench/src/bin/fsl-bench-kernel-report.rs",
      "role": "examined",
      "excerpt": "    for kind in [\n        KernelKind::ArithmeticRecursion,\n        KernelKind::ComparisonHeavy,\n        KernelKind::CollectionFold,\n    ] {\n        let name = kind.name();\n        for size in [5_usize, 10] {"
    },
    {
      "id": "fsl-bench-kernel-report::report",
      "path": "fsl-bench/src/bin/fsl-bench-kernel-report.rs",
      "role": "examined",
      "excerpt": "    if rows == 0 {\n        return Err(io::Error::other(\"no fresh evaluator/kernel Criterion estimates found\").into());\n    }"
    },
    {
      "id": "fsl-bench-kernel-report::estimate",
      "path": "fsl-bench/src/bin/fsl-bench-kernel-report.rs",
      "role": "examined",
      "excerpt": "    let modified = metadata.modified()?.duration_since(UNIX_EPOCH)?.as_nanos();\n    if modified <= marker {\n        return Ok(None);\n    }\n    let estimates: serde_json::Value = serde_json::from_slice(&fs::read(path)?)?;"
    },
    {
      "id": "smallest_kernels_match_their_plain_rust_references",
      "path": "fsl-bench/src/kernel.rs",
      "role": "examined",
      "excerpt": "    #[test]\n    fn smallest_kernels_match_their_plain_rust_references() {"
    },
    {
      "id": "fsl-bench-manifest",
      "path": "fsl-bench/Cargo.toml",
      "role": "examined",
      "excerpt": "[[bench]]\nname = \"evaluator_kernel\"\nharness = false"
    },
    {
      "id": "fsl-bench-modules",
      "path": "fsl-bench/src/lib.rs",
      "role": "examined",
      "excerpt": "pub mod kernel;"
    },
    {
      "id": "BENCHMARKING-evaluator-kernels",
      "path": "fsl-bench/BENCHMARKING.md",
      "role": "examined",
      "excerpt": "The report reads only estimates written by\nthat invocation, so a filtered run does not reuse an earlier estimate. The\nchecked package and arguments are exposed by `KernelCase` for reuse by another\nevaluator backend."
    },
    {
      "id": "MP-012",
      "path": "spec/assurance/MP-012-evaluator-kernel-wall-time.md",
      "role": "examined",
      "excerpt": "Recursive kernels have `n + 1` frames\nand `n` logical operations; the fold has one frame and `n` logical operations.\nInputs are generated deterministically before timing. The report prints the\nCriterion estimate divided by the frame and operation counts, plus the ratio\nof interpreter ns per operation to Rust ns per operation."
    },
    {
      "id": "MP-012",
      "path": "spec/assurance/MP-012-evaluator-kernel-wall-time.md",
      "role": "examined",
      "excerpt": "A performance claim compares current main with the candidate in one\ninterleaved same-session A/B run. It uses only the session noise defined in\n`fsl-bench/BENCHMARKING.md`; earlier measurements are not thresholds. The\ninterpreter-to-Rust ratio is printed as context and does not decide the claim."
    },
    {
      "id": "MP-012",
      "path": "spec/assurance/MP-012-evaluator-kernel-wall-time.md",
      "role": "examined",
      "excerpt": "Run `make bench-evaluator_kernel` five times on one machine, recording load\naverage before and after each run. Alternate current main and candidate for\nfive rounds each."
    },
    {
      "id": "FR-064-AC-1",
      "path": "spec/functional/FR-064-restrict-string-dispatch-to-marked-edges.md",
      "role": "examined",
      "excerpt": "Given an FSL source tree with one string comparison inside a function marked `#[string_edge]` and one string `match` inside an unmarked function, `xtask string-edge` reports exactly the unmarked occurrence, naming its file and line, and does not report the marked occurrence."
    },
    {
      "id": "FR-068",
      "path": "spec/functional/FR-068-split-expression-checking-into-check-stage.md",
      "role": "context_only",
      "excerpt": "`value::expression::evaluate` and `CheckedPackage::call` stay at layer 5,\nunchanged in stage."
    },
    {
      "id": "MP-005",
      "path": "spec/assurance/MP-005-evaluator-wall-time.md",
      "role": "context_only",
      "excerpt": "title: Evaluator cost per call frame"
    }
  ]
}
```

## Verification

Reviewed head: 1dce3f80fb4bc9a5967dacb4398f4b78933e721f. Base: 8ff14a5e2defa9e2cbe305b896fe3e04c5127d84 (qsl-1906-bench). Ticket: QSL-1918. GitHub reports MERGEABLE, but that is conflict status only. No repository source, spec or branch edits were made.

Independent checks: cargo fmt --all -- --check and git diff --check passed; cargo test --locked -p fsl-bench --all-features passed 24 tests, with zero report-binary tests; make string-edge failed (17 occurrences: 2 introduced here, 15 in unchanged base files). The first two build commands were launched without qsl-slot before the wrapper path was supplied; later Rust probes used the absolute wrapper. No aggregate gate ran. The dispatch forbids sandboxed make ci because four existing xtask tests invoke denied ps.

A scratch Rust probe compiled the unchanged report source against this worktree's built libraries: a fresh 600 ns / 60 ns sum/5 pair prints 100/120 ns per frame/op, 10/12 Rust ns per frame/op, and 10.00x; a new start rejects the old pair; an empty run returns exit 1. The unchanged Criterion bench compiled against the same built dependencies exits 0 for --bench parser (no kernel selected), while the following empty report exits 1. This is a functional probe, not performance evidence. No release benchmark or live five-round A/B claim was made. Coder logs separately report workspace checks, default/no-default Clippy, fsl-bench all-features Clippy and arch-lint passing; those are not represented as reviewer reruns.

Tooling on Draco: /Users/peter/.local/bin/quoin 0.29.4; /Users/peter/.cargo/bin/quire 0.38.3 (engine 0.52.1); spec-artifacts-process@v0.29.2. MP-012 validation exits 0 with DuplicateArchetype and semantic.inline-data-schema warnings. These versions differ from the rollout notice; no older binary was substituted. nice emits setpriority EPERM while commands complete. Exact model metadata is unavailable; marker model=unknown records that limitation.

## Coverage

Plan completion: not assessed

Reconciliation: quoin matrix --repo /Users/peter/dev/qsl-1918-code --json (quoin 0.29.4). It read 452 requirements and 4,809 criteria: 3,628 tagged, 1,069 untagged, 3 tagged-by-ignored-test, 96 method-without-symbol and 13 retired. All 4,809 have no run evidence. These are repository context, not new PR defects; this review is confined to the requested delta. No FR/AC is added or changed. MP-012 is a MeasurementPlan, not a criterion-bearing artifact. The new harness maps to QSL-1918 and MP-012; FR-068 supplies checked-call architectural context. No claim of repository-wide assurance is made. The new smallest-kernel test has no trace annotation; no existing evaluator architecture criterion is claimed to prove the new reporting behavior. No production or test stubs were found in the changed generator.

Optional whole-repository semantic audit was not selected. Direct code/test agreement for the changed kernels was inspected as part of the authorized code review.

## Artifact Custody

Scratch artifact only. The dispatching lead or fix-round coder must copy this latest artifact to reviews/qsl-1918-code-review.md; this reviewer never commits or pushes the frozen candidate.

## Dispositions

| ID | Outcome | Evidence |
| --- | --- | --- |
| FND-001 | fixed | KernelKind::ALL and SIZES are shared by benchmark, report and generator test. |
| FND-002 | fixed | main and criterion_filter both carry #[string_edge]. |
| FND-003 | still-open | The parser and interpreter filters now succeed, but no-complete-pair handling is still incorrect for Criterion options. --exact evaluator/kernel and --ignored select no benchmarks in Criterion, yet the report interprets them as selecting all pairs and fails for missing fresh estimates. --list similarly performs no measurements but still triggers the report. The separate option parser also misclassifies values for -c/--colour, --baseline-lenient and --output-format as filters. |
| FND-003 | fixed | Round 2: report rows require only complete fresh pairs; no-output and partial invocations succeed without interpreting Criterion options. |
| FND-004 | fixed | Round 2: criterion_filter, its copied option grammar and the direct regex dependency are removed. |

## New findings (disposition pass 1)

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-004 | high | criterion_filter duplicates Criterion's argument grammar and selection rules in a partial hand-maintained parser. Inspection of criterion 0.8.2 configure_from_args found omitted aliases, value-taking flags, exact/reject-all selection and non-measurement modes. Remove the second parser and derive reporting from Criterion's actual selected/executed measurements; extending the copied flag list preserves the duplication. High severity follows code-review's mandatory duplication rule. | fsl-bench/src/bin/fsl-bench-kernel-report.rs:43 |

## Disposition pass 1 verdict

**FAIL** — two original code findings are fixed, FND-003 remains open and FND-004 is new.

## Disposition pass 1 scope

```yaml
{
  "dispositions": [
    {
      "fnd": "FND-001",
      "outcome": "fixed",
      "after_excerpt": "    for kind in KernelKind::ALL {\n        let name = kind.name();\n        for size in SIZES {"
    },
    {
      "fnd": "FND-002",
      "outcome": "fixed",
      "after_excerpt": "#[string_edge]\nfn main() -> Result<(), Box<dyn Error>> {"
    },
    {
      "fnd": "FND-003",
      "outcome": "still-open",
      "reason": "The parser and interpreter filters now succeed, but no-complete-pair handling is still incorrect for Criterion options. --exact evaluator/kernel and --ignored select no benchmarks in Criterion, yet the report interprets them as selecting all pairs and fails for missing fresh estimates. --list similarly performs no measurements but still triggers the report. The separate option parser also misclassifies values for -c/--colour, --baseline-lenient and --output-format as filters."
    }
  ],
  "new_findings": [
    {
      "fnd": "FND-004",
      "severity": "high",
      "confidence": "high",
      "method": "code-review",
      "check": "other",
      "artifact_id": "fsl-bench-kernel-report::criterion_filter",
      "related_ids": [
        "QSL-1918",
        "MP-012"
      ],
      "path": "fsl-bench/src/bin/fsl-bench-kernel-report.rs",
      "lines": "43-81",
      "excerpt": "#[string_edge]\nfn criterion_filter(arguments: &[String]) -> Result<Option<String>, io::Error> {",
      "finding": "criterion_filter duplicates Criterion's argument grammar and selection rules in a partial hand-maintained parser. Inspection of criterion 0.8.2 configure_from_args found omitted aliases, value-taking flags, exact/reject-all selection and non-measurement modes. Remove the second parser and derive reporting from Criterion's actual selected/executed measurements; extending the copied flag list preserves the duplication. High severity follows code-review's mandatory duplication rule."
    }
  ],
  "scope": [
    {
      "id": "fsl-bench-kernel-report::criterion_filter",
      "path": "fsl-bench/src/bin/fsl-bench-kernel-report.rs",
      "role": "examined",
      "excerpt": "#[string_edge]\nfn criterion_filter(arguments: &[String]) -> Result<Option<String>, io::Error> {"
    }
  ],
  "bindings": []
}
```

## Disposition pass 1 verification

Reviewed head: 4e7e95c268dc5686f37255299f70c4a3e5f51052; fix commits d8119bfa and 4e7e95c; original reviewed head 1dce3f80fb4bc9a5967dacb4398f4b78933e721f. Original findings above remain unchanged. No repository source edits or pushes were made by the reviewer. No aggregate gate ran.

The kernel population now has one KernelKind::ALL catalog and one SIZES constant shared by the bench, reporter and generator test. Both report CLI boundaries carry ix_trace_rs::string_edge. The specialized Make recipe is excluded from the generic recipe set; make -n bench-evaluator_kernel BENCH_ARGS=parser and make -n bench BENCH_ARGS=parser both succeed without the previous override warning and retain the axis.

The report now has three Rust tests covering fresh normalization for both sizes, stale and incomplete expected pairs, and the parser/interpreter filters. MP-012 and BENCHMARKING.md have no fix-round delta; no new specification issue was found, and the base/integrity reviews remain PASS. MP-012 validation exits 0 with the previously reported module warnings.

The Criterion source at installed criterion 0.8.2 src/lib.rs::configure_from_args is the authority for flag behavior. Its exact filter and --ignored paths accept no benchmark for --exact evaluator/kernel and --ignored respectively; its --list mode emits no measurements. The copied criterion_filter does not preserve these distinctions. Additional missing option spellings include -c, --colour, --baseline-lenient and --output-format. This is a functional review, not a live performance comparison.

## Disposition pass 1 execution results

The slot-wrapped cargo test --locked -p fsl-bench --all-features --bin fsl-bench-kernel-report exits 0: all three new tests pass. Slot-wrapped make string-edge exits 2 and reports exactly 15 unchanged base occurrences, with none in fsl-bench-kernel-report.rs. No aggregate gate was run.

A scratch Rust driver compiled the unchanged current report source against this worktree's built dependencies and executed its real CLI. report -- parser and report -- interpreter exit 0. report -- --exact evaluator/kernel, report -- --ignored and report -- --list each exit 1 with "expected fresh interpreter Criterion estimate for sum/5". The Criterion benchmark executable accepts the corresponding no-match exact and ignored invocations with exit 0. Logs: /private/tmp/qsl-1918-review-round1-report-tests.log, /private/tmp/qsl-1918-review-round1-string-edge.log and /private/tmp/qsl-1918-review-round1-cli-probe.log.

The new source has no whitespace errors. The full fix-round diff check also identified trailing blank lines in the copied original review artifacts; the updated scratch artifacts are normalized for their next custody commit.

## Disposition pass 2 verdict

**PASS** — FND-003 and FND-004 are fixed. Original FND-001/FND-002 remain fixed; no new findings.

## Disposition pass 2 scope

```yaml
{
  "dispositions": [
    {
      "fnd": "FND-003",
      "outcome": "fixed",
      "after_excerpt": "            let Some(interpreter) = estimate(&criterion, \"interpreter\", name, size, marker)? else {\n                continue;\n            };\n            let Some(rust) = estimate(&criterion, \"rust\", name, size, marker)? else {\n                continue;\n            };"
    },
    {
      "fnd": "FND-004",
      "outcome": "fixed",
      "after_excerpt": "    match command.as_str() {\n        \"start\" => mark_start(target),\n        \"report\" => report(target),\n        _ => Err(io::Error::other(format!(\"unknown command: {command}\")).into()),\n    }"
    }
  ],
  "new_findings": [],
  "scope": [
    {
      "id": "fsl-bench-kernel-report::run",
      "path": "fsl-bench/src/bin/fsl-bench-kernel-report.rs",
      "role": "examined",
      "excerpt": "fn run(target: &Path, arguments: impl Iterator<Item = String>) -> Result<(), Box<dyn Error>> {"
    },
    {
      "id": "fsl-bench-kernel-report::collect_rows",
      "path": "fsl-bench/src/bin/fsl-bench-kernel-report.rs",
      "role": "examined",
      "excerpt": "fn collect_rows(target: &Path, marker: u128) -> Result<Vec<ReportRow>, Box<dyn Error>> {"
    },
    {
      "id": "no_measurement_criterion_invocations_report_an_empty_result",
      "path": "fsl-bench/src/bin/fsl-bench-kernel-report.rs",
      "role": "examined",
      "excerpt": "        for arguments in [\n            vec![\"report\", \"--exact\", \"evaluator/kernel\"],\n            vec![\"report\", \"--ignored\"],\n            vec![\"report\", \"--list\"],\n        ] {"
    },
    {
      "id": "partial_filter_results_report_only_complete_fresh_pairs",
      "path": "fsl-bench/src/bin/fsl-bench-kernel-report.rs",
      "role": "examined",
      "excerpt": "        let rows = collect_rows(&target.0, 0).expect(\"partial benchmark population succeeds\");\n        assert_eq!(rows.len(), 1);\n        assert_eq!(rows[0].kernel, \"sum\");\n        assert_eq!(rows[0].size, 5);"
    },
    {
      "id": "malformed_fresh_estimates_remain_errors",
      "path": "fsl-bench/src/bin/fsl-bench-kernel-report.rs",
      "role": "examined",
      "excerpt": "        let error = collect_rows(&target.0, 0).expect_err(\"a malformed fresh estimate is invalid\");\n        assert!(error\n            .to_string()\n            .contains(\"Criterion estimate has no mean point estimate\"));"
    },
    {
      "id": "fsl-bench-manifest",
      "path": "fsl-bench/Cargo.toml",
      "role": "examined",
      "excerpt": "serde_json = \"1.0.151\""
    },
    {
      "id": "MP-012",
      "path": "spec/assurance/MP-012-evaluator-kernel-wall-time.md",
      "role": "context_only",
      "excerpt": "The report prints the\nCriterion estimate divided by the frame and operation counts, plus the ratio\nof interpreter ns per operation to Rust ns per operation."
    }
  ],
  "bindings": []
}
```

## Disposition pass 2 verification

Reviewed head: e086513a3868018c5b1c0532c8b5c9a6eeab77e1; prior head: 4e7e95c268dc5686f37255299f70c4a3e5f51052. No repository source or branch edits were made by the reviewer. No aggregate gate ran.

FND-003 is fixed: the reporter no longer predicts selection from command-line flags. It renders only complete fresh backend pairs from the current invocation; absent, partial and stale pairs produce no row. The freshness guard remains intact, and malformed estimates encountered by the reader remain errors. The run entry accepts forwarded Criterion options without interpreting them, so --exact, --ignored and --list no-measurement invocations succeed.

FND-004 is fixed: criterion_filter, the copied option table, regex matching, the regex import and the direct fsl-bench regex dependency are removed. Criterion owns its own selection semantics. Shared KernelKind::ALL/SIZES and the string_edge boundary markers remain intact, so original FND-001/FND-002 stay fixed. The gap finding remains fixed: the report now contains five behavioral tests. No new finding was identified.

Independent reviewer checks: slot-wrapped cargo fmt --all -- --check, git diff --check against the prior reviewed head, and the two Make dry runs for --exact evaluator/kernel and --ignored all passed. MP-012 and all four scratch review artifacts validate with the previously recorded module warnings. MP-012/BENCHMARKING.md have no fix-round delta, and both specification reviews remain PASS.

Execution evidence was inspected in the coder's current /private/tmp/qsl-1918-round2-test.log: 24 library tests and five report tests passed, including normalization for both sizes, stale/incomplete pairs, partial fresh output, malformed estimates and no-measurement invocations. The current /private/tmp/qsl-1918-round2-check.log, -clippy.log and -clippy-nodef.log show completed checks. The current string-edge log reports only the 15 unchanged base occurrences, none in the report. The ticket-local target directory was deleted before this disposition pass; these Rust execution results are credited to the coder, not represented as reviewer reruns. The reviewer independently inspected the changed implementation and every changed test against the open findings.

All review findings are now resolved. This is approval of the reviewed delta, not a claim that the aggregate gate or a live performance comparison ran.
