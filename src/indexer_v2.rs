use std::collections::HashMap;
use std::ffi::OsStr;
use std::fs;
use std::path::{Path, PathBuf};

use std::sync::Arc;
use std::time::Instant;

use anyhow::Result;

use itertools::Itertools;
use log::{error, info, warn};
use rayon::prelude::*;
use tree_sitter::{Node, Parser, Point, Tree};
use tree_sitter_ruby::language;
use walkdir::WalkDir;

use crate::parsers;
use crate::parsers::{get_context_scope, get_parent_scope_resolution, parse, parse_constant};
use crate::progress_reporter::ProgressReporter;
use crate::ruby_env_provider::RubyEnvProvider;
use crate::ruby_filename_converter::{self, RubyFilenameConverter};
use crate::symbols_matcher::SymbolsMatcher;

pub enum RSymbol {
    Class(RClass),
    Module(RClass),
    Method(RMethod),
    SingletonMethod(RMethod),
    Constant(RConstant),
    Variable(RVariable),
    ClassVariable(RVariable),
}

impl RSymbol {
    pub fn name(&self) -> &str {
        match self {
            RSymbol::Class(class) => &class.name,
            RSymbol::Module(module) => &module.name,
            RSymbol::Method(method) => &method.name,
            RSymbol::SingletonMethod(method) => &method.name,
            RSymbol::Constant(constant) => &constant.name,
            RSymbol::Variable(variable) => &variable.name,
            RSymbol::ClassVariable(variable) => &variable.name,
        }
    }

    pub fn file(&self) -> &Path {
        match self {
            RSymbol::Class(class) => &class.file,
            RSymbol::Module(module) => &module.file,
            RSymbol::Method(method) => &method.file,
            RSymbol::SingletonMethod(method) => &method.file,
            RSymbol::Constant(constant) => &constant.file,
            RSymbol::Variable(variable) => &variable.file,
            RSymbol::ClassVariable(v) => &v.file,
        }
    }

    pub fn location(&self) -> &Point {
        match self {
            RSymbol::Class(class) => &class.location,
            RSymbol::Module(module) => &module.location,
            RSymbol::Method(method) => &method.location,
            RSymbol::SingletonMethod(method) => &method.location,
            RSymbol::Constant(constant) => &constant.location,
            RSymbol::Variable(variable) => &variable.location,
            RSymbol::ClassVariable(variable) => &variable.location,
        }
    }
}

pub struct RClass {
    pub file: PathBuf,
    pub name: String,
    pub location: Point,
    pub scopes: Vec<String>,
    pub superclass_scopes: Vec<String>,
    pub parent: Option<Arc<RSymbol>>,
}

pub struct RMethod {
    pub file: PathBuf,
    pub name: String,
    pub location: Point,
    pub parameters: Vec<RMethodParam>,
    pub parent: Option<Arc<RSymbol>>,
}

pub enum RMethodParam {
    Regular(String),
    Optional(String),
    Keyword(String),
}

pub struct RConstant {
    pub file: PathBuf,
    pub name: String,
    pub location: Point,
    pub parent: Option<Arc<RSymbol>>,
}

pub struct RVariable {
    pub file: PathBuf,
    pub name: String,
    pub location: Point,
    pub parent: Option<Arc<RSymbol>>,
}

struct IndexingContext {
    source: Vec<u8>,
    tree: Tree,
}

impl IndexingContext {
    pub fn new(file_path: &Path) -> Result<IndexingContext> {
        let source = fs::read(file_path)?;

        let mut parser = Parser::new();
        parser.set_language(language())?;
        let parsed = parser.parse(&source[..], None).unwrap();

        Ok(IndexingContext {
            source,
            tree: parsed,
        })
    }
}

pub struct IndexerV2<'a> {
    root_dir: PathBuf,
    progress_reporter: ProgressReporter<'a>,
    ruby_env_provider: RubyEnvProvider,
    ruby_filename_converter: RubyFilenameConverter,
    symbols: Vec<Arc<RSymbol>>,
    file_index: HashMap<PathBuf, Vec<Arc<RSymbol>>>,
}

impl<'a> IndexerV2<'a> {
    pub fn new(root_dir: &Path, progress_reporter: ProgressReporter<'a>) -> IndexerV2<'a> {
        let root_dir = root_dir.to_path_buf();
        let ruby_env_provider = RubyEnvProvider::new(root_dir.clone());
        let ruby_filename_converter = RubyFilenameConverter::new(root_dir.clone(), &ruby_env_provider).unwrap();
        IndexerV2 {
            ruby_env_provider,
            ruby_filename_converter,
            root_dir,
            progress_reporter,
            symbols: Vec::new(),
            file_index: HashMap::new(),
        }
    }

    pub fn fuzzy_find_symbol(&self, query: &str) -> Vec<Arc<RSymbol>> {
        let start = Instant::now();
        let result = if query.is_empty() {
            // optimization to not overload telescope on request without a query
            vec![]
        } else {
            SymbolsMatcher::new(self.root_dir.as_path()).match_rsymbols(query, &self.symbols)
        };

        info!("Finding symbol by {} took {:?}", query, start.elapsed());

        result
    }

    pub fn file_symbols(&self, file: &Path) -> Option<&Vec<Arc<RSymbol>>> {
        self.file_index.get(file)
    }

    pub fn find_definition(&self, file: &Path, position: Point) -> Vec<Arc<RSymbol>> {
        let ctx = IndexingContext::new(file).unwrap();

        let node = ctx.tree.root_node();
        let node = match node.descendant_for_point_range(position, position) {
            None => {
                info!("No node found to determine definition");
                return vec![];
            }
            Some(n) => n,
        };

        let node = match node.kind().try_into() {
            Err(_) => {
                error!("Unknown node kind in find definition: {}", node.kind());
                return vec![];
            }
            Ok(nk) => match nk {
                parsers::NodeKind::Constant => node,
                parsers::NodeKind::Call => node
                    .child_by_field_name(parsers::NodeName::Reciever)
                    .unwrap(),
                _ => {
                    warn!("Find definition of {} node is not supported", node.kind());
                    return vec![];
                }
            },
        };

        // traverse down till we hit the whole symbol name
        let constant_scope = get_parent_scope_resolution(&node, &ctx.source);
        let is_global = constant_scope
            .first()
            .map(|s| *s == parsers::GLOBAL_SCOPE_VALUE)
            .unwrap_or(false);
        let constant_scope = if is_global {
            constant_scope
                .into_iter()
                .skip(1)
                .join(parsers::SCOPE_DELIMITER)
        } else {
            constant_scope.into_iter().join(parsers::SCOPE_DELIMITER)
        };

        let context_scope = get_context_scope(&node, &ctx.source)
            .into_iter()
            .chain([constant_scope.as_str()])
            .join(parsers::SCOPE_DELIMITER);

        let file_scope = self.ruby_filename_converter.path_to_scope(file);
        let mut file_scope = file_scope.unwrap_or(vec![]);
        file_scope.pop();
        let file_scope = file_scope
            .iter()
            .map(|s| s.as_str())
            .chain([constant_scope.as_str()])
            .join(parsers::SCOPE_DELIMITER);

        let symbols = self.symbols.iter().filter(|s| {
            matches!(
                ***s,
                RSymbol::Class(_) | RSymbol::Module(_) | RSymbol::Constant(_)
            )
        });

        if is_global {
            info!("Global scope, searching for {constant_scope}");
            symbols
                .filter_map(|s| {
                    if s.name() == constant_scope {
                        Some(s.clone())
                    } else {
                        None
                    }
                })
                .collect()
        } else {
            info!(
                "Searching for {context_scope} or {file_scope} or {context_scope} in the same file"
            );
            // search in contexts first
            let found_symbols: Vec<Arc<RSymbol>> = symbols
                .clone()
                .filter_map(|s| {
                    let name = s.name();

                    if name == context_scope
                        || name == file_scope
                        || (name == constant_scope && s.file() == file)
                    {
                        Some(s.clone())
                    } else {
                        None
                    }
                })
                .collect();

            // then global
            if found_symbols.is_empty() {
                info!("Haven't found anything, searching for global {constant_scope}");
                symbols
                    .clone()
                    .filter_map(|s| {
                        if constant_scope == s.name() {
                            Some(s.clone())
                        } else {
                            None
                        }
                    })
                    .collect()
            } else {
                found_symbols
            }
        }
    }

    pub fn index(&mut self) -> Result<()> {
        let start = Instant::now();
        let stubs_dir = self.ruby_env_provider.stubs_dir()?;
        let gems_dir = self.ruby_env_provider.gems_dir()?;

        let symbols = [stubs_dir.as_ref(), gems_dir.as_ref(), Some(&self.root_dir)]
            .into_iter()
            .flatten()
            .flat_map(|d| self.index_dir(d))
            .flatten()
            .collect::<Vec<Arc<RSymbol>>>();

        self.symbols = symbols;
        self.build_file_index();

        info!(
            "Found {} symbols, took {:?}",
            self.symbols.len(),
            start.elapsed()
        );

        Ok(())
    }

    fn build_file_index(&mut self) {
        self.file_index = self
            .symbols
            .iter()
            .group_by(|s| s.file().to_path_buf())
            .into_iter()
            .map(|(k, v)| (k, v.cloned().collect()))
            .collect();
    }

    fn index_dir(&self, dir: &Path) -> Result<Vec<Arc<RSymbol>>> {
        let progress_token =
            self.progress_reporter
                .send_progress_begin(format!("Indexing {dir:?}"), "", 0)?;

        let classes: Vec<Arc<RSymbol>> = WalkDir::new(dir)
            .into_iter()
            .par_bridge()
            .filter_map(Result::ok)
            .filter(|e| !e.file_type().is_dir())
            .filter(|e| "rb" == e.path().extension().and_then(OsStr::to_str).unwrap_or(""))
            .flat_map(|entry| Self::index_file_cursor(entry.into_path()).unwrap())
            .collect();

        self.progress_reporter
            .send_progress_end(progress_token, format!("Indexing of {dir:?}"))?;

        Ok(classes)
    }

    fn index_file_cursor(path: PathBuf) -> Result<Vec<Arc<RSymbol>>> {
        let ctx = IndexingContext::new(path.as_path())?;

        let mut result: Vec<Arc<RSymbol>> = Vec::new();
        let mut cursor = ctx.tree.walk();
        loop {
            let node = cursor.node();
            let source = &ctx.source[..];

            if node.kind() == "program" {
                cursor.goto_first_child();
            }

            let mut parsed = parse(path.as_path(), source, cursor.node(), None);
            result.append(&mut parsed);

            if !cursor.goto_next_sibling() {
                break;
            }
        }

        Ok(result)
    }
}
