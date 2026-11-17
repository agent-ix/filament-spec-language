---
id: SR-6026
title: "Code review of the filament-examples gate"
type: SpecReview
analysis: code-review
scope: "agent-ix/filament-driver; QSL-2100; PR 157 at 5348e33665ca62083e030ce3bf835e084483adba; tests/examples_gate.rs, Makefile, Cargo.toml, Cargo.lock, CLAUDE.md"
review_set: subset
---

## Summary

Ticket: QSL-2100. Diff-only code review, with the Rust lane (rust-style), of the gate that builds `fsl` from this workspace and runs the filament-examples runner over every running and staged case. Checked each of the six ticket ACs against its test, the runner's message strings and exit codes on filament-examples main (tools/fsl-examples/src/outcome.rs, main.rs, runner.rs), the Makefile plumbing, and the three coder deviations. No cargo or make runs (lead's make ci is running in the tree); the coder reports 6/6 gate tests passing (282 running, 21 staged).

## Verdict

APPROVE. All six ACs are met by tests that can fail: both suite runs assert exit 0 plus a parsed `N passed, 0 failed` summary with N above zero; the two negative tests edit a tempdir copy of one system (the fetched checkout is never written) and assert exit 1 plus the exact runner message naming `<system>/<case>`, which matches outcome.rs on examples main; a missing or unset checkout panics in `checkout()`, so every gate test fails rather than skips, and the runner exits 2 on a suite with no examples/. The built fsl is the workspace build (version equals fsl-cli 0.1.3 and the path lies under the workspace target dir), and the runner receives it through `FSL`. Makefile is plumbing only and follows examples main via fetch + detached FETCH_HEAD with no hash. No Python, no Node, no .github change. Deviations are sound: `test = false` keeps plain `cargo test` offline, and `--all-targets` skips such targets, so the extra clippy line is required; the runner crate has no `[workspace]`, so `workspace.exclude = ["target"]` is required; tempfile is a dev-dependency used only by the gate. Two low findings, neither blocking.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | The gate tests carry no trace tags and no driver requirement owns the gate, so the six ACs exist only in the Linear ticket and the computed matrix cannot see them. Spec lane to add an owning FR, then tag the tests. | tests/examples_gate.rs:236-379 |
| FND-002 | low | `case_lines` hand-parses example.toml by `key = value` lines and keeps assigning keys to the last `[[case]]` after any later table header; the runner itself parses with the `toml` crate. A manifest with a table after the cases, or an inline comment on an exit line, would mis-select or corrupt the edited case. Parse with `toml` for selection and keep the line edit only for the swap. | tests/examples_gate.rs:161-187 |
