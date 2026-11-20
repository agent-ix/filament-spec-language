# QSL-2084 authoring validation

Twelve new active tests each passed focused execution. No new ignored test is counted as coverage. The independent no_std evaluator and retirement facets remain pending IR-590 and IR-593; these std cases do not satisfy those unavailable capabilities.

| Focused execution segment | Actual result |
| --- | --- |
| Initial new-case filter | Seven passed; two stopped in an author fixture using reserved word `identity`. Those fixture names were corrected. |
| Corrected checked-expression admission case, exact name | One passed, exit 0. |
| Corrected closed/protocol/kernel charge case, exact name | One passed, exit 0. |
| Complete-empty/missing/incomplete population case, exact name | One passed, exit 0, after grounding the two distinct missing/incomplete causes in FR-391 Evaluation. |
| History read and missing-history refusal, exact name | One passed, exit 0. |
| PopulationObjects ordering and complete charge-record stability, exact name | One passed, exit 0. The ten iterations are the explicit FR-406 AC6 stability obligation within this one test. |

The seven initial passes are terminal/state predicate and clause trails, reward arithmetic and charges, temporal holds, separation witness replay, refinement population rows, history pre/post updates, and reachable value invariants. Passing assertions were not rerun for confidence. The first compilation found an authoring use of an unavailable `Outcome::map`; the fixture code was corrected before runtime execution.

All authoring invocations used the single owned target, jobs 2, debug 0, incremental 0, and one test thread. Cargo invocations retained both user skips. The exact-name invocations of the already-built integration binary executed only the named new test. No full component gate has run during authoring.

## Required current feature modes

With 211G free before the builds:

- `cargo build --locked -p fsl-foundation --no-default-features --target thumbv7em-none-eabi`: exit 0. This is the actual embedded-target dependency-closure build, with no dev-dependency feature unification.
- `cargo check --locked -p fsl-package --no-default-features`: exit 0. This is the host crate-tier no_std row; it does not claim its permitted std dependencies form a no_std closure.

Both used the same owned target and jobs 2/debug 0/incremental 0. No product binary or workspace build was run. No fake no_std evaluator consumer was substituted for the unavailable checked-term evaluator.

## Scoped lint limitation

Scoped Clippy was not run through known unchanged warning blockers observed in actual compilation: `fsl-eval/src/value/expression/family.rs:156` (`EvaluationEnv::charged`, dead code) and `fsl-eval/tests/it/reference_adapter.rs:13` (`LiveContext`, unused import). This is not a green lint claim. The previously reported 595 semantics findings, four analyze library findings and two root findings were not retried or suppressed. The independent Rust review must inspect the changed test code directly.

The independent feature review and one final applicable component gate remain for the code lead. Private mode logs and exact exits are retained under `/home/peter/dev/.agent/qsl-2084/logs/`.
