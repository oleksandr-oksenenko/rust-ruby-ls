use std::collections::HashMap;
use std::ffi::OsStr;
use std::fs;
use std::path::{Path, PathBuf};

use std::sync::Arc;
use std::time::Instant;

use anyhow::{Context, Result};

use itertools::Itertools;
use log::{debug, error, info, warn};
use rayon::prelude::*;
use tree_sitter::{Node, Parser, Point, Tree};
use tree_sitter_ruby::language;
use walkdir::WalkDir;

use crate::parsers;
use crate::parsers::{get_context_scope, get_parent_scope_resolution, parse};
use crate::progress_reporter::ProgressReporter;
use crate::ruby_env_provider::RubyEnvProvider;
use crate::ruby_filename_converter::RubyFilenameConverter;
use crate::symbols_matcher::SymbolsMatcher;

#[allow(dead_code)]
#[derive(PartialEq, Eq)]
pub enum RSymbol {
    Class(RClass),
    Module(RClass),
    Method(RMethod),
    SingletonMethod(RMethod),
    Constant(RConstant),
    Variable(RVariable),
    GlobalVariable(RVariable),
    ClassVariable(RVariable),
}

impl RSymbol {
    pub fn kind(&self) -> &str {
        match self {
            RSymbol::Class(_) => "class",
            RSymbol::Module(_) => "module",
            RSymbol::Method(_) => "method",
            RSymbol::SingletonMethod(_) => "singleton_method",
            RSymbol::Constant(_) => "constant",
            RSymbol::Variable(_) => "variable",
            RSymbol::GlobalVariable(_) => "global_variable",
            RSymbol::ClassVariable(_) => "class_variable",
        }
    }

    pub fn name(&self) -> &str {
        match self {
            RSymbol::Class(class) => &class.name,
            RSymbol::Module(module) => &module.name,
            RSymbol::Method(method) => &method.name,
            RSymbol::SingletonMethod(method) => &method.name,
            RSymbol::Constant(constant) => &constant.name,
            RSymbol::Variable(variable) => &variable.name,
            RSymbol::GlobalVariable(variable) => &variable.name,
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
            RSymbol::GlobalVariable(variable) => &variable.file,
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
            RSymbol::GlobalVariable(variable) => &variable.location,
            RSymbol::ClassVariable(variable) => &variable.location,
        }
    }

    pub fn parent(&self) -> &Option<Arc<RSymbol>> {
        match self {
            RSymbol::Class(s) => &s.parent,
            RSymbol::Module(s) => &s.parent,
            RSymbol::Method(s) => &s.parent,
            RSymbol::SingletonMethod(s) => &s.parent,
            RSymbol::Constant(s) => &s.parent,
            RSymbol::Variable(s) => &s.parent,
            RSymbol::GlobalVariable(s) => &s.parent,
            RSymbol::ClassVariable(s) => &s.parent,
        }
    }
}

impl std::fmt::Debug for RSymbol {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{} in {:?} at {:?}, name = {}, parent = {:?}",
            self.kind(),
            self.file(),
            self.location(),
            self.name(),
            self.parent()
        )
    }
}

#[derive(PartialEq, Eq)]
pub struct RClass {
    pub file: PathBuf,
    pub name: String,
    pub location: Point,
    pub scopes: Vec<String>,
    pub superclass_scopes: Vec<String>,
    pub parent: Option<Arc<RSymbol>>,
}

#[derive(PartialEq, Eq)]
pub struct RMethod {
    pub file: PathBuf,
    pub name: String,
    pub location: Point,
    pub parameters: Vec<RMethodParam>,
    pub parent: Option<Arc<RSymbol>>,
}

#[derive(PartialEq, Eq)]
pub enum RMethodParam {
    Regular(MethodParam),
    Optional(MethodParam),
    Keyword(MethodParam),
}

#[derive(PartialEq, Eq)]
pub struct MethodParam {
    pub file: PathBuf,
    pub name: String,
    pub location: Point,
}

#[derive(PartialEq, Eq)]
pub struct RConstant {
    pub file: PathBuf,
    pub name: String,
    pub location: Point,
    pub parent: Option<Arc<RSymbol>>,
}

#[derive(PartialEq, Eq)]
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

        Ok(IndexingContext { source, tree: parsed })
    }
}

pub struct Indexer<'a> {
    root_dir: PathBuf,
    progress_reporter: ProgressReporter<'a>,
    ruby_env_provider: RubyEnvProvider,
    ruby_filename_converter: RubyFilenameConverter,
    symbols: Vec<Arc<RSymbol>>,
    file_index: HashMap<PathBuf, Vec<Arc<RSymbol>>>,
}

impl<'a> Indexer<'a> {
    pub fn new(root_dir: &Path, progress_reporter: ProgressReporter<'a>) -> Indexer<'a> {
        let root_dir = root_dir.to_path_buf();
        let ruby_env_provider = RubyEnvProvider::new(root_dir.clone());
        let ruby_filename_converter = RubyFilenameConverter::new(root_dir.clone(), &ruby_env_provider).unwrap();
        Indexer {
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

    pub fn find_definition(&self, file: &Path, position: Point) -> Result<Vec<Arc<RSymbol>>> {
        let ctx = IndexingContext::new(file).unwrap();

        let node = ctx.tree.root_node();
        let node = node
            .descendant_for_point_range(position, position)
            .ok_or(anyhow!("Failed to find node of definition"))?;

        let node_kind = node
            .kind()
            .try_into()
            .with_context(|| format!("Unknown node kind: {}", node.kind()))?;

        match node_kind {
            parsers::NodeKind::Constant => Ok(self.find_constant(&node, file, &ctx.source)),
            parsers::NodeKind::Identifier => self.find_identifier(&node, file, &ctx.source),
            parsers::NodeKind::GlobalVariable => self.find_global_variable(&node, &ctx.source),
            _ => Err(anyhow!("Find definition of {} node kind is not supported", node.kind())),
        }
    }

    fn find_identifier(&self, node: &Node, file: &Path, source: &[u8]) -> Result<Vec<Arc<RSymbol>>> {
        info!(
            "Trying to find an identifier in {:?} at {:?}",
            file,
            node.start_position()
        );
        let identifier = node.utf8_text(source).unwrap();

        let parent = node.parent().with_context(|| {
            format!(
                "Failed to find parent for identifier in {:?} at {:?}",
                file,
                node.start_position()
            )
        })?;

        let context_node = parsers::get_identifier_context(node).ok_or(anyhow!(
            "Failed to determine context of node in {:?} at {:?}",
            file,
            node.start_position()
        ))?;

        match context_node.kind().try_into()? {
            parsers::NodeKind::Call => {
                let receiver = parent.child_by_field_name(parsers::NodeName::Receiver);
                self.find_method_definition(identifier, file, receiver)
            }

            parsers::NodeKind::Method | parsers::NodeKind::SingletonMethod => {
                let variable_def =
                    parsers::get_method_variable_definition(node, &context_node, file, source).ok_or(anyhow!(
                        "Failed to find variable definition in {:?} at {:?}",
                        file,
                        node.start_position()
                    ))?;
                let symbol = Arc::new(RSymbol::Variable(RVariable {
                    file: file.to_path_buf(),
                    name: variable_def.utf8_text(source).unwrap().to_string(),
                    location: variable_def.start_position(),
                    parent: None,
                }));
                Ok(vec![symbol])
            }

            _ => Ok(vec![]),
        }
    }

    fn find_method_definition(
        &self,
        method_name: &str,
        file: &Path,
        receiver: Option<Node>,
    ) -> Result<Vec<Arc<RSymbol>>> {
        let receiver_kind = receiver.map(|n| n.kind());
        info!("Trying to find method: {method_name}, receiver kind = {receiver_kind:?}");

        let receiver_definitions = receiver
            .map(|r| self.find_definition(file, r.start_position()))
            .transpose()?;

        Ok(self
            .symbols
            .iter()
            // TODO: depends on the type of receiver, change after adding more definition types
            .filter(|s| matches!(***s, RSymbol::SingletonMethod(_)))
            .filter(|s| {
                let receiver_definitions = match &receiver_definitions {
                    None => return true,
                    Some(rd) => rd,
                };
                let parent = match s.parent() {
                    None => return true,
                    Some(p) => p,
                };
                if receiver_definitions.is_empty() {
                    return true;
                }
                receiver_definitions.contains(parent)
            })
            .filter(|s| {
                let last_scope = s.name().split("::").last().unwrap();
                method_name == last_scope
            })
            .cloned()
            .collect())
    }

    fn find_global_variable(&self, node: &Node, source: &[u8]) -> Result<Vec<Arc<RSymbol>>> {
        info!("Trying to find a global variable");

        let node_kind: parsers::NodeKind = node.kind().try_into()?;
        if node_kind != parsers::NodeKind::GlobalVariable {
            bail!("Node kind is not global variable")
        }

        let name = node.utf8_text(source).unwrap();

        Ok(self
            .symbols
            .iter()
            .filter_map(|s| {
                let global_var = matches!(**s, RSymbol::GlobalVariable(_));
                let name_equals = s.name() == name;

                if global_var && name_equals {
                    Some(s.clone())
                } else {
                    None
                }
            })
            .collect())
    }

    fn find_constant(&self, node: &Node, file: &Path, source: &[u8]) -> Vec<Arc<RSymbol>> {
        info!("Trying to find a constant");
        // traverse down till we hit the whole symbol name
        let constant_scope = get_parent_scope_resolution(node, source);
        let is_global = constant_scope
            .first()
            .map(|s| *s == parsers::GLOBAL_SCOPE_VALUE)
            .unwrap_or(false);
        let constant_scope = if is_global {
            constant_scope.into_iter().skip(1).join(parsers::SCOPE_DELIMITER)
        } else {
            constant_scope.into_iter().join(parsers::SCOPE_DELIMITER)
        };

        let context_scope = get_context_scope(node, source)
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

        let symbols = self
            .symbols
            .iter()
            .filter(|s| matches!(***s, RSymbol::Class(_) | RSymbol::Module(_) | RSymbol::Constant(_)));

        let results = if is_global {
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
            info!("Searching for {context_scope} or {file_scope} or {context_scope} in the same file");
            // search in contexts first
            let found_symbols: Vec<Arc<RSymbol>> = symbols
                .clone()
                .filter_map(|s| {
                    let name = s.name();

                    if name == context_scope || name == file_scope || (name == constant_scope && s.file() == file) {
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
        };

        debug!("Found {} results", results.len());

        results
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

        info!("Found {} symbols, took {:?}", self.symbols.len(), start.elapsed());

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
        let progress_token = self
            .progress_reporter
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
