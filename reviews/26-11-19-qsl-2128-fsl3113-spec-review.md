---
id: SR-20261119220819000001
title: "spec-review of QSL-2128 package form of a hyper clause"
type: SpecReview
analysis: integrity
scope: "agent-ix/filament-spec-language; PR #3113 at 980e2d61; spec/functional/FR-940 (Package form of a hyper clause, AC-16..19), FR-517 (node counts, AC-25), FR-488 (match conjunct parts), FR-163 (checker rebuild); spec/test-cases/TC-3126, TC-3127, TC-3128, TC-2958; context: fsl-command checked_handoff/selection.rs derive/check/form_counts, fsl-semantics check/lowering/hyper.rs and hybrid.rs, check/hyper_clause.rs, fsl-package emit.rs temporal_bindings and claim_names, checked_v2/hyper_root.rs, filament-contract-model dd390f5 v2/temporal.rs check_hyper_shape and v2/temporal_binding.rs, FR-087 R-10, FR-174, FR-532, code PR #3093 hyper_wire.rs"
review_set: subset
---

## Summary

Spec review of FSL PR #3113. Ticket: QSL-2128. The PR rules how a hyper clause is analyzed from package bytes on both routes (FR-940 "Package form of a hyper clause", AC-16..19). It fixes the handoff sidecar's node counts (FR-517 AC-25), states how a package reader recovers the conjunct parts (FR-488), and has FR-163's checker rebuild the clause from the route's view. TC-3126/3127/3128 are new; TC-2958 gains FR-940-AC-18.

## Examined scope

- Diagnosis of the `package.nodes` failure (examined). True. `Selection::derive` (selection.rs:339, 425-429) counts `graph.temporal_clauses()` only. `form_counts` counts every `TemporalClause` node in the package. The hyper lowering mints the body wrapper as a `temporal`/`temporal_clause` node (lowering/hyper.rs:356-376). So the counts differ for any unit with a hyper clause.
- What the package carries (examined). Slot nodes are `value`/`parameter` nodes named `slot_s<p>_s<t>` at their level, and the wrapper's `over` is slot 0 (hyper.rs:291-321). The quantifier Groups carry `variable`, `quantifier`, `alias` and fairness references (hyper.rs:447-488). `match` is written in span order (hyper.rs:491-540). `claim_names` rows sit on the `hyperproperty` node (emit.rs:8531-8537). The rows of the hyper form table match the emitter.
- μ_U/μ_E recovery (examined). Sound. S3 files a conjunct under μ_E when a step-label read names an existential trace (hyper_clause.rs:2283-2294, 2490-2511). Each such read gets a slot node; a read with no slot name is a stage fault in the emitter. Rank within a part equals S3's index.
- R-10 (examined). `CheckedHyperClause::of_package_form` follows the precedent of `CheckedTemporalClause::of_package_form` (temporal_clause.rs:2588) and `CheckedStateGraphClaim::of_package_form` (state_graph_claim.rs:861). R-10 forbids only `CheckedGraph`/`CheckedPackage` built from bytes. No source fallback is allowed on the package route.
- FR-940-AC-16 / TC-3126 (examined). It can fail through its μ_U negative control and through equality with S3.
- FR-940-AC-17 / TC-3127 (examined). It can fail because records and certificates are compared across routes and a refuted variant is included.
- FR-940-AC-18 / TC-2958 (examined). It can fail because any source fallback fails once source is destroyed. It reverses PR #3093's `a_package_hyper_item_refuses_without_falling_back_to_source` (hyper_wire.rs:89).
- FR-940-AC-19 (examined, FND-001, FND-002).
- FR-517-AC-25 / TC-3128 (examined, FND-004, FND-005).
- Copy group row (examined, FND-003).
- Source-route change and package identity (examined, FND-006). S3/S4 emission is untouched, so package bytes and identity do not change. Source-route atom and automaton numbering does change. No hyper golden certificate file exists under fsl-analyze/tests.
- Ids (examined). TC-3126..3128, FR-517-AC-25 and FR-940-AC-16..19 are free on main and in all 26 open FSL PRs. No other open PR touches FR-940, FR-517, FR-488, FR-163 or TC-2958. The branch is up to date with main.
- `quire validate` (examined). It exits 0 on all eight changed files.

## Verdict

Not mergeable. The diagnosis and the core contract are right. FR-940-AC-19 cannot pass as written, because the strict reader refuses one of its "admitted" fixtures. The MalformedNode slot-level rule would also refuse every clause that has a `match` block. Two medium gaps in the copy-group key and in the FR-517 count rules need text fixes.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | FR-940-AC-19 lists a `body` binding that references an `expression` node as a fixture "admitted by the strict reader" that settles `failed`/`MalformedNode`. The pinned reader refuses it. `check_hyper_shape` step 3 (filament-contract-model dd390f5 v2/temporal.rs ~2596) refuses a `behaviours`-domain hyper node whose body is not a `temporal_clause` (`ineligible`). That case is a request-stage refusal, so the AC cannot pass, and the MalformedNode body case in "Refusals" cannot be reached | spec/functional/FR-940-analyze-through-one-sealed-package-read-view.md:551 |
| FND-002 | medium | The MalformedNode rule "two slot nodes of one clause at one level" misfires on valid packages. Each `match` conjunct has its own slot scope starting at level 0 (`op_s`/`rcv_s`/`arg_s` parameter nodes, lowering/hyper.rs:516-531), next to the atom slot at level 0. A clause with a `match` block, such as AC-16's projection-aligned clause, would be refused. The rule must be scoped to atom slots (`slot_s` names) and per conjunct. "Name not in the reserved form" must also admit the conjunct slot forms | spec/functional/FR-940-analyze-through-one-sealed-package-read-view.md:519-524 |
| FND-003 | medium | The copy group row sorts operands "by the FR-322 node key of each operand's term". Copy-swap compares the normal form of σ(φ) with φ. σ(φ)'s operands, and the flattened or normalized subterms, are generally not package nodes, because σ renames slots, which are parameter nodes keyed by name and level. The spec does not say how a key is computed for a term the package does not hold, or whether it is taken before or after normalization. The AC-16 oracle (copy generators equal to S3's) depends on this | spec/functional/FR-940-analyze-through-one-sealed-package-read-view.md:450,474-476 |
| FND-004 | medium | The per-quantifier `temporal_bindings` count for clock-configured hyper clauses has no AC: AC-25's `Equal` is an unclocked `always` clause. Main's emitter (fsl-package emit.rs:8825-8885) writes one row per clause, keyed on the wrapper with `trace` None. That contradicts FR-488 and is refused by the reader (temporal_binding.rs:628-635, `is_wrapper`). The new count therefore rests on emitter behaviour that does not exist, and neither the PR nor the ruling states or tickets this | spec/functional/FR-517-write-a-checked-package-handoff-directory-from-source-in-process.md:125-128 |
| FND-005 | medium | The FR-517 count formula names only checked temporal clauses and hyper body wrappers. Hybrid claims also mint `temporal`/`temporal_clause` nodes (lowering/hybrid.rs:380-400), and `hybrid_claims()` is separate from `temporal_clauses()`. The paragraph's first sentence ("whichever declaration mints it") and its formula disagree, and an implementer coding the formula leaves the same `package.nodes` refusal in place for hybrid units | spec/functional/FR-517-write-a-checked-package-handoff-directory-from-source-in-process.md:117-124 |
| FND-006 | low | The PR moves the source route onto the rebuilt form, which renumbers atoms in pre-order and re-sorts the normal form by node key. It does not state the consequences: product-state keys, counterexample letter indices and source-route certificate bytes change, while the emitted package bytes and identity do not, because S3/S4 are untouched. FR-163 AC-1/AC-9 vectors (TC-645/646) may need regenerating, and no ticket or sentence covers this | spec/functional/FR-940-analyze-through-one-sealed-package-read-view.md:394-427 |
| FND-007 | low | TC-2958's Test line names `fsl-analyze/tests/it/analyze_package_selection.rs`. The AC-18 test it now scopes in is the reversed `a_package_hyper_item_refuses_without_falling_back_to_source` in `hyper_wire.rs` (PR #3093). The TC should name both files, or the test should move | spec/test-cases/TC-2958-source-analysis-keeps-its-checked-graph-and-the-package-route-reaches-every-evaluator.md:33 |

## New findings (disposition pass 1)

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-008 | high | The package normal form's opaque set contradicts S3. FR-940 names "a call, a select, an index" as opaque nodes that make a pair non-swappable "as on the source route". S3's `is_opaque` (fsl-semantics check/hyper_clause.rs:2859-2913) does not treat `Call`, `Select`, `Index`, `If` or `Field` as opaque: it encodes them by tag (`call` + name, `select`, `index`) and normalizes through them. Its opaque set is `Case`, `Let`, `Record`, `Collection`, `Convert`, `Query`, `Flatten`, `Accumulate`, `Count`, `Sum`, `Population`, `Seq`, `AllInstances`, `Lookup`, `Dispatch`, `Quantity`, `ReferenceLiteral`, `Power`, `Reaches` and `ProtocolReference`. A clause whose atom reads a slot under a call, select or index is swappable in S3 and not swappable by the package rule. The rebuilt copy group then differs from S3's, contradicting the "copy group is equal to S3's" requirement and the source-route "every verdict and record unchanged" list (basis `reduced` vs whole product, certificate). The rule needs S3's set, mapped to package node forms | spec/functional/FR-940-analyze-through-one-sealed-package-read-view.md:487-489 |

## Dispositions

| FND | Outcome | Reason |
| --- | --- | --- |
| FND-001 | fixed | AC-19 moves the body-not-temporal_clause case to a request-stage refusal (`ill_typed`/`operator-ineligible` at the `hyperproperty` node, matching `ineligible` in check_hyper_shape, contract-ir temporal.rs:2599, unchanged dd390f5 to e576f74). The four MalformedNode variants are slot name/level/root cases that the pinned reader does not inspect. |
| FND-002 | fixed | Atom slots form their own level scope, and each conjunct has its own scope from level 0. The MalformedNode list is split into atom-slot and conjunct-slot cases. AC-19 adds the unmodified projection-aligned clause as the control. |
| FND-003 | fixed | The "package normal form" section defines the key as a computed encoding: slots by decoded (p, t) with sigma applied, no level, and local binders by level, never a node-key lookup. The flatten/sort matches S3's `flatten`/`binary_tag`, and the `Equal` worked example and generator `{0,1,[0]}` match `copy_group`/`atom_exchange`. The opaque set is wrong (new FND-008). |
| FND-004 | fixed | FR-517 states the emitter/reader disagreement. FR-517-AC-26 (clocked `Ordered`, two rows `a` then `b` on the `hyperproperty` node, wrapper-row negative) is added, and QSL-2631 exists with matching done-when. |
| FND-005 | fixed | Three minters are listed (temporal clauses, hybrid claims FR-447, hyper wrappers), and AC-25 adds a hybrid case with count 3. emit.rs writes no hybrid `temporal_bindings` row, so the binding count is unaffected. |
| FND-006 | fixed | The new "What changes on the source route" section lists package bytes, keys and identity as unchanged, and atom indices, automaton numbering, product-state keys and certificate bytes as changed. Its "every verdict and record unchanged" claim is broken by FND-008 until that is fixed. |
| FND-007 | fixed | TC-2958 now names `analyze_package_selection.rs` (steps 1-3) and `hyper_wire.rs` (step 4). |
