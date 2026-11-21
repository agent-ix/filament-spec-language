---
id: SR-202611210022442313144108557
title: "code-review of QSL-2739 PR #3385 registered composition provider path"
type: SpecReview
analysis: code-review
scope: "agent-ix/filament-spec-language; BASE 97ac3336d6b6a660a4752486b39a276bb31c82c2..HEAD 13623d294863e357a3d854915a38d74f342c1b11 (26 files): fsl-analyze/src/analyze.rs, fsl-analyze/src/analyze/document.rs, fsl-analyze/src/analyze/document/composition_item.rs, fsl-analyze/src/analyze/request.rs, fsl-analyze/src/provider.rs, fsl-analyze/tests/it/analyze_composition_cli.rs, fsl-package/src/emit/tests/composition.rs, fsl-replay/src/outcome.rs, fsl-replay/src/planner.rs, fsl-replay/src/planner/{compose,composition_tests,declaration,from_package,mismatch,settle,tests}.rs, fsl-replay/src/proof_result.rs, fsl-replay/src/proof_result/{plugin,plugin_tests}.rs, fsl-route/src/{declaration,request}.rs, fsl-semantics/src/check/{check,composition,guarantee,mod}.rs, tests/it/composition_planner.rs; FR-1044, FR-530, FR-507, TC-1330 at ef92a5a0d45956983dbba9030c667a95776f637d; FSpec FR-331, FR-492, FR-491 at 5423fcf4829b2b9d927ecae95305fcc0c1e06aa6"
review_set: subset
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-1044
    type: references
  - target: ix://agent-ix/filament-spec-language/FR-530
    type: references
---

## Summary

Ticket: QSL-2739. Reviewed draft PR #3385 at frozen head 13623d294863e357a3d854915a38d74f342c1b11 over base 97ac3336d6b6a660a4752486b39a276bb31c82c2 (verified as the merge base). origin/main has moved to dab8c60624; that newer main is not counted as PR diff. The Rust lane (installed dev-tools 0.2.0 `rust-review` with `rust-style`) is folded into this artifact. The spec authority is the actual normative text of FSL FR-1044 and FR-530 at ef92a5a0 (unchanged at HEAD) and FSpec FR-331, FR-491 and FR-492 at 5423fcf4.

The PR does what its contract asks. It removes the lattice and `fixed-point` hooks completely; no references are left. The cycle answer comes before premises, the provider and the request writer under all four rules. An `assume-guarantee` selection becomes the routed item. `assumptions` is derived and written to the echo and the provider. The guard names the first unconfirmed `component-guarantee` entry with `provider-answer-unconfirmed{premise}`, and a composition with zero guarantees stands. Readers refuse the producer-only cause. A provider `interface-mismatch` is admitted only on full typed equality with the reason-major recomputation, and any other is refused `invalid_result`/`invalid-value` at `/counterexample`. The assignment path stays `replay-refused`.

There are six findings: two medium correctness findings, one medium coverage finding and three low findings. The medium correctness findings are a peer index matched by clause node alone in the new `assumptions` derivation, and a declared mismatch that silently lifts the caller's planner `max_work`.

## Verdict

**CONDITIONAL**: no high findings, three medium and three low. Examined and sound:
- The `ItemKey` premise dedup carries the assumption nodes in the temporal subject text, so the equal-wire `Capped` under `SysAGRely` is its own item. The guard reads it by that request index, never by claim node.
- `gate_compositions` walks `required` in `premises` order and names the first entry whose item is not `proved`, including items that settled by disposition and so have no record. An empty conjunction stands.
- `read_plugin_cause_kind`, `from_plugin_inconclusive` and `read_result_entry` (first-party and plugin origins, every kind) refuse `provider-answer-unconfirmed` at `/cause/type`. The linked `ProviderVerdict` has no cause member, so a linked provider cannot supply that cause (closed by type).
- `interface_mismatch` checks every `absent` entry in ascending slot order before any `weaker` one, and `premise` is the slot index of the `premises` list.
- Admission compares all four members, `interface` included as a typed `NodeKey`.
- An admitted mismatch is written as `refuted` with backend, with no certification, conclusion or cause, as a typed S7 witness with no replay target.
- `RequestWriter` writes no item for a cyclic declaration. The FR-507 family is now read for every rule.
- Cancellation and provider options and limits are unchanged.

No vendoring, copy, pin, manifest or ceremony was added. `fsl-replay/src/lib.rs` and `outcome/counterexample.rs` are untouched.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | `assumption_indices` maps each assumed peer to the first `component-guarantee` slot whose clause node equals the peer's node. It ignores the peer's component and subject, and `ComponentGuaranteeKey.assumptions` is a node-only `BTreeSet`. FSpec FR-491 "Assumption binding" requires the entry "over `D`'s subject at `H`'s clause". Take two components built from one package (e.g. `component PA from A using M; component PC from A using N;`). `rely PA::Bounded on PC::Bounded` then derives PA's own index, a self index FR-331 forbids. A guarantee relying on both peers gets one index instead of two. The provider and the echo receive an `assumptions` array that a conforming FR-331 reader refuses. | fsl-semantics/src/check/composition.rs:665 |
| FND-002 | medium | When the declaration holds a mismatch, `settle_composition` replaces the caller's `limits.max_work` (and the published default) with `u64::MAX`, and the TC-1330 test pins `max_work: 1` settling `refuted`. FSpec FR-492 "Interface mismatch" says the section "relaxes none of admission, replay, certification, trust, extent or limits", and FR-492-AC-9 settles a budget below the slot count `incomplete`. FR-530 also asks for the complete slot list, so the contract is ambiguous. The code resolves it silently by dropping a caller bound rather than through a recorded ruling. | fsl-replay/src/planner/compose.rs:204 |
| FND-003 | medium | FR-1044-AC-14 is tagged on the CLI tests, but some of its clauses have no test. (a) No test sends a `weaker` witness through provider admission, so the clause "slot-3 `weaker` refused / slot-4 `absent` accepted at the recomputation seam" is only tested at `interface_mismatch` and never at `finish`. (b) The malformed-witness clause (missing or extra member, other `reason`) has no FSL test. The linked typed shape cannot be constructed and FSL has no plugin reader for composition entries. The author disclosed this, but the tag claims the whole AC. | fsl-analyze/tests/it/analyze_composition_cli.rs:1202 |
| FND-004 | low | The FR-1044-AC-12 test asserts only `trusted`, because the recording fixture registers through `descriptor_from_manifest`, which is process origin. The AC's linked-origin `uncertified` half is not exercised for a composition `proved`. | fsl-analyze/tests/it/analyze_composition_cli.rs:1101 |
| FND-005 | low | `assert_cycle_answer` checks the label, the disposition, the absent backend and request, and the detail. It does not check the basis `unavailable` or the stage `S4` that FR-1044-AC-6 names. The AC-6 clause that a self-rely still refuses `malformed-declaration` with exit 20 through analyze is covered only at fsl-package S3. | fsl-analyze/tests/it/analyze_composition_cli.rs:869 |
| FND-006 | low | `TerminalRecord::refuted_by_interface_mismatch` takes `_mismatch: &InterfaceMismatch` and ignores it. Unlike `refuted_by_probability_witness`, where the token binds the evidence, the parameter binds nothing, so its signature suggests a binding the function does not make (rust-style API design). Drop it or make it carry the witness. | fsl-replay/src/proof_result.rs:1845 |

## Gates

The review brief does not allow the reviewer to run Cargo, fmt, clippy, tests, Kani or the aggregate gate, so none was run. Compile evidence comes from others and was not re-run. The integration report records `cargo check --locked -p fsl-analyze --test it` at 13623d29 with exit 0 and 0 errors. Its log, `/tmp/qsl-2739-provider-current-integration-check.log`, reads `Finished` with 0 `error` lines and two unused-import warnings in untouched files. The coder report records the other touched crates' `--lib --tests` check with exit 0. No provider runtime result exists for this head, and none is claimed. The #3386 2-pass run is not evidence for this PR. No CI workflow file changed, and no AssuranceProfile applies.

The two main-conflict resolutions are correct and keep main's behaviour. `item_stage` keeps main's `ProtocolAnalysis` acting-instance arm and adds `InterfaceMismatch` as an S7 typed witness. `replay_target` returns `None` for `ScheduleMiss | InterfaceMismatch | ProtocolAnalysis`.

Remaining work outside this PR: FR-1044-AC-10 package projection and parity stays OPEN with the package-consumer API lane. The external driver text is done in PR223 at 891ca755cf742712d7ff4f4adfa3208b775948f6 and is not pending. The PR correctly says `Refs QSL-2739` and never `Closes`.
