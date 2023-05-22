use std::cmp::Reverse;

use lsp_types::SymbolInformation;

use fuzzy_matcher::FuzzyMatcher;
use fuzzy_matcher::skim::SkimMatcherV2;

use rayon::prelude::*;

pub struct SymbolsMatcher {
    matcher: SkimMatcherV2
}

impl SymbolsMatcher {
    pub fn new() -> SymbolsMatcher {
        SymbolsMatcher {
            matcher: SkimMatcherV2::default()
        }
    }

    pub fn match_symbols<'a>(&self, query: &str, symbols: &[&'a SymbolInformation]) -> Vec<&'a SymbolInformation> {
        let mut scores: Vec<(&SymbolInformation, i64)> = symbols.par_iter()
            .map(|s| (*s, self.matcher.fuzzy_match(&s.name, query)))
            .filter(|m| m.1.is_some())
            .map(|m| (m.0, m.1.unwrap()))
            .collect();

        scores.sort_by_key(|m| Reverse(m.1));

        scores.iter()
            .map(|m| m.0)
            .collect()
    }
}
