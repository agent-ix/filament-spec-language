---
id: SR-2026112520490001
title: "Code review — filament-spec-language partial measure lower proof"
type: SpecReview
analysis: code-review
scope: "agent-ix/filament-spec-language; fsl-replay/src/probability_certificate/document/measure.rs, fsl-replay/src/probability_certificate/document/measure/bounds.rs, fsl-replay/src/probability_certificate/document.rs, fsl-replay/src/lib.rs, fsl-analyze/src/analyze/exact/measure.rs, fsl-analyze/tests/it/probability_from_source.rs, tools/fsl-probability-check/tests/it/value_bounds.rs, tools/fsl-probability-check/tests/it/bounded.rs, tools/fsl-probability-check/tests/it/certificate_kinds.rs; FR-1042-AC-10, FR-1042-AC-27, FSpec FR-411-AC-8, FSpec FR-412-AC-5, FSpec FR-412-AC-16"
review_set: subset
relationships:
  - { target: "ix://agent-ix/filament-spec-language/FR-1042", type: references }
---

## Summary

Ticket: QSL-3352 (pointer on QSL-3371). PR #3970 at head `0b64d104a3b74c1377c7bb1dd777cb5b2397c3c9`, diff `origin/main...HEAD`; Rust review folded in. Main is four commits past the base (#3958, #3962, #3965, #3948); none touches a `probability_certificate` file, `analyze/exact/measure.rs`, FR-1042 or TC-3151, and `FR-1042-AC-27` / TC-3151 step 15 are not used on main or by any other open PR. Open PR #3968 edits `analyze/exact/measure.rs` only in `refute_mean` (no overlap).

## Verdict

**PASS** — no soundness hole found; one low coverage finding.

Soundness argument of the PR body, checked by reading:

1. Omitted states: `Scope::Reached` expands only named keys reached from the initial states through named keys; unnamed successors become empty frontier states (`terminal: None`, no choices, `activated: false`). Keys are never restored from bytes in this scope, so an unreachable state cannot enter the part or supply the quantile's reachable activation (the `activated` test at `check` reads expanded named states only).
2. `-q` floor: `MeasurePayoff::Shifted` terminal values are `0` (not activated), `1 - q` or `-q`, so `-q` is the least value; `event` is 0/1 with floor 0. Ranks are the monitor's `remaining`, which every measure step decrements, so the product is acyclic and `x <= Q_a(x)` (Minimum objective, every action) with the floor bounds the minimum by induction on rank.
3. Unreached keys: any named key absent from the part returns the original `States` stop; the row-count guard is shadowed by the reader's own `values.len() > limits.states` check (reader line ~1553).
4. Claim-side gate: the part is read only for `bound_side == Lower && role == Claim`, after the misplaced-kind check, so only `>= θ` (event) and `<= c` quantile (shifted, certified `AtLeast 0`) reach it; `>= c` quantile and `<= θ` claim-side lowers are `Malformed` at `kind` first.
5. ThresholdMissed mapping: producer-only, maps a prefix that misses the threshold to `incomplete` at `exact_prob.max_states`; it never yields `proved`.
6. Fairness and payoff guards only narrow: FSpec FR-411-AC-11 makes a bounded form's fair value equal its unrestricted value, and a `fraction` floor of 0 would also be valid, so deleting either would not admit a false claim.

Wrong-kind refusal: matches FR-1042 "Pairing" (claim side `lower` for `AtLeast` / `<= c` shifted, `upper` for `AtMost` / `>= c` shifted) and FR-1043's measure table; FSpec FR-411 "Evidence on a partial product" admits partial evidence only for a proof of `>= θ` (or a refutation of `<= θ`, which goes through the refutation route, not `check`). The only producer caller that offers a wrong-kind claim side is `finish_measure_interval` for `AtMost` / `>= c` claims: on main `TerminalRecord::proved_by_value_bounds` already returned `None` there (`claim.bound() != side_kind`, proof_result.rs:1919), settling `failed`; now it settles `certificate-rejected` at `kind`. Neither proves; pre-existing producer gap (draft ticket in the report).

QSL-3414 (complete claim-side `upper` for a `>= c` quantile at initial 0 with a tight activation accepted): `check_value_bounds` on main already compared the initial value `<= 0` with no tight-action search; the PR's refactor through `certified_bound` keeps the same comparison and side, and the new partial route never reaches a `>= c` claim. Predates the PR; not widened (the PR narrows main, which also accepted a claim-side `lower` there).

`#[implements]` tags: separate commit `0b64d104a3`, adds only `use ix_trace_rs::implements;` and two bare attributes; `quire trace` lists `check` as implements FR-1042-AC-27 and `whole_or_lower_part` as implements FSpec-FR-412-AC-5. Both fit.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | The fairness and payoff guards in `whole_or_lower_part` have no test of their own; deleting either fails no test. Not a soundness hole (both only narrow; see Verdict item 6). | fsl-replay/src/probability_certificate/document/measure.rs:701-705 |

## Review Notes

- Tests run by the reviewer at this head are listed in the posted Linear comment.
- Producer: `fsl-analyze` `analyze/exact/measure.rs` `run` builds `partial_lower_measure_probability_certificate_document` and self-checks through `check_probability_certificate_document_as(.., Claim)`; the independent route is `fsl-probability-check` -> `check_document_bytes` -> `measure::check`. The mean-refutation route (`measure/bounds.rs`) keeps `Scope::Restored`, behaviour unchanged.
