---
id: SR-96750
title: "code-review of QSL-3500 definitional reference"
type: SpecReview
analysis: code-review
scope: "agent-ix/filament-spec-language; fsl-definitional-reference/, fsl-analyze/, tools/arch-lint/, xtask/src/string_edge.rs, Cargo.toml, Cargo.lock, Makefile; PR4054 96e6b140d89ccb5fa1ba1db542cb475387701c61"
review_set: subset
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-1051
    type: references
---

## Summary

Ticket: QSL-3500. PR agent-ix/filament-spec-language#4054 at 96e6b140d89ccb5fa1ba1db542cb475387701c61. code-review with the rust-review lane over all eleven changed files.

## Verdict

**FAIL**. Four high findings.

Examined and clean: the reference's direct dependencies and every use path in lib.rs name no fsl-eval, fsl-analyze, fsl-replay or fsl-hybrid-reference item; it passes its own RewardEvaluator; it is a dev-dependency of fsl-analyze only; no duplicate interface type was found. The horizon test's values re-derive by hand: horizon 1, action A gives 3/5 and action B gives 0; horizon 2, A gives 3/5 and B gives 9/10.

Runs: `cargo test -p fsl-analyze --test it false_claim_corpus::` (6 PR tests pass; 2 temporary reviewer probes, removed afterwards). `arch-lint qualified-core` on a temporary copy with three escaping files (AC-18 line PASS; ambient-input FAIL with 15 findings in files this PR does not change). `cargo fmt --all --check` (diff on main.rs:32-33; it also reports files outside this PR, not assessed). clippy, cargo deny and the other reference tests were not run.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | validate_tiny_fragment refuses a quantile claim whose scheduler is ProbScheduler::Workload. Workload is `under W`, a declared workload that fixes the resolution (a chain the reference must decide), not 'some scheduler'; the only other variant, Every, is admitted. Observed: `quantile ... under every scheduler` returns Ok(()), and the in-fragment `under Even` quantile is refused. The AC-19 test reference_refuses_some_scheduler_quantiles (fsl-analyze/tests/it/false_claim_corpus.rs:104-121) uses `under Even` and so asserts the inversion. Settle what 'over some scheduler' means against the grammar (it has only Workload and Every) before coding it. | fsl-definitional-reference/src/lib.rs:253-259 |
| FND-002 | high | validate_conjunctive_guards looks only for Connective::Or (and multi-part timed guards). Observed: untimed preconditions `self.v = 0 implies self.v = 1` and `not (self.v != 0 and self.v != 1)`, both disjunctions, pass validate_tiny_fragment with Ok(()). By reading, a disjunction inside a called function body is not followed either. | fsl-definitional-reference/src/lib.rs:157-185 |
| FND-003 | high | The check is a deny-list of a few shapes, then Ok(()). The spec requires accept-only-the-fragment. Observed Ok(()) for: precondition `self.v * self.v = 1`; a native Rational field; a native Text field (both infinite state). By reading, nothing examines effects, frames, the 2^16 valuation bound of a checked model (only TinyMdp rows are bounded), task sets, hybrid guards or resets, or claim shapes. Build an allow-list that matches every node and form exhaustively and refuses the rest. | fsl-definitional-reference/src/lib.rs:190-263 |
| FND-004 | high | The lint does not fail on what AC-18 names. The fsl-eval RewardEvaluator implementation is fsl_eval::value::S6aRewardEvaluator; the scanner never looks for it, and instead flags `impl fsl_eval::..::RewardEvaluator for _`, a trait fsl_eval does not export (the trait is fsl_semantics::check::RewardEvaluator). Observed: a copy of the tree with three added reference files (`use fsl_eval::value::S6aRewardEvaluator;`, `use fsl_eval::simulation::model as stepping; stepping::ModelSystem`, `use fsl_eval::simulation::model::*; ModelSystem`) reports 'FR-1051-AC-18 [definitional reference isolation]: PASS'. The unit test's fixture encodes the same misreading. | tools/arch-lint/reference_base.rs:29-110 |
| FND-005 | medium | some_scheduler_reachability_within recurses to `horizon` with no memo and no limit, and returns Result but never refuses a horizon. Two successors and horizon 64 is about 2^64 calls (hang); a large horizon on a self-loop overflows the stack and aborts the test process. Memoise on (state, remaining) iteratively and refuse above a declared limit. (By reading; not run.) | fsl-definitional-reference/src/lib.rs:92-141 |
| FND-006 | medium | `cargo fmt --all --check` (Makefile ci lane) reports a diff here: false_claim_corpus must sort before fill. The gate fails on this PR's line. | fsl-analyze/tests/it/main.rs:32-33 |
| FND-007 | low | scan returns no findings when fsl-definitional-reference/src is not a directory, so the report prints PASS after a crate rename or move with nothing checked. | tools/arch-lint/reference_base.rs:131-135 |
| FND-008 | low | The manifest test splits text between `[dependencies]` and `[lints.rust]`. A `[dependencies.fsl-eval]`, `[build-dependencies]` or target dependency table placed after `[lints.rust]` is not read, so the forbidden dependency passes this test. | fsl-analyze/tests/it/false_claim_corpus.rs:10-80 |
