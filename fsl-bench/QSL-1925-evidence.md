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

## Live five-round A/B qualification

Baseline is `608e201178a28f96a0b4e738114ce7227cb66c7b`; candidate is
`3646e16cde09f5017df3c36429705d4684bde740`. The runs used separate release
targets, alternated A then B for each round, and ran on Linux x86_64 with
rustc 1.99.1. Each Criterion benchmark used ten samples. Times below are
Criterion point estimates in milliseconds; every round is retained in order.

The noise limit for each row is the maximum of A's MAD/median, B's
MAD/median, and the widest observed Criterion confidence-interval
half-width/estimate among the ten live observations. The NFR-141-AC-14 rule
is `B median <= A median * (1 + noise)`.

| Axis | A rounds 1–5 (ms) | B rounds 1–5 (ms) | A median | B median | Noise | B/A | Limit | Result |
| --- | --- | --- | ---: | ---: | ---: | ---: | ---: | --- |
| `checker/chain/250` | 13.047, 23.380, 28.482, 6.0711, 9.6154 | 18.065, 23.455, 14.497, 28.858, 11.987 | 13.047 | 18.065 | 1.2072 | 1.3846 | 2.2072 | Pass |
| `checker/chain/1000` | 119.86, 121.15, 61.967, 36.752, 194.26 | 29.962, 39.071, 39.190, 87.457, 27.253 | 119.86 | 39.071 | 0.7251 | 0.3260 | 1.7251 | Pass |
| `checker/chain/2000` | 157.58, 320.42, 334.43, 100.94, 290.74 | 50.588, 122.46, 180.56, 105.49, 49.184 | 290.74 | 105.49 | 1.3065 | 0.3628 | 2.3065 | Pass |
| `checker/independent/1000` | 93.189, 40.255, 117.55, 53.513, 115.33 | 23.037, 28.251, 103.07, 79.358, 102.87 | 93.189 | 79.358 | 1.1935 | 0.8516 | 2.1935 | Pass |
| `checker/independent/5000` | 714.21, 542.58, 613.25, 191.35, 440.73 | 160.09, 144.07, 234.36, 954.25, 138.99 | 542.58 | 160.09 | 1.1035 | 0.2951 | 2.1035 | Pass |
| `checker/object_chain/500` | 20.761, 26.121, 39.095, 25.355, 39.635 | 51.213, 9.2213, 19.371, 47.266, 54.226 | 26.121 | 47.266 | 1.8104 | 1.8095 | 2.8104 | Pass |
| `checker/object_chain/1000` | 179.22, 42.516, 139.67, 121.41, 80.165 | 55.776, 117.90, 307.21, 231.80, 126.63 | 121.41 | 126.63 | 1.4140 | 1.0430 | 2.4140 | Pass |
| `checker/object_chain/5000` (default-budget refusal) | 700.86, 504.17, 783.36, 677.33, 606.60 | 371.16, 579.74, 581.20, 633.10, 433.30 | 677.33 | 579.74 | 1.4879 | 0.8559 | 2.4879 | Pass |
| `checker/text_cluster/checked/3` | 0.089509, 0.43320, 0.26906, 0.12308, 0.10578 | 0.15362, 0.14866, 0.24691, 0.34296, 0.14583 | 0.12308 | 0.15362 | 1.6223 | 1.2481 | 2.6223 | Pass |
| `checker/text_cluster/checked/4` | 0.13323, 0.24920, 0.94036, 0.34285, 0.38678 | 0.21764, 0.31052, 0.14334, 0.56541, 0.26135 | 0.34285 | 0.26135 | 1.1582 | 0.7623 | 2.1582 | Pass |
| `checker/text_cluster/checked/5` | 0.49000, 2.5547, 3.5355, 3.0161, 0.33806 | 0.99856, 0.84155, 0.38897, 1.6363, 0.89276 | 2.5547 | 0.89276 | 1.4592 | 0.3495 | 2.4592 | Pass |
| `checker/text_cluster/checked/6` | 4.8311, 3.7468, 1.5636, 9.7501, 2.8963 | 6.5625, 1.8267, 2.2896, 2.3988, 4.3376 | 3.7468 | 2.3988 | 1.4488 | 0.6402 | 2.4488 | Pass |
| `checker/text_cluster/checked/7` | 16.133, 67.278, 14.828, 32.206, 19.752 | 52.462, 18.180, 15.192, 52.327, 22.365 | 19.752 | 22.365 | 1.1557 | 1.1323 | 2.1557 | Pass |
| `checker/text_cluster/checked/8` | 381.90, 376.37, 142.20, 535.18, 660.85 | 183.78, 450.03, 214.58, 444.51, 253.70 | 381.90 | 253.70 | 1.0968 | 0.6643 | 2.0968 | Pass |
| `checker/text_cluster/refused/9` | 71.747, 110.84, 47.535, 122.26, 21.145 | 93.277, 103.36, 56.512, 65.058, 37.360 | 71.747 | 65.058 | 1.2936 | 0.9068 | 2.2936 | Pass |

The live session load averages (1/5/15 minute) before and after each paired
round were: R1 `39.74/48.45/45.67` -> `26.69/34.15/40.03`; R2
`28.04/34.01/39.89` -> `23.46/23.32/31.47`; R3 `21.98/23.02/31.32` ->
`23.00/25.35/29.13`; R4 `23.00/25.35/29.13` -> `31.65/28.46/28.42`; R5
`29.52/28.07/28.30` -> `32.93/37.79/33.60`.

The measured exponents from median endpoints were `checker/chain`: A 1.50,
B 0.85 (250 to 2,000); `checker/independent`: A 1.09, B 0.44 (1,000 to
5,000); `checker/text_cluster`: A 8.20, B 7.56 (3 to 8 records);
`checker/object_chain`: A 2.22, B 1.42 (500 to 1,000). The high measured
noise is included above; small apparent regressions at individual sizes
remain within the specified session-noise bound.

For output and boundary comparison, a temporary Rust snapshot utility was
run against both revisions. The exact checked function identities and
checked type-node Debug rows had identical output (SHA-256
`54b9ead04779ec051a28dce1639ccfced12a3c664c16ac0361b035eda9651eed`). Both
revisions checked every chain and independent input, checked text clusters
3 through 8, refused the 9-record text cluster, admitted object chains 500
and 1,000, and refused the default-budget 5,000-type chain. The
`checker/object_chain/5000` timed row is that default-budget refusal; the
separate unbudgeted 5,000-type admission benchmark was excluded. The utility
was temporary and is not part of the change.
