// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-820: the letter tape of a carried clause state. Each admitted position
//! is read once, when it arrives, into its letter: the value of every atom
//! of the clause there, the `on each` guard and whether its captures are
//! fixed. A later evaluation reads the letter, never the position's value, so
//! the state holds a position by value only where a reading did not reduce to
//! a letter ([`TapeEntry::Held`]).

use filament_exact::{Meter, Value};
use fsl_package::CheckedPackage;
use fsl_semantics::check::{FormulaNode, TemporalClauseNode};

use super::letters::{AtomReading, CaptureReading, TemporalLetters, UndefinedCause};
use super::observed::{ObservedLetters, ObservedPositions};
use super::window::InsufficientData;
use super::{TemporalPosition, TemporalTrace};

/// One atom's value in a letter: a truth value, or a window aggregate that
/// held fewer values than its `min_count` (ADR-024 WA-4).
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TapeReading {
    /// The atom is true.
    True,
    /// The atom is false.
    False,
    /// The aggregate's window held too few values.
    Insufficient(InsufficientData),
}

impl TapeReading {
    /// The reading as the evaluator reads it.
    pub fn reading(self) -> AtomReading {
        match self {
            Self::True => AtomReading::True,
            Self::False => AtomReading::False,
            Self::Insufficient(data) => {
                AtomReading::Undefined(UndefinedCause::InsufficientData(data))
            }
        }
    }

    /// The reading of `reading`, when it is a letter: `None` for an undefined
    /// reading that is not a data shortfall, which the tape does not summarize.
    pub fn of(reading: &AtomReading) -> Option<Self> {
        match reading {
            AtomReading::True => Some(Self::True),
            AtomReading::False => Some(Self::False),
            AtomReading::Undefined(UndefinedCause::InsufficientData(data)) => {
                Some(Self::Insufficient(*data))
            }
            AtomReading::Undefined(_) => None,
        }
    }
}

/// The letter of one position: each atom's reading by formula node index, the
/// `on each` guard's truth (absent for an unguarded clause) and whether the
/// clause's captures were fixed there.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct TapeLetter {
    atoms: Vec<(usize, TapeReading)>,
    activation: Option<bool>,
    captures: bool,
}

impl TapeLetter {
    /// A letter of `atoms` readings, by node index, with the guard's truth.
    pub fn new(atoms: Vec<(usize, TapeReading)>, activation: Option<bool>, captures: bool) -> Self {
        Self {
            atoms,
            activation,
            captures,
        }
    }

    /// The reading of the atom at node `index`.
    pub fn atom(&self, index: usize) -> Option<TapeReading> {
        self.atoms
            .iter()
            .find(|(node, _)| *node == index)
            .map(|(_, reading)| *reading)
    }

    /// Every atom reading, by node index.
    pub fn atoms(&self) -> &[(usize, TapeReading)] {
        &self.atoms
    }

    /// The guard's truth, for a guarded clause.
    pub fn activation(&self) -> Option<bool> {
        self.activation
    }

    /// Whether the clause's captures were fixed at the position.
    pub fn captures(&self) -> bool {
        self.captures
    }
}

/// One position of a tape.
#[derive(Clone, Debug)]
pub enum TapeEntry {
    /// The position's letter, read at admission.
    Read(TapeLetter),
    /// The position held by value: some reading there did not reduce to a
    /// letter (an undefined atom, a refused condition or a failed read), so an
    /// evaluation reads the value again exactly as the batch run does.
    Held(Value),
}

/// FR-820: the letters of the positions a carried clause state has seen, in
/// trace order.
#[derive(Clone, Debug, Default)]
pub struct LetterTape {
    entries: Vec<TapeEntry>,
    /// The number of entries held by value, kept as entries are pushed and
    /// dropped so reading it does not walk the tape.
    held: u64,
}

impl LetterTape {
    /// An empty tape.
    pub fn new() -> Self {
        Self::default()
    }

    /// Append the next position.
    pub fn push(&mut self, entry: TapeEntry) {
        if matches!(entry, TapeEntry::Held(_)) {
            self.held = self.held.saturating_add(1);
        }
        self.entries.push(entry);
    }

    /// Keep the first `len` positions, undoing the advances after them.
    pub fn truncate(&mut self, len: usize) {
        while self.entries.len() > len {
            if let Some(TapeEntry::Held(_)) = self.entries.pop() {
                self.held = self.held.saturating_sub(1);
            }
        }
    }

    /// The number of positions.
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Whether the tape has no position.
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// The entry at `index`.
    pub fn entry(&self, index: u64) -> Option<&TapeEntry> {
        usize::try_from(index)
            .ok()
            .and_then(|index| self.entries.get(index))
    }

    /// Every entry, in trace order.
    pub fn entries(&self) -> &[TapeEntry] {
        &self.entries
    }

    /// The number of positions held by value.
    pub fn held(&self) -> u64 {
        self.held
    }
}

/// FR-820: read the record sample `sample` into its letter for an evaluation
/// that reads samples through FR-330's observed letters (the infinite-trace
/// route, and FR-504's open-scope reading of a closed-trace clause): each
/// `holds` atom, the guard of a guarded clause and the captures of a captured
/// clause, each read once on `meter`. A reading that does not reduce to a
/// letter (an undefined atom, a refused condition, a failed or stopped read)
/// keeps the sample by value. Window aggregates are not read here.
pub fn read_sample_letter(
    package: &CheckedPackage,
    clause: &TemporalClauseNode,
    sample: &Value,
    meter: &mut Meter,
) -> TapeEntry {
    let held = || TapeEntry::Held(sample.clone());
    let samples = std::slice::from_ref(sample);
    let mut letters = ObservedLetters::over(
        package,
        clause,
        ObservedPositions::Samples(samples),
        TemporalTrace::Finite { len: 1 },
    );
    let at = TemporalPosition::new(0);
    let checked = clause.clause();
    let formula = checked.formula();
    let mut atoms = Vec::new();
    for (index, node) in formula.nodes().iter().enumerate() {
        match node {
            FormulaNode::Holds { .. } => {
                let Some(id) = formula.node_id(index) else {
                    return held();
                };
                let Ok(reading) = letters.atom(id, at, meter) else {
                    return held();
                };
                let Some(letter) = TapeReading::of(&reading) else {
                    return held();
                };
                atoms.push((index, letter));
            }
            // An aggregate reads a window of values, not one sample.
            FormulaNode::Aggregate { .. } => return held(),
            _ => {}
        }
    }
    let activation = match checked.activation() {
        fsl_semantics::check::Activation::Each { guarded: true } => {
            match letters.activation(at, meter) {
                Ok(AtomReading::True) => Some(true),
                Ok(AtomReading::False) => Some(false),
                _ => return held(),
            }
        }
        _ => None,
    };
    let captures = if checked.captures().len() == 0 {
        false
    } else {
        match letters.captures(at, meter) {
            Ok(CaptureReading::Fixed) => true,
            _ => return held(),
        }
    };
    TapeEntry::Read(TapeLetter::new(atoms, activation, captures))
}
