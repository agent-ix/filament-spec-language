---
id: SR-2026112014571801
title: "code-review of PR 3196 hybrid process providers (QSL-2532, QSL-2534)"
type: SpecReview
analysis: code-review
scope: "agent-ix/filament-spec-language; PR 3196 head 2496539bba0ddfb387f57a3408fbf69ceaf005cb; fsl-analyze/src/{analyze.rs,analyze/hybrid_item.rs,analyze/hybrid_process.rs,analyze/request.rs}, fsl-replay/src/{hybrid/process_request.rs,outcome.rs,proof_result/plugin.rs,result.rs}, tools/fsl-hybrid-test-provider/**, fsl-analyze/tests/it/hybrid_requests.rs"
review_set: subset
---

## Summary

Ticket: QSL-2532 (with QSL-2534). Code review with the Rust lane (rust-review), read-only over `git diff origin/main...HEAD`. No build was run: the one hole found is traced through the code path end to end, and confirming it by a run would need a new test (a code edit, out of scope for this role).

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | A process `proved` entry with a non-hybrid basis (`exhaustive`, `bounded-complete`, `inductive`, `reduced`) is NotRead and then settled by the generic plugin reader as `proved`/`trusted` with that basis, bypassing FSpec FR-193's hybrid basis rule and FR-250's coverage rule; `tested`/`measured` entries take the same path | fsl-analyze/src/analyze/hybrid_process.rs:88, fsl-analyze/src/analyze/hybrid_process.rs:155-171, fsl-replay/src/hybrid/provider.rs:274 |
| FND-002 | medium | The request frame body is the bare FR-331 `Request`, not the request stage (`contract_version`, `capability_vocabulary`, `manifest`, `request`); `dependencies` is never written for a package with a non-empty dependency closure; `candidates` is the serving id instead of the computed candidate set | fsl-analyze/src/analyze/hybrid_process.rs:93-114, fsl-replay/src/hybrid/process_request.rs:211-260 |
| FND-003 | medium | The outcome reader accepts a body that is not an FR-331 outcome stage (no `contract_version`, `dispositions`, `accounting`; unknown members), an entry whose `backend` is another backend or that has no `kind`, and `counterexamples` entries for another item or backend | fsl-analyze/src/analyze/hybrid_process.rs:119-152 |
| FND-004 | medium | The serialized `hybrid-trace` disagreement carries only `type`; FR-450 requires the complete provider counterexample in it | fsl-replay/src/outcome.rs:258-262, fsl-replay/src/outcome.rs:872-879 |
| FND-005 | medium | Cancellation is untested, and the test host is not FR-462 faithful on it: a `result` after `cancel` is passed through unchanged (a post-cancel `proved` settles `proved`), and a cancel before `hello` settles `envelope-member` | tools/fsl-hybrid-test-provider/tests/it/host.rs:132-158, tools/fsl-hybrid-test-provider/tests/it/host.rs:171-173 |
| FND-006 | medium | Two parallel tests share `Workspace::new("failure")` (same pid), and `Workspace::new` deletes the directory, so one test can delete the other's `script.json` while its child reads it: a flaky test | tools/fsl-hybrid-test-provider/tests/it/support.rs:26-31, tools/fsl-hybrid-test-provider/tests/it/process_provider.rs:232 |
| FND-007 | low | Trace tags bind the wrong ACs: the misbehaving-provider and deadline tests claim FR-750-AC-5 (refutation replay) and the missing-executable test claims FR-750-AC-4 (`unsupported`/`tool-unavailable`, while the test asserts `failed`/`crash`) | tools/fsl-hybrid-test-provider/tests/it/process_provider.rs:248-249, tools/fsl-hybrid-test-provider/tests/it/process_provider.rs:296-297, tools/fsl-hybrid-test-provider/tests/it/process_provider.rs:309-310 |
| FND-008 | low | The FR-1021-AC-3 stub still leads with `QSL-2534` (Done) in `ignore` and `todo!`; the owner is QSL-1978, as on the FR-1021-AC-1 stub | fsl-analyze/tests/it/hybrid_requests.rs:1161-1163 |
| FND-009 | low | `let _ = (interval("0", "1"), PROVIDER);` exists only to silence unused-item warnings | tools/fsl-hybrid-test-provider/tests/it/process_provider.rs:348 |

## Verdict

Not mergeable until FND-001 is fixed. A provider `proved` with a solver basis settles `trusted`, `certified` only through FR-751 (tested). A refutation counts only after exact replay or an accepted witness (`from_provider_certified`, tested). Garbage, non-canonical frames, a foreign protocol, oversize frames, crashes, non-zero exits, a silent provider, a duplicate or missing item-0 entry and a timeout all settle `failed` or `incomplete`, never a verdict. The one hole is the NotRead fallback for `proved`.

Example bytes for FND-001: the `result` body `{"counterexamples":[],"results":[{"backend":"hybrid-process","basis":{"type":"exhaustive"},"certification":"trusted","kind":"temporal-satisfaction","request_index":0,"result":"proved"}]}` for `Positive` (`always [0, 12] holds(h.v > 0)` over `Braking`, which the AC-5 point trace refutes). `read_entry` returns `NotRead("proved")`; `unread_entry` → `read_result_entry(.., Plugin)` → `from_plugin_proof` admits `Exhaustive` → the item is `proved`, `trusted`, basis `exhaustive`. With `{"type":"bounded-complete","depth":1}`, a provider avoids the `insufficient-horizon` that the same proof gets as `bounded-solver` with horizon 1.

Rulings: (a) FR-750 "The driver's plugin host implements the exchange over the child process (FR-635)" and FR-635 "The driver's `ProcessProvider` (FR-287) implements `ExternalProvider` and only transports" put the real host in the driver, so a test-only host is correct and nothing production-grade is duplicated. It is not faithful on cancellation (FND-005). (b) Yes in part, see FND-002. (c) See FND-005. (d) A finding: FND-004. (e) The dependency is right but the label is not: FND-008.

## Scope and evidence

- FR-750 (spec/functional/FR-750-run-a-hybrid-item-through-its-negotiated-hybrid-provider.md): For a hybrid item whose candidate is a process provider, analyze SHALL write the FR-447 process request with the emitted package's v2 bytes, read the provider's FR-331 result entry, and settle it by the same rules.
- FR-750-AC-5 (spec/functional/FR-750-run-a-hybrid-item-through-its-negotiated-hybrid-provider.md): A process fixture provider receiving the v2 bytes of a clause request over SlowEnough returns a refuted point trace that exact replay reproduces; the item settles refuted with one HybridCounterexample; a trace that does not reproduce settles inconclusive, replay-parity.
- FR-750-AC-7 (spec/functional/FR-750-run-a-hybrid-item-through-its-negotiated-hybrid-provider.md): process fixture whose entry is unsupported with unsupported_construct/expression-form settles unsupported with that diagnostic.
- FR-750-AC-11 (spec/functional/FR-750-run-a-hybrid-item-through-its-negotiated-hybrid-provider.md): a failed tool-failure entry returns NotRead and the item settles failed by FR-290.
- FR-750-AC-13 (spec/functional/FR-750-run-a-hybrid-item-through-its-negotiated-hybrid-provider.md): read_entry reads the process expression-form entry via the emitted-package locus.
- FR-751 (spec/functional/FR-751-check-hybrid-provider-certificates-in-the-core.md): certificates checked in the core; only an accepted certificate makes a proof certified.
- FR-290 (spec/functional/FR-290-settle-plugin-results-as-typed-terminal-records.md): plugin proved settles trusted.
- FR-450 (spec/functional/FR-450-replay-a-hybrid-trace-against-the-checked-hybrid-model.md): The serialized disagreement cause SHALL use {"cause":"hybrid-trace","disagreement":{"type":"not-reproduced","counterexample":{...}}}.
- FR-447, FR-635, FR-287-AC-1, FR-1021-AC-3: context.
- FSpec FR-193 (filament-specification origin/main): A proof SHALL be a proved entry with the basis bounded-solver{...} or inductive-solver{method}.
- FSpec FR-462: Lifecycle (cancel), Failures table, request frame = FR-331 request stage.
- FSpec FR-331: RequestStage/OutcomeStage closed members; dependencies/provision optional (minItems 1).
- FSpec FR-652: certificate checking (context).

## Recommendations

- FND-001: in `HybridAnswer::read_entry`, replace `_ => NotRead("proved")` with `Entry(PluginResultRefusal::Malformed { pointer: "/basis/type", cause: InvalidValue })`. In `unread_entry`, admit only `failed`, `unsupported` and the `inconclusive`/`incomplete` causes `read_entry` leaves unread, and refuse any other `result` `invalid_result`/`invalid-value` at `/result`. Add process tests for the `exhaustive` and `bounded-complete` bases and for a `bounded-solver` proof whose horizon is short of the claim (settles `insufficient-horizon`).
- FND-002: write the FR-331 request stage `{contract_version, capability_vocabulary, manifest, request}`, with `manifest` the registry snapshot (FR-635 step 2). Write `request.dependencies` (`[{identity, package}]`, ordered by name) whenever the closure is non-empty. `provision` is not needed for hybrid items. `candidates` is the item's computed `CandidateOutcome` set.
- FND-003: check the outcome stage's closed member set. Require `entry.backend == call.backend` (`unrouted-item` otherwise), require `kind`, and require each `counterexamples` entry to name `request_index` 0 and the backend.
- FND-004: serialize `disagreement: {type, counterexample: HybridCounterexample::to_wire(model, wire)}`, and assert the counterexample in the FR-450-AC-3/AC-9 tests.
- FND-005: after `cancel`, settle `incomplete`/`cancelled` whatever the provider answers, and treat a cancel before `hello` as cancelled. Add a test that cancels the run's `Cancel` during a `hang` script (`incomplete`, `cancelled`, `source` `requested`) and one whose script answers `proved` after `cancel`.
- FND-006: make each `Workspace` name unique, for example with an `AtomicU64` counter or the test name.
- FND-007: retag to FSpec FR-462-AC-4/AC-5/AC-6 (TC-428), or drop the tags.
- FND-008: `#[ignore = "QSL-1978: hybrid plan synthesis ..."]` and `todo!("QSL-1978: FR-1021-AC-3")`.
- FND-009: drop the unused helpers or their imports.
