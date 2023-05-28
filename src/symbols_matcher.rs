use std::cmp::Reverse;
use std::path::Path;

use lsp_types::SymbolInformation;

use fuzzy_matcher::skim::SkimMatcherV2;
use fuzzy_matcher::FuzzyMatcher;

pub struct SymbolsMatcher<'a> {
    matcher: SkimMatcherV2,
    root_path: &'a Path
}

impl<'a> SymbolsMatcher<'a> {
    pub fn new(root_path: &'a Path) -> SymbolsMatcher {
        SymbolsMatcher {
            matcher: SkimMatcherV2::default().smart_case(),
            root_path
        }
    }

    pub fn match_symbols<'b, I>(&self, query: &str, symbols: I) -> Vec<&'b SymbolInformation>
    where
        I: IntoIterator<Item = &'b SymbolInformation>,
    {
        let mut scores: Vec<(&SymbolInformation, [i32; 5])> = symbols
            .into_iter()
            .filter_map(|s| {
                let name = &s.name;

                match self.matcher.fuzzy_indices(name, query) {
                    None => None,
                    Some((score, indices)) => {
                        let start = *indices.first().unwrap_or(&0);
                        let end = *indices.last().unwrap_or(&0);
                        let len = name.len();

                        let s_path = s.location.uri.to_file_path().unwrap();
                        let in_root = if s_path.starts_with(self.root_path) { 1 } else { -1 };


                        let rank = [score as i32, in_root, -(start as i32), -(end as i32), -(len as i32)];

                        Some((s, rank))
                    }
                }

            })
            .map(|m| (m.0, m.1))
            .collect();

        scores.sort_by_key(|m| { Reverse(m.1) });

        scores.iter().map(|m| m.0).collect()
    }
}