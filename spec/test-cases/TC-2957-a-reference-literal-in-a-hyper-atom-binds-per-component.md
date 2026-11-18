---
id: TC-2957
title: "A reference literal in a hyper atom binds per component"
type: TC
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-1000
    type: verifies
  - target: ix://agent-ix/filament-spec-language/FR-487
    type: verifies
---
# TC-2957: A reference literal in a hyper atom binds per component

## Description

Verify how a reference literal in a hyper clause's `holds` atom is bound
across the clause's components. Scope: FR-1000-AC-8 to FR-1000-AC-11.

Catches a fixed literal refused in an atom over two traces, a created-object
literal read in a default component when the atom spans several components
or none, and hyper letters that accept components whose binding tables
disagree.

## Test Procedure

1. Analyze FR-1000-AC-8's hyper clause `HF` over the Counter subject with
   initial state `{a}` and universe `{a, b}`.
2. Analyze over that subject `always holds((deref(M::Counter#"b").value = 0) @ a)`.
3. Analyze over that subject `always holds((deref(M::Counter#"b").value) @ a = c.value @ b)`
   and `always holds(c = M::Counter#"b")`, then both with `M::Counter#"a"`.
4. Build `CheckedHyperLetters` for a vault clause comparing `v` with
   `V::Vault#"w"`, once over two systems whose initial states disagree on
   `w` and once over one system for both components.

## Expected Results

- Step 1: admitted, exit 0, no `operator-ineligible`.
- Step 2: the atom is undefined with reason `absent-key`.
- Step 3: each refuses `ill_typed`/`operator-ineligible` at the literal
  naming `Counter`, `62` and the components `[0, 1]` and `[]`, exit 20;
  with `M::Counter#"a"` both are admitted.
- Step 4: `LettersRefusal::ReferenceBindingsDisagree` naming `Vault`, `77`
  and `[0, 1]`; then the letters admit.
