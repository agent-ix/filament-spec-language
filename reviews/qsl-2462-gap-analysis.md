---
id: SR-2026111612172400000047
title: QSL-2462 gap-analysis of function claim population admission
type: SpecReview
analysis: gap-analysis
scope: agent-ix/filament-spec-language; fsl-replay/src/spine/call.rs, fsl-replay/src/spine/clause.rs,
  tests/it/config_version_spine.rs
review_set: subset
relationships:
- target: ix://agent-ix/filament-spec-language/spec/spec.md
  type: references
- target: ix://agent-ix/filament-spec-language/FR-650
  type: references
- target: ix://agent-ix/filament-spec-language/FR-867
  type: references
---

## Summary

QSL-2462, PR #2958 at 19840fa081a4c0b6415d2b4f682a0c49568aeffe. The change admits snapshot population arguments for function claims alongside references.

## Verdict

CONDITIONAL. The focused test carries an incorrect acceptance-criterion tag.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | The constant-true ConfigVersion regression asserts exit 0 but claims the shop examples inspection criterion. It neither evaluates the undefined lookup nor checks its exit 10, refuted item and cause. Trace local admission to FR-650-AC-7 and verify the six examples under FR-867-AC-3 in their owning gate. | tests/it/config_version_spine.rs:845 |

## Coverage

Quoin 0.36.7 computed the matrix at the frozen head. FR-100-AC-16 has twelve binders. FR-650-AC-4 and FR-650-AC-7 have one binder each. All three are tagged. FR-867-AC-3 has method Inspection and status method-without-symbol. Its sole Rust binder is the new ConfigVersion test. All four records say no run evidence. The new behavior has an owner in FR-650 and uses real snapshot admission. The reported focused test passed. The external examples gate was not verified in this scoped pass. Optional semantic audit was not requested.

Plan completion: not assessed

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
- id: FR-867-AC-3
  path: spec/functional/FR-867-name-the-function-and-its-result-type-when-a-non-boolean-function-is-selected-as-a-claim.md
  role: examined
  excerpt: '`claim-function-undefined` exit 10, item `refuted`, `cause` `{"kind": "undefined-evaluation", "where": W, "cause": R}` with `R` its FR-100 undefined
    reason; `claim-function-integer` (`seven`) exit 20, item `declined`, diagnostic `ill_typed`/`type-mismatch` with detail `function` `seven`, `expected`
    `Boolean`, `actual` `Integer`; `claim-function-bounded` (`boundedValue`) the same with `actual` `Int[0, 3]`; `claim-function-unknown` (`nope`) exit
    20, item `declined`, diagnostic `missing_declaration`/`missing-name` with detail `name` `nope`. Each has its recorded `expected/<case>.json`, and the
    ex'
```
