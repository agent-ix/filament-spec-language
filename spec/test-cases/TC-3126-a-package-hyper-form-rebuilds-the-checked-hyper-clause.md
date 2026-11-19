---
id: TC-3126
title: "A package hyper form rebuilds the checked hyper clause"
type: TC
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-940
    type: verifies
---
# TC-3126: A package hyper form rebuilds the checked hyper clause

## Description

Verify that the hyper form read through the analysis view is equal over the
source route's admitted read-back and the package route's bare package, and
that the clause rebuilt from it equals S3's checked clause in every member
the engines read. Scope: FR-940-AC-16.

## Test Procedure

1. Compile the Counter unit with the hyper clauses `Equal`, `Partner`,
   `CanReachThree` and a projection-aligned clause with an `align skip` list
   and a `match` block holding one universal and one existential conjunct,
   with a `handoff` member.
2. For each clause, resolve its `claim_name` occurrence and read
   `hyper_clause_form` over the `AdmittedPackageRead` and over the
   `BarePackage` of the handoff directory; rebuild each with
   `CheckedHyperClause::of_package_form`.
3. Compare each rebuilt clause with S3's checked clause, renumbering S3's
   atoms by FR-532's atom index.
4. Repeat step 3 with a test-only view that files the existential conjunct
   under `μ_U`.

## Expected Results

- Step 2: the two forms of each clause are equal.
- Step 3: `Equal`'s copy group is the class `[a, b]` with the generator
  `{first: 0, second: 1, atoms: [0]}`, and a clause whose body is
  `always holds(c.value @ a <= 3)` alone has an empty copy group.
- Step 3: name, span, profile, aliases, read object parameters, quantifier
  prefix with fairness sets, atom slots, conjunct count per part,
  `align skip`, form, copy-group classes, occurrence, formula arena, atom
  indices and copy generators are equal; `condition` of every rebuilt atom
  is `None`.
- Step 4: the comparison reports the conjunct partition as different.
