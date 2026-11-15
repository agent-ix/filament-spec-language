---
id: SR-2026111414542500000010
title: "QSL-1918 spec-review/base of evaluator kernel benchmark"
type: SpecReview
analysis: base
scope: "agent-ix/filament-spec-language; QSL-1918; Makefile; fsl-bench/Cargo.toml; fsl-bench/benches/evaluator_kernel.rs; fsl-bench/src/kernel.rs; fsl-bench/src/bin/fsl-bench-kernel-report.rs; fsl-bench/src/lib.rs; fsl-bench/BENCHMARKING.md; spec/assurance/MP-012-evaluator-kernel-wall-time.md"
review_set: subset
relationships:
  - target: ix://agent-ix/filament-spec-language/MP-012
    type: references
---

## Summary

MP-012 declares its subject, fixed population, paired collection procedure, estimator, uncertainty, controls and existing FR-068 measurement relationship. The changed artifact validates; no base-checklist defect was found.

No applicable AssuranceProfile was found under spec/. Direct review is permitted; no enforced review or acknowledgement is required. Methods run: code-review with rust-review/rust-style, gap-analysis, spec-review/base and spec-review/integrity. Other language lanes and unrelated requirement lenses were not applicable. MP-012 changes no requirement statement or domain object. Its measures relationship names existing FR-068; no new dependency ordering is introduced. Jev/criterion-strength was not selected.

## Verdict

**PASS** — MP-012 declares its subject, fixed population, paired collection procedure, estimator, uncertainty, controls and existing FR-068 measurement relationship. The changed artifact validates; no base-checklist defect was found.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No findings (placeholder) | - |

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

Scratch artifact only. The dispatching lead or fix-round coder must copy this latest artifact to reviews/qsl-1918-spec-review-base.md; this reviewer never commits or pushes the frozen candidate.

