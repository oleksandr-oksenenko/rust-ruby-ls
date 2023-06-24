use std::cmp::Reverse;
use std::path::Path;
use std::sync::Arc;

use fuzzy_matcher::skim::SkimMatcherV2;
use fuzzy_matcher::FuzzyMatcher;

use crate::indexer::RSymbol;

pub struct SymbolsMatcher<'a> {
    matcher: SkimMatcherV2,
    root_path: &'a Path,
}

impl<'a> SymbolsMatcher<'a> {
    pub fn new(root_path: &'a Path) -> SymbolsMatcher {
        SymbolsMatcher {
            matcher: SkimMatcherV2::default().smart_case(),
            root_path,
        }
    }

    pub fn match_rsymbols(&self, query: &str, symbols: &[Arc<RSymbol>]) -> Vec<Arc<RSymbol>> {
        let mut scores: Vec<(Arc<RSymbol>, [i32; 5])> = symbols
            .iter()
            .filter_map(|s| {
                let name = s.name();

                match self.matcher.fuzzy_indices(name, query) {
                    None => None,
                    Some((score, indices)) => {
                        let start = *indices.first().unwrap_or(&0);
                        let end = *indices.last().unwrap_or(&0);
                        let len = name.len();

                        let s_path = s.file();
                        let in_root = if s_path.starts_with(self.root_path) { 1 } else { -1 };

                        let rank = [score as i32, in_root, -(start as i32), -(end as i32), -(len as i32)];

                        Some((s.clone(), rank))
                    }
                }
            })
            .map(|m| (m.0, m.1))
            .collect();

        scores.sort_by_key(|m| Reverse(m.1));

        scores.iter().map(|m| m.0.clone()).collect()
    }
}
