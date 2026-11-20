# QSL-2090 authoring validation

One worktree target, jobs 2, dev/test debug 0, incremental 0. All runtime
commands use one test thread and both prescribed skips. The root integration
target uses `--locked --features checked-handoff --test it`; the analysis
target uses `--locked -p fsl-analyze --test it`. Disk before the initial build
was 210 GiB available. No process remains running at handoff.

## Accepted active cases

Twenty distinct new active functions have passing focused receipts: nineteen
root integration functions and one real source/model terminal-lasso function.
Four strict pending functions are excluded from coverage. Passing cases were
not repeated unchanged; later execution of two already passing replay cases
backs new settlement/disagreement assertions.

Private receipts are under `/home/peter/dev/.agent/qsl-2090/logs/`:

- `initial-lowering-calls.log`, exit 101: own unsupported CallIncomplete
  category-method assumption; no tests ran. Corrected before execution.
- `corrected-lowering-calls.log`, exit 101: 7 passed, 12 failed. Own fixtures
  used reserved names, the wrong wire discriminator/node tag, and the wrong
  accounting-setting spelling. One actual widening/strict-reader failure is
  QSL-2709. Exact original failures remain recorded.
- `changed-lowering-replay.log`, exit 101: only changed/new cases, 10 passed,
  2 failed, 1 pending ignored. The failures were own raw wire-id comparison
  and an input-admission envelope assumption.
- `changed-two.log`, exit 101: strengthened Boolean refusal assertion passed;
  the operand diagnostic localized the own raw wire-id comparison error.
  Current FSpec FR-322 requires node-held operand slots to resolve through
  their owner before comparison with document-held graph indexes.
- `final-authoring-changed-five.log`, exit 101: 4 passed, 1 unpublished invalid
  fixture failed, 1 pending ignored. The four passes include the corrected
  typed resolved-operand oracle and strengthened replay settlement checks.
  The invalid attempt to reach a kernel integer-conversion record through
  Linked source was correctly refused by S3 UnprovedRange; that unpublished
  fixture and its helper were withdrawn, with no bug or coverage claimed.
- `source-model.log`, exit 101: actual terminal-lasso case passed; two bounded
  source/model cases reached the QSL-2714 internal failures. Short-item and
  original infinite-counterpart assertions were not reached and are not
  passing evidence.
- `infinite-operator.log`, exit 101: the separate original infinite-profile
  bounded-operator case also reached QSL-2714; pending and not passing.
- `strengthened-operands.log`, exit 0: only the two changed local-binding and
  arithmetic functions, 2 passed and 1 known pending ignored. New assertions
  require exact `a`/`b` operand order, distinct `a`/`y` binding names, and the
  expected nested operation behind each unary operand, through typed resolved
  structural keys. Other passing cases were excluded.

Each `.log` has its own actual `.exit` receipt. The final branch does not
claim a green aggregate gate: one independent review and one complete
affected-component gate are still reserved for the lead. The two components
are root `lowering_intake_boundaries::` and analysis
`source_model_boundaries::`.

Leaf rustfmt checks and `git diff --check` emitted no diagnostics. Known
unchanged root/semantics/analyze lint limitations were accepted by the lead;
no repeated Clippy rejection, suppression, or unrelated production edit was
made. There is no green lint claim. No matrix, shared product build, proof
gate, main-move gate, or confidence rerun was performed.
