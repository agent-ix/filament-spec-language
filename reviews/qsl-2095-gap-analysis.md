---
id: SR-1508
title: "Scoped gap analysis of QSL-2095 checked state-core tests"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/filament-spec-language; QSL-2095; tests/it/checked_package_state_core.rs, tests/it/bare_recovery_binder.rs, tests/it/bare_recovery_binder/checked_state_core.rs, tests/it/main.rs"
review_set: subset
relationships:
  - type: references
    target: ix://agent-ix/filament-spec-language/FR-495
  - type: references
    target: ix://agent-ix/filament-spec-language/FR-520
  - type: references
    target: ix://agent-ix/filament-spec-language/FR-531
---

## Summary

Planless, diff-scoped semantic gap review of QSL-2095 at 4045ba623807db73626192c47504f043917c2a86, PR #3181. Compared the new test claims with the current criterion text and the actual checked-package/handoff evaluation path; this is not a repository-wide matrix or completion audit.

## Verdict

**PASS** for the declared partial slice, with no newly introduced scoped gap. Active state-function refusal checks establish declaration-form classification, one diagnostic and exact declaration span, while the value-profile counterpart executes the same bodies against independently expected values. Active recovery checks distinguish duplicate occurrence counts, exact quantity sums, emitted function calls and activation. The conditional unavailable test checks both lazy success and the exact unavailable binder/cause with no fabricated source span.

The strict operator-locus and conditional-equality assertions remain present but ignored under QSL-2722 and QSL-2723, respectively; neither counts as coverage or completion. Existing production defects are retained with their assigned owners rather than duplicated as review findings.

## Examined scope

Excerpts are verbatim portions of the named units; examined does not mean fully satisfied.

| Unit | Path | Role | Excerpt |
| --- | --- | --- | --- |
| FR-495-AC-4 | spec/functional/FR-495-reach-the-integer-division-zero-divisor-and-state-core-rows-from-fsl-source.md | examined | Under a header profile `filament.state.core/v1` alone, `function f using v(a: Int[0, 9]): Int[0, 9] pure { a div 3 }` refuses `unsupported_construct`/`declaration-form` at the function declaration, naming `filament.state.core/v1`, and no `expression-form` refusal is made for its `div`; |
| FR-520-AC-1 | spec/functional/FR-520-evaluate-state-and-compensation-expressions-through-the-checked-package-evaluation-entry-points.md | examined | For every outcome row of the map, a test over **R** (FR-517) produces the successor outcome: true and false `trigger.guard`, `retry.relation` and `recovery.predicate`; a binder supplied `Unavailable`; a supplied value of the wrong type; a construct the family reports as unsupported; a meter one below measured usage. |
| FR-531 | spec/functional/FR-531-evaluate-from-a-checked-package-read-from-its-bytes-alone.md | examined | This requirement states the one entry that takes a `CheckedPackageV2` and nothing from the source side, what it admits, what it derives from the package's bytes, how it refuses, how its outputs spell identity where the bytes carry no source names or spans, and that it gives the same results and the same charges as the routes that start from source. |

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No findings (placeholder) | - |

## Coverage

Plan completion: not assessed.

Three active test functions have accepted focused passing receipts; two strict ignored functions are pending and uncounted. All five new functions carry trace tags and both new modules are registered. FR-495-AC-4 is partial while state-clause operator loci await QSL-2722. FR-520-AC-1 contributes true/false/unavailable recovery outcomes only: other slots, wrong types, unsupported constructs and meter limits are unclaimed. FR-531 contributes bytes-backed in-process evaluation only; no fresh-process, API-surface, complete route inventory or charge-parity criterion is claimed. Other historical ticket families are unclaimed, and accepted QSL-2082 through QSL-2094 work was not duplicated for tags.

No matrix, test, build or lint was executed by this review. No whole-repository traceability, stub, assurance-profile, history or ticket-completeness conclusion is made. The supplied actual command and exit history in /tmp/qsl-2095-pr-body.md is accepted without rerunning passing cases or ignored failures.
