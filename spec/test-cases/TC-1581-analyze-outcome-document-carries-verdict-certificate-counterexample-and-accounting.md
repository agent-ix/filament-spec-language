---
id: TC-1581
title: "The analyze outcome document carries verdict, certificate, counterexample, replay and accounting"
type: TC
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-621
    type: verifies
---
# TC-1581: The analyze outcome document carries verdict, certificate, counterexample, replay and accounting

## Description

Verify FR-621's item members and document category. Scope: FR-621-AC-1 to FR-621-AC-7.

## Test Procedure

1. Analyze `Bounded` over the Counter fixture.
2. Analyze `always holds(c.value <= 2)`.
3. Analyze `Bounded` with `temporal.limits.max_states` 2.
4. Check every category and label of steps 1 to 3 against FSpec FR-352's registry; hand the writer an unregistered kind.
5. Compare the library document with the driver's machine-mode document for step 2.
6. Render step 2 in text mode.
7. Analyze a `finite-replay` item whose clause is pending at the trace end, and read the value spelling of step 2's counterexample.

## Expected Results

- Step 1: `success`, `proved`, basis `exhaustive`, `certified`, `certificate.accepted` true, `exploration.states` 4, `exploration.transitions` 3, `aggregate.status` `complete`, exit 0.
- Step 2: `refuted`, counterexample ending at `value = 3`, `replay.reproduced` true, exit 10.
- Step 3: `incomplete` `LimitReached{max_states}`, `exploration.states` 2, no counterexample, `aggregate` incomplete naming index 0, exit 22.
- Step 4: every kind registered; the unregistered kind gives `internal-failure`, exit 30.
- Step 5: byte-equal documents.
- Step 6: first line `0 Bounded refuted`, exit 10.
- Step 7: kind `pending`, category `pending`, basis and `axes.settlement` `unsettled`, exit 0; `value` spelled `{"integer": "3"}`.
- Assertions compare the outcome document's typed members and the exit code, never message text.
