---
id: TC-2979
title: "Lowering preserves the clause's classification"
type: TC
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-931
    type: verifies
---
# TC-2979: Lowering preserves the clause's classification

## Description

Verify that a supported `tl_lowering` record carries the classification read
from its lowered formula alone, equal to the native clause's, with no
reverse lowering. Scope: FR-931-AC-8.

## Test Procedure

1. Lower every past and every future case of FR-932's corpus over every trace of length 1 to 4 over the values 0, 1 and 2.
2. Lower the future clause `always[0,3] holds(c.level <= 2)` under `"correspondence": "mltl.online-prefix/v1"`.
3. Lower the mixed case `eventually[1,1] once[1,1] not p`.
4. Through the `fsl-command` library entry, lower `always[0,2] holds(c.level = 1)` faithfully and with the lowered formula's correspondence replaced by `mltl.origin-complete-history/v1`.

## Expected Results

- Step 1: each past record carries `{"correspondence": "mltl.origin-complete-history/v1", "fragment": "bounded-past"}` and each future record `{"correspondence": "mltl.closed-trace/v1", "fragment": "bounded-future"}`, its correspondence equal to the record's; no `classification_fault`.
- Step 2: `{"correspondence": "mltl.online-prefix/v1", "fragment": "bounded-future"}`.
- Step 3: `unsupported`, exit 21, no `classification`.
- Step 4: the faithful lowering carries `{"correspondence": "mltl.closed-trace/v1", "fragment": "bounded-future"}`, exit 0; the replaced one carries no `classification` and a `classification-loss` `classification_fault` naming `C`, the native classification and `{"correspondence": "mltl.origin-complete-history/v1", "fragment": "bounded-future"}`, exit 30.

Test: `tests/it/mltl_differential.rs` `lowering_preserves_the_classification`.
