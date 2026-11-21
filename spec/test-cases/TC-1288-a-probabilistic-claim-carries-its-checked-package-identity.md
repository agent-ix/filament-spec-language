---
id: TC-1288
title: "A probabilistic claim carries its checked-package identity"
type: TC
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-498
    type: verifies
---
# TC-1288: A probabilistic claim carries its checked-package identity

## Description

Verify that the checked-package producer emits one local
`claim`/`probabilistic_claim` node and one declared `claim` occurrence per
included `CheckedProbabilisticClaim`, writes one matching
`probabilistic_claims` row, that the checked claim exposes the same
producer-owned identity, and that the FR-421 causes carry it. Scope:
FR-498-AC-1 to FR-498-AC-8.

## Test Procedure

1. Compile one unit with three probabilistic claims: two sharing a declared
   name and differing in written content (the FSL counterpart of FSpec
   TC-233 `PC-01` and `PC-02`, referenced by ID; one source cannot give two
   declarations equal spans) and a third with another name. Emit the package
   and record each claim's `{declaration, occurrence}`, telling the claims
   apart by their written content, never by the identity under test.
   Re-emit every tested permutation of the three declarations and record the
   identities again.
2. For each claim, match the checked claim, its local `claim` node with
   `semantic_form` `probabilistic_claim`, its declared `claim` occurrence and
   its sidecar row by `{declaration, occurrence}`. Rename the third claim,
   whose name no other claim shares, and
   re-emit; parse a claim moved, reformatted and commented, and one with a
   changed token, and compare their written content. Compile two claims
   written identically.
3. Check an elapsed-time claim and a punctual-interval claim, each sharing
   its name with another claim, and read their exact verdicts and the
   selected claim's `{node, occurrence}` locus; check FR-498-AC-8's bounded
   punctual claim, its nested and root `eventually[2 ms, *]` claims and a
   `TimedFormShape` cause on the form, and read each cause's member path and
   FR-331 wire locus.
4. Read the emitted package with the strict reader and check every row names
   a node of the emitted graph with that node's own `claim` occurrence.
5. Emit FR-498-AC-5's three claims; read each claim node's body and path;
   rename `A` to `B`; change one weight of `Steady` and one support weight
   of `d`; reorder `Steady`'s weight lines and the claims; compare every
   claim `node_id` and row with the first emission; read each package with
   the strict reader.
6. Emit one claim of each FR-498-AC-6 form over the extended `Service` and
   over the `time dense` unit; read each `reward` and `delays` binding;
   rewrite `duration` on `Server::request` and the delay weight and compare
   every claim `node_id`; read each package with the strict reader.
7. Emit FR-498-AC-7's `P95` and `A`; read each claim's `form`, `over`,
   `activation` and `confidence` bindings; make each change of FR-498-AC-7
   alone and compare both claims' `node_id`s and rows with the first
   emission; make each spelling of FR-498-AC-7 and compare both claims'
   `node_id`s and bodies; emit the second claim named `A` with `0.6` and with
   `rational(2, 4)` and read each path and ordinal; read each package with the
   strict reader.

Tag the test `#[trace("FR-498-AC-1", "FR-498-AC-2", "FR-498-AC-3", "FR-498-AC-4", "TC-1288")]`
and the step 5, 6 and 7 tests `#[trace("TC-1288", "FR-498-AC-5")]`,
`#[trace("TC-1288", "FR-498-AC-6")]` and `#[trace("TC-1288", "FR-498-AC-7")]`;
the step 3 test also carries `#[trace("TC-1288", "FR-498-AC-8")]`.
Vectors are referenced by FSpec case ID, never copied.

## Expected Results

- Step 1: three local claim nodes, each with one declared `claim`
  occurrence, and three matching rows with distinct declaration identities in
  structural order; every permutation keeps each claim's identity and emits
  byte-identical rows.
- Step 2: each checked claim, its graph node and its row expose the same
  `{declaration, occurrence}`; renaming changes only the renamed claim's
  identity; moving, reformatting and commenting leave written content equal
  and a changed token makes it differ; the identical twins have distinct
  identities.
- Step 3: the elapsed-time, punctual-interval and bounded punctual claims
  settle exact verdicts against the selected claim's `{node, occurrence}`,
  never `unsupported` and never `PunctualInterval`; each `TimedFormShape`
  cause carries that locus and no other claim's, and the member paths and
  wire loci FR-498-AC-8 states.
- Step 4: the strict reader admits the package and every row joins its node
  and occurrence.
- Steps 5 to 7: the bindings, paths, changed and unchanged `node_id`s and
  ordinals FR-498-AC-5, FR-498-AC-6 and FR-498-AC-7 state; the strict reader
  admits every package.
