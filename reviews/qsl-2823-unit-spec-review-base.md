---
id: SR-967175026302197768900474181941935084716
title: "Specification review of the FR-982 canonical-root fixture"
type: SpecReview
analysis: base
scope: "agent-ix/filament-spec-language; QSL-2823; spec/functional/FR-982-end-a-window-aggregate-threshold-at-the-next-temporal-connective.md FR-982-AC-4"
review_set: base
---

## Summary

Reviewed PR 3472's sole change to FR-982-AC-4. It replaces the non-identity targetless millisecond unit with a scale-one second root and a millisecond edge to that root. The numeric threshold still checks and the Boolean threshold still refuses at the same expression under both profiles.

## Verdict

**PASS** — no defects found in this specification-only amendment. The corrected input matches FSpec FR-142's canonical-root rule and the existing unit syntax in FR-194-AC-11. This does not claim that the stale aggregate test has been repaired or run; that test change must follow this prerequisite merge.

## Scope

```yaml
{
  "scope": [
    {
      "id": "FR-982-AC-4",
      "path": "spec/functional/FR-982-end-a-window-aggregate-threshold-at-the-next-temporal-connective.md",
      "role": "examined",
      "excerpt": "Under the timestamped-event finite-window profile, on a clock `\\\"events\\\"` over a unit declaring `dimension Time; unit s : Time = rational(1, 1); unit ms : Time = rational(1, 1000) * s;`, the same two formulas with the window `past[0 ms, 1000 ms]` check and refuse identically. No formula carries an interval-less temporal operator, which neither profile admits (FSpec FR-090-AC-7, FR-048-AC-9)."
    },
    {
      "id": "FR-982/Behavior",
      "path": "spec/functional/FR-982-end-a-window-aggregate-threshold-at-the-next-temporal-connective.md",
      "role": "context_only",
      "excerpt": "The checker SHALL refuse a parenthesized threshold whose type is not\n  numeric `ill_typed`/`type-mismatch` at the threshold, and keep FSpec\n  FR-414's `unit-mismatch` for a threshold of another dimension."
    },
    {
      "id": "FSpec-FR-142/Dimension and conversion algebra",
      "path": "spec/functional/type-model/FR-142-evaluate-quantities-and-units.md (filament-specification)",
      "role": "context_only",
      "excerpt": "The unit declarations for one I04 semantic-node dimension identity form a closed directed\ngraph with exactly one canonical root. The root has no target, scale one and\noffset zero."
    },
    {
      "id": "FSpec-FR-142/refusal-row-5",
      "path": "spec/functional/type-model/FR-142-evaluate-quantities-and-units.md (filament-specification)",
      "role": "context_only",
      "excerpt": "the dimension's one targetless unit (its canonical root) has a scale other than one or an offset other than zero"
    },
    {
      "id": "FR-194-AC-11",
      "path": "spec/functional/FR-194-evaluate-window-aggregates-in-monitors.md",
      "role": "context_only",
      "excerpt": "the unit declaring `dimension Time; unit s : Time = rational(1, 1); unit ms : Time = rational(1, 1000) * s;`"
    }
  ]
}
```

## Review evidence

- Read the complete one-row diff, FR-982's description and behavior, FSpec FR-142's dimension/unit algebra and refusal row 5, and the existing FR-194-AC-11 fixture.
- A root `s` with scale one and no target satisfies the root constraint. The `ms` edge has the same dimension, nonzero exact scale `1/1000`, target `s`, and no explicit offset; it yields one acyclic graph with one identity root.
- The source examples preserve `past[0,1]` and `past[0 ms, 1000 ms]`, the numeric and Boolean threshold expressions, their result categories, exact refusal code/cause and locus, and the interval-less operator restriction.
- The verification cell now names only the `Test` method, as required by the current criterion authoring contract. The active trace tag still names FR-982-AC-4.
- Static `quire matrix --scope /home/peter/dev/worktrees/qsl-2823-unit-spec --format json` exited 0 and marks FR-982-AC-4 `tagged`: the active `aggregate_threshold::a_parenthesized_threshold_is_typed_against_the_aggregate` test carries it; a legacy ignored test also carries it. Repository-wide symbol-collision and unmarked-trace diagnostics are disclosed in the scratchpad matrix stderr and are not a green runtime claim.
- The current active test fixture still declares the old non-identity root. This is the assigned dependent stale-test repair, intentionally excluded from the spec-first PR.
- Changed-FR structural validation and `git diff --check origin/main...HEAD` exited 0. Module registry warnings from the isolated ISO schema invocation are nonfatal and recorded; no validation failure occurred.
- No applicable AssuranceProfile is declared under this repository's `spec/` tree. No enforced review workflow is required by a profile.
- No repository builds, tests, compiled scanners or titan jobs were run or requested. Installed tools: quoin 0.36.8, quire 0.52.0 (engine 0.62.2).

## Method selection

Applied the base specification checklist to the changed criterion: identifier continuity, deterministic input, expected results, error path, boundary preservation, trace binding and cross-requirement consistency. No requirement statement, relationship, domain object, system boundary or architecture changes, so optional analysis lenses do not apply. Existing whole-repository coverage diagnostics are outside this one-row review; the scoped criterion has an active binding.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No findings (placeholder) | - |
