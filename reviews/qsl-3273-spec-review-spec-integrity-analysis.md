---
id: SR-21023
title: "spec-integrity-analysis of QSL-3273 refinement and abstraction-relation identity (filament-spec-language PR 3883)"
type: SpecReview
analysis: integrity
scope: "agent-ix/filament-spec-language@700527b7c5aa06ca693562ec0d21573f787bcd9e; spec/decisions/ADR-013, ADR-015, ADR-017, ADR-020; spec/functional/FR-135, FR-137, FR-147, FR-306; spec/test-cases/TC-540, TC-542, TC-552, TC-804; spec/usecase/US-031"
review_set: subset
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-147
    type: references
  - target: ix://agent-ix/filament-spec-language/FR-306
    type: references
  - target: ix://agent-ix/filament-spec-language/FR-135
    type: references
---

# Spec integrity analysis of QSL-3273 refinement and abstraction-relation identity (filament-spec-language PR 3883)

## Summary

Ticket: QSL-3273. Head 700527b7c5aa06ca693562ec0d21573f787bcd9e, diff `git diff origin/main...HEAD` (13 files). FSpec read at filament-specification origin/main 691ac5a6.

The lead's ruling holds against FSpec. FSpec FR-375 "Checked-package node" fixes the refinement node id as `{package, path ["relation","refinement",name], ordinal 0}`. Its "Refinement identity" paragraph says a changed meaning gives a structurally unequal node whose id changes only on a rename. FSpec FR-322 "Structural node identity" says body content is not part of the path, and its package rule says no identifier, hash, digest or seal of a package exists. FSL ADR-013 O-02 defines the package identity as identity and version. The code agrees: `v2_identity` keys a refinement by `declared_node_key(package, [name], 0)` (fsl-semantics/src/check/refinement.rs:869-881). The path differs from FSpec's, which FR-135 records as QSL-3277. The abstraction relation is keyed by `derived_node_key(package, correspondence, "abstraction_relation", target operands, 0)` (fsl-semantics/src/check/abstraction.rs:1269-1281), so a `RustField` change keeps its node id and changes its body only.

Examined and clean: FR-135 "Package and identity" and AC-4; FR-137 bullet and AC-1 (FSpec FR-375 `ensure` entry `{operation, kind, granularity}`); FR-147 Behavior bullet, AC-9 (matches admission_corpus.rs:1850 `abstract_internal` absent or `["Local::Done"]`); ADR-015 refinement internals; ADR-020 §1; ADR-013 O-04 Equality; FR-306 AC-2 and TC-804; US-031-EX-1. No remaining FSL text claims a refinement, fairness row or internal list changes a node id. FR-147:211 (an alias rename changes no node identity) still holds. The O-04 rewording does not weaken an invariant in use: I2 admits one `package_id` per owner in a check, the reader refuses duplicate ids, and the old sentence ("equal keys mean structurally identical nodes") was already false for declared nodes keyed by declaration path. No test reads an edited file by path: tests/it/standard_pin.rs (TC-1647) opens only requirement files named by consumer-fact URIs, and no edited file holds a `CF-` id. The `quire validate --scope .` run over the 13 files exited 0.

## Findings

| ID | Severity | Summary | Refs | Escape Cause |
| --- | --- | --- | --- | --- |
| FND-001 | high | FR-147-AC-15, as amended, requires two things that cannot both hold. It requires equal refinement node keys over the local and the dependency `Twice` in a package of one identity. It also keeps the requirement that the dependency form's record has an obligation identity that "names the dependency's `package_id`, and differs from" the local form's. `ObligationIdentity` is `{node, model, ordinal}` (fsl-package/src/obligation_identity.rs:37-41; ADR-013 O-09). With node keys equal, ordinal 0 and the same model identities in both forms, the two identities are equal, and no member can name `test/protocol-dependency`. TC-552 step 15's expected result states both sides in one bullet. The unchanged FR-147 "Requirement record" bullet ("gives different obligation identities") has the same defect. Fix in this PR: state that the identities are equal and that the two records differ in the refinement node body's `protocol` and in `AbstractSide::Protocol.dependency`. Alternatively, drop the obligation-identity comparison and cite STD-726. | spec/functional/FR-147-check-a-refinement-whose-abstract-side-is-a-protocol.md:339; spec/functional/FR-147-check-a-refinement-whose-abstract-side-is-a-protocol.md:287-292; spec/test-cases/TC-552-s3-checks-a-refinement-whose-abstract-side-is-a-protocol.md:141-147 | wrong-requirement |
| FND-002 | medium | FR-306 "Rebinding is a new package value" now says the package identity (O-02, the `package_id`) does not depend on a binding. It still gives "every generated contract carries the `package_id` it was generated from" as the reason a rebinding does not reinterpret an earlier contract (FSpec FR-353-AC-5). Contracts from both revisions carry an equal `package_id`, so the cited mechanism cannot tell them apart, and O-25 (witnesses) defines no contract `package_id`. FR-353-AC-5 can still hold through FSpec FR-322 and FR-451 value comparison: two packages are the same only when their values are equal. The term is ambiguous in the code as well: fsl-replay's `PackageId` is the SHA-256 of the emitted bytes (call_replay_context.rs:25-32, compile.rs:49-52), and that one does change on a rebinding. Fix: state that a revision is told apart by checked-package value comparison (FSpec FR-322, FR-451 `node_id`), and drop or re-point the `package_id` sentence here and in ADR-017 AR-4 "Relation revision". | spec/functional/FR-306-carry-the-abstraction-relation-in-the-checked-package.md:55-62; spec/decisions/ADR-017-protocol-refinement-abstraction-boundaries.md:687-692 | wrong-requirement |

## Verdict

Not mergeable. FND-001 (high) blocks: the PR edits FR-147-AC-15 and leaves that AC requiring an obligation identity that the identity tuple cannot produce. FND-002 should be fixed in the same round. The ruling itself, the refinement amendments, the extension to the abstraction relation and the O-04 rewording are correct against FSpec and the code.
