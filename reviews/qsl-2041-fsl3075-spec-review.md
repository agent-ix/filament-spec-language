---
id: SR-2026111919022500000002
title: "spec-review of QSL-2041 FR-104 qualified operation parameter name"
type: SpecReview
analysis: integrity
scope: "agent-ix/filament-spec-language; PR #3075 at 74d10aee; spec/functional/FR-104-check-state-clauses.md (Typing bullets at lines 202-221, FR-104-AC-18), spec/functional/FR-035-parse-composed-native-units.md (reserved-word prose, FR-035-AC-3 context), spec/test-cases/TC-3083-a-reserved-word-operation-parameter-is-read-by-its-qualified-name.md"
review_set: subset
---

## Summary

Spec review of FSL PR #3075. Ticket: QSL-2041. The PR adds FR-104 Typing bullets: in a pre/post body, `<op>::p` resolves to the parameter `p` of the clause's operation, where `<op>` is spelled as the clause's `on` writes it (`M::T::op` or `T::op`). The name lowers to the same `value`/`parameter` node as the bare name. A wrong prefix, an unknown parameter or use in an invariant/constraint refuses `missing_declaration`/`missing-name`. The PR also adds FR-104-AC-18, TC-3083 and an FR-035 prose note. The review checked the code facts on main. `fsl-cst` `QualifiedName = ident {'::' member}` and `Terminal::MemberName` accepts reserved words (`parser.rs:2685`). `Primary` takes `EnumValue` and `QualifiedName`, and `fsl-forms` `value.rs:2056` lowers both to `Shape::Name`. The `typing.rs:1471` `ExprNode::Name` arm sends the name through `resolve_target` (`call_target.rs:66`) and `qualified_name_refusal`, which turns it into `MissingQualifiedName` and so into `missing_declaration`/`missing-name` (`refusal.rs:3757`, `3851`). The review also compared the PR with FR-035-AC-3, FR-104-AC-17 (qualified edges, no fallback), FR-105's `state_clause` content (no clause name, FR-105-AC-14 dedup), FR-569 "Name resolution and collisions" and the enum, union and import qualified resolution. `quire validate` passes on the changed files.

## Examined scope

- Grammar and code facts (examined). They are all confirmed. `by` is reserved (`token.rs:172`). A bare `by` fails `Terminal::Identifier` and so is no primary.
- The `<op>` spelling rule (examined). It is sound against FR-104:131-137 (`T::op` is source-declared and `M::T::op` is a model operation, with no fallback between them) and against FR-104-AC-17's no-fallback rule for qualified edges. FR-035 limits reserved-word parameters to model-declared operations, which is correct: a source operation's parameter is a binder position (FR-569:191) and cannot be reserved.
- Enum and variant collision (examined). `M::Meter::adjust::by` has four segments. Enum and union member paths are `M::E::case` (three segments, `enum_members_named` keys `M::E`), so the model form cannot collide. The source form `T::op::p` cannot collide either, because FR-569 refuses a source type that shares its name with a model or import alias. The precedence is not stated (FND-003).
- `pre()`/forbidden-pre-read (examined). The rule is stated ("same … observation rules") and covered by AC-18.
- Node identity (examined). The `state_clause` content is the kind, the parameter aggregate, the anchor and the condition, with no clause name. So "S1 and S2 lower to one node id" can fail only if the parameter node is derived from the spelling, which matches the FSpec principle that identity covers what a node reads.
- Invariants (examined). They refuse, and AC-18 covers them. Constraints are stated in the bullets but have no AC (FND-002).
- FR-035 and FR-035-AC-3 (examined). They are consistent: `M::T::result` already parses in a qualified position.
- Ids (examined). FR-104-AC-18 is free on main, where the table ends at AC-17. TC-3083 is free on main. None of the 21 other open PRs adds either id.

## Verdict

Not mergeable as is. Two medium findings need small edits. FND-001: "the same … as the bare name `p`" does not hold under a shadowing binder. FND-002: three SHALL branches have no AC, and one of them, the prefix spelled through another type, is the branch an effective-view implementation would get wrong while AC-18 still passed. The grammar facts, the refusal causes, the pre() rule, the node-identity claim and the FR-035 note are correct. Merge after FSpec #752.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | The bullet resolves `<op>::p` to "the same parameter reference … as the bare name `p`". With a same-named `let` or quantifier binder in scope (an ordinary parameter such as `step`), the bare name denotes the binder, because resolution is innermost-first (check.rs:3327). The qualified name's meaning is then undefined. State that it always denotes the parameter, whatever lexical binder is in scope | spec/functional/FR-104-check-state-clauses.md:202-214 |
| FND-002 | medium | AC-18 covers no case for three SHALL branches: the source-declared `T::op::p` form, a prefix that spells the same operation through another type (an inherited operation named through its declaring ancestor while `on` names the subtype), and the refusal in a `constraint`. An implementation that resolved the prefix through FR-081's effective view would accept the ancestor spelling and still pass AC-18 | spec/functional/FR-104-check-state-clauses.md:459 |
| FND-003 | low | The bullets do not place the new resolution relative to the existing qualified resolution of enum, union and import names (resolve_target, FR-317 `AmbiguousQualified`). The forms are disjoint today (segment count, and FR-569's alias-collision refusal), but the spec does not say so, so a later rule could open an unstated precedence question | spec/functional/FR-104-check-state-clauses.md:215-221 |
| FND-004 | low | The new bullets cite no FSpec FR-012, the upstream rule they implement (FR-012-AC-9 in FSpec #752), unlike the neighbouring bullets, which cite FSpec | spec/functional/FR-104-check-state-clauses.md:202 |
| FND-005 | low | TC-3083 step 6 names the corpus case `decimal-keyword-parameter.native`, which exists only on the unmerged branch `codex/qsl-2041-parameters` (path `tests/corpus/operations/parameters/pending/`), and it gives no path. Name the path, or make the step conditional on that corpus landing | spec/test-cases/TC-3083-a-reserved-word-operation-parameter-is-read-by-its-qualified-name.md:35-38 |
