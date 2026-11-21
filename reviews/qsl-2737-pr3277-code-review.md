---
id: SR-202611210022442313144108502
title: "code-review of QSL-2737 model-unit inheritance and member narrowing"
type: SpecReview
analysis: code-review
scope: "agent-ix/filament-spec-language; Cargo.lock, fsl-semantics/Cargo.toml, fsl-semantics/src/model/intake.rs, fsl-semantics/src/model/intake/model_unit.rs, fsl-semantics/src/model/normalize.rs, fsl-semantics/tests/it/main.rs, fsl-semantics/tests/it/model_unit_inheritance.rs; FR-710, FR-081, FR-082"
review_set: subset
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-082
    type: references
---

## Summary

Ticket: QSL-2737. Reviewed frozen PR #3277 at a12a7f6e1adc4fa4757a8fecec99a35cfffc8d20, scope `git diff origin/main...HEAD` (7 files, no spec files). Rust lane (installed dev-tools 0.2.0 `rust-review` plus `rust-style`) is folded into this artifact; the repository has no `agent-skills/rust-review/SKILL.md` checkout, so the installed skill was used. The model-unit compiler now writes object, machine and process `extends`, field `subsets`/`redefines` and operation `redefines` into the document, resolving forward and transitive names to the original declaring owner, and resolves `modifies self.f` frame fields the same way. `normalize.rs` now refuses an unreachable operation redefinition target through the extracted field helper. The FCD dependency moves to rev 424ba49e (merged PR361). The intake diagnostic filter becomes a narrow `(code, meaning, member)` table.

Two correctness defects remain. The narrow table hands an operation `redefines` whose target is a real operation to normalization, but normalization's reachability test is "effective member of the type", which includes the type's own operations, while FCD and FR-082 require a proper ancestor; a same-owner or self operation redefinition is therefore accepted with no refusal. Separately, `read_object` now parses `extends`, `subsets` and `redefines` for every caller, but only object, machine and process wire them up, so on record, event, interface and store declarations these clauses are silently discarded or written unchecked.

## Verdict

**FAIL** — two high findings (FND-001, FND-002). Everything else in scope was examined and is sound. The table rows match the ruling exactly. Unknown targets, field targets, wrong meanings, out-of-range indices, trailing pointer segments and non-`/ir/types` pointers all keep their admission errors, and the unit negatives cover each of them. There is no blanket suppression and no second traversal. Forward and transitive lookup is right (BFS with a `seen` set, so it terminates on cycles and does at most O(objects × members) work under the 100,000-token source limit). The operation unreachable branch now skips the contest exactly as the field branch does, so `push_conflict_charge` and `phase4_fact_count` charging are unchanged, and owner/rank dedup is preserved by reusing the helper verbatim. The process identity inheritance, the machine supertype retention and the FCD rev pin (a real dependency pin, not ceremony) are correct.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | Same-owner or self operation `redefines` (e.g. `object Child { operation a(); operation b() redefines Child::a; }` or `operation bump() redefines Child::bump`) is accepted. FCD emits UNRESOLVED_FEATURE_REF because its target is not an ancestor operation. The new OperationRedefines row suppresses it because the target identity exists. Normalization's `reachable = store.contains(type_key, target)` is true for the type's own members, so no RedefinitionTarget refusal is raised, contrary to FR-082 ("a redefines naming no field of a proper ancestor SHALL refuse with a redefinition-target cause") and the ancestors-only ruling. | fsl-semantics/src/model/intake.rs:2162 |
| FND-002 | high | `read_object` now accepts `extends`, field `subsets`/`redefines` and operation `redefines` for every caller, but only object/machine/process push `ObjectLinks` and supertype references. `event E extends X` is now admitted with the clause silently dropped (supertypes reset to []), where it was previously invalid_syntax. `record R extends Ghost` and `interface I extends Ghost` write a dangling `supertypes` entry with no `missing-name` check, so FR-154 refuses a compiled document (FR-710: a compiler defect). A `subsets`/`redefines` on a record, event or interface field (grammatical `domain-field`) is parsed and discarded, so FR-082 never checks it. | fsl-semantics/src/model/intake/model_unit.rs:1237 |
| FND-003 | low | `declaring` takes `start: &[(String, Span)]` but never reads the spans. Every caller clones the owner name into a one-element array only to pass it. rust-style asks for immutable inputs borrowed as `&str`, so take `start: &str`. | fsl-semantics/src/model/intake/model_unit.rs:1152 |

## Gates

No local Cargo, fmt, clippy, test, deny, Kani or aggregate gate was run, per the review brief. Focused runtime evidence was delegated to draco:QSL:code and is reported as follows. At 12441e0197, `normalization_diagnostic_rows` passed 2 and the combined integration run passed 17 with 1 fixture failure. At a12a7f6e, `machine_and_process_inheritance_keeps_the_declared_supertype` passed 1. All runs used `--locked`, exit 0 for the covered cases. The final delta changes only the machine fixture text. No test covers FND-001 or FND-002. No CI workflow file changed. No AssuranceProfile applies.
