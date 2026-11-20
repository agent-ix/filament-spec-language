---
type: reference
---
# Diagnostic producer validation

QSL-2079 adds 32 active public-input producer test groups in
`tests/it/error_producer_inputs.rs`. Their focused authoring runs passed.
Four strict functions remain ignored under implementation owners; they do
not count as passing coverage. The scoped post-review gate passed; its
non-overlapping segments are recorded below.

The tests use real source, request files, host edits, checked declarations,
model documents, observation provisions and replay inputs. They assert the
owning stage's typed result, code and cause where exposed, along with the
specified source or document context. They do not construct diagnostics to
make a code reachable. Bundle and model causes retain their own types rather
than being converted into `CompleteCause`.

| Behavior exercised | Public producer |
| --- | --- |
| Syntax, declaration forms, source identity and UTF-8 | CST parser and source preflight |
| Original edit predecessor, character ranges, foreign nodes and document requests | Editor and CST host entries |
| Cancellation and source/token/node/work limits | Parser and lifecycle entries |
| Missing definitions, cycles, facets, required capabilities, authority conflicts and bundle budgets | Definition catalogue and bundle linking |
| Request I/O, JSON, format dispatch, missing names, typing and definedness | Command compile entry |
| Malformed and noncanonical models, duplicate model selection and abstract instances | Model intake and population admission |
| Foreign models, closure completeness and cardinality | Bound population admission and queries |
| Recorded deltas and permitted creation | Invocation admission |
| Missing snapshots and wrong observation roles | Snapshot admission |
| Present and absent protocol references in complete and incomplete populations | Admitted protocol reference seeds |
| Lost host model input after checking | Observation admission's real internal-fault path |
| Extraction combinations, binding count and Quire context diagnostics | Public clause-run request reader |
| Clock declarations, random weights and fixed sequence bounds | Public S3 compile path |
| Duplicate operation bodies and missing dispatch targets | Source assembly and checking |
| Unsupplied execution backend | Public spine run entry |
| Stale clause identity before observation admission | Public state-clause replay entry |
| Protocol refusal preservation and shorthand projection refusal | Public protocol check and emission entries |

The replay case uses two original checked clauses and genuine emitted package
bytes. Its canonical-input envelope deliberately names the other clause; the
typed refusal retains both identities before reading the unprovided snapshot.
It claims no backend proof. The protocol projection case retains an actual
missing-name source refusal: the shorthand emitter requires the result-preserving
entry, which returns that original refusal and admits no unit package.

## Pending implementation cases

| Owner | Strict expected behavior |
| --- | --- |
| QSL-2687 | A malformed escape reaches its lexical diagnostic before unrelated selection-identity cascades. |
| QSL-2688 | A diagnostic-list cut names the actionable `s1.diagnostics` setting. |
| QSL-2692 | Public observation inputs retain source-declared model type and universe authority. |
| QSL-2693 | An unknown request extension capability retains `unknown_required_feature`, rather than becoming `invalid_package`. |

Known source causes under QSL-2680 through QSL-2685 keep their existing owner
cases. They were not probed again by this feature.

## Authoring receipts

Focused commands used the owned worktree target with jobs2, debug0,
incremental0 and one test thread:

```sh
CARGO_TARGET_DIR="$PWD/target" CARGO_BUILD_JOBS=2 CARGO_INCREMENTAL=0 \
CARGO_PROFILE_DEV_DEBUG=0 CARGO_PROFILE_TEST_DEBUG=0 \
cargo test --locked -p filament-spec-language --test it \
  error_producer_inputs::<qualified-function> -- --exact --test-threads=1 \
  --skip two_protocols_declaring_one_channel_name_keep_their_queues_apart \
  --skip a_recovery_predicate_over_the_recovery_view_is_evaluated
```

The segmented accepted record is 32 active groups passing. Logs retain both
original failed authoring attempts and the focused corrections; their counts
must not be added together and called a component gate. Receipts retained in
the local `.agent/` directory include:

- `qsl-2079-source-host-focused.log`, `qsl-2079-current-producer-corrections.log`,
  `qsl-2079-bundle-compile-model.log` and `qsl-2079-proper-selection-and-limits.log`.
- `qsl-2079-population-producers-corrected.log`, `qsl-2079-invocation-producers.log`,
  `qsl-2079-snapshot-producers.log` and `qsl-2079-extraction-producers-corrected.log`.
- `qsl-2079-semantic-model-producers-corrected.log`,
  `qsl-2079-model-producers-selected-layer.log` and
  `qsl-2079-operation-and-backend-producers.log`.
- `qsl-2079-selected-model-reference-producer.log` and
  `qsl-2079-host-internal-fault-producer.log`.
- `qsl-2079-replay-stale-producer-corrected.log`,
  `qsl-2079-capability-closure-producer-corrected.log` and
  `qsl-2079-protocol-projection-producer-profile-corrected.log`: each one passed,
  exit0.

The single scoped authoring lint command was:

```sh
cargo clippy --locked -p filament-spec-language --test it --no-deps -- -D warnings
```

It exited101 in `qsl-2079-root-producer-clippy.log`. The one new trivial
`err().expect()` finding was corrected. Two unchanged integration findings
remain: `run_wire.rs:1289` (`useless_format`) and
`spine_retained_criteria.rs:72` (`err_expect`). No lint pass is claimed and no
lint retry was run.

## Coverage boundaries

This feature does not establish complete `Code::all()` or `CompleteCause`
reachability. Protected formatter invariants were not fabricated through
opaque CST state. The attempted `PostalState` monitor ambiguity remains
outside the runnable corpus: native I04 admission refused the base as
`ambiguous_declaration` at
`/semantic_graph/nodes/1/declaration/qualified_name`, before the later
`MonitorPlanAmbiguous` guard. Its exact receipt is
`qsl-2079-monitor-projection-correspondence-native-source.log`; the local
prototype is `qsl-2079-monitor-projection-unreachable-draft.rs`. It proves
neither guard coverage nor a separate source/emission contract violation.

Host-only output/serialization failure retains the accepted QSL-2476 L3
contract evidence for that frozen driver cohort. It was not rebuilt or run
again here, and is not evidence for this newer language cohort. No required
code variant was removed, no fake fault or private engine was introduced, and
no upward crate dependency was added.

## Confined review corrections

The ignored QSL-2687 case retains its located first lexical diagnostic and now
rejects unrelated selection/identity errors anywhere in the complete diagnostic
list. It was not run again. The ignored QSL-2692 observation uses source names
and omits the `model` member, as FR-569-AC-9 requires; its known authority
failure was not probed again.

The stale-clause case no longer claims FR-122-AC-3's missing-name criterion.
Producer tests bind the current criteria their assertions exercise. The abstract
instance rule, duplicate selection rule and unknown request capability rule
retain explicit requirement/ADR bindings where the governing behavior has no
matching criterion in this scoped correction; no replacement criterion was
invented. Active runtime assertions and their 32 passing authoring results are
unchanged. These corrections ran no tests, builds or lint.

## Final post-review gate

The gate ran at `7bc410146226ded23cfc77da0e7b86b8cc20a120`, retaining the
same owned target and jobs2/debug0/incremental0 profile. Each test segment
used one thread and both standing skip operands. The disk had 209 GiB free
before the gate. Review records SR-1481 and SR-1482 are retained unchanged.

| Segment | Result | Receipt |
| --- | --- | --- |
| Root `--test it error_producer_inputs::` | 32 passed, 0 failed, 4 ignored; exit0 | `qsl-2079-final-producers.log` |
| Foundation `--test code_catalogue` | 3 passed, 0 failed, 0 ignored; exit0 | `qsl-2079-final-catalogue.log` |
| Whitespace check of the two changed docs | exit0 | `qsl-2079-final-docs.log` |

The root command is the authoring command above with module filter
`error_producer_inputs::` and without `--exact`. The foundation command is
`cargo test --locked -p fsl-foundation --test code_catalogue --` followed by
the same thread and skip operands. The docs command was:

```sh
git diff --check f38cb49aebd84b3d8ca54bd86e01db7c298c01df..HEAD -- \
  docs/native-error-codes.md docs/qsl-2079-producer-validation.md
```

The catalogue segment also checks the native-code documentation rows. The
Python docs structure checker was not run under the repository's no-Python
rule; the independent review already inspected docs style and accuracy.
The aggregate scoped gate passed. The three catalogue tests remain metadata
checks and are not counted as additional producer coverage. Original focused
failures, the failed lint receipt, four owned ignored cases and the coverage
boundaries above remain unchanged. No unrelated component, product binary,
workspace, proof or lint gate ran.
