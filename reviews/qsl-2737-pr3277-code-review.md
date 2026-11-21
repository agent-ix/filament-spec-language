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

## Dispositions

Disposition round 1. Fix head: a756b1f685fcd4e406a8d89c07463c9383f47b5d (one fix commit over a12a7f6e). Outcomes come from inspecting the fix diff. Draco's focused fix-head run (2 unit tests plus 32 integration tests) is queued, not green, so no runtime result or merge eligibility is claimed here.

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | Each redefinition edge now carries `inherited = member_owner(target) != Some(owner) && store.contains(owner, target)`, judged against the original redefining owner. A group is reachable only if some edge is inherited. Non-inherited edges (self, same-owner, sibling) are partitioned out and refused through `record_unreachable_redefinition_target`. For valid programs (every edge inherited), the conflict and fact charges are unchanged. Tests cover self, same-owner and mixed local-plus-descendant cases at model_unit_inheritance.rs:302. |
| FND-002 | fixed | `validate_declaration_inheritance_syntax` rejects `extends` on record, event, interface and store, any record/event/interface header modifier, record `subsets`/`redefines`, event operations and non-operation store members. Event, interface and store links now go through `push_model_type` into `link_inheritance`. `validate_nonobject_links` runs the shared FR-154 validator before publishing and projects refusals to the declaration span. Tests start at model_unit_inheritance.rs:523. |
| FND-003 | fixed | `declaring` now takes `start: &str` and seeds `VecDeque::from([start])`, and the three callers pass the borrowed names. |

Disposition round 2 (final verification). Head: 9d6cc5074dd85242c50b4e07d813ec2ddeba8493. Inspected only the source and test repairs in a756b1f6..9d6cc507, excluding review artifacts.

- `fsl-forms::cst` re-exports `resource_exhausted`, and `project_link_refusal` calls `cst::resource_exhausted`. `fsl-cst` remains a dev-dependency of fsl-semantics, so the semantics → forms → CST layering is preserved. No manifest or pin change.
- `operation` is matched as a declaration only when the next token is not `:`, in `record_operation_span`, `validate_declaration_inheritance_syntax` and `read_object`. This follows FR-710's contextual-word rule ("a keyword only where its declaration or clause begins").
- The compiler's S1 early return (`if !parsed.is_admissible() { return Err(parsed.diagnostics().to_vec()); }`) is unchanged from a756b1f6, and the provisional rewrite in b84cacfd85 is fully removed. CST `EventDeclaration` admits only `DomainField | DomainInvariant` and `DomainField` is `ident ':' …`, so `operation bump();` correctly refuses at `bump`. The repaired test asserts the compiler's diagnostics equal the parser's, plus phase, code, cause and token. That is stronger than before, not weaker.
- The original fixes are intact: FND-001's `inherited` edge and partition, FND-002's grammar gate and FR-154 projection, and FND-003's `&str` borrow are all unchanged.

No new rows: all three findings' latest outcome remains `fixed`, and no new finding. The round-1 comment's FND-002 after_excerpt was re-recorded as a byte-verbatim excerpt (same lines). Draco's focused targeted-10 run at this head is queued; no runtime green or merge eligibility is claimed.
