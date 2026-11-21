---
type: reference
---
# Native diagnostic codes

Native diagnostic codes identify why an FSL operation stopped. Use the code
and phase to choose a correction, then use the source region or document
pointer to locate the input. A refusal does not report that a rule is false.

For source identity errors, supply both nonblank caller labels and a display
path before parsing. For UTF-8 errors, correct the original bytes without
renumbering their source positions. For syntax errors, correct the reported
token or missing terminal; a diagnostic-list cut means more independent errors
remain after the reported bound.

An editor's `invalid_source_map` can mean a foreign node, an edit against
another source, a mismatched request identity, overlapping edits or a range
inside a UTF-8 character. Keep the node, request and edits bound to the source
that produced them. Reparse the intended source before issuing a new request.

For cancellation, begin a new uncancelled operation when the caller intends
to continue. For a named budget, raise only that caller-selected ceiling when
the work is intended and affordable. Neither result supplies predicate truth.

For population input, bind the document to the selected model, supply a closed
subtype environment and keep the selected count within its declared bound.
Abstract types cannot have direct instances. A missing snapshot is incomplete
evidence; a supplied snapshot with the wrong observation role is refused.
Recorded creation and deletion lists must match the complete pre/post
observations, and the operation's frame must separately authorize each change.
A reference into a complete population must name a present object. An
incomplete population cannot establish that an absent target is dangling.

Extracted Markdown belongs to a clause run. Select one authored clause binding,
keep the source package and extraction modes separate, and correct the semantic
package named by retained Quire diagnostics. These request defects settle
before compilation; they do not report the clause's truth.

A clock requires dense time on its own source model. A random distribution
requires positive exact weights. Correct either declaration at its reported
locus. A sequence whose computed maximum exceeds the fixed representable bound
needs a smaller declared range; raising a work budget cannot change that bound.
Give each resolved operation at most one body, and supply a unique applicable
body for every concrete subtype reached by a dispatched call. For an execution
backend the caller has not supplied, supply that backend or explicitly choose
an available one.

Replay a counterexample against the clause and occurrence it names. A stale
identity refuses before observation admission; replacing a missing snapshot
cannot repair it. Regenerate the counterexample from the intended checked
package. A definition closure must supply every required capability, even when
each individual capability name is valid.

Use the result-preserving emission entry when a graph retains protocol-local
refusals. It retains those source errors and admits no unit package; the
shorthand emission entry cannot discard them to produce a package.

The producer tests use each stage's own typed cause: source tooling returns
`CompleteCause`, bundle resolution returns `ResolutionCause`, and model
admission retains its model or observation cause. A catalogue mapping alone
does not establish that an input can reach a producer. Internal-fault codes
describe a broken established invariant, rather than a supported way to refuse
user input; an ordinary malformed document must retain its input-defect code.
For example, observation admission reports an internal fault if the host loses
the model package that its checked graph already selected. Restore the
checked graph's exact model inputs; changing the observation cannot repair
that host inconsistency.

FR-010 exposes one native `Diagnostic` envelope with phase, code, exact caller
source labels, display path, original span and explanatory message. `Display`
renders `code: message`; `std::error::Error` has no underlying cause. The public
`source` field is provenance, so the standard traits are implemented directly:
the thiserror derive would incorrectly treat that field as an error cause.

`Code::all()` enumerates this vocabulary, `as_str()` supplies its stable spelling,
and `from_code()` returns `None` for unknown spellings. Codes are not renamed or
reused; prose may change without changing classification.

| Code | Meaning |
| --- | --- |
| io-error | A selected local file could not be opened or read. |
| invalid-request | A closed command request has invalid JSON or fields. |
| invalid-identifier | A selected command identifier fails its constructor. |
| output-failure | A typed command result could not be serialized. |
| unsupported_projection | The selected producer entry cannot supply a required checked-package/v2 projection, including the shorthand emitter when protocol-local refusals require result-preserving emission. |
| invalid_projection_correspondence | Checked native correspondence could not be retained by the projection. |
| extraction-requires-run | The source-only compile command does not admit extracted programs. |
| extraction-package-conflict | Selected package bytes and extracted compilation were both requested. |
| extraction-clause-count | Extracted compilation requires exactly one authored binding and an extracted body declaring one clause; the refusal retains the actual count. |
| invalid-quire-context | Quire rejected the semantic context; details identify its contract versions and original diagnostics. |
| invalid_source_identity | Required source identity, revision or display path is absent. |
| invalid_source_map | Correspondence, source binding or queried range is invalid. |
| invalid_utf8 | Source bytes cannot be decoded as UTF-8. |
| invalid_syntax | Malformed source, including forbidden source characters. |
| unsupported_construct | Recognized construct is outside the admitted grammar/profile. |
| unrepresentable_constraint | A native declaration or expression violates a fixed representability limit or required policy of the admitted profile, rather than naming a real capability outside it. |
| unknown_language | The selected language label is not admitted. |
| unknown_edition | The selected edition label is not admitted. |
| unknown_profile | The selected profile label is not admitted. |
| invalid_package | Native package encoding, closed structure or reconstructed claims are invalid. |
| unknown_wire | The selected command or artifact format is unsupported. |
| unknown_required_feature | A package feature is unknown or outside the consumer's declared support. |
| missing_import | A selected formal package or native import alias is absent. |
| ambiguous_declaration | Multiple exact candidates or visible exports match; related formal loci are retained. |
| stale_dependency | A selected dependency identity, revision or bytes differ from the artifact. |
| missing_declaration | A required value, field, type or variant cannot be resolved. |
| invalid_model_binding | An explicit context binding or clause name is invalid. |
| invalid_runtime_input | Native runtime input structure is invalid, including a local root/child outside its admitted arena ordering. |
| wrong_snapshot | A native result/pre expression has the wrong observation, or a runtime selection's artifact role, observation or invocation context/name/anchor disagrees with its checked clause. |
| dangling_reference | A runtime object target is absent from its complete required population. |
| incomplete_population | A required runtime population is absent or explicitly incomplete; missing targets cannot be inferred as dangling. |
| unavailable_observation | A selected runtime artifact or required State-root counterpart is unavailable. |
| population_delta_mismatch | Recorded created/deleted identities repeat, intersect or disagree with complete pre/post population differences. |
| frame_violation | A surviving object field or State root changes without permission, or a created/deleted object type is outside the model frame. |
| cancelled | The caller's runtime cancellation poll stopped validation or evaluation with actual prior usage. |
| runtime_invariant | Reference evaluation encountered a violated established typing or validated-input invariant; no Boolean is produced. Exits 30 (internal fault, a tool failure). |
| ill_typed | Native contextual types, nominal identities, units or operator eligibility disagree, or a scalar context is ambiguous. |
| undefined_expression | The actual IR prover could not establish a potentially evaluated operation's definedness under its preceding guards. |
| resource_exhausted | A source, syntax, formatter, map, linking, checking/proof or run ceiling prevented completion. |
| conflicting_declaration | Two declarations that must be unique resolve to one identity: two unit `operation-body` declarations resolving to one operation (FR-302), cause `duplicate-declaration`. |
| ambiguous_dispatch | FR-151 dispatch linking found no, or several undominated, applicable candidates for a closed subtype. |
| foreign_reference | FR-153: a population document, lookup key or population member names an identity outside the bound closed environment. |
| cardinality_out_of_bound | FR-153: a formed population selection count is outside its declared `allInstances<T>(p)` bound. |
| duplicate_selection | ADR-013 O-01/QC-5: a domain-package identity is selected more than once. |
| stage_limit_exceeded | A syntax, stage or work limit stopped the operation before completion. |
| invalid_probabilistic_model | A random parameter, reward, workload or `probabilistic` claim breaks a rule of FSpec FR-405 or FR-407: cause `invalid-weight`, `repeated-support-value`, `duplicate-distribution`, `duplicate-weight`, `duplicate-reward`, `state-read-in-reward`, `qualitative-threshold`, `quantile-over-fraction`, `partial-confidence` or `invalid-confidence`; `duplicate-reward`, `quantile-over-fraction` and `invalid-confidence` are FSL-proposed pending FSpec (STD-188). |
| invalid_timed_model | FSpec FR-415/FR-416: a timed model, clock binding or timed interval breaks the rule its cause names. |
| unimplemented_capability | FSpec FR-271: a valid complete-V1 capability the selected producer does not implement. |
| noncanonical_wire | A document is not in its canonical form: a domain package or observation document carries a number with no exact RFC 8785 spelling (FR-056, FR-106), cause `inexact-integer` or `inexact-number` with the number's `document_pointer`; or a checked-package/v2 wire's bytes are not RFC 8785 bytes (FR-322's I2 reader), with neither cause nor pointer. FSpec FR-271's `## Values` row catalogs the code, and FR-272's `noncanonical_wire` row its causes and `document_pointer` (filament-specification#180). |

Phase identifies the observing boundary: source, lex, parse, profile, format or
source_map, link, check or validate. `Diagnostic` itself carries only the
phase, code, source locus and message (ADR-011 §6.1: the foundation layer
does not import `crate::linking` or IR types). Related formal declaration
locations and an upstream IR canonicalization or proof failure are retained
as typed sibling fields on the owning stage's own error type instead —
`LinkingError`, `CheckingError`, `FormalSourceError` and `NativeModelError`
each pair a `Box<Diagnostic>` with `related`/`upstream` as applicable.
Runtime validation diagnostics pair a `Box<Diagnostic>` with `runtime` (the
exact actual/expected artifact, authored clause, observation and typed value
path) and `related` instead. The retired `native-run-result/1` output carried
both `upstream` and `runtime` on every diagnostic view: a `link`- or
`check`-stage diagnostic populates `upstream` from its `LinkingError`/
`CheckingError` wrapper (null when there was no upstream refusal) and leaves
`runtime` null; a `validate`-stage diagnostic populates `runtime` from its
`ValidationDiagnostic` wrapper and leaves `upstream` null. A `CheckingError`
built from a `FormalSourceError` (a `to_ir` coordinate-mapping failure) keeps
the `source_map` phase from its origin and can still populate `upstream`, so
`source_map` is not one of the phases that always leaves both fields null.
Resource exhaustion is incomplete, never false. This
six-code contract is FR-301's, and the `fsl` driver (filament-driver) carries it
to a process exit status; FSL's `command` API returns each outcome with the
code FR-285 gives its category. On that contract the driver exits 0 for a
successful parse or a completed-true run predicate; 10 for a completed-false
run predicate, a logical violation, not a tool defect; 20 for malformed
requests, invalid command usage, request syntax, identifier and I/O failures
and semantic refusals other than an admitted-but-unsupported capability; 21 for
a real, profile-gated capability this build does not admit —
`unsupported_projection`, `unsupported_construct` and
`unknown_required_feature`, via `Code::is_unsupported()`; 22 for incompleteness;
and 30 for a tool failure distinct from the requested work: a broken
serialization or write, or `runtime_invariant` (an internal fault). Usage/I/O text does not pretend to be a source
diagnostic. A successful parse exits 0 and reports parsed, without
model-linking or evaluation claims.

Validation reports keep each diagnostic's classification. Incomplete population,
unavailable observation, cancellation and exhausted work are incomplete. A known
invalid defect makes the overall report Refused even when another defect is
incomplete or detail capacity is zero. The optional terminal stop reason is
separate from the detail vector. Neither a failed report nor a successful
ValidatedContext contains predicate truth.

The native model profile resolves operation parameters only in the selected
operation and results through the result keyword. Early binding refusals use
the link phase; resolving an actual result in a precondition leaves its
post-only availability for the checker. Inventory identity conflicts identify
the conflicting model's source at byte zero and retain both models' related
formal declarations. Existing formal-profile linking keeps its original codes.
