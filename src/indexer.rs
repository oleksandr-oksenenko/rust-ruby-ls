use std::ffi::OsStr;
use std::path::{Path, PathBuf};

use std::rc::Rc;
use std::sync::Arc;
use std::time::Instant;

use anyhow::Result;

use log::info;
use rayon::prelude::*;
use walkdir::WalkDir;

use crate::parsers_v2::general::{read_file_tree, parse};
use crate::progress_reporter::ProgressReporter;
use crate::ruby_env_provider::RubyEnvProvider;
use crate::ruby_filename_converter::RubyFilenameConverter;

use crate::types::RSymbolV2;

pub struct Indexer<'a> {
    root_dir: PathBuf,
    progress_reporter: Rc<ProgressReporter<'a>>,
    ruby_env_provider: Rc<RubyEnvProvider>,
    ruby_filename_converter: Rc<RubyFilenameConverter>,
}

impl<'a> Indexer<'a> {
    pub fn new(
        root_dir: &Path,
        progress_reporter: Rc<ProgressReporter<'a>>,
        ruby_env_provider: Rc<RubyEnvProvider>,
        ruby_filename_converter: Rc<RubyFilenameConverter>,
    ) -> Indexer<'a> {
        let root_dir = root_dir.to_path_buf();

        Indexer {
            ruby_env_provider,
            ruby_filename_converter,
            root_dir,
            progress_reporter,
        }
    }

    pub fn index(&mut self) -> Result<Vec<Arc<RSymbolV2>>> {
        let start = Instant::now();
        let stubs_dir = self.ruby_env_provider.stubs_dir()?;
        let gems_dir = self.ruby_env_provider.gems_dir()?;
        
        let symbols = [stubs_dir.as_ref(), gems_dir.as_ref(), Some(&self.root_dir)]
            .into_iter()
            .flatten()
            .flat_map(|d| self.index_dir(d))
            .flatten()
            .collect::<Vec<Arc<RSymbolV2>>>();

        info!("Found {} symbols, took {:?}", symbols.len(), start.elapsed());

        Ok(symbols)
    }

    fn index_dir(&self, dir: &Path) -> Result<Vec<Arc<RSymbolV2>>> {
        let progress_token = self.progress_reporter.send_progress_begin(format!("Indexing {dir:?}"), "", 0)?;

        let classes: Vec<Arc<RSymbolV2>> = WalkDir::new(dir)
            .into_iter()
            .par_bridge()
            .filter_map(Result::ok)
            .filter(|e| !e.file_type().is_dir())
            .filter(|e| "rb" == e.path().extension().and_then(OsStr::to_str).unwrap_or(""))
            .flat_map(|entry| Self::index_file_cursor(entry.into_path()).unwrap())
            .collect();

        self.progress_reporter.send_progress_end(progress_token, format!("Indexing of {dir:?}"))?;

        Ok(classes)
    }

    fn index_file_cursor(path: PathBuf) -> Result<Vec<Arc<RSymbolV2<'a>>>> {
        let (tree, source) = read_file_tree(&path)?;
        let mut result: Vec<Arc<RSymbolV2>> = Vec::new();
        let mut cursor = tree.walk();
        loop {
            let node = cursor.node();
            let source = &source[..];

            if node.kind() == "program" {
                cursor.goto_first_child();
            }

            let mut parsed = parse(&path, source, cursor.node(), None);
            result.append(&mut parsed);

            if !cursor.goto_next_sibling() {
                break;
            }
        }

        Ok(result)
    }

    pub fn index_file(&self, path: &Path) -> Result<Vec<Arc<RSymbolV2>>> {
        return Self::index_file_cursor(path.to_path_buf());
    }
}
