---
id: SR-2026111612172400000046
title: QSL-2462 code-review of function claim population admission
type: SpecReview
analysis: code-review
scope: agent-ix/filament-spec-language; fsl-replay/src/spine/call.rs, fsl-replay/src/spine/clause.rs,
  tests/it/config_version_spine.rs
review_set: subset
relationships:
- target: ix://agent-ix/filament-spec-language/FR-650
  type: references
- target: ix://agent-ix/filament-spec-language/FR-100
  type: references
---

## Summary

QSL-2462, PR #2958 at 19840fa081a4c0b6415d2b4f682a0c49568aeffe. The change admits snapshot population arguments for function claims alongside references.

## Verdict

CONDITIONAL. Diagnostic precedence, snapshot copying and formatting need disposition.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | The new population prepass can refuse a later argument before checking a missing name or converting an earlier value. With parameters (flag, p[3], q[4]), omit flag and supply the same snapshot population for p and q. The existing identity-conflict guard returns WrongValueKind at position 2 before the runner reports the missing flag. Validate all names first, then bind each population as its declared slot is converted. | fsl-replay/src/spine/clause.rs:3075 |
| FND-002 | medium | Each population parameter now deep-clones the whole snapshot, including object-reference vectors, history, and population-member maps. Native calls previously moved the accumulating environment into with_population. Accept the owned environment and return it directly on no-op paths to avoid a full snapshot copy per parameter. | fsl-replay/src/spine/call.rs:1151 |
| FND-003 | low | The compiled run_function continuation and the changed import in tests/it/config_version_spine.rs:12 fail rustfmt on the changed lines. Restore the wrapped import and align the continuation with the formatter. | fsl-replay/src/spine/clause.rs:2264 |

## Coverage

The code-review and Rust lanes examined both callers, admission order, ownership, changed integration assertions and gate integrity. The PR changes no CI files. The dispatch reports the focused regression passed through locked-build. git diff --check passed. Changed-file rustfmt --check exited 1, including the two changed hunks above.

```yaml
scope:
- id: run_clause_reading
  path: fsl-replay/src/spine/clause.rs
  role: examined
  excerpt: "} => run_function(\n            &run,\n            models.models(),\n            name,\n            arguments,\n            snapshot,\n    \
    \        snapshot_path.as_deref(),\n        ),"
- id: run_compiled_clause_selection
  path: fsl-replay/src/spine/clause.rs
  role: examined
  excerpt: "} => run_function(\n            &run,\n            compiled.models.models(),\n            name,\n            arguments,\n            snapshot,\n\
    \            snapshot_path.as_deref(),\n        )\n            .unwrap_or_else(|_| {"
- id: run_function
  path: fsl-replay/src/spine/clause.rs
  role: examined
  excerpt: "let mut environment = environment;\n    for (position, ((_, value_type), value)) in selected.parameters.iter().zip(&slots).enumerate() {\n \
    \       let (ValueType::Population(maximum), Some(ClauseArgumentValue::Form(form))) =\n            (value_type, value)\n        else {\n           \
    \ continue;\n        };\n        let Some(population) = form.get(\"population\").and_then(serde_json::Value::as_str) else {\n            continue;\n\
    \        };\n        environment = match super::call::bind_snapshot_population(\n            &environment,"
- id: bind_snapshot_populations
  path: fsl-replay/src/spine/call.rs
  role: examined
  excerpt: environment = bind_snapshot_population(&environment, models, named, *maximum, position)?;
- id: bind_snapshot_population
  path: fsl-replay/src/spine/call.rs
  role: examined
  excerpt: "objects\n        .clone()\n        .with_population(binding)\n        .map_err(|_| wrong())"
- id: function_claim_maps_population_and_reference_arguments_by_role
  path: tests/it/config_version_spine.rs
  role: examined
  excerpt: "assert_eq!(\n        output.status.code(),\n        Some(0),\n        \"{}\",\n        String::from_utf8_lossy(&output.stdout)\n    );\n   \
    \ let document: Value = serde_json::from_slice(&output.stdout).unwrap();\n    assert_eq!(document[\"disposition\"][\"stage\"], \"evaluate\", \"{document}\"\
    );\n    assert_eq!(document[\"disposition\"][\"truth\"], true, \"{document}\");"
- id: FR-100-AC-16
  path: spec/functional/FR-100-run-a-named-function-through-the-spine.md
  role: examined
  excerpt: Unknown, duplicate and missing parameter failures retain precedence over value admission, and declared parameter order selects the first failing
    input. A function ignoring its arguments cannot bypass these checks.
- id: FR-650-AC-4
  path: spec/functional/FR-650-carry-every-argument-and-result-kind-of-a-run-in-the-native-value-grammar.md
  role: examined
  excerpt: 'A function over a Population parameter called with `{"population": P}` binds that population with the parameter''s declared maximum and completes;
    with a population the snapshot does not hold it refuses `invalid_runtime_input`/`wrong-role-mapping` at stage `call`.'
- id: FR-650-AC-7
  path: spec/functional/FR-650-carry-every-argument-and-result-kind-of-a-run-in-the-native-value-grammar.md
  role: examined
  excerpt: 'FR-312''s `function` selection admits the run value form: FR-109-AC-4''s `sameIdentity` with `a` given as the identity-reference form of `child`
    and `b` as `{"reference": {"population": P, "key": "child"}}` reports `success`, `truth: true`, exit 0; a Boolean claim `function short using v(t: Text[0,
    8; nfc]): Boolean pure { size(t) < 3 }` selected with `t` `{"kind": "text", "text": "ab"}` over the same snapshot reports `success`.'
```

## Dispositions

Round 1 at `98bf9a61cab8ec925022329989b14bf212aa61f4`.

| FND | Outcome | Reason |
| --- | --- | --- |
| FND-001 | still-open | Commit 98bf9a61ca removed the full missing-parameter check added in d788390536. The missing-first fixture now passes, but declaring (p[4], q[1], flag), supplying the same population for p and q and omitting flag still reaches the binding conflict for q before checking flag. FR-109 requires FR-100 name binding, and FR-100-AC-16 gives missing parameters precedence over all value admission. Check all slots for missing arguments after validating supplied names and before converting the first slot. |
| FND-002 | fixed | The helper takes ownership of the accumulator. Both no-op branches return the environment directly and the final insertion consumes it. |
| FND-003 | fixed | Targeted rustfmt reported only unchanged imports at the top of clause.rs. The edited continuation and integration-test import now match rustfmt. |
