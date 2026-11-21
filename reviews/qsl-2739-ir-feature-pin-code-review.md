---
id: SR-202611212119190339300000001
title: "Combined composition producer and IR pin code review"
type: SpecReview
analysis: code-review
scope: "agent-ix/filament-spec-language; QSL-2739; all 12 changed files in e4578fa3...13f2420"
review_set: subset
---

## Summary

Ticket: QSL-2739. Independent combined-diff review of agent-ix/filament-spec-language PR3393, base e4578fa3ca51b22d60f863dd3525043f1f7ced5c through frozen head 13f242003f7964e0735c19807b2c316671473988. Exactly one existing worktree was inspected. No source/spec edits, builds, runtime tests, fetch/rebase, commits or tracker operations were performed. Root retains final qualification and merge custody.

No applicable AssuranceProfile was found by searching for type: AssuranceProfile under spec/. Direct review selected code-review with Rust lane, gap-analysis and spec-review/base. The sole spec edit corrects descriptive current behavior; no normative requirement, AC, domain object, edge or structural change calls for an extra analysis. Optional whole-repository semantic fan-out was not run.

Publication is QUEUED PRIVATELY: NOT POSTED; no tracker readback yet; has-review status was not read or changed. Root must publish each method once, deduplicate and perform actual byte/YAML readback. QSL-2739, QSL-2271, QSL-2726 and QSL-2128 remain In Progress; FR-1044-AC-10 is unmet.

## Verdict

PASS for the scoped combined code/Rust diff. No scoped defect found. This is not a whole-repository assurance PASS or an AC-10 completion claim. The build/check gate belongs to root and remains outside this review.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No findings (placeholder) | - |

## Coverage

Examined all changed files:

- Cargo.lock
- Cargo.toml
- fsl-analyze/tests/it/analyze_composition_cli.rs
- fsl-package/src/emit.rs
- fsl-package/src/emit/tests/admission_corpus.rs
- fsl-package/src/emit/tests/composition.rs
- fsl-semantics/src/check/assemble.rs
- fsl-semantics/src/check/composition.rs
- fsl-semantics/src/check/lowering.rs
- fsl-semantics/src/check/node_key/composition.rs
- fsl-semantics/src/check/region.rs
- spec/functional/FR-1044-request-a-component-composition-item-through-analyze.md

Checked source custody from composition assembly through SourceOwner, declared NodeKey, SemanticNode declaration, Declaration/0 occurrence, claim row and exact whole declaration region. The name row reaches the existing package PendingFamily::Declaration branch; no analyzer implementation or reader weakening was added. Typo remains unnamed-claim; default package composition pending and neighboring Local settlement remain explicit.

Checked six canonical arrays and their paired original spans, including NodeRef-derived guarantee order rather than lexical clause names. The new test independently discovers named checked dependency NodeKeys and actual emitted WireGraph positions; reverses authored order in every group; checks 14 exact member intervals, owner, QName, declaration interval and occurrence count; then compares full normalized raw foreign structural references to independently selected document-held IDs. Each raw operand is required to be Dependency with the authenticated owner and a valid owner-local exact index, and full equality checks the complete sequence. Expanded structural keys and authenticated clause-name rows provide a separate check. No implementation helper computes expected member regions. Sparse-array and reversed-source-map controls and negative validation precedence exercise real strict readback.

The ConfigVersion test emits actual source against authentic domain-package evidence and checks generated model-root ownership, self typing and selected identity; it is not a forged shortcut. Existing domain fixture reuse was inspected; no vendored implementation, compatibility layer, duplicate schema, disabled gate, new unsafe, async task, narrowing cast or source panic was introduced. Canonical sort keys remain in the existing ClauseEntry/PremiseEntry/SupplyEntry implementations; decorated term/span sorting preserves their pairing.

Dependency custody: manifest and lock agree on IR 96554ea506a19b16ff9863ad3d30066578bc609c and both FSpec aliases f6b0480cd90f5d1b5c5ea64f694895bb7b5ce2e0. The lock diff changes only those two sources. exact b26a8e4c592503fccdc4a0e3d2cce23dd8487616, SV fa5e78aa71fcea5418492b67f13a33124af84459, observation a583c51260420829c0cd99b24c3af96311de05b2, mltl c21d0d0a41664adb57fe5a1065b9816f81f6a37d and checked/eval a0e8e1254827de71738cbf87646e09ef68531692 are preserved. The locally installed pinned IR owner rule was read directly; declared nodes require SourceOwner. No NEXT WorkScope or unmerged HP1/correlation producer work is imported.

Evidence read: /private/tmp/qsl-nextir-final-package11-20261121.log reports 11 passed, 0 failed, 0 ignored, 1069 filtered, 0.38s; /private/tmp/qsl-nextir-final-analyze17-20261121.log reports 17 passed, 0 failed, 0 ignored, 2739 filtered, 0.67s. These are root-supplied exact-head qualification logs, not tests rerun by this reviewer. Analyze log warnings: existing unused sample_fingerprint and unused TimedRefinementRefusal/ZoneCheckRefusal imports; no warning suppression added. Protocol tests preserve unsupported Package boundaries, not new parity. Parent-reported upstream ancestry/reachability is custody context, not independently fetched evidence. git diff --check passed; working tree remained unchanged.

Direct rustfmt --check --edition 2021 --config skip_children=true over all nine changed Rust files passed (exit 0); no Cargo command was run by this reviewer.

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

