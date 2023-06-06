use std::collections::HashMap;
use std::ffi::OsStr;
use std::fs;
use std::path::{Path, PathBuf};
use std::rc::Rc;
use std::sync::{Arc, Mutex};
use std::time::Instant;

use anyhow::Result;

use itertools::Itertools;
use log::{error, info, warn};
use rayon::prelude::*;
use tree_sitter::{
    Language, Node, Parser, Point, Query, QueryCapture, QueryCursor, Tree, TreeCursor,
};
use tree_sitter_ruby::language;
use walkdir::WalkDir;

use crate::progress_reporter::ProgressReporter;
use crate::symbols_matcher::SymbolsMatcher;

pub enum RSymbol {
    Class(RClass),
    Module(RClass),
    Method(RMethod),
    SingletonMethod(RMethod),
    Constant(RConstant),
    MethodCall(RMethodCall),
    Variable(RVariable),
}

impl RSymbol {
    pub fn name(&self) -> &str {
        match self {
            RSymbol::Class(class) => &class.name,
            RSymbol::Module(module) => &module.name,
            RSymbol::Method(method) => &method.name,
            RSymbol::SingletonMethod(method) => &method.name,
            RSymbol::Constant(constant) => &constant.name,
            RSymbol::MethodCall(method_call) => &method_call.name,
            RSymbol::Variable(variable) => &variable.name,
        }
    }

    pub fn file(&self) -> &Path {
        match self {
            RSymbol::Class(class) => &class.file,
            RSymbol::Module(module) => &module.file,
            RSymbol::Method(method) => &method.file,
            RSymbol::SingletonMethod(method) => &method.file,
            RSymbol::Constant(constant) => &constant.file,
            RSymbol::MethodCall(method_call) => &method_call.file,
            RSymbol::Variable(variable) => &variable.file,
        }
    }

    pub fn location(&self) -> &Point {
        match self {
            RSymbol::Class(class) => &class.location,
            RSymbol::Module(module) => &module.location,
            RSymbol::Method(method) => &method.location,
            RSymbol::SingletonMethod(method) => &method.location,
            RSymbol::Constant(constant) => &constant.location,
            RSymbol::MethodCall(method_call) => &method_call.location,
            RSymbol::Variable(variable) => &variable.location,
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
    parameters: Vec<RMethodParam>,
    parent: Option<Arc<RSymbol>>,
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
    parent: Option<Arc<RSymbol>>,
}

pub struct RMethodCall {
    pub file: PathBuf,
    pub name: String,
    pub location: Point,
    parent: Option<Arc<RSymbol>>,
}

pub struct RVariable {
    pub file: PathBuf,
    pub name: String,
    pub location: Point,
    parent: Option<Arc<RSymbol>>,
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
    symbols: Vec<Arc<RSymbol>>,
    file_index: HashMap<PathBuf, Vec<Arc<RSymbol>>>,
}

impl<'a> IndexerV2<'a> {
    pub fn new(root_dir: &Path, progress_reporter: ProgressReporter<'a>) -> IndexerV2<'a> {
        let root_dir = root_dir.to_path_buf();
        IndexerV2 {
            ruby_env_provider: RubyEnvProvider::new(root_dir.clone()),
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

        // traverse down till we hit the whole symbol name
        let names_to_find = match node.kind() {
            "constant" => {
                let mut scopes = Self::get_partial_scope(&node, &ctx.source);

                let mut parent_scopes = Self::get_node_parent_scope(&node, &ctx.source);
                parent_scopes.reverse();
                scopes.reverse();

                let scopes = scopes.into_iter().join("::");

                if !parent_scopes.is_empty() {
                    let parent_scopes = parent_scopes.into_iter().join("::") + "::" + &scopes;
                    vec![scopes, parent_scopes]
                } else {
                    vec![scopes]
                }
            }
            "call" => {
                let reciever = node.child_by_field_name("reciever").unwrap();
                let constant = Self::parse_constant(file, &ctx.source, &reciever, None).unwrap();

                vec![constant.name().to_string()]
            }

            _ => {
                warn!("Find definition of {} node is not supported", node.kind());
                return vec![];
            }
        };

        info!("Searching for {:?}", names_to_find);

        let symbols = self
            .symbols
            .iter()
            .filter_map(|s| {
                let name = match &**s {
                    RSymbol::Class(c) | RSymbol::Module(c) => Some(&c.name),
                    RSymbol::Constant(c) => Some(&c.name),
                    _ => None,
                };

                match name {
                    Some(n) => {
                        let mut symbol = None;
                        for name_to_find in &names_to_find {
                            if n == name_to_find {
                                symbol = Some(s.clone());
                                break;
                            }
                        }
                        symbol
                    }
                    None => None,
                }
            })
            .collect();

        symbols
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

            let mut parsed = Self::parse(path.as_path(), source, cursor.node(), None);
            result.append(&mut parsed);

            if !cursor.goto_next_sibling() {
                break;
            }
        }

        Ok(result)
    }

    fn parse(
        file: &Path,
        source: &[u8],
        node: Node,
        parent: Option<Arc<RSymbol>>,
    ) -> Vec<Arc<RSymbol>> {
        match node.kind() {
            "class" | "module" => Self::parse_class(file, source, node, parent),

            "method" => {
                vec![Arc::new(Self::parse_method(file, source, node, parent))]
            }

            "singleton_method" => {
                vec![Arc::new(Self::parse_singleton_method(
                    file, source, node, parent,
                ))]
            }

            "assignment" => Self::parse_assignment(file, source, node, parent)
                .unwrap_or(Vec::new())
                .into_iter()
                .map(Arc::new)
                .collect(),

            "program" => {
                info!("empty file: {:?}", file);
                vec![]
            }

            "comment" | "call" => {
                // TODO: Implement
                vec![]
            }

            _ => {
                // warn!( "Unknown node kind: {}", node.kind());
                vec![]
            }
        }
    }

    fn parse_class(
        file: &Path,
        source: &[u8],
        node: Node,
        parent: Option<Arc<RSymbol>>,
    ) -> Vec<Arc<RSymbol>> {
        assert!(node.kind() == "class" || node.kind() == "module");

        let name_node = node.child_by_field_name("name").unwrap();
        let scopes = Self::get_scopes(&name_node, source);
        let name = scopes.iter().join("::");
        let superclass_scopes = node
            .child_by_field_name("superclass")
            .map(|n| Self::get_scopes(&n, source))
            .unwrap_or(Vec::default());

        let rclass = RClass {
            file: file.to_path_buf(),
            name,
            location: name_node.start_position(),
            scopes: Self::get_scopes(&name_node, source),
            superclass_scopes,
            parent,
        };

        let parent_symbol = if node.kind() == "class" {
            Arc::new(RSymbol::Class(rclass))
        } else {
            Arc::new(RSymbol::Module(rclass))
        };

        let mut result: Vec<Arc<RSymbol>> = Vec::new();
        if let Some(body_node) = node.child_by_field_name("body") {
            let mut cursor = body_node.walk();
            cursor.goto_first_child();
            let mut node = cursor.node();
            loop {
                let mut parsed = Self::parse(file, source, node, Some(parent_symbol.clone()));
                result.append(&mut parsed);

                node = match node.next_sibling() {
                    None => break,
                    Some(n) => n,
                }
            }
        }
        result.push(parent_symbol);

        result
    }

    fn parse_method(
        file: &Path,
        source: &[u8],
        node: Node,
        parent: Option<Arc<RSymbol>>,
    ) -> RSymbol {
        assert!(node.kind() == "method" || node.kind() == "singleton_method");

        let scopes = match &parent {
            Some(p) => match &**p {
                RSymbol::Class(c) | RSymbol::Module(c) => Some(&c.scopes),
                _ => None,
            },

            None => None,
        };

        let name_node = node.child_by_field_name("name").unwrap();
        let name = Self::get_node_text(&name_node, source);
        let name = match scopes {
            Some(s) => s.iter().join("::") + "::" + &name,
            None => name,
        };

        let mut cursor = node.walk();
        let mut params: Vec<RMethodParam> = Vec::new();
        if let Some(method_parameters) = node.child_by_field_name("method_parameters") {
            for param in method_parameters.children(&mut cursor) {
                let param = match param.kind() {
                    "identifier" => RMethodParam::Regular(Self::get_node_text(&param, source)),
                    "optional_parameter" => {
                        let name = Self::get_node_text(
                            &param.child_by_field_name("name").unwrap(),
                            source,
                        );
                        RMethodParam::Optional(name)
                    }
                    "keyword_parameter" => {
                        let name = Self::get_node_text(
                            &param.child_by_field_name("name").unwrap(),
                            source,
                        );
                        RMethodParam::Keyword(name)
                    }

                    _ => unreachable!(),
                };

                params.push(param);
            }
        }

        RSymbol::Method(RMethod {
            file: file.to_owned(),
            name,
            location: name_node.start_position(),
            parameters: params,
            parent,
        })
    }

    fn parse_singleton_method(
        file: &Path,
        source: &[u8],
        node: Node,
        parent: Option<Arc<RSymbol>>,
    ) -> RSymbol {
        match Self::parse_method(file, source, node, parent) {
            RSymbol::Method(method) => RSymbol::SingletonMethod(method),
            _ => unreachable!(),
        }
    }

    fn parse_assignment(
        file: &Path,
        source: &[u8],
        node: Node,
        parent: Option<Arc<RSymbol>>,
    ) -> Option<Vec<RSymbol>> {
        assert_eq!(node.kind(), "assignment");

        let lhs = node.child_by_field_name("left").unwrap();

        match lhs.kind() {
            "constant" => Self::parse_constant(file, source, &lhs, parent).map(|c| vec![c]),

            "left_assignment_list" => {
                // Only handle constants
                let mut cursor = lhs.walk();
                Some(
                    lhs.named_children(&mut cursor)
                        .filter(|n| n.kind() == "constant" || n.kind() == "rest_assignment")
                        .filter_map(|node| Self::parse_constant(file, source, &node, parent.clone()))
                        .collect(),
                )
            },

            "global_variable" => {
                // TODO: parse global variables as constants
                None
            },

            "scope_resolution" => {
                // TODO: parse scope resolution constant assignment
                None
            },

            "instance_variable" | "class_variable" => {
                // TODO: parse instance and class variables
                None
            },

            "identifier" => {
                // TODO: variable declaration, should parse?
                None
            },

            "element_reference" => {
                // TODO: e.g. putting into a Hash or Array, should parse?
                None
            },

            "call" => {
                // TODO: parse attr_accessors
                None
            },

            _ => {
                warn!("Unknown assignment 'left' node kind: {}, file: {:?}, range: {:?}", lhs.kind(), file, lhs.range());
                None
            }
        }
    }

    fn parse_constant(
        file: &Path,
        source: &[u8],
        node: &Node,
        parent: Option<Arc<RSymbol>>,
    ) -> Option<RSymbol> {
        if node.kind() != "constant" && node.kind() != "rest_assignment" {
            error!("{} instead of constant in {file:?} at {:?}", node.kind(), node.range());
        }

        let node = if node.kind() == "rest_assignment" {
            node.child(0).unwrap()
        } else { *node };

        let scopes = match &parent {
            Some(p) => match &**p {
                RSymbol::Class(c) | RSymbol::Module(c) => Some(&c.scopes),
                _ => None,
            },

            None => None,
        };
        let text = Self::get_node_text(&node, source);

        let name = match scopes {
            Some(s) => s.iter().join("::") + "::" + &text,
            None => text,
        };

        Some(RSymbol::Constant(RConstant {
            file: file.to_owned(),
            name,
            location: node.start_position(),
            parent,
        }))
    }

    fn get_node_text(node: &Node, source: &[u8]) -> String {
        node.utf8_text(source).unwrap().to_owned()
    }

    fn get_node_parent_scope(node: &Node, source: &[u8]) -> Vec<String> {
        let mut scopes = Vec::new();

        let mut node = Some(*node);
        while let Some(p) = node {
            match p.kind() {
                "class" | "module" => {
                    let name_node = p.child_by_field_name("name").unwrap();
                    let mut class_scopes = Self::get_scopes(&name_node, source);
                    class_scopes.reverse();
                    scopes.append(&mut class_scopes);

                    node = p.parent()
                }

                _ => {
                    node = p.parent()
                }
            }
        }

        scopes
    }

    fn get_partial_scope<'b>(node: &Node, source: &'b [u8]) -> Vec<&'b str> {
        assert!(node.kind() == "constant");

        let parent = node.parent().unwrap();
        if parent.kind() != "scope_resolution" {
            // single constant without a scope
            return vec![node.utf8_text(source).unwrap()];
        }

        // determine if node is a "scope" or a "name"
        let scope_node = parent.child_by_field_name("scope").unwrap();
        let name_node = parent.child_by_field_name("name").unwrap();
        let is_scope = scope_node.range() == node.range();
        let is_name = name_node.range() == node.range();
        assert!(is_scope || is_name);

        // it's the first constant in the "scope_resolution", just return it (e.g. A in A::B::C)
        if is_scope {
            return vec![node.utf8_text(source).unwrap()];
        }

        // go down from the current node to get scopes on the left (e.g. A::B::C in A::B::C::D if
        // cursor is on C)
        let mut scopes = Vec::new();
        let parent = node.parent();
        if let Some(p) = parent {
            // if let + condition is in nightly only
            if p.kind() == "scope_resolution" {
                let name = p.child_by_field_name("name").unwrap();
                scopes.push(name.utf8_text(source).unwrap());

                let mut scope = p.child_by_field_name("scope");
                while let Some(s) = scope {
                    match s.kind() {
                        "scope_resolution" => {
                            let name = s.child_by_field_name("name").unwrap();
                            scopes.push(name.utf8_text(source).unwrap());
                            scope = s.child_by_field_name("scope");
                        }
                        "constant" => {
                            scopes.push(s.utf8_text(source).unwrap());
                            break;
                        }
                        _ => unreachable!(),
                    }
                }
            }
        }

        scopes
    }

    fn get_scopes(main_node: &Node, source: &[u8]) -> Vec<String> {
        let mut scopes = Vec::new();

        if main_node.kind() == "scope_resolution" {
            let mut node = *main_node;
            while node.kind() == "scope_resolution" {
                let name_node = node.child_by_field_name("name").unwrap();
                let name = name_node.utf8_text(source).unwrap().to_owned();
                scopes.push(name);

                let child = node.child_by_field_name("scope");
                match child {
                    None => break,
                    Some(n) => node = n,
                }
            }
            if node.kind() == "constant" {
                let name = node.utf8_text(source).unwrap().to_owned();
                scopes.push(name);
            }
        }
        if main_node.kind() == "constant" {
            let name = main_node.utf8_text(source).unwrap().to_owned();
            scopes.push(name);
        }

        let class_node = main_node.parent();
        let mut class_parent_node = class_node.and_then(|p| p.parent());
        while let Some(parent) = class_parent_node {
            if parent.kind() == "class" || parent.kind() == "module" {
                let parent_class_name = parent.child_by_field_name("name").unwrap();
                let scope = parent_class_name.utf8_text(source).unwrap().to_owned();
                scopes.push(scope);
            }
            class_parent_node = parent.parent();
        }
        scopes.reverse();

        scopes
    }
}

struct RubyEnvProvider {
    dir: PathBuf,
}

impl RubyEnvProvider {
    pub fn new(dir: PathBuf) -> RubyEnvProvider {
        RubyEnvProvider { dir }
    }

    pub fn stubs_dir(&self) -> Result<Option<PathBuf>> {
        let ruby_version = match self.ruby_version()? {
            None => return Ok(None),
            Some(version) => version,
        };

        let segments: Vec<&str> = ruby_version.split('.').collect();
        let major = segments[0];
        let minor = segments[1];

        let path = "/Users/oleksandr.oksenenko/code/rust-ruby-ls/stubs/rubystubs".to_owned()
            + major
            + minor;

        Ok(Some(PathBuf::from(path)))
    }

    pub fn gems_dir(&self) -> Result<Option<PathBuf>> {
        let ruby_version = match self.ruby_version()? {
            None => return Ok(None),
            Some(version) => version,
        };

        let path = "/Users/oleksandr.oksenenko/.rvm/gems/ruby-".to_owned() + &ruby_version;
        match self.gemset()? {
            None => Ok(Some(PathBuf::from(path))),
            Some(gemset) => Ok(Some(PathBuf::from(path + "@" + &gemset))),
        }
    }

    fn ruby_version(&self) -> Result<Option<String>> {
        let ruby_version_file = self.dir.join(".ruby-version");
        if ruby_version_file.exists() {
            Ok(Some(
                fs::read_to_string(ruby_version_file)?.trim().to_owned(),
            ))
        } else {
            Ok(None)
        }
    }

    fn gemset(&self) -> Result<Option<String>> {
        let gemset_file = self.dir.join(".ruby-gemset");
        if gemset_file.exists() {
            Ok(Some(fs::read_to_string(gemset_file)?.trim().to_owned()))
        } else {
            Ok(None)
        }
    }
}
