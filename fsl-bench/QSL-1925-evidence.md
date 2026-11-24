# QSL-1925 evidence and disposition

Owner criteria: NFR-141-AC-13 and NFR-141-AC-14.

`filament-exact` already caches each `NodeKey`'s structural hash in its
`NodeData` when the key is constructed. `NodeKey::hash` writes that cached
value, and equality checks it before structural comparison. The dependency
revision used by this checkout is `08a5a1763ef4bd1ea7d1147717c959af5a3522cc`;
see `src/node.rs` (`NodeData`, `structural_hash`, and `impl Hash for
NodeKey`). FSL cannot add the requested construction-time cache to this
external type without changing `filament-exact`; this ticket does not add a
second wrapper cache. `BodyTerm` is the checked body representation and is
not the structural key used by the checker maps.

`checker/object_chain` measures `TypeEnvironment` admission from
`fsl-bench/src/check.rs::object_chain` and `admit_object_types`. Its generated
chain has one field per type, and each later type inherits the prior chain.
The flattened output therefore contains `n(n+1)/2` slots; the benchmark
definition in `fsl-bench/benches/checker.rs` records that admission work grows
with the flattened slot count. This is the source of the near-quadratic
scaling. The admission implementation is in the external
`filament-semantic-value` crate, so the cause is outside QSL-1925's ownership
and is not changed here.

The QSL-owned optimization in this change uses a fixed deterministic hasher
only for `key_order`'s internal integer allocation/class indices. Those maps
are used for membership/lookup; their iteration order is not observed. Node
ordering and checked outputs remain defined by the existing `NodeKey` order.

Live five-round A/B results, output comparisons, and refusal-boundary results
remain pending for the code lead's qualification run. No speedup is claimed
by this change until that run is recorded.
