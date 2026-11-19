---
id: TC-3007
title: "A repeated quantifier variable is refused at the hyperproperty shape"
type: TC
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-488
    type: verifies
---
# TC-3007: A repeated quantifier variable is refused at the hyperproperty shape

## Description

Verify that a checked package whose `claim`/`hyperproperty` node holds two
quantifier Groups with one `variable` text is refused at FSpec FR-402's
Shape step, so each Group's variable keys exactly one clock row (FSpec
FR-322 "Hyper clause rows"). Scope: FR-488-AC-27, FSpec FR-402-AC-10.

## Test Procedure

1. Compile `hyper Same using inf over behaviours (c: Config::ConfigVersion)
   forall trace a of Config forall trace b of Config
   { always holds(c.versionNumber @ a = c.versionNumber @ b) }` over the
   `example/config-version` model, and read the package back.
2. Set the second quantifier Group's `variable` binding to `a` and read the
   package again.

Tag both steps `#[trace("TC-3007", "FR-488-AC-27")]` and
`#[trace("FSpec-TC-346", "FSpec-FR-402-AC-10")]`.

## Expected Results

- Step 1: the package is admitted.
- Step 2: the reader refuses `ambiguous_declaration`/`ambiguous-name` at
  `/semantic_graph/nodes/{n}/body/members/3/value/members/1/members/0`, `n`
  the `hyperproperty` node's graph index.
