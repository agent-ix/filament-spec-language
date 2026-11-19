---
id: SR-15208994
title: "code review of PR 2742 clocks override entries"
type: SpecReview
analysis: code-review
scope: "agent-ix/filament-spec-language; PR 2742 head de5d2fee68ac93089d115c514c3f6bdd940233b4; fsl-command/src/command.rs, fsl-command/src/command/output.rs, fsl-command/src/command/wire.rs, fsl-command/tests/it/request_overrides.rs"
review_set: subset
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-640
    type: references
---

## Summary

Ticket: QSL-2294. Code review with the rust-review lane (rust-style loaded) of `git diff origin/main...HEAD`. The PR adds `RequestOverrides { clocks }`, the `*_with_overrides` compile and run entries, and `RunCause::ClocksGivenTwice`. No defects found.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No findings (placeholder) | - |

## Verdict

APPROVE. Checked against FR-640 "Behavior":

- Both-given refusal: `ClocksGivenTwice` maps to `Code::InvalidRequest`, stage `request` (output.rs), exit 20 (asserted through `category().exit_code()`). The message names `clocks` and `--clocks`. The check is on member presence in the raw request object, so a request carrying `"clocks": {}` plus an override also refuses. That is the right reading, because the decoded `RequestClocks` cannot tell an empty member from an absent one, and this justifies the second envelope parse.
- Override read as the member: `wire::read_clocks` calls the same private `clocks` deserializer the request field uses, so the unknown-kind, repeated-name, empty-name, non-object and `RequestClocks::new` refusals are shared. A `RawValue` is one complete JSON value, so the missing `Deserializer::end` cannot admit trailing input.
- The override replaces `request.clocks` in `request_with_overrides` before `prepare_run_bytes` branches, so call runs and clause runs (`clause::read` clones `request.clocks`) both see it. The compile path sees it before `ExtractionRequiresRun`.
- Path-only entries (`compile`, `compile_outcome`, `run`, `run_with_limits`, `prepare_run`) delegate with `RequestOverrides::default()`. `request_with_overrides` with no override is exactly `request(bytes)`, so their behaviour is unchanged. The existing `clock_selection` and `run_wire` suites pass, as does a path-only assertion in the new tests.
- `read_clause_run_request` takes no override. Only tests use it, not the driver, so it is outside QSL-2294's scope.

Evidence: `cargo test -p fsl-command --test it request_overrides` (4 passed); `cargo test --test it -- clock_selection run_wire` (8 passed, 19 ignored, pre-existing); `cargo clippy -p fsl-command --all-targets -D warnings` and `cargo fmt --check` clean. No Python or logic-bearing shell in the diff.
