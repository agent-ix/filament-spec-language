---
id: NFR-140
org: agent-ix
title: "Trace a test to a standard artifact by its qualified id"
type: NFR
quality_attribute: maintainability
relationships:
  - target: ix://agent-ix/filament-spec-language/NFR-005
    type: depends_on
---
# NFR-140: Trace a test to a standard artifact by its qualified id

## Statement

When an FSL test verifies a filament-specification acceptance criterion or
test case, its `#[trace]` tag shall name that artifact as `FSpec-<id>`, and
the computed test matrix over the two repositories shall bind the tag to the
filament-specification artifact `<id>` and to no FSL artifact.

## Scope

FSL and filament-specification number their artifacts independently, so one
bare id such as `FR-149-AC-22` names a different criterion in each. FSL's
trace convention therefore has two forms, and both are part of the trace
grammar:

- A bare id (`TC-196`, `FR-075-AC-4`) names the FSL artifact with that id.
- `FSpec-<id>` (`FSpec-TC-194`, `FSpec-FR-149-AC-22`) names the
  filament-specification artifact `<id>`. It is the short spelling of the
  qualified id `ix://agent-ix/filament-specification/<id>`, and it means
  exactly that id. A tag may also be written in the qualified form, with the
  same meaning.

The `FSpec-` qualifier is declared data of FSL's traceability model, not a
convention known only to readers: the model declares the prefix `FSpec-`
and the repository identity `agent-ix/filament-specification` it stands
for, so the matrix engine reads the qualifier from the declaration and no
engine hard-codes a repository name. FSL's own rows are computed from its
bare ids as before.

The computed matrix is the one `quire matrix` (and `quoin matrix`) builds
over the corpus `--repo agent-ix/filament-specification=<dir> --repo
agent-ix/filament-spec-language=<dir>`. Over that corpus a criterion of
filament-specification that an FSL test tags `FSpec-<id>` has that test
among its binders, with the test's ignored state as for any other binder.
A run over FSL alone, with no filament-specification member, reports such a
tag as a reference to a repository outside the corpus, never as an
unmatched or malformed tag and never as a binder of the FSL row with the
same bare id.

A filament-specification criterion with no binder in that corpus is
uncovered. When FSL tests carry its `FSpec-` tag and the engine still
reports no binder, the defect is in the matrix tooling's reading of the
qualifier, and the tags stay as written.

## Measurement and Evaluation

| Metric | Target | Threshold | Method |
| --- | --- | --- | --- |
| `FSpec-<id>` tags in FSL tests that bind no filament-specification artifact in the two-repository matrix, where `<id>` exists there | 0 | 0 | Test |
| `FSpec-<id>` tags that bind an FSL artifact | 0 | 0 | Test |

## Verification

Build the two-repository matrix and read the filament-specification rows of
criteria that FSL tests tag with `FSpec-` ids; run the matrix over FSL alone;
inspect FSL's traceability declaration for the qualifier (TC-3033).

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| NFR-140-AC-1 | In the two-repository matrix, filament-specification `FR-149-AC-22` lists the FSL tests `fsl-eval/tests/it/collection_algebra.rs` `c06b_float_sets_are_admitted_and_compare_members_by_bit_identity` and `fsl-semantics/tests/it/native_scalar_equality.rs` `string_equals_text_refuses_with_the_specified_cause` among its binders; `FR-149-AC-25`, `FR-149-AC-26`, `FR-149-AC-28` and `FR-149-AC-29` and test case `TC-194` each list every FSL test tagged with their `FSpec-` id; FSL's own `FR-149-AC-22` lists none of those tests. | Test (TC-3033) |
| NFR-140-AC-2 | A test tagged `ix://agent-ix/filament-specification/FR-149-AC-22` binds the same row as one tagged `FSpec-FR-149-AC-22`; over FSL alone, an `FSpec-` tag is reported as a reference outside the corpus and binds no FSL row; an `FSpec-<id>` tag whose `<id>` filament-specification does not mint is reported as an unknown id of that repository. | Test (TC-3033) |
| NFR-140-AC-3 | FSL's traceability declaration names the qualifier prefix `FSpec-` and the repository `agent-ix/filament-specification`; removing that declaration makes the two-repository matrix report every `FSpec-` tag as unresolved, never as a binder of a bare-id row. | Inspection (TC-3033) |
