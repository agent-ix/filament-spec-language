---
id: TC-1005
title: "Domain package refusals retain the source origin of the refused node"
type: TC
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-056
    type: verifies
---
# TC-1005: Domain package refusals retain the source origin of the refused node

## Description

Verify that normalization `malformed-declaration` refusals retain the source artifact and position of the node they name, while reader refusals are reported at intake without a fabricated source origin. Scope: FR-056-AC-19 and FR-056-AC-33.

## Test Procedure

1. Read the TC-235 K2 package with every node carrying a source origin at its own line.
2. For each of the following mutations, normalize and compare its refusal list: a record value type with identity fields; an event type with identity fields; an event whose occurrence field resolves to a non-Timestamp scalar; an operation declared by an event type; a state machine with no transitions or no operations; a state machine transition that emits a non-event; a process with an empty `identityFields` list whose supertype is a state machine; a relationship whose source end, and one whose target end, is not an object type.
3. Declare a population whose second member names a record value type, an event type, a scalar, a persistence interface or a namespace, and one whose member names nothing.
4. Repeat one mutation of step 2 with every node carrying a `generated` origin.
5. Normalize a package whose model selection has an empty identity and one with a declaration key whose package is empty; separately offer a package document that is not JSON at intake. Inspect each refusal's stage, source origin, declaration output and `normalize.record` charge.

## Expected Results

- Each step 2 refusal is `invalid_model_binding`/`malformed-declaration` at the refused node (the operation, not the event type, for an operation an event type declares) with the typed member path, source artifact and source line and column. The process with an empty identity list and a state-machine supertype has two ordered refusals at `TypeDefinition.identityFields` and `TypeDefinition.supertypes[0]`, both at the process node.
- Step 3 refuses the wrong-meaning member `invalid_model_binding`/`malformed-declaration` at the population node, path `PopulationDefinition.members[1]`, with the population's artifact and position, and the empty member `missing_declaration`.
- Step 4 leaves the artifact and source span absent for the generated-origin refusal.
- Step 5's empty selection and empty declaration key are refused during normalization as `invalid_model_binding`/`malformed-declaration` with no artifact or source span. The unreadable package document is refused at intake as `invalid_model_binding`/`malformed-declaration`, with no source artifact or span, no declaration and no `normalize.record` charge; it names document node `$` (FR-056-AC-19).
- Assertions compare typed code, cause, node, member path, artifact and position, never detail text.
