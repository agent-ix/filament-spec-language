// SPDX-License-Identifier: AGPL-3.0-or-later
//! The one shared region product-key type (ADR-038 RL-5, FR-920 "Keys").
//!
//! The producer and the checker each build their own region and read its
//! places and fraction classes themselves; this module only validates and
//! writes the bytes (wire format only, FR-921). Nothing here derives a place or
//! a class from a clock valuation, and there is no constructor from one.
//!
//! Places are in base units: every constant is an integer after the FR-196 LCD
//! scale. A clock compared with constants up to `c` has cap `c + 1`. A clock
//! above `c` is saturated, whatever its fraction: the order of fractions
//! matters only among clocks at most `c` (Alur-Dill).

use ix_trace_rs::implements;
use serde_json::{json, Value};

use super::{canonical_json, DigitalTail, ExactProbabilityGraphError, ProbabilityProductKey};

/// Where one clock lies, in base units.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ClockPlace {
    /// The clock equals the integer; at most the cap minus one.
    Integer(u64),
    /// The clock lies strictly between `integer` and `integer + 1`, at most
    /// the cap minus two below the upper end. `class` is the 1-based rank of
    /// its fractional part among the key's distinct nonzero fractions.
    Open {
        /// The integer part.
        integer: u64,
        /// The fraction class, 1-based.
        class: u32,
    },
    /// The clock is above its largest compared constant: encoded as the cap
    /// with fraction class 0.
    Saturated,
}

/// One model clock in a region key.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RegionClock {
    /// The checked declaring type, field and receiver tuple.
    pub identity: Value,
    /// The clock's cap: its largest compared constant plus one, in base units.
    pub cap: u64,
    /// Where the clock lies.
    pub place: ClockPlace,
}

/// A clock the claim adds: a list in the key type so that step 7 adds no wire
/// change. At most one entry is encodable today.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ClaimClock {
    /// The never-reset deadline clock; its cap is `limit + 1`.
    Deadline {
        /// The closed deadline bound in base units.
        limit: u64,
        /// Where the clock lies.
        place: ClockPlace,
    },
    /// The elapsed-time measure clock; `None` before activation. Its cap is
    /// `window + 1`.
    Elapsed {
        /// The measure window in base units.
        window: u64,
        /// Where the clock lies, `None` before activation.
        place: Option<ClockPlace>,
    },
}

impl ClaimClock {
    fn cap(self) -> Option<u64> {
        match self {
            Self::Deadline { limit, .. } => limit.checked_add(1),
            Self::Elapsed { window, .. } => window.checked_add(1),
        }
    }

    fn place(self) -> Option<ClockPlace> {
        match self {
            Self::Deadline { place, .. } => Some(place),
            Self::Elapsed { place, .. } => place,
        }
    }
}

/// The vertex of the closure of a region a boundary-region key names.
///
/// The coordinates follow the clock entries in input order: the model clocks
/// as listed in [`RegionKey::clocks`], then a deadline claim clock. Each is a
/// count of `ticks_per_unit` ticks per base unit (1 once the corner is a
/// vertex in base units); the key writes the counts as decimal strings.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Corner {
    /// One coordinate per clock entry.
    pub coordinates: Vec<u64>,
    /// Ticks per base unit in which the coordinates are counted.
    pub ticks_per_unit: u64,
}

/// The checked semantic components of one region or boundary-region key.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RegionKey {
    /// The checked claim's `over` binding, or `None` when there is no binding.
    pub binding: Option<Value>,
    /// The complete non-clock simulation-state form.
    pub discrete: Value,
    /// FR-196's closed time-unit form.
    pub unit: Value,
    /// The FR-196 LCD scale of the constraint constants.
    pub scale: u64,
    /// Every model clock; the encoder orders them by their identity bytes.
    pub clocks: Vec<RegionClock>,
    /// The clocks the claim adds.
    pub claim_clocks: Vec<ClaimClock>,
    /// TS-5 tail control for the state.
    pub tail: DigitalTail,
    /// The checked claim's monitor state, or `None` when it has no monitor.
    pub monitor: Option<Value>,
    /// The checked claim's exact measure accumulator, if present.
    pub accumulator: Option<Value>,
    /// The actual pending draw control, if present.
    pub pending: Option<Value>,
    /// The corner, for a boundary-region key; `None` for a region key.
    pub corner: Option<Corner>,
}

fn place_is_valid(place: ClockPlace, cap: u64) -> bool {
    match place {
        ClockPlace::Integer(integer) => cap >= 1 && integer <= cap - 1,
        ClockPlace::Open { integer, class } => {
            class >= 1 && cap >= 2 && integer <= cap - 2
        }
        ClockPlace::Saturated => cap >= 1,
    }
}

impl RegionKey {
    /// Check the key's structure: places within caps, classes `1..=K` with no
    /// gap over every open place, identities are 3-element arrays and are
    /// distinct, at most one claim clock, and a corner of the right shape.
    ///
    /// Structural only. Whether a corner is a vertex of the closure of the
    /// region is geometry, which each side tests against its own region.
    #[implements("FR-920-AC-2")]
    pub fn validate(&self) -> Result<(), ExactProbabilityGraphError> {
        let invalid = ExactProbabilityGraphError::InvalidDigitalKey;
        if self.scale == 0 || self.claim_clocks.len() > 1 {
            return Err(invalid);
        }
        let mut classes = Vec::new();
        let mut identities = Vec::with_capacity(self.clocks.len());
        for clock in &self.clocks {
            if clock
                .identity
                .as_array()
                .is_none_or(|parts| parts.len() != 3)
                || !place_is_valid(clock.place, clock.cap)
            {
                return Err(invalid);
            }
            identities.push(canonical_json(&clock.identity)?);
            if let ClockPlace::Open { class, .. } = clock.place {
                classes.push(class);
            }
        }
        identities.sort();
        if identities.windows(2).any(|pair| pair[0] == pair[1]) {
            return Err(invalid);
        }
        for claim in &self.claim_clocks {
            let cap = claim.cap().ok_or(invalid)?;
            if let Some(place) = claim.place() {
                if !place_is_valid(place, cap) {
                    return Err(invalid);
                }
                if let ClockPlace::Open { class, .. } = place {
                    classes.push(class);
                }
            }
        }
        classes.sort_unstable();
        classes.dedup();
        let contiguous = classes
            .iter()
            .zip(1_u32..)
            .all(|(class, expected)| *class == expected);
        if !contiguous {
            return Err(invalid);
        }
        if let Some(corner) = &self.corner {
            let deadline = self
                .claim_clocks
                .iter()
                .any(|claim| matches!(claim, ClaimClock::Elapsed { .. }));
            let entries = self.clocks.len()
                + self
                    .claim_clocks
                    .iter()
                    .filter(|claim| matches!(claim, ClaimClock::Deadline { .. }))
                    .count();
            if deadline || corner.ticks_per_unit == 0 || corner.coordinates.len() != entries {
                return Err(invalid);
            }
            let caps = self
                .clocks
                .iter()
                .map(|clock| Some(clock.cap))
                .chain(
                    self.claim_clocks
                        .iter()
                        .filter(|claim| matches!(claim, ClaimClock::Deadline { .. }))
                        .map(|claim| claim.cap()),
                );
            for (coordinate, cap) in corner.coordinates.iter().zip(caps) {
                let limit = cap
                    .and_then(|cap| cap.checked_mul(corner.ticks_per_unit))
                    .ok_or(invalid)?;
                if *coordinate > limit {
                    return Err(invalid);
                }
            }
        }
        Ok(())
    }

    /// Validate, then write the FR-920 canonical ProductKey.
    #[implements("FR-920-AC-2")]
    pub fn encode(&self) -> Result<ProbabilityProductKey, ExactProbabilityGraphError> {
        self.validate()?;
        let encode_place = |place: ClockPlace, cap: u64| match place {
            ClockPlace::Integer(integer) => (integer, 0),
            ClockPlace::Open { integer, class } => (integer, class),
            ClockPlace::Saturated => (cap, 0),
        };
        let mut order = self
            .clocks
            .iter()
            .enumerate()
            .map(|(index, clock)| {
                canonical_json(&clock.identity).map(|bytes| (bytes, index))
            })
            .collect::<Result<Vec<_>, _>>()?;
        order.sort();
        let clocks = order
            .iter()
            .map(|(_, index)| {
                let clock = &self.clocks[*index];
                let (integer, class) = encode_place(clock.place, clock.cap);
                json!([
                    clock.identity,
                    integer.to_string(),
                    clock.cap.to_string(),
                    class.to_string()
                ])
            })
            .collect::<Vec<_>>();
        let invalid = ExactProbabilityGraphError::InvalidDigitalKey;
        let claim = match self.claim_clocks.first() {
            None => Value::Null,
            Some(ClaimClock::Deadline { limit, place }) => {
                let (integer, class) = encode_place(*place, limit.checked_add(1).ok_or(invalid)?);
                json!([
                    "deadline",
                    limit.to_string(),
                    integer.to_string(),
                    class.to_string()
                ])
            }
            Some(ClaimClock::Elapsed { window, place }) => {
                let cap = window.checked_add(1).ok_or(invalid)?;
                match place.map(|place| encode_place(place, cap)) {
                    Some((integer, class)) => json!([
                        "elapsed",
                        window.to_string(),
                        integer.to_string(),
                        class.to_string()
                    ]),
                    None => json!(["elapsed", window.to_string(), null, "0"]),
                }
            }
        };
        let mut region = vec![
            json!(if self.corner.is_some() {
                "boundary-region"
            } else {
                "region"
            }),
            self.discrete.clone(),
            self.unit.clone(),
            json!(self.scale.to_string()),
            Value::Array(clocks),
            claim,
            json!(self.tail.tag()),
        ];
        if let Some(corner) = &self.corner {
            // The corner lists coordinates in the order of the clock entries
            // as written: the clocks as sorted, then the deadline.
            let mut coordinates = order
                .iter()
                .map(|(_, index)| corner.coordinates[*index].to_string())
                .collect::<Vec<_>>();
            coordinates.extend(
                corner.coordinates[self.clocks.len()..]
                    .iter()
                    .map(u64::to_string),
            );
            region.push(json!(coordinates));
        }
        let product = json!([
            "filament.probability-product-key/v1",
            self.binding,
            null,
            null,
            ["filament.simulation.state-key/v1", region],
            self.monitor,
            self.accumulator,
            self.pending
        ]);
        canonical_json(&product).map(ProbabilityProductKey)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ix_trace_rs::trace;

    fn identity(name: &str) -> Value {
        json!(["Type", name, ["u", "object"]])
    }

    fn base() -> RegionKey {
        RegionKey {
            binding: None,
            discrete: json!([]),
            unit: json!(["unit", "second"]),
            scale: 1,
            clocks: Vec::new(),
            claim_clocks: Vec::new(),
            tail: DigitalTail::Running,
            monitor: Some(json!(["measure", null, "active"])),
            accumulator: None,
            pending: None,
            corner: None,
        }
    }

    fn text(key: &RegionKey) -> String {
        String::from_utf8_lossy(key.encode().expect("encodes").as_bytes()).into_owned()
    }

    fn with_clock(place: ClockPlace, cap: u64) -> RegionKey {
        RegionKey {
            clocks: vec![RegionClock {
                identity: identity("x"),
                cap,
                place,
            }],
            ..base()
        }
    }

    #[trace("FR-920-AC-1")]
    #[test]
    fn a_clock_above_its_constant_is_saturated_with_no_fraction() {
        // cap 3: the largest constant is 2.
        assert!(text(&with_clock(ClockPlace::Saturated, 3)).contains(r#","3","3","0"]"#));
        assert!(text(&with_clock(ClockPlace::Integer(2), 3)).contains(r#","2","3","0"]"#));
        assert_ne!(
            text(&with_clock(ClockPlace::Saturated, 3)),
            text(&with_clock(ClockPlace::Integer(2), 3)),
            "the largest constant is not saturated"
        );
        let deadline = |place| RegionKey {
            claim_clocks: vec![ClaimClock::Deadline { limit: 2, place }],
            ..base()
        };
        // A deadline clock in (2, 3) is the same region as one at 5.
        assert!(text(&deadline(ClockPlace::Saturated)).contains(r#"["deadline","2","3","0"]"#));
        assert!(text(&deadline(ClockPlace::Open {
            integer: 1,
            class: 1
        }))
        .contains(r#"["deadline","2","1","1"]"#));
    }

    #[trace("FR-920-AC-1")]
    #[test]
    fn the_elapsed_measure_clock_encodes_with_its_fraction_and_null_before_activation() {
        let elapsed = |place| RegionKey {
            claim_clocks: vec![ClaimClock::Elapsed { window: 2, place }],
            ..base()
        };
        assert!(text(&elapsed(Some(ClockPlace::Open {
            integer: 1,
            class: 1
        })))
        .contains(r#"["elapsed","2","1","1"]"#));
        assert!(text(&elapsed(None)).contains(r#"["elapsed","2",null,"0"]"#));
        assert_ne!(
            text(&elapsed(Some(ClockPlace::Integer(1)))),
            text(&elapsed(Some(ClockPlace::Open {
                integer: 1,
                class: 1
            }))),
            "a whole clock differs from a fractional one"
        );
        assert_eq!(
            elapsed(Some(ClockPlace::Integer(3))).encode(),
            Err(ExactProbabilityGraphError::InvalidDigitalKey),
            "the clock is saturated above the window"
        );
    }

    #[trace("FR-920-AC-2")]
    #[test]
    fn keys_that_differ_only_in_a_fraction_class_are_distinct() {
        let two = |first, second| RegionKey {
            clocks: vec![
                RegionClock {
                    identity: identity("a"),
                    cap: 4,
                    place: ClockPlace::Open {
                        integer: 1,
                        class: first,
                    },
                },
                RegionClock {
                    identity: identity("b"),
                    cap: 4,
                    place: ClockPlace::Open {
                        integer: 1,
                        class: second,
                    },
                },
            ],
            ..base()
        };
        let (a, b) = (two(1, 2).encode().unwrap(), two(2, 1).encode().unwrap());
        assert_ne!(a, b);
        assert_ne!(a, two(1, 1).encode().unwrap());
    }

    #[trace("FR-920-AC-2")]
    #[test]
    fn validate_refuses_a_class_gap_a_place_above_the_cap_and_a_bad_corner() {
        let refused = ExactProbabilityGraphError::InvalidDigitalKey;
        assert_eq!(
            with_clock(
                ClockPlace::Open {
                    integer: 0,
                    class: 2
                },
                3
            )
            .validate(),
            Err(refused),
            "classes 1..=K with no gap"
        );
        assert_eq!(
            with_clock(
                ClockPlace::Open {
                    integer: 2,
                    class: 1
                },
                3
            )
            .validate(),
            Err(refused),
            "an open place has integer at most cap - 2"
        );
        assert_eq!(
            with_clock(ClockPlace::Integer(3), 3).validate(),
            Err(refused)
        );
        let mut corner = with_clock(ClockPlace::Integer(1), 3);
        corner.corner = Some(Corner {
            coordinates: vec![1, 1],
            ticks_per_unit: 1,
        });
        assert_eq!(corner.validate(), Err(refused), "wrong arity");
        corner.corner = Some(Corner {
            coordinates: vec![4],
            ticks_per_unit: 1,
        });
        assert_eq!(corner.validate(), Err(refused), "above the cap");
        corner.corner = Some(Corner {
            coordinates: vec![3],
            ticks_per_unit: 1,
        });
        assert!(text(&corner).contains("boundary-region"));
    }

    #[trace("FR-920-AC-1")]
    #[test]
    fn the_corner_follows_the_sorted_clock_entries() {
        let mut key = RegionKey {
            clocks: vec![
                RegionClock {
                    identity: identity("b"),
                    cap: 4,
                    place: ClockPlace::Integer(1),
                },
                RegionClock {
                    identity: identity("a"),
                    cap: 4,
                    place: ClockPlace::Integer(2),
                },
            ],
            ..base()
        };
        key.corner = Some(Corner {
            coordinates: vec![10, 20],
            ticks_per_unit: 8,
        });
        // "a" sorts before "b", so its coordinate (20) is written first.
        assert!(text(&key).contains(r#"["20","10"]"#));
    }
}
