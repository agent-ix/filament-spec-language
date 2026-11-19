# Refinement and hyperproperty source cases

These original programs reference the existing `fsl-conformance/cases/counter-step-package.json` domain model and initial snapshot. Their operation contract advances the bounded counter cyclically from 0 through 3.

Each `.native` file has a public source compile request and a semantic expectation. The nine `.analyze.json` requests select the named claim through the public analysis command. Tests in `tests/it/refinement_source_routes.rs` assert certificate acceptance for proofs and counterexample publication for refutations.

The identity refinement maps the counter to itself. Changing its step row to stutter or its field mapping to zero remains type correct but violates simulation. Negative compile cases independently remove population, object or step coverage, name an absent abstract operation, or supply a Boolean where a numeric map is required.

Hyperproperties compare two universal traces, universal/existential partner traces and a single existential trace. Equal cyclic counters have equal values, but they cannot stay at zero, begin with different values or reach four. Trace-name and plain-temporal boundary cases must refuse before analysis.

No case records an old unsupported/empty-output result as desired behavior. Copy-swap selection, projection-alignment and finite trace-set cases remain additional QSL-2044 scope after these concrete source journeys.
