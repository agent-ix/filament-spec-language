---
id: SR-17952695000002
title: gap-analysis — QSL-2742 source identity reachability
type: SpecReview
analysis: gap-analysis
scope: agent-ix/filament-spec-language; QSL-2742; eight changed files and FR-081/FR-710
  source bindings
review_set: subset
relationships:
- target: ix://agent-ix/filament-spec-language/FR-081
  type: references
---

## Summary

Ticket: QSL-2742. Reviewed frozen PR #3309 head 7aa2425bf8f888866f7284a63fd3b000671fcb68 against 7bb12494287a23e2983267eaa5ecf80a79a82c49, in the sole ticket worktree. Original keys and the inventory are asserted against authored declarations; equal-shaped fields remain distinct; inherited members change owner identity. Hidden Base/Mid entries remain readable while Leaf is visible. Sibling conflicts name Join, Base/field and both redefiners and return no selected view. Diamond checks two direct parents, exact expanded paths and one effective member. Replay/permutation compares the ordered correspondence and type identities. Native-model-unit compilation and same-origin generated Semantic IR produce identical package bytes, both run true/0, changed data runs false/10, and sibling conflict refuses 20 at I1 with derivation-conflict. No new production API, identity writer, wire format, pin, gate weakening, vendoring or stub was introduced. No applicable AssuranceProfile exists.

## Verdict

**FAIL** for repository assurance; scoped source reachability is acceptable. The computed repository matrix retains baseline gaps. This is not a repository-wide reverse-gap/stub PASS; broader inventory was explicitly excluded by the dispatch.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | Baseline repository trace gap: FR-081-CON-1 has no bound test in the computed matrix, although existing AC-4 display-name behavior is tested. This predates the PR and does not invalidate the seven new source reachability tests. | spec/functional/FR-081-preserve-model-correspondence-and-declaration-identity.md:281 |

## Coverage

This is source reachability evidence over the existing #3277 implementation, not global FR-081 qualification. AC-5 adds same-process source replay only; existing model_identity_contracts retains separate-process library evidence. AC-9 adds the diamond case only; chain/hidden full expansion and malformed-link refusal remain existing library coverage. AC-4, AC-8, AC-10 onward, limits, full-size/resource qualification and unrelated model forms were not requalified. Partial test bindings support their stated subcase; they do not discharge every clause of a broad AC. The external FSpec-FR-150-AC-6 tag is intentional under AGENTS.md and its normative target exists in FSpec.

Supplied focused logs were inspected: /tmp/qsl-2742-semantics.log records 6 passed, 0 failed/ignored, 807 filtered; /tmp/qsl-2742-command.log records the exact native-request test passed, 154 filtered. Supplied rustfmt and Vale checks exit 0; Vale reports no errors/warnings. Reviewer git diff --check passed. Supplied retained CLI JSON records category success, clause Correspondence truth true, with empty stderr; the lead reports exit 0 from this worktree using retained driver da01aa94cd4f65a8e9661f6af6d514556af4825e. Reviewer ran no build/test reruns, clippy, deny or aggregate gates, per dispatch.

Reconciliation: quoin matrix (quoin 0.36.7, quire 0.52.0, engine 0.62.2), static quire matrix and coverage also inspected. All seven changed functions are bound. Raw engine totals: tagged 4806; untagged 1129; ignored-only 82; method-without-symbol 104; retired 13. All 6134 evidence rows report no run evidence; supplied focused logs are separate evidence. These raw totals include explicitly retired FR-081-AC-11 misclassified as untagged, so they are not a count of live deficits. FR-081-AC-11 requires no hash work. The existing model_identity_cost FSpec-FR-081-AC-11 namespace is baseline, not a new defect.

Scope inventory: two public source entry paths and their normalization closure inspected, seven new test functions and all five example files; zero new unowned production behaviors, production stubs or test stubs in the diff. Whole-repository reverse-gap/stub inventory not performed under the bounded dispatch; no global assurance PASS. Matrix/coverage emit symbol collisions and ancillary unmarked-trace warnings; actual binders for this diff exist. The FSpec external tag is locally untracked because its target belongs to the other repository.

Plan completion: not assessed

Semantic review: ran over the bounded FR-081 source cases, FR-710 request bridge and FSpec diamond criterion, explicitly requested by dispatch. Spec-review/subanalyses skipped: no spec or plan edits. React/Python lanes skipped: no such changes. Rust lane folded into code-review.

## Examined units and trace bindings

```yaml
clean: false
scope:
- id: FR-081-AC-1
  path: spec/functional/FR-081-preserve-model-correspondence-and-declaration-identity.md
  role: examined
  excerpt: Given a domain package admitted by FR-056, every original declaration key
    the binder's correspondence names is byte-identical to the key FR-056 admitted
    for it; no key in the correspondence is absent from, or added beyond, the admitted
    set.
- id: FR-081-AC-2
  path: spec/functional/FR-081-preserve-model-correspondence-and-declaration-identity.md
  role: examined
  excerpt: Two effective declarations derived from distinct original declaration keys
    are never equal, even when their declared shape (fields, types, multiplicities)
    is identical; each effective identity differs because its preimage's `original`
    component differs.
- id: FR-081-AC-3
  path: spec/functional/FR-081-preserve-model-correspondence-and-declaration-identity.md
  role: examined
  excerpt: Given a three-level chain where a base type declares a member and two of
    its descendants each independently redefine it, so that both redefinitions reach
    the most specific type's effective member, the more specific (descendant) redefiner's
    entry is the one a name resolves to, and the less specific redefiner's entry remains
    present in the correspondence with its own original key, derivation facts and
    effective identity readable rather than being deleted.
- id: FR-081-AC-5
  path: spec/functional/FR-081-preserve-model-correspondence-and-declaration-identity.md
  role: examined
  excerpt: Given the same `DomainPackage` value and the same limits, two independent
    binder invocations — including invocations in two separate process runs — produce
    byte-identical original keys, effective identities and correspondence ordering,
    with no observable interaction between the runs.
- id: FR-081-AC-6
  path: spec/functional/FR-081-preserve-model-correspondence-and-declaration-identity.md
  role: examined
  excerpt: Given two declared redefinitions of one member reaching the same effective
    type where neither redefining owner is a proper descendant of the other, normalization
    refuses derivation-conflict naming both redefiners and their owning types, and
    the correspondence exposes no effective member for either redefinition — not the
    winning one FR-081-AC-3's descendant case would pick, because no descendant exists
    here.
- id: FR-081-AC-7
  path: spec/functional/FR-081-preserve-model-correspondence-and-declaration-identity.md
  role: examined
  excerpt: Given the same `DomainPackage` value with its IR nodes presented in two
    different orders, normalization from each produces byte-identical original keys,
    effective identities and correspondence ordering.
- id: FR-081-AC-9
  path: spec/functional/FR-081-preserve-model-correspondence-and-declaration-identity.md
  role: examined
  excerpt: Expanding the retained facts of every effective declaration through its
    parent identities yields the path-form facts of an independent enumeration over
    the original declarations, in phase, input and rule order with multiplicity, for
    a chain, a diamond and a hidden redefinition; a link naming a declaration of the
    wrong kind, owner or original is refused.
- id: FR-081-CON-1
  path: spec/functional/FR-081-preserve-model-correspondence-and-declaration-identity.md
  role: examined
  excerpt: The model binder SHALL NOT read `title`, `displayName` or any other presentation
    field of an admitted declaration for any identity, key, ordering or resolution
    purpose.
- id: FR-710-AC-3
  path: spec/functional/FR-710-author-a-domain-package-in-an-fsl-model-unit.md
  role: examined
  excerpt: One model unit holding one declaration of every form in the grammar (object
    with relationship, contract clauses and invariant; alias, record, tuple, enum,
    union, option field and map; event; machine with states and a transition; process
    with a step; store; namespace; open and closed populations; dimension and unit;
    interface, part, port, connection and allocation) compiles to a document that
    FR-154 admits, with one node of each FR-208 meaning in the compilation table.
- id: FR-710-AC-7
  path: spec/functional/FR-710-author-a-domain-package-in-an-fsl-model-unit.md
  role: examined
  excerpt: 'A `native-compile/1` request over a program with `model M = "example/shop";`
    and `invariant I using P on M::Order at current { self.total >= 0 }` whose `models`
    holds `{"format": "native-model-unit/1", "source": {"file": "shop.fsl", ...}}`
    naming the AC-1 model unit compiles with exit 0 to package bytes equal to the
    same request whose `models` entry is the compiled document in format `semantic-ir/2.0.0`,
    and a `native-run/1` request over the same members runs equally; the entry naming
    a model unit whose `relationship next: Ghost [0, 1];` is unresolved refuses at
    stage `I1` with `missing_declar'
- id: FSpec-FR-150-AC-6
  path: filament-specification/spec/functional/type-model/FR-150-preserve-model-normalization-provenance.md
  role: examined
  excerpt: Diamond paths reaching one original declaration collapse to one effective
    member with one inherit fact per direct parent, whose expansion lists every path,
- id: AD-006
  path: filament-specification/spec/assurance/AD-006-model-graph-state-contracts.md
  role: context_only
  excerpt: An original declaration is keyed by (domain package identity, IR node identity);
    it carries no digest.
- id: examples/model-identity/README.md
  path: examples/model-identity/README.md
  role: examined
  excerpt: |-
    ---
    type: tutorial
    description: Check inherited and redefined fields from an authored domain model.
    ---
    # Inherited and redefined fields

    Check a `Leaf` object whose fields come from a three-level type hierarchy.
    The example uses an installed `fsl` command and runs from the repository root.

    In [model.fsl](model.fsl), `Base` declares `inherited` and `field`.
    `Mid` and `Leaf` each redefine `Base::field`.
    The effective field selected on `Leaf` is its own redefinition.
    Its inherited field keeps the declaration originally authored on `Base`.

    An original identity names a declar
- id: examples/model-identity/model.fsl
  path: examples/model-identity/model.fsl
  role: examined
  excerpt: |
    language "ix:native" edition "1-draft";
    domain "example/identity";
    object Base {
      inherited: Integer;
      field: Integer;
    }
    object Mid extends Base {
      field: Integer redefines Base::field;
    }
    object Leaf extends Mid {
      field: Integer redefines Base::field;
    }
    population leaves: Leaf;
- id: examples/model-identity/program.fsl
  path: examples/model-identity/program.fsl
  role: examined
  excerpt: |
    language "ix:native" edition "1-draft";
    profile P = "filament.model.complete/v1";
    model M = "example/identity";
    invariant Correspondence using P on M::Leaf at current {
      self.inherited = 3 and self.field = 7
    }
- id: examples/model-identity/current.json
  path: examples/model-identity/current.json
  role: examined
  excerpt: |
    {
      "format": "filament.state.snapshot/v1",
      "identity": {"authority": "agent-ix", "identity": "example:identity:current"},
      "observation": "current",
      "anchor": {"kind": "handler", "name": "validate"},
      "model": {"identity": "example/identity"},
      "populations": [{
        "population": "ix://example/identity/leaves",
        "complete": true,
        "objects": [{
          "key": "leaf-1",
          "type": "ix://example/identity/Leaf",
          "fields": {"inherited": {"integer": "3"}, "field": {"integer": "7"}}
        }]
      }]
    }
- id: examples/model-identity/run.json
  path: examples/model-identity/run.json
  role: examined
  excerpt: |-
    {
      "format": "native-run/1",
      "request": {
        "models": [{"format": "native-model-unit/1", "source": {
          "file": "model.fsl", "authority": "agent-ix", "identity": "test:model.fsl",
          "document": "ModelUnit", "formal_revision": 1
        }}],
        "program": {"source": {
          "file": "program.fsl", "authority": "agent-ix", "identity": "test:program.fsl",
          "document": "ModelUnit", "formal_revision": 1
        }},
        "clause": {
          "selection": {"clause": {
            "name": "Correspondence", "observation": "current",
            "self": {"population": "ix://example/iden
- id: fsl-command/tests/it/model_unit_request.rs
  path: fsl-command/tests/it/model_unit_request.rs
  role: examined
  excerpt: fn original_effective_identity_reaches_native_requests() {
- id: fsl-semantics/tests/it/model_unit_identity.rs
  path: fsl-semantics/tests/it/model_unit_identity.rs
  role: examined
  excerpt: "//! Original/effective correspondence reached from authored model units."
- id: fsl-semantics/tests/it/main.rs
  path: fsl-semantics/tests/it/main.rs
  role: examined
  excerpt: mod model_unit_identity;
findings:
- fnd: FND-001
  severity: high
  confidence: high
  method: gap-analysis
  check: coverage
  artifact_id: FR-081-CON-1
  related_ids:
  - FR-081-AC-4
  path: spec/functional/FR-081-preserve-model-correspondence-and-declaration-identity.md
  lines: '281'
  excerpt: The model binder SHALL NOT read `title`, `displayName` or any other presentation
    field of an admitted declaration for any identity, key, ordering or resolution
    purpose.
  finding: 'Baseline repository trace gap: FR-081-CON-1 has no bound test in the computed
    matrix, although existing AC-4 display-name behavior is tested. This predates
    the PR and does not invalidate the seven new source reachability tests.'
bindings:
- test_id: inherited_source_member_keeps_original_and_changes_effective_owner
  ac_id: FR-081-AC-1
  trace: correct
- test_id: inherited_source_member_keeps_original_and_changes_effective_owner
  ac_id: FR-081-AC-2
  trace: correct
- test_id: inherited_source_member_keeps_original_and_changes_effective_owner
  ac_id: FR-710-AC-3
  trace: correct
- test_id: equally_shaped_source_fields_have_distinct_original_and_effective_keys
  ac_id: FR-081-AC-2
  trace: correct
- test_id: most_specific_source_redefinition_retains_hidden_correspondence
  ac_id: FR-081-AC-3
  trace: correct
- test_id: most_specific_source_redefinition_retains_hidden_correspondence
  ac_id: FR-710-AC-3
  trace: correct
- test_id: sibling_source_redefinitions_refuse_without_a_selected_view
  ac_id: FR-081-AC-6
  trace: correct
- test_id: sibling_source_redefinitions_refuse_without_a_selected_view
  ac_id: FR-710-AC-3
  trace: correct
- test_id: source_diamond_retains_two_direct_facts_and_both_expanded_paths
  ac_id: FR-081-AC-9
  trace: correct
- test_id: source_diamond_retains_two_direct_facts_and_both_expanded_paths
  ac_id: FSpec-FR-150-AC-6
  trace: correct
- test_id: source_diamond_retains_two_direct_facts_and_both_expanded_paths
  ac_id: FR-710-AC-3
  trace: correct
- test_id: source_compilation_and_declaration_permutation_preserve_correspondence_order
  ac_id: FR-081-AC-5
  trace: correct
- test_id: source_compilation_and_declaration_permutation_preserve_correspondence_order
  ac_id: FR-081-AC-7
  trace: correct
- test_id: original_effective_identity_reaches_native_requests
  ac_id: FR-081-AC-1
  trace: correct
- test_id: original_effective_identity_reaches_native_requests
  ac_id: FR-081-AC-3
  trace: correct
- test_id: original_effective_identity_reaches_native_requests
  ac_id: FR-081-AC-6
  trace: correct
- test_id: original_effective_identity_reaches_native_requests
  ac_id: FR-710-AC-7
  trace: correct
```

## Dispositions

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | accepted-no-change | The lead explicitly accepts this unchanged baseline trace gap for the scoped source reachability PR. FR-081-CON-1 and the existing namespace/matrix backlog predate the diff; AC-4 already has a display-name behavioral test. The new seven binders remain valid. Repository assurance stays FAIL and cleanup is outside QSL-2742; this disposition makes no global FR-081 assurance claim. |
