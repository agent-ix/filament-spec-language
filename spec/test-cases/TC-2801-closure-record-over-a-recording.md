---
id: TC-2801
title: "Closure record over a recording"
type: TC
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-990
    type: verifies
---
# TC-2801: Closure record over a recording

## Description

Verify that the complete-global-conformance closure record over a recording is copied from the recording, checked by the strict reader, never upgraded, and alone decides complete global success. Scope: FR-990-AC-3, FR-990-AC-4.

Catches `closed` inferred from an observed `finish`, a malformed premise set accepted, and closure changing a settled verdict.

## Test Procedure

1. Assess FR-895-AC-1's recording with closure `closed`, then `open`, then `contradicted`.
2. Assess it with closure `closed` and the `Ship` branch premise omitted, the execution premise duplicated, and another instance's premise.
3. Assess a recording that observes `finish` with closure `open`.

## Expected Results

- Step 1: `closed`, producer `recording`, `complete` true; `inconclusive` `open-history`, `complete` false, decided verdicts unchanged; `inconclusive` `open-history` with `closure.state` `contradicted`.
- Step 2: each refused `invalid_runtime_input` naming the premise, no verdict.
- Step 3: `closure.state` stays `open`.
