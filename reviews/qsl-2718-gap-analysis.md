---
id: "SR-20261121003283002"
title: "QSL-2718 PR 3283 gap-analysis"
type: SpecReview
analysis: gap-analysis
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

CONDITIONAL on current execution evidence only; no scoped defects found. The scoped source/test inspection clears independent review. This is not an engine-backed whole-repository assurance PASS.

## Coverage

Plan completion: not assessed

The caller explicitly restricted this review to the two-file delta and allowed only already computed cheap matrix evidence. The full-repository matrix, coverage census and optional semantic audit were not run. An existing engine-generated snapshot was inspected at /home/peter/.cache/handoff/terra-QSL-package-consumer-api-review-3276/matrix.json. It reports FR-411-AC-1 tagged, including the focused root test as ignored at its previous line 114. The current frozen diff removes exactly that ignore; the test is now at line 113. The snapshot is historical and is not presented as a matrix recomputed at this head. It also reports FR-411-AC-8 untagged; that broader pre-existing criterion and the full-repository trace backlog are outside this two-file review and are not newly introduced findings.

Reverse ownership: terminal model-root binding is directly owned by FR-411 Behavior and accepted FSpec FR-476 "Several terminal nodes on one model node" at f32bca6c7937e0652742b2bbe27a52419bf5a0ae. The activated test is real source-to-emission-to-independent-reader evidence, tagged FR-411-AC-1 and FSpec-FR-476-AC-6; it supplies partial rather than exhaustive evidence for those composite criteria. The independent expected root distinguishes object_type from model_root without importing the lowering helper.

Stub/inflation inspection: no todo!, unimplemented!, TODO, no-op, skipped branch or replaced unit under test appears in the changed production logic; the only test change enables the strict existing assertion. No requirement is declared complete from this delta. Alias/key-order/ordinal and occurrence assertions in neighboring tests remain intact. Negative reader rejection is explicitly retained as remaining ticket scope under the accepted architecture sequence.

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

