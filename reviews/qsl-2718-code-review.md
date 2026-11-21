---
id: "SR-20261121003283001"
title: "QSL-2718 PR 3283 code-review"
type: SpecReview
analysis: code-review
scope: "agent-ix/filament-spec-language; QSL-2718; fsl-semantics/src/check/lowering/terminal.rs; fsl-package/src/emit/tests/terminal/emission_boundaries.rs; cecae62b03ac368c1faf4d1a14b0c37cfceab185..4e31f21148742fe1524439a0af48bfd171a37cfe"
review_set: subset
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-411
    type: references
---

## Summary

Ticket: QSL-2718. PR filament-spec-language#3283. Frozen head 4e31f21148742fe1524439a0af48bfd171a37cfe; reviewed diff against stacked base cecae62b03ac368c1faf4d1a14b0c37cfceab185 only (two files). No spec/plan, dependency, pin, CI or public API edits. AGENTS.md and README.md were read. No applicable AssuranceProfile was found in spec/.

This review covers the FSL emission/read-back half only. The architect's accepted sequencing retains object_type acceptance in the independent reader for now; strict negative rejection remains on QSL-2718. The external pin landing hold remains. The lead must retarget to main, integrate main and validate its final head before merging; never merge into the types branch.

## Verdict

PASS for the bounded code/Rust inspection: no actual findings, including low-severity findings. Execution evidence is pending; this is not final merge authorization.

## Review evidence

The new selection predicate matches terminal_object's existing alias plus selection identity predicate exactly. It borrows the admitted selection and reuses model_root_key, which mints package selection.identity, path ["model", "model_root"], ordinal 0. It does not introduce a duplicate key formula. Missing selection and InvalidNodeKey map to the existing typed UnkeyableRequirements refusal at claim_at. No unchecked panic, new conversion, unsafe or async path is introduced.

The object remains available for the unchanged Reference<object> self type; the model reference alone changes. Equal aliases of one identity receive equal root keys; distinct identities remain separate. Body member order, content addressing, claim recording and source-order occurrences are unchanged. Existing tests compare reversed declarations' graph bodies and reader rows, structural key order and ordinals 0/1; equal declarations retain both source spans and one row. These tests were inspected, not run.

The changed test removes only its ignore attribute. Its expected root key is constructed independently of production model_root_key, requires two real independent-reader rows and compares every row with that root. Reverting the lowering fix defeats that assertion. It uses real compile, emit, admission and read_terminals seams. Its existing FR-411-AC-1 tag is partial evidence for that broad AC; FSpec-FR-476-AC-6 directly owns the root and two-row scenario. No trace tag or assertion is changed. No vendoring, new helper duplication, stub or weakened gate is introduced.

## Coverage

No local Cargo invocation, cold target, workspace gate or source mutation was performed, as instructed by the review brief. The exact requested test remains pending with the lead: `cargo test --locked -p fsl-package --lib emit::tests::terminal::emission_boundaries::a_terminal_row_names_the_selected_model_root_instead_of_its_object_type -- --exact`. There is no green execution claim. Independent `git diff --check` passed; edition-2021 rustfmt success is a lead-supplied receipt.

Full examined/context scope, including clean units:
```yaml
[
  {
    "id": "terminal:model-selection",
    "path": "fsl-semantics/src/check/lowering/terminal.rs",
    "role": "examined",
    "excerpt": "model.alias() == Some(input.model) && model.selection().identity == input.package"
  },
  {
    "id": "terminal:model-key",
    "path": "fsl-semantics/src/check/lowering/terminal.rs",
    "role": "examined",
    "excerpt": "let model = super::model_root_key(selection).map_err(|_| {"
  },
  {
    "id": "terminal:root-oracle",
    "path": "fsl-package/src/emit/tests/terminal/emission_boundaries.rs",
    "role": "examined",
    "excerpt": "assert_eq!(rows.len(), 2);\n    for row in rows {\n        assert_eq!(row.model, expected);\n    }"
  },
  {
    "id": "FR-411:model-binding",
    "path": "spec/functional/FR-411-emit-a-checked-model-terminal-declaration-as-a-v2-terminal-node.md",
    "role": "examined",
    "excerpt": "The emitter SHALL write `model` as a reference to the `model` node of the declaration's model alias."
  },
  {
    "id": "FR-411-AC-1",
    "path": "spec/functional/FR-411-emit-a-checked-model-terminal-declaration-as-a-v2-terminal-node.md",
    "role": "context_only",
    "excerpt": "The I2 reader returns `When`, `Any` and `None` respectively. No checked declaration is refused as not emitted."
  },
  {
    "id": "FSpec-FR-476:model-selection",
    "path": "filament-specification@f32bca6c7937e0652742b2bbe27a52419bf5a0ae/spec/objects/interfaces/FR-476-checked-package-model-terminal-declaration-body.md",
    "role": "examined",
    "excerpt": "A package names a model by the identity of its model selection, never by a source alias: the `model` binding references the selection's `model`/`model_root` node"
  },
  {
    "id": "FSpec-FR-476-AC-6",
    "path": "filament-specification@f32bca6c7937e0652742b2bbe27a52419bf5a0ae/spec/objects/interfaces/FR-476-checked-package-model-terminal-declaration-body.md",
    "role": "examined",
    "excerpt": "the rows are the two nodes in ascending terminal node structural key (the test establishes that order by comparing the rebuilt keys), with ordinals 0 and 1 and keys (that identity, 0) and (that identity, 1)."
  },
  {
    "id": "FR-411-AC-8",
    "path": "spec/functional/FR-411-emit-a-checked-model-terminal-declaration-as-a-v2-terminal-node.md",
    "role": "context_only",
    "excerpt": "checks, emits two terminal nodes whose `model` binding is the one `test/vault` `model_root` node, and emit-then-admit succeeds with no `ambiguous-name`"
  }
]
```

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No findings (placeholder) | - |

