---
id: SR-1503
title: QSL-2094 terminal emission code-review
type: SpecReview
analysis: code-review
scope: agent-ix/filament-spec-language; QSL-2094; fsl-package/src/emit/tests/terminal.rs;
  fsl-package/src/emit/tests/terminal/emission_boundaries.rs; .agent/receipts/qsl-2094/{scope,validation}.md
review_set: subset
---

## Summary

Frozen source e10ecdc27f37b183f391f539a019108d7ccfde88. Ticket: QSL-2094. Review covers only the four changed files: terminal.rs, terminal/emission_boundaries.rs and the two scope/validation receipts. No production, configuration, dependency or gate changes.

## Verdict

**PASS (diff scope only).** Rust and code review found concrete, executable expectations: exact claim spans and occurrence ordinals; semantic graph equality after source reorder; rebuilt structural key order and ordinal lookup; literal Boolean values, absent self parameters and mode identity; unique dependency targets and typed refusal codes at independently specified first-error loci. Existing compiler/domain/I2 fixtures exercise real behavior; no mocks, copied fixtures or synthesized proof success were introduced. The pending model-root expectation remains strict and clearly excluded.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No findings (placeholder) | - |

## Coverage

Six active focused cases passed according to the coder’s in-lane runtime report (five original passes plus the changed ordering case’s pass). No tests, builds, lint or matrix commands were rerun. The strict model-root case is ignored under QSL-2718, excluded from coverage and remains blocking parent completion. No full criterion, historical family, matrix or parent completion claim is made. Plan completion: not assessed. Computed-matrix and repository-wide reverse-gap audits were intentionally excluded by the dispatch; this is a diff-scoped gap review, not an engine-backed whole-repository gap PASS.

## Examined units

```yaml
scope:
- id: FR-411-AC-1
  path: spec/functional/FR-411-emit-a-checked-model-terminal-declaration-as-a-v2-terminal-node.md
  role: examined
  excerpt: Compiling FR-126-AC-9's `Counter` unit with `terminal Done using v on Counter
    when { self.value = 3 };` emits one `temporal`/`terminal` node with `mode` `when`,
    `parameters` an aggregate holding one reference to the `self` parameter node,
    `condition` a reference to the `expression` node of `self.value = 3`, no `declaration`
    and one `claim` occurrence at the declaration's span; with `terminal Halt using
    v on Counter any;` it emits `mode` `any` with empty `parameters` and `condition`;
    without a declaration it emits no terminal node. The I2 reader returns `When`,
    `Any` and `None` respectively. No
- id: FR-411-AC-2
  path: spec/functional/FR-411-emit-a-checked-model-terminal-declaration-as-a-v2-terminal-node.md
  role: examined
  excerpt: AC-1's `when` and `any` terminal nodes have different node ids; renaming
    `Done` to `Stop` keeps the node id.
- id: FR-411-AC-4
  path: spec/functional/FR-411-emit-a-checked-model-terminal-declaration-as-a-v2-terminal-node.md
  role: examined
  excerpt: Fed to FSL's I2 reader, with `dependencies` set to the body's targets,
    AC-1's `when` node with `mode` `some` refuses `invalid_package`/`invalid-value`,
    and with `condition` the empty aggregate refuses `invalid_semantic_graph`, each
    at FR-476's locus.
- id: FSpec-FR-476-AC-4
  path: FSpec@f32bca6c7937e0652742b2bbe27a52419bf5a0ae:spec/objects/interfaces/FR-476-checked-package-model-terminal-declaration-body.md
  role: examined
  excerpt: 'With `dependencies` rebuilt after each edit: a body with `mode` and `condition`
    swapped in order refuses `invalid_semantic_graph`; `mode` `some` refuses `invalid_package`/`invalid-value`;
    an `any` node with a `condition` reference, a `when` node with an empty `condition`,
    a `parameters` with two entries, and an `any` node with both a `parameters` entry
    and a `condition` reference (at `parameters`) refuse `invalid_semantic_graph`;
    a `model` target that is a `value`/`parameter` node, and one that is a `model`/`object_type`
    node, refuse `invalid_model_binding`/`malformed-declaration`; a `paramete'
- id: FSpec-FR-476-AC-5
  path: FSpec@f32bca6c7937e0652742b2bbe27a52419bf5a0ae:spec/objects/interfaces/FR-476-checked-package-model-terminal-declaration-body.md
  role: examined
  excerpt: '`terminal Done using v on M when { true };` produces a node whose `condition`
    references the Boolean `value`/`literal` node of `true`, with empty `parameters`
    and `dependencies` the `model` node and that literal node; the reader admits it,
    and `when { false }` likewise. The `true` node''s id differs from MT-02''s `any`
    node''s id. With `dependencies` rebuilt: `condition` referencing the `self` `value`/`parameter`
    node refuses `ill_typed`/`operator-ineligible` at the target, and referencing
    an added integer `value`/`literal` node of `3` refuses `ill_typed`/`non-boolean-root`
    at the target.'
- id: FSpec-FR-476-AC-6
  path: FSpec@f32bca6c7937e0652742b2bbe27a52419bf5a0ae:spec/objects/interfaces/FR-476-checked-package-model-terminal-declaration-body.md
  role: examined
  excerpt: 'In a unit whose aliases `M` and `N` select one model identity, `terminal
    Done using v on M when { self.value = 3 };` with `terminal Halt using v on N any;`
    produces two terminal nodes whose `model` binding references the one `model_root`
    node of that identity; the reader admits the package, and the rows are the two
    nodes in ascending terminal node structural key (the test establishes that order
    by comparing the rebuilt keys), with ordinals 0 and 1 and keys (that identity,
    0) and (that identity, 1). The same unit with the two `model` alias lines swapped,
    or with the two `terminal` declarations '
```
