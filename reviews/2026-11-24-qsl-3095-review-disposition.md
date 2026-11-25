# QSL-3095 reviewer finding and disposition

## Finding

FR-1041-AC-1 / TC-3150 requires `fsl-probability-check` to accept the emitted W fair-MDP document in a separate process. The existing `exact_probabilistic.rs` test checked the document in-process, while subprocess coverage only covered AC-5.

## Disposition

Added `tools/fsl-probability-check/tests/it/fair_mdp.rs`, which builds the AC-1 W fixture, obtains the analyzer-emitted certificate request, and passes it to the `fsl-probability-check` executable through the existing child-process test helper. The test asserts the exact values `{0, 1/2, 1}`, ranks `{0, 0, 1}`, the strong whole fairness record naming `send_b`, and child-process acceptance at value `1/2`. The test is tagged `TC-3150` and `FR-1041-AC-1`.
