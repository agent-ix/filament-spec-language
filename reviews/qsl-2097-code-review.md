---
id: SR-1511
title: "QSL-2097 metadata dependency aliases code-review"
type: SpecReview
analysis: code-review
scope: "agent-ix/filament-spec-language; tools/arch-lint/metadata.rs; FR-356-AC-1"
review_set: subset
relationships:
  - type: references
    target: ix://agent-ix/filament-spec-language/FR-356
---

## Summary

Ticket: QSL-2097. Reviewed frozen commit 4dcb369d53317c5ed586c1736eb1820c7b8e558a, whose only changed file is tools/arch-lint/metadata.rs. This code review includes the Rust review covers the three new FR-356-AC-1 tests and their ten metadata inputs.

## Verdict

**PASS** for this confined metadata-gate fixture slice. No defects found. The walker admission, aliased FSL refusals, and renamed facade exceptions exercise production parse_edges and graph::check. Expected refused edges independently specify backend, dependency kind and resolved crate identity. The normal/build/dev cases preserve the current gate's build-to-normal model and target-independent direction policy.

Rust review applied repository conventions and Rust implementation/testing idioms: private borrowed-input helper, deterministic in-memory metadata fixtures, real parser/checker calls, explicit expected edges, and criterion trace attributes. No production path, gate configuration, dependency or copied artifact changes are introduced.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No findings (placeholder) | - |

## Examined Scope

```yaml
scope:
  - id: FR-356-AC-1
    path: spec/functional/FR-356-walk-nested-structures-through-one-iterative-walker-toolkit.md
    role: examined
    excerpt: "arch-lint's direction check admits an IR, RT or CG edge to `filament-walk`, sourced from `agent-ix/filament-walk`, and refuses one to any `fsl-*` crate other than CG's normal edges to `fsl-replay` and `fsl-package` (FB-05). The crate's own `no_std` build, features and dependency tree are AC-1 of `agent-ix/filament-walk` FR-356."
bindings:
  - {test_id: renamed_target_specific_walker_dependencies_remain_outside_the_fsl_direction_graph, ac_id: FR-356-AC-1, trace: correct}
  - {test_id: a_walker_alias_cannot_hide_a_target_specific_fsl_dependency, ac_id: FR-356-AC-1, trace: correct}
  - {test_id: renamed_facades_keep_only_the_codegen_normal_edge_exception, ac_id: FR-356-AC-1, trace: correct}
```

## Coverage

Author-reported focused tests: three passed, exit 0; scoped Clippy and rustfmt passed, exit 0. These in-lane results were accepted without rerunning tests, builds, lint or gates. One final affected component gate remains with the author/lead.

No matrix computation or whole-repository audit was performed, per the dispatch scope. This PASS records diff review only, not an engine-backed matrix PASS, historical ticket completion, walker crate qualification, deep-input runtime results, or unrelated FR/NFR coverage. Plan completion: not assessed.

