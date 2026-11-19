---
id: SR-2026111911313600000003
title: "Code review — QSL-2274 replay library clock selections, PR #3025"
type: SpecReview
analysis: code-review
scope: "agent-ix/filament-spec-language; QSL-2274; PR #3025 at c7a88053eff5f2820025171f288e247ab449c4c9 against main 48d0a94b; fsl-analyze/src/monitor_root.rs; fsl-analyze/tests/it/{monitor_root.rs,protocol_constructs.rs}; fsl-command/src/{command/output.rs,emit_checked_v2.rs}; fsl-package/src/emit.rs; fsl-replay/src/{admitted.rs,execute.rs,execute/tests.rs,execute/tests/complete_package.rs,execute/tests/composite.rs,outcome.rs,request/package.rs,spine/clause/tests/call_site.rs,spine/clause/tests/frame_replay.rs,spine/clocks.rs,spine/lifecycle/tests.rs}; fsl-replay/tests/compile_package_facade.rs; fsl-replay/tests/it/compile_protocol_profile.rs; tests/it/{diagnostic_golden.rs,replay_library_clocks.rs,replay_multi_source.rs}; xtask/src/qualification/adapters.rs; FR-640-AC-8; FR-640-AC-11"
review_set: subset
relationships:
  - { target: "ix://agent-ix/filament-spec-language/FR-640", type: references }
---

## Summary

Ticket QSL-2274. Static code review with the Rust lane folded in. The change records dependency clock selections in the proving package lock, reads separate program and dependency clock maps during replay, preserves the dependency lock pointer through nested compile refusals, and tests the success and refusal cases. No code defect was found. The PR description reports that TC-2213 passed; this reviewer did not independently complete the focused test.

## Verdict

**PASS** — no findings in the changed code. No aggregate gates were run.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No findings (placeholder) | - |

## Coverage

The source and call sites for each changed Rust file were inspected against base `48d0a94b`. The requirement context examined was FR-640-AC-8 and FR-640-AC-11. `ReplayPackage::clocks` reads the package's program rows and lock dependency rows into separate maps; `RequestClocks` selects by source identity; compile refusals from a dependency retain the correct dependency-clock pointer through wrappers; `EmitRefusal` cause and pointer are surfaced by replay and command adapters. The emitter walks the dependency closure and writes dependency selections in deterministic clock-name order. The integration test covers a library-only clock, missing/unused/wrong-kind lock entries, and same-name program/library usage.

Rust review: no new public API contract promises are inconsistent with implementation; no unsafe, unchecked narrowing, panic path from wire values, or unbounded traversal was found. The changed public refusal variant shape is deliberate to carry the wire pointer; this bootstrap repository documents no released compatibility promise. Test assertions exercise the real compile and replay paths. The modified trace tags preserve the existing criterion tags.

Focused verification: `ulimit -v 16777216; CARGO_HOME=/tmp/qsl2274-cargo CARGO_BUILD_JOBS=1 CARGO_TARGET_DIR=/tmp/qsl2274-target cargo test -p filament-spec-language --test it replay_library_clocks::replay_recovers_a_library_only_clock_from_the_proving_package -- --exact --test-threads=1` — not completed by this reviewer. The direct build was stopped during dependency compilation without a test result; a retry through `locked-build.sh` waited over two minutes for occupied lanes and was stopped. The PR description reports TC-2213 passed.

No AssuranceProfile applies to the changed behavior. No plan was supplied; Plan completion: not assessed. No code was edited by the reviewer.
