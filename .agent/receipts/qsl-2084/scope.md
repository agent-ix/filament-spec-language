# QSL-2084 evaluator closure cases

The supported feature exercises current public std evaluator entries with real checked packages and admitted model observations. The new cases cover checked expression and closed frame evaluation, protocol body evaluation, refinement population rows, history pre/post selection, terminal predicates, state predicates, state-clause read paths, reward arithmetic, temporal holds, separation replay and reachable value invariants. Expected values come from the inputs and mathematical operations; arithmetic charge records are compared against direct kernel evaluation. Route parity is explicitly not an independent-evaluator comparison.

The current locked filament-eval 0.1.6 dependency provides scheduler and body-provider interfaces. The current FSL expression and clause entries still execute the std Machine. No copied reference interpreter, generated replacement evaluator, or fake no_std consumer is added.

The required independent no_std checked-term evaluator remains pending IR-590 (`ee0a4b98-0aae-4498-aba5-1000c9be03f1`). Switching and retiring the std path remains pending IR-593 (`314a260a-b087-48bc-b575-705c3eca91fa`). Both existing issues block QSL-2084 in Linear. These facets are not claimed as passing coverage or as delivered by the std regression cases.

Current NFR-015 has two FSL rows: the fsl-foundation dependency closure builds on thumbv7em-none-eabi without default features; fsl-package is crate-tier no_std with a host no-default-features check and may depend on std fsl-semantics. It has no no_std row for fsl-eval. Necessary feature-mode build results are recorded separately; no build of the entire workspace or product binary is part of this feature.

One initial test compilation found an authoring error: Outcome has no map method. It was corrected with a typed Completed match before any tests executed. No runtime defect was accepted as passing.

An accidental workspace formatter invocation touched unrelated files. All its formatting-only changes were restored in this initially clean owned worktree; only the assigned new test files are individually formatted. No production change is part of the feature.
