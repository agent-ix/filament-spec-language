---
id: SR-2026112117074738624702
title: "Spec integrity (narrow addendum) — FR-1044 Current system behavior, AC-10 package handoff"
type: SpecReview
analysis: integrity
scope: "agent-ix/filament-spec-language; PR #3348; frozen 79aa75be18b07aab639f8078caa3d4c674826ac5; spec/functional/FR-1044-request-a-component-composition-item-through-analyze.md lines 287-313 (Current system behavior, AC-10 bullet) only"
review_set: subset
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-1044
    type: references
---

## Summary

Ticket: QSL-2739. This is a narrow integrity check of the rewritten FR-1044 "Current system
behavior" AC-10 bullet only. It is not a review of the whole FR and not a fresh normative
catalog. The obsolete `invalid_package` early-reader refusal and "no public request reaches it" claims
are correctly removed. The normative AC-10 row (line 352) is unchanged and stays open, and the
ignored parity test remains. One low precision finding is recorded.

## Verdict

**CONDITIONAL**: one low wording-precision finding. These statements in the bullet agree with the code:

- admission of the handoff, which #3302 made admissible;
- no reading of composition slots or component packages;
- default request refused whole at the request stage, `unsupported_construct`/`not-yet-implemented`,
  naming the `composition` rule, exit 20, no item (`package_run.rs:298-310`, `:78-112`);
- named composition or unknown name refused naming `unnamed-claim`, never `unknown-claim`
  (`package_run.rs:839-841`);
- names carried by a row resolve as usual;
- parity deferred to the package-consumer lane.

The bullet is consistent with the AC-10 row and with the section's preamble ("Each AC below stays
as written and open on QSL-2739").

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | "an item naming … a name no row carries is refused … naming the `unnamed-claim` family" overstates the code: a name matching a non-claim declaration (for example the fixture's own `function Noop`) is refused as the `declaration` family, and a derived item's unmatched model name settles `model-alias`, both before the `unnamed` fallback | spec/functional/FR-1044-request-a-component-composition-item-through-analyze.md:306 |

FND-001 detail: `resolve_names` checks `declared_names` (`package_run.rs:824-826`, `PendingFamily::Declaration`)
and derived model aliases (`:827-837`) before `unnamed` (`:839-841`). The narrowest fix is to say
"a non-derived item whose name no name table row or declaration carries". An alternative is to leave
the sentence and accept the imprecision as current-behaviour prose. The earlier wording had the
same imprecision. This PR rewrote the sentence, so it is in scope.
