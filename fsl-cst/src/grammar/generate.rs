// SPDX-License-Identifier: AGPL-3.0-or-later
//! Grammar-derived source inputs for downstream property tests.
//!
//! This module is available only with `test-support`. It derives complete
//! units directly from the parser's rule table; no parallel grammar is kept.

use std::collections::BTreeMap;

use super::{complete_v1, Grammar, Rule, Terminal};
use crate::Production;

const INFINITE: usize = usize::MAX / 4;

/// One bounded derivation of a complete unit containing a requested production.
#[derive(Clone, Debug)]
pub struct Derivation {
    /// Production whose rule was expanded inside this unit.
    pub target: Production,
    /// Source derived from the real complete-V1 grammar.
    pub source: String,
}

fn cost(rule: &Rule, costs: &BTreeMap<Production, usize>) -> usize {
    match rule {
        Rule::Terminal(_) => 1,
        Rule::Production(p) => costs.get(p).copied().unwrap_or(INFINITE),
        Rule::Sequence(rules) => rules
            .iter()
            .map(|rule| cost(rule, costs))
            .fold(0usize, |a, b| a.saturating_add(b).min(INFINITE)),
        Rule::Choice(rules) => rules
            .iter()
            .map(|rule| cost(rule, costs))
            .min()
            .unwrap_or(INFINITE),
        Rule::Optional(_) => 0,
        Rule::Repeat { rule, minimum, .. } => {
            cost(rule, costs).saturating_mul(*minimum).min(INFINITE)
        }
    }
}

fn targeted_cost(
    rule: &Rule,
    normal: &BTreeMap<Production, usize>,
    targeted: &BTreeMap<Production, usize>,
) -> usize {
    match rule {
        Rule::Terminal(_) => INFINITE,
        Rule::Production(p) => targeted.get(p).copied().unwrap_or(INFINITE),
        Rule::Sequence(rules) => rules
            .iter()
            .enumerate()
            .map(|(selected, _)| {
                rules
                    .iter()
                    .enumerate()
                    .map(|(index, rule)| {
                        if index == selected {
                            targeted_cost(rule, normal, targeted)
                        } else {
                            cost(rule, normal)
                        }
                    })
                    .fold(0usize, |a, b| a.saturating_add(b).min(INFINITE))
            })
            .min()
            .unwrap_or(INFINITE),
        Rule::Choice(rules) => rules
            .iter()
            .map(|rule| targeted_cost(rule, normal, targeted))
            .min()
            .unwrap_or(INFINITE),
        Rule::Optional(rule) => targeted_cost(rule, normal, targeted),
        Rule::Repeat { rule, minimum, .. } => targeted_cost(rule, normal, targeted)
            .saturating_add(cost(rule, normal).saturating_mul(minimum.saturating_sub(1)))
            .min(INFINITE),
    }
}

struct Builder {
    grammar: Grammar,
    normal: BTreeMap<Production, usize>,
    targeted: BTreeMap<Production, usize>,
    target: Production,
}

impl Builder {
    fn render(&self, rule: &Rule, seek: bool, tokens: &mut Vec<String>) {
        match rule {
            Rule::Terminal(terminal) => {
                let token = match terminal {
                    Terminal::Exact(word)
                    | Terminal::Literal(word)
                    | Terminal::ContextualWord(word) => (*word).to_owned(),
                    Terminal::TextValue(value) => format!("{value:?}"),
                    Terminal::Text => "\"x\"".to_owned(),
                    Terminal::UnsignedInteger | Terminal::Bound => "1".to_owned(),
                    Terminal::DecimalLiteral => "1.0".to_owned(),
                    Terminal::ClockType { word, .. } => (*word).to_owned(),
                    Terminal::Identifier
                    | Terminal::CallableName(_)
                    | Terminal::MemberName
                    | Terminal::TypeIdentifier { .. } => "sample".to_owned(),
                    Terminal::ExtensionKeyword(word)
                    | Terminal::ExtensionClause(word)
                    | Terminal::ExtensionSeparator(word) => word.to_string(),
                    Terminal::End | Terminal::Peek(_) => return,
                };
                tokens.push(token);
            }
            Rule::Production(production) => {
                let start = tokens.len();
                self.render(
                    &self.grammar[production],
                    seek && *production != self.target,
                    tokens,
                );
                if matches!(production, Production::Hex32 | Production::Hex64) {
                    let hex = tokens.drain(start..).collect::<String>();
                    tokens.push(hex);
                }
            }
            Rule::Sequence(rules) => {
                let selected = seek
                    .then(|| {
                        rules
                            .iter()
                            .enumerate()
                            .min_by_key(|(index, _)| {
                                rules
                                    .iter()
                                    .enumerate()
                                    .map(|(other, rule)| {
                                        if *index == other {
                                            targeted_cost(rule, &self.normal, &self.targeted)
                                        } else {
                                            cost(rule, &self.normal)
                                        }
                                    })
                                    .fold(0usize, |a, b| a.saturating_add(b).min(INFINITE))
                            })
                            .map(|(index, _)| index)
                    })
                    .flatten();
                for (index, rule) in rules.iter().enumerate() {
                    self.render(rule, selected == Some(index), tokens);
                }
            }
            Rule::Choice(rules) => {
                if let Some(rule) = rules.iter().min_by_key(|rule| {
                    if seek {
                        targeted_cost(rule, &self.normal, &self.targeted)
                    } else {
                        cost(rule, &self.normal)
                    }
                }) {
                    self.render(rule, seek, tokens);
                }
            }
            Rule::Optional(rule) => {
                if seek {
                    self.render(rule, true, tokens);
                }
            }
            Rule::Repeat { rule, minimum, .. } => {
                for index in 0..(*minimum).max(usize::from(seek)) {
                    self.render(rule, seek && index == 0, tokens);
                }
            }
        }
    }
}

/// Derive one size-minimal complete-unit context per grammar-table production.
///
/// The finite least-cost calculation bounds recursion without filtering parser
/// results. The caller must check admission and actual CST coverage: ambiguous
/// grammar alternatives can derive identical source, so a derivation alone is
/// deliberately not reported as parser coverage.
pub fn covering_derivations() -> Vec<Derivation> {
    let grammar = complete_v1();
    let mut normal = BTreeMap::new();
    for _ in 0..grammar.len() {
        let previous = normal.clone();
        for (production, rule) in &grammar {
            normal.insert(
                *production,
                cost(rule, &previous).saturating_add(1).min(INFINITE),
            );
        }
        if normal == previous {
            break;
        }
    }
    let mut output = Vec::new();
    for target in grammar.keys().copied() {
        let mut targeted = BTreeMap::from([(target, normal[&target])]);
        for _ in 0..grammar.len() {
            let previous = targeted.clone();
            for (production, rule) in &grammar {
                if *production != target {
                    targeted.insert(
                        *production,
                        targeted_cost(rule, &normal, &previous)
                            .saturating_add(1)
                            .min(INFINITE),
                    );
                }
            }
            if targeted == previous {
                break;
            }
        }
        if targeted
            .get(&Production::CompleteUnit)
            .copied()
            .unwrap_or(INFINITE)
            >= INFINITE
        {
            continue;
        }
        let builder = Builder {
            grammar: complete_v1(),
            normal: normal.clone(),
            targeted,
            target,
        };
        let mut tokens = Vec::new();
        builder.render(
            &Rule::Production(Production::CompleteUnit),
            true,
            &mut tokens,
        );
        output.push(Derivation {
            target,
            source: tokens.join(" "),
        });
    }
    output
}
