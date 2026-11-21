---
id: SR-202611210022442313144108553
title: QSL-2734 cohort spec-review/ears-conformance
type: SpecReview
analysis: ears-conformance
scope: agent-ix/filament-spec-language; QSL-2734/QSL-2743; Cargo.lock, Cargo.toml,
  fsl-command/Cargo.toml, fsl-eval/src/value/expression/evaluate.rs, fsl-eval/tests/it/quantity_class_convert.rs,
  fsl-eval/tests/it/variant_fields.rs, fsl-package/src/bare.rs, fsl-package/src/bare_units.rs,
  fsl-package/src/bare_world.rs, fsl-package/src/emit.rs, fsl-package/src/emit/tests/variant_types.rs,
  fsl-package/src/reader_protocol_projection.rs, fsl-semantics/src/check/assemble/equivalence.rs,
  spec/functional/FR-093-lower-checked-value-expressions-to-fr-322-terms.md, spec/functional/FR-531-evaluate-from-a-checked-package-read-from-its-bytes-alone.md,
  spec/functional/FR-533-evaluate-a-protocol-slot-from-the-checked-package-s-own-nodes.md,
  spec/test-cases/TC-1333-evaluate-a-protocol-slot-from-the-checked-package-s-own-nodes.md
review_set: subset
relationships:
- target: ix://agent-ix/filament-spec-language/FR-533
  type: references
---

## Summary

Quire validation of all four changed artifacts reports 4/4 grammar-clean and zero grammar findings. Semantically the SHALL subjects, local/package operand conditions, exact result obligations and denied-charge response are concrete. TC procedure prose is not treated as a requirement-grammar violation.

## Verdict

PASS for the frozen scoped change.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No findings (placeholder) | - |

## Coverage

Frozen code head: d1aac7fb0fd1afb51baa5e88ee7301a71b90e891. Ticket: QSL-2734; related partial delivery QSL-2743; PR 3371. Same sole worktree, reviewer-only. No Cargo, source edit, worktree, relay or tracker request. No applicable AssuranceProfile exists under spec/. Direct subset review, no enforced workflow requested.

Fresh GitHub base deccdc2a50c1632c8c7dbda448995e162ce7f76c was inspected: PR 3343 binder/root-budget changes do not overlap production source in this patch; its separate FR-531 capability-inventory line must survive integration. Frozen tests do not claim qualification of a later integrated source tree. Observation a583c512 remains explicitly authorized even though upstream main later advanced; do not chase that head here.

Execution evidence: /private/tmp/qsl2734cohort-eval1.log: 13 passed, 0 failed, 0 ignored; /private/tmp/qsl2734cohort-package1.log: 5 passed, 0 failed, 0 ignored; /private/tmp/qsl2734cohort-clippy1.log: selected clippy exit 0. These are supplied actual Titan frozen-head runs inspected by reviewer, not rerun locally. Existing touched-file rustfmt qualification retained; git diff --check independently passed. No aggregate, full-crate or workspace gates run.

QSL-2743 remains open for selected-model payload unions including Clock. Existing unsupported guard and full Clock/source-object identity requirements were not removed. Enum function equality delivery does not claim the still-missing protocol-slot enum fixture in FR-533's remaining verification inventory.

Reconciliation: quire matrix 0.52.0, engine 0.62.2, no run evidence read. quoin 0.36.7 matrix exited 1: CORE_REFUSED request exceeds accepted size; authorized Quire fallback completed exit 0. Counts: {"untagged"=>1163, "tagged"=>4883, "method-without-symbol"=>106, "retired"=>13, "tagged-by-ignored-test"=>77}. Matrix stderr includes existing symbol-id-collision and unmarked-trace-id diagnostics; retained in quire-matrix.stderr. Counts differ from earlier reported snapshots; no causal delta is asserted without an equal-base comparison.

Plan completion: not assessed

Scoped reverse inventory: four production behaviors (nominal definition retention, class graph reconstruction, native enum equality routing, emitted fact locality), four owned by FR-093/531/533; zero scoped unowned behaviors, source stubs or test stubs. Whole-repository reverse inventory incomplete. Optional independent semantic gap fanout skipped; intent/test/code agreement examined within authorized code review.

Method selection: code-review with Rust folded into same SR; gap-analysis; base, integrity, failure-domain, ears-conformance and evidence. Dependency skipped (no new relationship edges); scope-boundary/risk-complexity/object review skipped (no new system allocation, domain object or risk contract); criterion-strength unavailable/not selected (no Jev invocation).

Publication: PENDING Peter's Linear hold and root-owned single batch; no comments, labels or read-backs performed. Local comment YAML is parser-validated, not tracker read-back.

## Examined scope

```yaml
- id: FR-093-AC-46
  path: spec/functional/FR-093-lower-checked-value-expressions-to-fr-322-terms.md
  role: examined
  excerpt: 'Unequal roots (FSpec FR-142-AC-26). With FR-091-AC-44''s units, `dimension
    Beat;`, `unit beat : Beat = rational(1, 1);`, `unit bar : Beat = rational(4, 1)
    * beat;` and `dimension Beat ~ Time by 1 beat = rational(1, 4) s;` (root ratio
    1/4: a beat is a quarter of a second), each of these checks and lowers: `function
    u1 using v(d: Quantity<beat>): Boolean pure { convert<Quantity<s>>(d) = rational(1,
    4) s }`, true for `d` = `1 beat` and false for `d` = `2 beat`; `function u2 using
    v(d: Quantity<s>): Boolean pure { convert<Quantity<beat>>(d) = 12 beat }`, true
    for `d` = `3 s`; `function u3 using v(d'
- id: FR-360-AC-8
  path: spec/functional/FR-360-type-a-field-by-a-variant-type-through-intake-check-evaluation-and-replay.md
  role: examined
  excerpt: An operation parameter of type `Tier` is admitted at intake, its call with
    `Config::Tier::Gold` checks and runs, and a call with a `Config::Level` value
    refuses `ill_typed`/`type-mismatch` naming the parameter; an operation result
    of type `Tier` is admitted and its value compares by case. The checked package
    emits `Config::Tier` as a `model`/`variant_type` node, the case literal as an
    `enum_value` of it and the argument by `ref` with no conversion node, and reads
    back `Verified`, also for a type whose cases include `Gold Plus`, `2x` and `Café`.
    A parameter or result naming an interface-feature
- id: FR-531-AC-8
  path: spec/functional/FR-531-evaluate-from-a-checked-package-read-from-its-bytes-alone.md
  role: examined
  excerpt: The handoff directory of **R** holds required `identities.json` with `format`
    `filament.checked-handoff.identities/5`; `ReadIdentityRows::read` over it and
    the package gives a slot call that evaluates; the file equals, byte for byte,
    the encoding above of the in-process unit's rows, and two writes of equal requests
    give equal files; omitting the file from a `BareSupply` makes the slot call refuse
    `MissingIdentityWitness` while the temporal call is unaffected.
- id: FR-531-AC-34
  path: spec/functional/FR-531-evaluate-from-a-checked-package-read-from-its-bytes-alone.md
  role: context_only
  excerpt: 'Over the unit `record W { c: Clock; n: Integer; }`, `tuple P (Clock, Integer);`,
    `union U { armed(Clock), idle }`, `object O { w: W; d: Clock; q: Sequence<Clock>[0,
    3]; p: P; u: U; operation arm(x: Clock, k: Integer) modifies self.d; }` and `time
    O dense unit ms;`, `identities.json` has `format` `/5`; `O`''s `source_objects`
    row has the data entries `w`, `p`, `u` and the clock entries `d` (`member_type`
    `ix://filament/native/Clock`, `collections` `[]`) and `q` (`Sequence<ix://filament/native/Clock>`,
    `[{"kind": "sequence", "minimum": "0", "maximum": "3"}]`) in field order, and
    the operation `{"'
- id: FR-531-AC-41
  path: spec/functional/FR-531-evaluate-from-a-checked-package-read-from-its-bytes-alone.md
  role: context_only
  excerpt: 'For two separate units, one holding `function d(x: Integer): Integer pure
    { x + x + ... + x }` with 100,000 `+` applications (100,001 terms), the other
    holding 100,000 nested `let`s whose first binding is `x + 1`, each subsequent
    binding is the previous binder plus 1 and the result is the last binder, S4 writes
    complete packages and their `body_locations` on a 512 KiB stack under finite explicit
    caller ceilings chosen before emission from the checked input and the declared
    linear bounds above. The actual complete canonical byte lengths, relative entry
    counts and charged native strict-read and '
- id: FR-531-AC-42
  path: spec/functional/FR-531-evaluate-from-a-checked-package-read-from-its-bytes-alone.md
  role: examined
  excerpt: A selected model with `Tier` cases `Gold`, `Standard` and an unused payload-free
    variant with non-identifier cases in noncanonical authored order writes all three
    required model-enum channels in identities/5. Reading the file preserves every
    original declaration, member and shape; full runtime projection retains both enums,
    and the bare world resolves every case to its authenticated canonical rank and
    member identity. Calling enum-valued functions, including a nested typed parameter
    round trip, and equality and inequality callers from source and the bytes gives
    equal values, variants, ranks an
- id: FR-531-AC-43
  path: spec/functional/FR-531-evaluate-from-a-checked-package-read-from-its-bytes-alone.md
  role: examined
  excerpt: Omitting or repeating any required enum channel, an unknown channel, or
    identities/4 refuses without a partial read. Foreign selected-document claims,
    missing or repeated members, reversed shape variants, and reordered canonical
    case rows each fail the native witness join.
- id: FR-531-AC-44
  path: spec/functional/FR-531-evaluate-from-a-checked-package-read-from-its-bytes-alone.md
  role: examined
  excerpt: For original model-enum rows, one cumulative native budget covers producer
    materialization, native identity join and full runtime projection. The exact measured
    work and storage ceilings succeed with the same consumed totals; zero and one-less
    work or storage ceilings refuse with the original first-unpaid resource and no
    partial enum view.
- id: FR-533-AC-19
  path: spec/functional/FR-533-evaluate-a-protocol-slot-from-the-checked-package-s-own-nodes.md
  role: examined
  excerpt: 'The bare route evaluates **R**''s recovery predicates, whose `sum` comprehension
    reads the Collection member `receipts` of the record `Recovery`: `Full` (`sum(receipt
    in recovered.receipts: receipt) = target and not activated`) is true for receipts
    [2,3], target 5, not activated, and false for target 6, for activated, and holds
    for no receipts and target 0; `Partial` (`0 < sum(...) and sum(...) < target`)
    is true for receipts [1,2] and [4] with target 5, false for no receipts and for
    [5]. A `recovered` supply that is no `Recovery` record and a supply that omits
    `target` refuse as `CallFailure::'
- id: FR-533-AC-20
  path: spec/functional/FR-533-evaluate-a-protocol-slot-from-the-checked-package-s-own-nodes.md
  role: context_only
  excerpt: 'A strictly admitted variant of **R** constructs `Shape::Circle(sum<M::Total>(receipt
    in recoveredFull.receipts: receipt))` and matches `Circle(r)` in `Full`''s recovery
    predicate. Matching, mismatching, empty and activated supplies produce the source
    route''s Boolean values and ordered charges. The bare environment preserves the
    source union''s original type handle, sealed declaration/member identities, member
    order and payload types; changing the payload changes the predicate, while adding
    an unrelated union leaves it unchanged. Missing required type projections refuse
    `MissingTypeRepresentation'
- id: FR-533-AC-22
  path: spec/functional/FR-533-evaluate-a-protocol-slot-from-the-checked-package-s-own-nodes.md
  role: context_only
  excerpt: Over a copy of fixture E-P's model that adds one variant (enum) type no
    slot reads, `with_supply` succeeds, `Refund.trigger.guard` evaluates to the source
    route's value and charge log, and `admit_protocol_observation` for `M` refuses
    `unknown_required_feature`/`unsupported-feature` with `package` the package name
    and `reason` `missing-type-representation`, not an `InternalFault`, where the
    source route admits the same snapshot (the unmet dependency of "Observation admission").
    Over fixture E-P, a native budget that pays every earlier join of `with_supply`
    and the declaration projection but not
- id: FR-533-AC-23
  path: spec/functional/FR-533-evaluate-a-protocol-slot-from-the-checked-package-s-own-nodes.md
  role: examined
  excerpt: In TC-1333 fixture Q, real emitted/read bare calls and source calls convert
    `1 beat` to exact `1/4 s` and `1 shifted` to exact `3/4 s`; product, quotient
    and both square equalities are true. Compound conversions return `9/2 s2`, dimensionless
    `8 one`, `4 s2` and `1/4 s2` with the exact declared target types. Both routes
    admit identical ordered kernel charges and, at each measured counter's required
    allowance minus one, return the identical Incomplete record and retained location
    without a quantity or payment of the denied charge; the exact required allowance
    completes. A real admitted direct-d
- id: Cargo.lock
  path: Cargo.lock
  role: examined
  excerpt: |
    # This file is automatically @generated by Cargo.
    # It is not intended for manual editing.
    version = 4

    [[package]]
- id: Cargo.toml
  path: Cargo.toml
  role: examined
  excerpt: |-
    [workspace]
    members = [".", "xtask", "tools/arch-lint", "tools/fsl-probability-check", "tools/fsl-hybrid-test-provider", "fsl-walk-grow", "fsl-foundation", "fsl-cst", "fsl-command", "fsl-conformance", "fsl-replay", "fsl-source", "fsl-forms", "fsl-semantics", "fsl-package", "fsl-route", "fsl-eval", "fsl-analyze", "fsl-hybrid-reference", "fsl-inspect", "fsl-bench", "fsl-string-edge", "tests/fixtures/checked-closure-consumer", "tests/fixtures/hyper-query-consumer", "tests/fixtures/facade-contract-consumer"]
    resolver = "2"
    # A cargo-fuzz crate with its own workspace; fuzz/Cargo.toml says why.
    excl
- id: fsl-command/Cargo.toml
  path: fsl-command/Cargo.toml
  role: examined
  excerpt: |
    [package]
    name = "fsl-command"
    version = "0.1.0"
    publish = false
    edition = "2021"
- id: fsl-eval/src/value/expression/evaluate.rs
  path: fsl-eval/src/value/expression/evaluate.rs
  role: examined
  excerpt: |
    // SPDX-License-Identifier: AGPL-3.0-or-later
    //! The metered evaluator of checked expressions and function calls.
    //!
    //! Evaluation runs on an explicit task stack, so neither expression nesting
    //! nor recursive call depth consumes host stack; `function.call` and every
- id: fsl-eval/tests/it/quantity_class_convert.rs
  path: fsl-eval/tests/it/quantity_class_convert.rs
  role: examined
  excerpt: |+
    // SPDX-License-Identifier: AGPL-3.0-or-later
    //! `convert` across a dimension equivalence class, from source text to its
    //! evaluated value (FR-093 "Dimension equivalence", FR-093-AC-46; FSpec
    //! FR-142-AC-26).

- id: fsl-eval/tests/it/variant_fields.rs
  path: fsl-eval/tests/it/variant_fields.rs
  role: examined
  excerpt: |
    // SPDX-License-Identifier: AGPL-3.0-or-later
    //! FR-360 / TC-1008: evaluation of a variant value over the FR-360 domain
    //! package checked and linked from source.
    //!
    //! Equality is by case and never a checked-invariant fault (items 6 and 7),
- id: fsl-package/src/bare.rs
  path: fsl-package/src/bare.rs
  role: examined
  excerpt: |
    // SPDX-License-Identifier: AGPL-3.0-or-later
    //! FR-531: the evaluation entry over a checked package read from its bytes
    //! alone.
    //!
    //! A [`BarePackage`] borrows the [`CheckedPackageV2`] that Contract IR's
- id: fsl-package/src/bare_units.rs
  path: fsl-package/src/bare_units.rs
  role: examined
  excerpt: |
    // SPDX-License-Identifier: AGPL-3.0-or-later
    //! FR-533 "What the bare view reads from the package": the definitions of the
    //! source-owned units the package's quantities name, read from the package's
    //! own unit and dimension nodes through Contract IR's
    //! `read_unit_definition` and admitted by the same `admit_unit_graph` the
- id: fsl-package/src/bare_world.rs
  path: fsl-package/src/bare_world.rs
  role: examined
  excerpt: |
    // SPDX-License-Identifier: AGPL-3.0-or-later
    //! FR-533 "What the bare view reads from the package": the declarations a
    //! protocol-slot evaluation runs against, read from the package's own nodes.
    //!
    //! The source route evaluates against the checked scope of the unit it
- id: fsl-package/src/emit.rs
  path: fsl-package/src/emit.rs
  role: examined
  excerpt: |
    // SPDX-License-Identifier: AGPL-3.0-or-later
    //! The S4 checked-package/v2 emitter (FSpec FR-322, ADR-011 T-8).
    //!
    //! It writes the checked semantic graph with structural node identities
    //! `{package, path, ordinal}` and converts checked body terms into IR's typed
- id: fsl-package/src/emit/tests/variant_types.rs
  path: fsl-package/src/emit/tests/variant_types.rs
  role: examined
  excerpt: |
    // SPDX-License-Identifier: AGPL-3.0-or-later
    //! FR-360-AC-8, FR-360-AC-4 (TC-1009, TC-1007 step 3): a unit whose
    //! function, clause or field read names a model variant type emits it as one
    //! model-owned `variant_type` node, each case as a `value`/`enum_value` node
    //! of it (any non-empty case name, kept exactly), passes a variant value by
- id: fsl-package/src/reader_protocol_projection.rs
  path: fsl-package/src/reader_protocol_projection.rs
  role: examined
  excerpt: |
    // SPDX-License-Identifier: AGPL-3.0-or-later
    //! Reader-owned protocol expression nodes projected from admitted I2 terms
    //! (FR-390 "Composed roots from the reader").
    //!
    //! The whole package is projected once, when its reader-backed view is
- id: fsl-semantics/src/check/assemble/equivalence.rs
  path: fsl-semantics/src/check/assemble/equivalence.rs
  role: examined
  excerpt: |
    // SPDX-License-Identifier: AGPL-3.0-or-later
    //! The assembler's dimension equivalences (FR-091 "Unit-graph topology",
    //! FSpec FR-142 "Dimension equivalence"): each `dimension A ~ B by x U =
    //! y V;` is decided by FR-142's steps 1 to 7, the first match winning, base
    //! pairs (steps 1 to 5, then step 7) before every other pair (steps 1 to
- id: TC-1333
  path: spec/test-cases/TC-1333-evaluate-a-protocol-slot-from-the-checked-package-s-own-nodes.md
  role: examined
  excerpt: "| 23 | 23 | fixture Q, emitted/read with actual identity/equality supply
    and called through source and `BarePackage`; repeat at each measured counter boundary
    and with an actual admitted direct-dependency dimension operand | Both routes
    return `1/4 s`, `3/4 s`, the specified compound-conversion quantities and true
    arithmetic equalities, with identical unit types and ordered kernel charges. Required
    allowance minus one returns the same located Incomplete record without a quantity
    or payment of the denied charge; the exact allowance completes. Same-dimension,
    affine, unrelated power and nominal "
- id: FSpec-FR-142-AC-26
  path: "/Users/peter/dev/filament-specification/spec/functional/type-model/FR-142-evaluate-quantities-and-units.md"
  role: context_only
  excerpt: 'The root ratio carries the size of one package''s canonical root in the
    other''s. With `acme/rpc` (alias `R`, root `s`) and `acme/clock` (alias `G`, base
    dimension `Time`, root unit `ms`, and `unit s : Time = 1000 * ms;`), `dimension
    R::Time ~ G::Time by 1 R::s = 1000 G::ms;` and `dimension R::Time ~ G::Time by
    1 R::s = 1 G::s;` state one root ratio, 1000 (`b` is 1000 for `G::s`), and `dimension
    G::Time ~ R::Time by 1000 G::ms = 1 R::s;` is the same equivalence written the
    other way; the class root is the root `R::s` of the member with the least key.
    `convert<Quantity<G::ms>>(3 R::s)` is `3000 G'
```
