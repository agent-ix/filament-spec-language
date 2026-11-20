---
id: SR-2026111612172400000054
title: "Code review of QSL-2234 slice 1, hybrid provider seam, reference crate and dispatch (filament-spec-language PR 2994)"
type: SpecReview
analysis: code-review
scope: "agent-ix/filament-spec-language@6c48a3ee242eff4e77c3557728eae87b1fb51563; git diff origin/main...HEAD (PR 2994): fsl-replay/src/hybrid.rs, fsl-replay/src/hybrid/provider.rs, fsl-replay/src/hybrid_replay.rs, fsl-replay/src/hybrid_replay/trace.rs, fsl-replay/src/hybrid_replay/tests.rs, fsl-replay/src/proof_result.rs, fsl-replay/src/lib.rs, fsl-hybrid-reference/, fsl-analyze/src/analyze.rs, fsl-analyze/src/analyze/hybrid_item.rs, fsl-analyze/src/analyze/document.rs, fsl-analyze/src/analyze/request.rs, fsl-analyze/src/provider.rs, fsl-route/src/lib.rs, fsl-analyze/tests/it/hybrid_*.rs, Cargo.toml"
review_set: subset
date: 2026-11-19
---

## Summary

Ticket: QSL-2234 (slice 1). This is a code-review pass with the rust-review lane folded in.
The reviewer's own clean-worktree gate runs are reported below. The arch rulings of W68
(language PR 2998 at bdb9a2efa, FSpec PR 725) were relayed during the review and merged to
main after the PR's head. FND-009 to FND-013 record where the slice deviates from the merged
FR-750, and each finding says whether it belongs in this PR or in a follow-on slice.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | Dispatch drops `HybridAnswer.certificate` silently. FR-750 says a returned certificate SHALL be checked by FR-751 before the record is written. Here a certificate-bearing `proved` answer settles `proved`/`uncertified` with no `certificate` member and no `certificate-rejected` beside it, and nothing in the code marks this as an unchecked gap. The crate doc states "certified only after the layer-6 checker accepts its certificate" as if the checker were delivered. Deferring the checker to the FR-751 slice is fine, but the drop should be explicit (a TODO naming the slice, or a refusal) | fsl-analyze/src/analyze/hybrid_item.rs:73-90; fsl-hybrid-reference/src/lib.rs:4-7 |
| FND-002 | medium | Every `HybridReplayRefusal` from `settle` becomes `failed` `runtime_invariant`/`established-invariant-broken`. That includes a provider trace that breaks FSpec FR-193 (`Trace`: the provider's `invalid_result`, not an FSL invariant), `ExactGap` (exact replay does not decide the claim form) and `UnsupportedCounterexampleKind`. In-process providers that build `HybridTrace` directly, as the reference will, bypass the reader's admission and hit this. Map each refusal to its own settlement | fsl-analyze/src/analyze/hybrid_item.rs:75-95 |
| FND-003 | medium | `deadline_ms` is derived from the resolved solver time budget (`max_solver_time*1000`), not from the request's deadline. `AnalyzeLimits.deadline` is never read, so a request deadline shorter than 60 s never reaches the provider. FR-750 lists `deadline_ms` as the request's own deadline | fsl-analyze/src/analyze/hybrid_item.rs:62-71 |
| FND-004 | low | `read_entry`'s `inconclusive` arm folds every other FR-331 cause (`insufficient-horizon`, `bound-reached`, ...) into `SolverOutcome::Unknown(None)`, which settles `solver-inconclusive` and loses the provider's cause. Its `coverage: Inductive` is a placeholder. Superseded in part by W68 (FND-011) | fsl-replay/src/hybrid/provider.rs:120-141 |
| FND-005 | low | Any `Disposition::Unsupported` hybrid item settles `HybridProviderAbsent` and overrides the negotiated diagnostic. FR-750 reserves that cause for an empty candidate set. If negotiation can return `Unsupported` with candidates present (mode or domain mismatch), the cause is mislabelled | fsl-analyze/src/analyze.rs:2171-2175 |
| FND-006 | low | The new crate is not in xtask `string_edge::crate_roots`, whose doc says "each extracted §6.1 layer crate adds its own entry here", so the string-edge gate does not scan `fsl-hybrid-reference/src` | xtask/src/string_edge.rs:26-49 |
| FND-007 | low | The reference stub reports `method: FlowpipeOutwardRounded` for an answer that ran no flowpipe, with placeholder `coverage: Inductive`. FR-1019 names that method for a bounded proof it computed. A stub that ran no method should not claim one (`Unclassified`) | fsl-hybrid-reference/src/lib.rs:33-39 |
| FND-008 | low | `HybridItem.proof_bounds` is always `Vec::new()`, never read from the item, though FR-750 says `HybridItem` carries the item's proof bounds. This is unremarked in code | fsl-analyze/src/analyze/document.rs:1776-1780 |
| FND-009 | high | W68 ruling 2: `HybridAnswer` is `{result: SolverResult, certificate}`. Merged FR-750 requires `{verdict: HybridVerdict, certificate}` with `Solved(SolverResult)`, `Refuted(HybridCounterexample)` and `Unsupported(UnsupportedConstruct{form, site, term, mode, variable})`, plus an `Unsupported` settlement (`unsupported_construct`/`expression-form`, detail with `locus`, no replay; a bad site or mode settles `failed`). This is the public seam slice 1 ships, so it belongs IN THIS PR | fsl-replay/src/hybrid/provider.rs:38-47; fsl-analyze/src/analyze/hybrid_item.rs:73-96 |
| FND-010 | high | W68 ruling 3: there is no `Refuted` arm. A refutation can only arrive as `SolverOutcome::Unknown(trace)`, whose non-reproduction settles `SolverInconclusive`. Merged FR-750 requires `Refuted` settled by FR-450 alone, with non-reproduction `ReplayParity(HybridTrace(NotReproduced{counterexample}))` carrying the claim. The in-process dispatch for this is in the PR, so it belongs IN THIS PR (FR-750-AC-8). The process-path half rides with process providers | fsl-analyze/src/analyze/hybrid_item.rs:75-90; fsl-replay/src/hybrid.rs:343-396 |
| FND-011 | high | W68 ruling 6: the signature is `read_entry(entry, model)`. Merged FR-750 requires `read_entry(entry, counterexamples, model)`: a `refuted` entry reads `counterexamples[i]` into `Refuted`, an out-of-range index refuses at `/counterexample`, an `unsupported` `expression-form` entry reads into `Unsupported`, and `failed`/`tool-failure` returns `NotRead`. Signature and arms belong IN THIS PR (FR-750-AC-11). The `NotRead` to FR-290 process settlement can follow with process providers | fsl-replay/src/hybrid/provider.rs:80-149 |
| FND-012 | medium | W68 ruling 4: `HybridCounterexample::to_wire` returns `Option` and is `None` for `undefined-evaluation`. `hybrid_item` then records `Err(Unspelled("hybrid undefined-evaluation"))`. Merged FR-750 requires a value for both kinds (`where{position, time, locus}`, `cause`) and says the outcome never fails for an unspelled counterexample. The writer is new in this PR, so its signature and both kinds belong IN THIS PR. The plugin reader's read-back of that form (FR-750-AC-9) may follow if AC-9 stays ignored with that reason | fsl-replay/src/hybrid_replay.rs:171-182; fsl-analyze/src/analyze/hybrid_item.rs:76-83 |
| FND-013 | medium | W68 ruling 1: the delivered reader and writer use `{numerator, denominator}`, and the three un-ignored fixtures were converted. Every other hybrid fixture still spells exact numbers as strings: the `interval()` helper, `"horizon": "10"`, certificate `"step": "10"`/`"1/16"`, and literal `{"lower": "0", ...}`. Those ignored tests will fail for the wrong reason when un-ignored. Convert them IN THIS PR. `options` values stay strings | fsl-analyze/tests/it/hybrid_requests.rs:190-192; fsl-analyze/tests/it/hybrid_requests.rs:353-357; fsl-analyze/tests/it/hybrid_requests.rs:465-477; fsl-analyze/tests/it/hybrid_reference.rs:299-341 |

## Scope and evidence

Examined, clean unless listed above:
- `Engine::hybrid` descriptor rule (`dynamics` holds `hybrid` and advertises
  `temporal-satisfaction`; typed `NotHybrid` naming the backend and `invalid-dynamics`).
- `Engine::hybrid_reference` descriptor: backend, both modes, `dynamics ["hybrid"]`, bounds
  1e6 and 60. The FR-1019 `options` defaults are not published; FR-1019-AC-6 stays ignored,
  which is honest.
- `Engines::fsl()` does not hold the reference provider (verified in code and by the
  FR-1019-AC-4 test). `fsl_with_reference_hybrid` is `fsl()` plus the reference.
- Layering: `fsl-hybrid-reference` depends only on filament-exact, fsl-replay and
  fsl-semantics. No core crate depends on it. arch-lint's direction check covers it as a
  non-core workspace member.
- `TerminalRecord::hybrid_provider_absent` / `hybrid_tool_unavailable` / `of_hybrid`
  (refuted requires a counterexample).
- `ItemForm::Hybrid` resolution: the `hybrid` member is read by the item and excluded from
  unrouted options, and an invalid `hybrid` member is `invalid-value`.
- Replay target `{"state_clause": Q}` for `Counterexample::Hybrid`, confirmed correct by
  W68 ruling 5. The FR-098 executor's hybrid arm (FR-750-AC-10) is a follow-on.
- `HybridTrace::to_wire` emits Rational objects and round-trips through the plugin reader.
- No `#[allow]` in the diff. New tests carry `#[trace]` tags.
- Honesty of framing: the PR body, crate doc and test-module doc say the flowpipe is not
  implemented and the reference answers `solver-inconclusive`. Remaining stubs stay
  `#[ignore]` with QSL-2234 reasons. The exceptions are FND-001 (certificate wording) and
  the gap-analysis SR's trace finding.

## Gates run (reviewer worktree at 6c48a3ee2)

- `cargo-slot cargo clippy -p fsl-hybrid-reference -p fsl-replay -p fsl-route -p fsl-analyze --all-targets -- --cap-lints warn`:
  no warning in any diff file. The remaining warnings are the inherited fsl-semantics (565),
  fsl-command, fsl-replay wire.rs:828 / derivation.rs:94 and pre-existing fsl-analyze test
  files.
- `cargo-slot cargo test -p fsl-replay --lib -- a_counterexample_wire_reads_back_to_itself --test-threads=1`: 1 passed.
- `cargo-slot cargo test -p fsl-analyze --test it -- hybrid --test-threads=1 --skip ... --skip ...`:
  7 passed, 0 failed, 18 ignored. Every ignored test has a QSL-2234 reason.

## Verdict

The seam, registration, provider-absent and tool-unavailable records, and routing are sound
for what slice 1 delivered, and the partial framing is mostly honest. The W68 rulings change
the shape slice 1 ships: `HybridAnswer`/`HybridVerdict` (FND-009), the `Refuted` arm
(FND-010), the `read_entry` signature (FND-011), the `to_wire` value (FND-012) and the
fixture wire spelling (FND-013). These should land in this PR, after a rebase onto main at
bdb9a2efa, before merge. FND-001 to FND-003 are pre-ruling defects to fix in the same round.
Not mergeable as is.

## New findings (disposition pass 1)

Pass 1 was checked against head 544461c15e18d11b7a724eca22e28eccf1989a8b, which includes fix rounds 1 and 2, the W72 rows, and the merge of origin/main d6d365427.

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-014 | medium | 17574bef4 added `#[allow(clippy::large_enum_variant)]` on `HybridVerdict`. The house rule allows no `#[allow]` in the diff | fsl-replay/src/hybrid/provider.rs:57 |
| FND-015 | low | `AnalyzeRecord.hybrid_certificate` is written but never emitted or read, and no test asserts that a certificate-bearing answer keeps it. The docs say it is "kept unchecked beside" the record, yet the outcome item carries no `certificate` member | fsl-analyze/src/analyze.rs:1695-1698; fsl-analyze/src/analyze.rs:2595 |
| FND-016 | low | `the_providers_deadline_is_the_requests_deadline` is tagged `FR-750-AC-1`. AC-1 is the covering delta-complete proof and says nothing about deadlines, so the tag misbinds a test of FR-750 behaviour that has no AC | fsl-analyze/tests/it/hybrid_provider.rs:376-378 |
| FND-017 | medium | `read_entry` takes a fifth parameter, `wire: &WireGraph`. Merged FR-750 (W72) specifies `read_entry(entry, counterexamples, graph, claim)` in its Behavior and in AC-11/AC-13. The coder's reason is that emitted node order differs from `semantic_graph()` order. The deviation is technically justified, but the spec and the code disagree until arch rules: either amend FR-750 or derive the wire ids from `graph` | fsl-replay/src/hybrid/provider.rs:153-160 |
| FND-018 | low | `wire_graph(package, spelling).unwrap_or_default()` hands the provider an empty `WireGraph` when emission or admission fails. A provider's correct `locus` then refuses `invalid-value` at `/diagnostic/detail/locus`, which blames the provider for an FSL emission failure. Each hybrid item also emits and admits the package up to three times (request, unsupported, counterexample) | fsl-analyze/src/analyze/hybrid_item.rs:87 |
| FND-019 | low | `WireGraph::of_keys` is new public API in the core crate fsl-semantics with no caller anywhere in the workspace. It also uses `at as u64` | fsl-semantics/src/check/wire_id.rs:895-909 |
| FND-020 | low | `read_unsupported` finds the claim's position by `std::ptr::eq(candidate, claim)`. A `claim` not borrowed from `graph.hybrid_claims()` (a clone, or one from another graph instance) refuses `UnknownModel`, a misleading cause for a claim whose model exists | fsl-replay/src/hybrid/provider.rs:681-685 |

## Dispositions

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | The certificate is carried in `HybridReport.certificate` and `AnalyzeRecord.hybrid_certificate`. The crate and seam docs now say FR-751 is a later slice and the record stays `uncertified` (FND-015 for the field never being emitted) |
| FND-002 | fixed | `fault_of` follows the W72 "Replay refusals" rows. `Trace` is `invalid_result`/`invalid-value` with `pointer`. `UnknownModel`, `Expansion` and `IllKindedTerm` are `runtime_invariant` naming `hybrid-claim-model`/`hybrid-call-expansion`/`hybrid-term-kind`. `ResourceExhausted` is `incomplete` `LimitReached{work_units}`. Tested by `each_replay_refusal_settles_by_its_row` (TC-3037) |
| FND-003 | fixed | `deadline_ms: Option<u64>` comes from `AnalyzeLimits.deadline`. Tested for 250 ms and for unbounded |
| FND-004 | fixed | Causes a `SolverResult` cannot hold are now `NotRead("<result>/<type>")` and are no longer folded into `Unknown(None)` |
| FND-005 | fixed | `Disposition::Unsupported if !engines.has_hybrid_candidate()` guards `HybridProviderAbsent` |
| FND-006 | fixed | `fsl-hybrid-reference/src` was added to the xtask string-edge `crate_roots` |
| FND-007 | fixed | The reference provider answers `SolverMethod::Unclassified`, and its doc says no method ran |
| FND-008 | fixed | The `HybridItem.proof_bounds` doc states why it stays empty (a hybrid request item names no subject) |
| FND-009 | fixed | `HybridAnswer{verdict: HybridVerdict{Solved, Refuted(Box<..>), Unsupported(UnsupportedConstruct)}, certificate}`. `Unsupported` settles `unsupported_construct`/`expression-form` with form, declaration, mode, variable, backend and locus, and runs no replay. A bad mode settles `failed` (AC-7 in-process test) |
| FND-010 | fixed | `Refuted` settles only through `from_provider_counterexample`. Non-reproduction gives `ReplayParity(HybridTrace(NotReproduced))`, `Unknown(trace)` keeps `SolverInconclusive`, and undefined-evaluation agreement follows the W72 rule (AC-8 and AC-14 comparison tests) |
| FND-011 | fixed | `read_entry` reads the frame's `counterexamples` (`refuted` by position, `/counterexample` refusal), `incomplete` timed-out/steps, and `unsupported` `expression-form` with its locus resolved through the graph (AC-13). `failed` and other `unsupported` return `NotRead`. The extra `wire` parameter is FND-017 |
| FND-012 | fixed | `to_wire(model, locate)` returns a `Value` for both kinds, with the four catalogued `UndefinedCause` spellings. An unlocatable locus gives a wire `Err`, never `failed` |
| FND-013 | fixed | The `interval()` helper builds `{lower: rational(..), upper: rational(..)}`. No string horizon, bound or certificate step is left in the hybrid fixtures |
| FND-014 | fixed | 8c8282076 boxes the payload (`Refuted(Box<HybridCounterexample>)`) and removes the `#[allow]`. No `#[allow]` remains in `git diff origin/main...HEAD` |
| FND-015 | fixed | Round 2 (5d18d1ba0): the unused `hybrid_certificate` field and the `HybridReport.certificate` field were removed. The module, seam and crate docs now say the certificate is neither checked nor written to the outcome until QSL-2531 (FR-751) |
| FND-016 | fixed | Round 2: the `FR-750-AC-1` tag was removed from `the_providers_deadline_is_the_requests_deadline`, with a doc line saying no FR-750 criterion states it |
| FND-017 | accepted-no-change | Arch W73 (language PR #3010, open at review time) amends FR-750 to the five-parameter `read_entry(entry, counterexamples, graph, claim, wire)`. The code stays as is, and spec and code agree once #3010 merges |
| FND-018 | fixed | Round 2: the package is emitted once per item, before the provider runs. An emission failure settles `failed` `runtime_invariant` (an FSL fault) instead of handing the provider an empty `WireGraph`. `unsupported_construct` and `counterexample_wire` reuse that wire graph |
| FND-019 | fixed | Round 2: the unused `WireGraph::of_keys` was removed from fsl-semantics |
| FND-020 | fixed | Round 2: the claim is found by `candidate == claim` (structural equality) instead of `std::ptr::eq`. `hybrid_locus_site` uses the position only for the claim's model, so equal claims resolve the same model |

## Gates run (disposition pass 1, reviewer worktree at 544461c15)

- clippy `-p fsl-hybrid-reference -p fsl-replay -p fsl-route -p fsl-analyze -p fsl-semantics -p xtask --all-targets -- --cap-lints warn`: no warning in any diff file. Only inherited warnings remain.
- `cargo test -p fsl-replay --lib -- hybrid`: 49 passed, 0 failed.
- `cargo test -p fsl-analyze --lib -- hybrid`: 1 passed (`each_replay_refusal_settles_by_its_row`).
- `cargo test -p fsl-analyze --test it -- hybrid` (with the two house skips): 12 passed, 0 failed, 26 ignored. Every ignore has a QSL-2234 reason.

## Gates run (disposition pass 2, reviewer worktree at 5d18d1ba0)

- clippy `-p fsl-hybrid-reference -p fsl-replay -p fsl-route -p fsl-analyze -p fsl-semantics -p xtask --all-targets -- --cap-lints warn`: no warning in any diff file.
- `cargo test -p fsl-replay --lib -- hybrid`: 49 passed. This covers the edited `tests.rs` after the duplicated `#[trace]` line was deleted.
- `cargo test -p fsl-analyze --lib -- hybrid_item`: 1 passed.
- `cargo test -p fsl-analyze --test it -- hybrid` (with both house skips, one thread): 12 passed, 0 failed, 26 ignored. Each ignore names its slice ticket, QSL-2529 to QSL-2534.
- No `#[allow]` in `git diff origin/main...HEAD`. The committed `reviews/` SRs matched the reviewer's round-1 versions.
