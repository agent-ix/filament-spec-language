---
id: SR-202611212119190339300000003
title: "FR-1044 current package behavior base review"
type: SpecReview
analysis: base
scope: "agent-ix/filament-spec-language; QSL-2739; FR-1044 Current system behavior package paragraph"
review_set: base
---

## Summary

Ticket: QSL-2739. Independent combined-diff review of agent-ix/filament-spec-language PR3393, base e4578fa3ca51b22d60f863dd3525043f1f7ced5c through frozen head 13f242003f7964e0735c19807b2c316671473988. Exactly one existing worktree was inspected. No source/spec edits, builds, runtime tests, fetch/rebase, commits or tracker operations were performed. Root retains final qualification and merge custody.

No applicable AssuranceProfile was found by searching for type: AssuranceProfile under spec/. Direct review selected code-review with Rust lane, gap-analysis and spec-review/base. The sole spec edit corrects descriptive current behavior; no normative requirement, AC, domain object, edge or structural change calls for an extra analysis. Optional whole-repository semantic fan-out was not run.

Publication is QUEUED PRIVATELY: NOT POSTED; no tracker readback yet; has-review status was not read or changed. Root must publish each method once, deduplicate and perform actual byte/YAML readback. QSL-2739, QSL-2271, QSL-2726 and QSL-2128 remain In Progress; FR-1044-AC-10 is unmet.

## Verdict

PASS for the changed descriptive paragraph. The declaration-family statement matches emitted authentic metadata and the existing package name-resolution branch. The unchanged normative Source/Package equality obligation remains unmet, with its ignored parity test retained.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No findings (placeholder) | - |

## Coverage

Base checklist: changed prose introduces no ID, requirement, criterion, verification method, relationship, scope allocation or duplicate contract. The composition-name row changes the current observed pending family to declaration. Unresolved names remain unnamed-claim, exit 21 and no items; admitted-package qualification remains explicit in the surrounding paragraph. Current behavior is separated from the unchanged AC-10 promise and its deferred package-consumer work. No file tracking, hash ledger, pin prose or new assurance machinery is specified. Normative neighboring FR-1044 obligations were context only, not reapproved wholesale.

Extra lenses skipped as inapplicable: EARS (no normative statement edit), integrity (no structural change), dependency (no edge edit), object (no object change), evidence (no verification-method edit), failure-domain/risk/scope-boundary (no new behavior contract). No Jev or enforced workflow was required by an applicable profile.

Plan completion: not assessed.

## Examined units

```yaml
{
  "scope": [
    {
      "id": "FR-530-AC-1",
      "path": "spec/functional/FR-530-carry-and-check-liveness-and-fairness-premises-in-native-composition.md",
      "role": "examined",
      "excerpt": "Compiling, emitting and reading back a `composition-decl` keeps the component subjects, the rule and the `environment`, `relies`, `liveness_dependencies` and `fairness` lists separate, with each slot's source span its member; a Boolean success flag in place of the slots is refused `invalid_runtime_input`/`wrong-value-kind`."
    },
    {
      "id": "FR-1044-current-package-behavior",
      "path": "spec/functional/FR-1044-request-a-component-composition-item-through-analyze.md",
      "role": "examined",
      "excerpt": "The package carries a composition's\n  declaration name row, so an item naming it is settled the same way naming\n  the `declaration` family."
    },
    {
      "id": "FR-1044-AC-10",
      "path": "spec/functional/FR-1044-request-a-component-composition-item-through-analyze.md",
      "role": "context_only",
      "excerpt": "Over `test/system-default`, a request with no `items` writes `Local` at index 0, `Sys` at 1, the subject's derived deadlock-freedom item at 2 (FR-620-AC-5), then the premise items `Bounded` at 3 and `Capped` at 4, and exits 21; over the unit's `package` handoff (FR-850) it writes the same items, records and exit code."
    },
    {
      "id": "FR-093-AC-19",
      "path": "spec/functional/FR-093-lower-checked-value-expressions-to-fr-322-terms.md",
      "role": "examined",
      "excerpt": "Every IR checked-package/v2 node kind (`node_tag`, `semantic_form`) is classified exactly once: as a family FSL's lowering writes, or as a kind FSL's lowering never writes, with the reason. For each family FSL writes, a package holding a node of it is admitted by IR's checked-package/v2 reader, through FSL's I2 read, as emitted."
    },
    {
      "id": "FSpec-FR-322-composition-declaration-names",
      "path": "filament-specification@f6b0480/spec/objects/interfaces/FR-322-checked-package-artifact.md",
      "role": "examined",
      "excerpt": "A\n`correspondence`/`component_composition` node SHALL carry its declared\ncomposition name in `declaration.qualified_name` and exactly one\n`claim_names` row, keyed to its `declaration` occurrence at ordinal 0."
    },
    {
      "id": "FSpec-FR-491-composition-member-occurrences",
      "path": "filament-specification@f6b0480/spec/functional/analysis/FR-491-spell-and-check-a-component-composition-declaration.md",
      "role": "examined",
      "excerpt": "The composition node SHALL carry one `generated` occurrence per body\nmember on its own node. The member sequence is the concatenation of the\nbody arrays `components`, `guarantees`, `relies`,\n`liveness_dependencies`, `supplies` and `environment`, in that order,\npreserving each array's canonical order."
    },
    {
      "id": "FSpec-FR-491-authored-regions",
      "path": "filament-specification@f6b0480/spec/functional/analysis/FR-491-spell-and-check-a-component-composition-declaration.md",
      "role": "examined",
      "excerpt": "The package writer SHALL capture each member's exact authored region\nbefore canonical body ordering. The corresponding `generated` entry's\n`regions` SHALL contain exactly that one region, under the declaration's\nsource reference and strictly within its declaration region."
    },
    {
      "id": "IR-source-owner-requirement",
      "path": "filament-contract-ir@96554ea/crates/filament-contract-model/src/checked_package/v2/owner.rs",
      "role": "context_only",
      "excerpt": "if declaration.is_some() {\n        return OwnerRequirement::Source;\n    }"
    },
    {
      "id": "FSpec-FR-154-artifact-id",
      "path": "filament-specification@f6b0480/spec/functional/type-model/FR-154-admit-domain-package-model.md",
      "role": "examined",
      "excerpt": "A type definition's or population node's artifact id does not match `^[A-Za-z][A-Za-z0-9_-]*$`"
    },
    {
      "id": "FSpec-FR-831-optional-carrier",
      "path": "filament-specification@f6b0480/spec/objects/interfaces/FR-831-carry-the-authored-correlation-clauses-of-protocol-controls.md",
      "role": "context_only",
      "excerpt": "Where a checked package carries protocol controls with `related` records, a\n  producer MAY publish the member."
    }
  ]
}
```

