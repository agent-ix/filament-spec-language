---
id: SR-2026112022385700000000002
title: "Code review of filament-spec-language PR #3234: FR-230-AC-68 recursive clock readings and the FR-230-AC-4 negative"
type: SpecReview
analysis: code-review
scope: "agent-ix/filament-spec-language@b550cd35526f11fe3c84a5ff8c23e110ed394d9a; PR #3234 diff (git diff origin/main...b550cd3552): fsl-semantics/src/check/mod.rs, fsl-semantics/src/check/timed_model.rs, fsl-semantics/src/check/timed_model/reading.rs, fsl-semantics/tests/it/timed_model.rs"
review_set: subset
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-230
    type: reviews
---
# Code review of PR #3234

## Summary

Ticket: QSL-2144 (remainder: FR-230-AC-68 and the FR-230-AC-4 negative). The
Rust review lane (rust-review, rust-style) is folded into this file. Only the
PR diff was reviewed; AC-67/69/70 and element presence are out of scope.

The tail-call loop for negation, a selected `if` branch and a Boolean call,
the bounded constant folder, the per-unfolding count and the AC-4 negative
vectors are sound. The constant folder cannot overflow: every integer it
keeps is bounded by `rational_bits`, so `+ - *` of two kept values is at most
twice that width before the bound rejects it, and comparisons use `Integer`
ordering. No Cargo.lock, pin, CI workflow or gate change is in the diff.
Trace tags are bare ids (`TC-685`, `FR-230-AC-4`, `TC-2392`, `FR-230-AC-68`).

Meter placement (lead question 1). The amount charged is honest: one unit
per unfolding, on the same contract meter, at `ChargePoint::DeclarationCheck`.
The placement is forced in part by structure: `timed_model::prepare` runs at
mod.rs:2662, before `contract_meter` exists (mod.rs:3471), so no per-unfolding
charge to the real meter is possible without moving the meter. AC-68's
"budget one unit below the admitted usage refuses at the call" is only true
when the unfoldings are the check's last charge; with the meter created first
and each unfolding charged as it happens, the one-below budget would refuse
at the check's last other charge (reward-sign validation, mod.rs:8352) with
no call region. So the AC text and a natural per-unfolding charge are
irreconcilable, and end-settling is the placement the AC selects, not a
fabrication. Its real cost is FND-004. Arch should decide whether AC-68 keeps
"at the call" for the one-below case.

Recursion test (lead question 2). Moving the unconditional Boolean recursion
from `TimedNotChecked` to a work-budget stop matches FR-230 ("A measured one
SHALL be expanded by unfolding ... a budget stop refuses ... work-budget-exceeded").
The new Clock-result assertion does not (FND-003).

AC-4 negative (lead question 3). Both vectors are a legitimate adverse case
for AC-4's disjunctive shape: the refusal kinds are the ones FR-230-AC-2
defines, and each assertion fails if the disjunct swallowed the operand's
refusal, refused with another tag, or located it elsewhere.

Gates: not run. The focused `timed_model::` test run waited in the
locked-build queue and was stopped. The findings rest on reading the code,
not on test output.

## Verdict

FAIL until FND-001 to FND-003 are fixed. FND-004 needs an arch ruling on the
AC text; FND-005 to FND-007 can be fixed in this PR.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | AC-68 requires the non-constant call to refuse "naming `loop`, `loop`" (FR-230: "naming the cycle's declarations in call order"); `ClockReadOutsideConstraint` carries only a span, nothing names the cycle, and the test asserts only code, tag and region text, so that clause can never fail | fsl-semantics/src/check/refusal.rs:1495; fsl-semantics/src/check/timed_model/reading.rs:1623; fsl-semantics/tests/it/timed_model.rs:1751 |
| FND-002 | high | Non-tail recursion nests the Rust stack once per unfolding: an `and`/`or`/`implies` operand is read by a nested `boolean`, the node budget is charged only after both operands return, and the only bound is unfoldings <= work budget (16,777,216 by default). `if n = 0 then c <= 3 ms else (c <= 3 ms and loop(c, n - 1))` called as `loop(self.x, 1000000)` overflows the thread stack and aborts the process instead of refusing. Before this PR the call was refused before any descent | fsl-semantics/src/check/timed_model/reading.rs:1574; fsl-semantics/src/check/timed_model.rs:2815 |
| FND-003 | high | A measured recursive declaration with a Clock result is still refused `TimedNotChecked`/`RecursiveClockReading`, and the test was rewritten from the Boolean to a Clock form to keep asserting it, with the comment "has no unfolding FR-230 defines". FR-230 "Recursive clock readings" applies to every clock-reading declaration ("A measured one SHALL be expanded by unfolding ... substitutes the argument places"); `pick(self.x, 3)` with `if n = 0 then c else pick(c, n - 1)` has a defined unfolding to `self.x` | fsl-semantics/src/check/timed_model/reading.rs:1420; fsl-semantics/tests/it/timed_model.rs:1352 |
| FND-004 | medium | The unfoldings are counted against the whole work budget during prepare, ignoring every other charge, and charged to the meter only after the rest of the check. A unit can do up to a full budget of unfoldings plus a full budget of other work (about twice the configured bound) before it is refused, so the meter does not stop the work it accounts | fsl-semantics/src/check/timed_model.rs:2815; fsl-semantics/src/check/mod.rs:8362 |
| FND-005 | medium | FR-230 evaluates "each `if` or `match` condition" over "an expression over literals and constants", but only `If` is evaluated (`Case` falls to the data-predicate path and refuses), `div`/`rem`/`mod` do not fold, and call-site arguments fold with `&\|_\| None`, so a named constant argument is not evaluated. Each refuses `clock-read-outside-constraint` where FR-230 says the call unfolds | fsl-semantics/src/check/timed_model/reading.rs:1552; fsl-semantics/src/check/timed_model/reading.rs:751; fsl-semantics/src/check/timed_model/reading.rs:972 |
| FND-006 | low | Weak oracles: the AC-68 positive asserts one clock constraint but not that the part has no data literal ("its guard the one atomic constraint"); the unconditional-loop case asserts only `WorkBudget` kind, not the call region, so a refusal from any other work stop passes it | fsl-semantics/tests/it/timed_model.rs:1744; fsl-semantics/tests/it/timed_model.rs:1399 |
| FND-007 | low | A constant whose fold exceeds `rational_bits` returns `None` and is reported as a non-constant (`clock-read-outside-constraint`) rather than as the rational-bits limit, which misnames the cause | fsl-semantics/src/check/timed_model/reading.rs:719 |
