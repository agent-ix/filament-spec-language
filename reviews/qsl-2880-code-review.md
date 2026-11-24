---
id: SR-202611230430001
title: "code-review (with rust-review lane) of PR3592 ValueBounds settlement for a decided interval"
type: SpecReview
analysis: code-review
scope: "agent-ix/filament-spec-language; PR3592 8c2b83a273b54f02af007665c817eb4e6c124c17; fsl-analyze/src/analyze/{exact,document}.rs, fsl-replay/src/{lib,proof_result,probability_certificate}.rs, fsl-replay/src/probability_certificate/document{,/value_bounds,/mdp,/timed,/long_run,/measure}.rs, fsl-analyze/tests/it/{exact_probabilistic_item,timed_product_certificate}.rs"
review_set: subset
---

## Summary

Ticket: QSL-2880. Review of `git diff origin/main...HEAD` (13 files, +1140/-129). The
change settles a decided dyadic interval `proved` with `ValueBounds{BackwardInduction}`
for bounded reachability and the timed deadline, writing FR-1042 `lower`/`upper`
documents and checking them with a new `ValueRelation` (Exact/Lower/Upper) in the
reachability checker. A refuting interval stays `Incomplete(LimitReached{MaxRationalBits})`.
Rust-review lane folded in. No cargo was run: titan load average was 32 with 9 cargo
processes live, and every question below was settled by reading code and spec.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | `counterpart` placement and shape diverge from a specified contract: FR-741 puts it at item level as `{kind, document, accepted}`, FSpec backend-provider `Result.counterpart` is a bare ProbabilityCertificate required on a value-bounds basis; the code writes `{checker, accepted, kind, document}` into the item's FR-331 `record`, and no test reads it | fsl-analyze/src/analyze/document.rs:4318 |
| FND-002 | medium | The soundness-critical `ValueRelation::Lower`/`Upper` comparisons have no negative test: no tampered/raised lower value or lowered upper value is offered; the relabel test is refused only by the `ranking` member's presence, so a `Lower` that always held would pass every test | fsl-replay/src/probability_certificate.rs:1782 |
| FND-003 | medium | FR-1042 Conditions require EN-5 to raise precision when either self-checked document is rejected; `finish_interval` settles `inconclusive` on the first rejection with no retry (specified, unbuilt, no ticket named) | fsl-analyze/src/analyze/exact.rs:1894 |
| FND-004 | medium | Ticket acceptance "replay accepts the evidence" is shown only in-process with a hand-picked `scalar_bits: 1<<14`; the analyzer self-checks under `max(max_rational_bits, 16*precision_bits)` while the default document limit is `max_rational_bits` (document.rs:242), so a replay under the request's own limits (e.g. the timed test's 4 bits) would refuse the emitted 64-bit dyadic documents; no separate-process or emitted-package check | fsl-analyze/src/analyze/exact.rs:1797 |
| FND-005 | low | Foreign rustfmt reflow remains in the diff despite the stated revert: `bounded_monitor_failure`, `precision_budget`, `timed_step_for` (exact.rs), `claim_binding` and two test asserts (probability_certificate.rs), the `protocol_temporal_wire` re-export list (lib.rs), and `settled_or_stopped` in the item test | fsl-analyze/src/analyze/exact.rs:380 |
| FND-006 | low | `value_bounds::unwrap` parses untrusted bytes into `serde_json::Value` twice and re-canonicalizes before any metered reader, outside the work/storage meter and without cancel polling during parse; FR-1042 requires the checker to obey caller storage/work limits | fsl-replay/src/probability_certificate/document/value_bounds.rs:41 |
| FND-007 | low | The kept Strict test's doc says the witness's exact masses outgrow the budget, but the code never builds a witness for an interval; the actual rule is FR-202 ("`Bounds` intervals ... SHALL NOT become a bounded refutation") | fsl-analyze/tests/it/timed_product_certificate.rs:1601 |

## Verdict

Not mergeable as is (CONDITIONAL on FND-001, FND-002, FND-004 fixed in this PR; FND-003
needs a ticket or an in-PR retry).

Soundness: an interval-decided `proved` is sound. It does not rest on the dyadic
rounding: each end is re-checked in exact rationals by the independent checker. `Lower`
requires `x(s) <= Q(x,s)` (min/max over the rebuilt actions, which matches FR-1042's
table for minimum-every-action and maximum-selected-action), decided states exactly
0/1, and over the bounded base the ranking must equal the checker's rederived
remaining horizon, so the product is a DAG and induction gives `x <= value`. `Upper`
requires `y(s) >= Q(y,s)`, a pre-fixed point of a least fixed point, which is sound
without ranking. The timed route uses the existing nonprogressive-component collapse
under the same monotone relation. `Bound` has only `AtLeast`/`AtMost` (inclusive), so
`lo == threshold` proving `>=` is correct. Bad rounding would make a document fail the
checker and settle inconclusive, never a false proof. Pairing is enforced in
`proved_by_value_bounds` (opposite kinds, claim side matching `side()`, `lower <= upper`,
otherwise `failed`). A refuting interval never reaches `proved`.

Refuted half: FR-203's settlement table refutes only through "Bound fails, witness
replays". FR-202 says "EN-5 SHALL emit bounded path evidence only from FR-197 `Exact`
results. `Bounds`, `PrecisionBudget` ... SHALL NOT become a bounded refutation". So
`Incomplete(LimitReached)` for a refuting bounded or timed interval is what the spec
requires. The ticket's "proved or refuted" overstates it and should be narrowed, citing
FR-202. Refuting bound documents are specified only for `mean of fraction` (FR-202,
FR-1043-AC-9), which is outside this diff.

Spec gaps the coder raised: `counterpart` placement is specified (FND-001). The
interval-document `scalar_bits` really is unspecified: FR-1042 says only "the caller's
... representation and arithmetic limits", and its own AC-1 (`max_rational_bits` 1)
implies the limit is not `max_rational_bits`. That needs an FR-1042 spec ticket.

Clean: the checker cannot be fooled by a relabelled document (its ranking shape is
refused) or a cheap but wrong interval (inequalities are exact). A `precision_bits`
mismatch is harmless because the documents carry no precision member. Arithmetic stays
bounded by the caller's `scalar_bits` on document scalars. Tests assert exact
containment of `(9999999/10000000)^1000` and the FR-197 width bound, so they would fail
if rounding broke a document.

Conflicts: #3571 conflicts in fsl-replay/src/probability_certificate/document/mdp.rs
(`check_with_context`/`check_threshold` signature). #3545 conflicts in
fsl-analyze/tests/it/exact_probabilistic_item.rs (the replaced QSL-2850 pin it edits for
`required: None`).

## Scope and evidence

- FR-203-AC-1 (spec/functional/FR-203-bound-an-exact-run-and-settle-its-verdict.md): examined. §15.1 with `max_rational_bits` 1,024 settles `Proved{ValueBounds{…, BackwardInduction{64}}, Certified}` with FR-1042's `lower` and `upper` documents (FR-1042-AC-6).
- FR-1042 Conditions and Pairing (spec/functional/FR-1042-certify-value-bounds-with-lower-and-upper-certificates.md): examined.
- FR-1042-AC-6: examined. Both documents accepted by `fsl-probability-check` from the emitted package.
- FR-741 `certificate`/`counterpart` member text: examined.
- FR-202 bounded refutation subset: context_only.
- FSpec backend-provider-v1 schema `Result.counterpart`, allOf[6]: context_only.

## New findings (disposition pass 1)

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-008 | medium | `ProbabilityDocumentLimits::for_exact` now sets `scalar_bits = max(max_rational_bits, 16*max_precision_bits)` for every document check, but a scalar stop still settles `LimitReached{ExactProbMaxRationalBits, value}` (`model_limit()`), naming a setting that does not control the bound once `16*max_precision_bits` dominates (FR-255); it also widens exact-document checks (default 65,536 bits at any `max_rational_bits`), and limit pins such as `measure_and_vacuity_limits_settle_incomplete_without_certification` (`max_rational_bits` 3) were not shown to still hold | fsl-replay/src/probability_certificate/document.rs:246 |
| FND-009 | low | `bounded_wrong_source_and_nonexact_rows_never_produce_an_accepted_document` keeps its name while its `("0.5", 64, 64)` row now expects accepted FR-1042 `lower`/`upper` documents and `ValueBounds`; the change is spec-driven (FR-201-AC-14, FR-203-AC-1) and stronger than the old `Disposed Unsupported` pin, but the name misdescribes the row | tools/fsl-probability-check/tests/it/bounded.rs:1137 |

## New findings (disposition pass 2)

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-010 | low | `fsl-probability-check` still names `exact_prob.max_rational_bits` for every `scalar_bits` stop, including one on a value-bounds document, whose producer-side bound the analyzer now names `exact_prob.max_precision_bits` when the 16x term binds, so the same document's stop is named differently by the analyzer and by the separate checker | tools/fsl-probability-check/tests/it/reward.rs:409 |

## Dispositions

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | Item-level `counterpart` is exactly `{kind, document, accepted}` (document.rs `counterpart_member`, moved out of `record`), validated against the pinned ProbabilityCertificate in `a_value_bounds_item_writes_its_counterpart_at_item_level` |
| FND-002 | fixed | `a_raised_lower_end_or_a_lowered_upper_end_is_rejected_by_its_inequality` asserts `InequalityFails` at the tampered key for both sides |
| FND-003 | fixed | `raise_until_accepted` doubles precision to `max_precision_bits`; it terminates (strictly increasing, capped), a final rejection settles inconclusive, and a proof still needs checker acceptance |
| FND-004 | fixed | The producer and replay share `for_exact`; the separate-process test and the in-process timed and §15.1 tests replay under the request's own limits |
| FND-005 | fixed | Foreign reflow reverted; the net diff against origin/main has no format-only hunks in exact.rs, probability_certificate.rs, lib.rs or the item test |
| FND-006 | fixed | `unwrap` admits the bytes through `quire_canonical::read_bounded` (bytes, storage) with the same mapping as `profile_dispatch`, and builds a serde `Value` only for canonical bound documents |
| FND-007 | fixed | The doc now cites FR-202's ban on `Bounds`-derived refutations |
| FND-008 | fixed | `for_exact` is back to `scalar_bits = max_rational_bits`; only the two interval sites use `for_interval`, and `interval_failure` names `ExactProbMaxPrecisionBits` with its configured value when `16*max_precision_bits` binds, else `ExactProbMaxRationalBits` |
| FND-009 | fixed | Renamed `bounded_wrong_source_and_precision_budget_rows_never_accept_while_a_decided_interval_proves` |
