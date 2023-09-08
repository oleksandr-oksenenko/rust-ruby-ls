use std::{
    path::{Path, PathBuf},
    rc::Rc,
    sync::Arc,
    time::Instant,
};

use log::{debug, error, info};

use itertools::Itertools;

use anyhow::{Context, Result};
use tree_sitter::{Node, Point, Range};

use crate::{
    parsers_v2::{
        general::read_file_tree,
        identifiers::find_first_assingment_with_lhs_text,
        scopes::{get_context_scope, get_parent_scope_resolution},
    },
    ruby_filename_converter::RubyFilenameConverter,
    symbols_matcher::SymbolsMatcher,
    types::{NodeKind, RSymbolKind, RSymbolV2, Scope, NodeName},
};

pub struct Finder {
    root_dir: PathBuf,
    symbols: Rc<Vec<Arc<RSymbolV2>>>,
    ruby_filename_converter: Rc<RubyFilenameConverter>,
}

impl Finder {
    pub fn new(
        root_dir: &Path,
        symbols: Rc<Vec<Arc<RSymbolV2>>>,
        ruby_filename_converter: Rc<RubyFilenameConverter>,
    ) -> Finder {
        Finder {
            root_dir: root_dir.to_path_buf(),
            symbols,
            ruby_filename_converter,
        }
    }

    pub fn find_by_path(&self, path: &Path) -> Vec<Arc<RSymbolV2>> {
        self.symbols.iter().filter(|s| s.file == path).cloned().collect()
    }

    pub fn fuzzy_find_symbol(&self, query: &str) -> Vec<Arc<RSymbolV2>> {
        let start = Instant::now();
        let result = if query.is_empty() {
            // optimization to not overload telescope on request without a query
            vec![]
        } else {
            SymbolsMatcher::new(&self.root_dir).match_rsymbols(query, &self.symbols)
        };

        info!("Finding symbol by {} took {:?}", query, start.elapsed());

        result
    }

    pub fn find_definition(&self, file: &Path, position: Point) -> Result<Vec<Arc<RSymbolV2>>> {
        let (tree, source) = read_file_tree(file)?;

        let node = tree
            .root_node()
            .descendant_for_point_range(position, position)
            .ok_or(anyhow!("Failed to find node of definition"))?;

        let node_kind: NodeKind =
            node.kind().try_into().with_context(|| format!("Unknown node kind: {}", node.kind()))?;

        match node_kind {
            NodeKind::Constant => Ok(self.find_constant(&node, file, &source)),
            NodeKind::GlobalVariable => Ok(self.find_global_variable(&node, &source)),
            NodeKind::Identifier => Ok(self.find_identifier(&node, file, &source)),
            _ => Err(anyhow!("Failed to determine symbol type, node kind: {node_kind:?}")),
        }
    }

    fn find_identifier(&self, node: &Node, file: &Path, source: &[u8]) -> Vec<Arc<RSymbolV2>> {
        info!("Trying to find an identifier");

        let identifier_text = node.utf8_text(source).unwrap().to_string();

        // identifier could be:
        // 1. local variable
        // 2. method parameter
        // 3. instance variable
        // 4. class variable
        // 5. instance method
        // 6. class method

        info!("Identifier start = {}, end = {}", node.start_position(), node.end_position());
        let scope_symbol = self
            .symbols
            .iter()
            .filter(|s| s.file == file)
            // .inspect(|s| info!("Symbol start = {}, end = {}", s.start, s.end))
            .filter(|s| s.start < node.start_position() && s.end > node.end_position())
            .sorted_by_key(|s| [s.end.row - s.start.row, s.end.column - s.start.column])
            .next();

        let scope_symbol = if let Some(sym) = scope_symbol {
            sym
        } else {
            error!("Failed to find scope symbol");
            return vec![];
        };

        info!("Scope symbol for identifier: {scope_symbol:?}");

        match &scope_symbol.kind {
            RSymbolKind::Class {
                ..
            } => todo!(),
            RSymbolKind::Module {
                ..
            } => todo!(),

            RSymbolKind::SingletonMethod { parameters } | RSymbolKind::InstanceMethod { parameters } => {
                // priority of the search
                // 1. local variable (search for assignment up from the node)
                // 2. method parameter
                // 3. instance variable/method
                // 4. class variable/method

                // 1.
                info!("Searching for local variable");
                let assignment_left_node = find_first_assingment_with_lhs_text(node, &scope_symbol.start, source);
                if let Some(def) = assignment_left_node {
                    let result = RSymbolV2 {
                        kind: RSymbolKind::Variable,
                        name: identifier_text,
                        scope: scope_symbol.scope.clone(),
                        file: file.to_path_buf(),
                        start: def.start_position(),
                        end: def.end_position(),
                        parent: Some(scope_symbol.clone()),
                    };
                    return vec![Arc::new(result)];
                }

                // 2.
                info!("Searching for parameter");
                if let Some(param) = parameters.iter().find(|p| p.name == identifier_text) {
                    let result = RSymbolV2 {
                        kind: RSymbolKind::Variable,
                        name: identifier_text,
                        scope: scope_symbol.scope.clone(),
                        file: file.to_path_buf(),
                        start: param.start,
                        end: param.end,
                        parent: Some(scope_symbol.clone()),
                    };
                    return vec![Arc::new(result)];
                }

                // 3.
                let parent = node.parent();
                let is_call = parent.as_ref().map(|p| p.kind() == NodeKind::Call).unwrap_or(false);
                if is_call {
                    let call = parent.unwrap();
                    let method = call.child_by_field_name(NodeName::Method).unwrap();
                    let receiver = call.child_by_field_name(NodeName::Receiver).unwrap();

                    match receiver.kind().try_into() {
                        Err(_) => {
                            error!("Unknown receiver kind: {}", receiver.kind());
                            return vec![]
                        },
                        Ok(kind) => match kind {
                            NodeKind::Zelf => {

                            },

                            _ => {
                                error!("Unsupported receiver node kind: {kind}");
                                return vec![]
                            }
                        }
                    }

                }
                // TODO: depends on the receiver
                info!("Searching for instance method or variable");
                let variables: Vec<_> = self.symbols.iter()
                    .filter(|s| matches!(s.kind, RSymbolKind::InstanceVariable | RSymbolKind::InstanceMethod { .. }))
                    .filter(|s| s.scope == scope_symbol.scope)
                    .inspect(|s| info!("Method or variable with the same scope as in scope_symbol: {}", s.name))
                    .filter(|s| s.name == identifier_text)
                    .cloned()
                    .collect();
                if !variables.is_empty() {
                    return variables
                }

                // 4.
                // TODO: depends on the receiver
                info!("Searching for class method or variable");
                let variables: Vec<_> = self.symbols.iter()
                    .filter(|s| matches!(s.kind, RSymbolKind::ClassVariable | RSymbolKind::SingletonMethod { .. }))
                    .filter(|s| s.name == identifier_text)
                    .cloned()
                    .collect();
                if !variables.is_empty() {
                    return variables;
                }
            }
            _ => {
                error!("Unexpected scope symbol: {scope_symbol:?}");
                return vec![];
            }
        };

        vec![]
    }

    fn find_global_variable(&self, node: &Node, source: &[u8]) -> Vec<Arc<RSymbolV2>> {
        info!("Trying to find a global variable");
        let name = node.utf8_text(source).unwrap();

        self.symbols
            .iter()
            .filter(|s| s.kind == RSymbolKind::GlobalVariable)
            .filter(|s| s.name == name)
            .cloned()
            .collect()
    }

    fn find_constant(&self, node: &Node, file: &Path, source: &[u8]) -> Vec<Arc<RSymbolV2>> {
        info!("Trying to find a constant");
        // traverse down till we hit the whole symbol name
        let constant_scope = get_parent_scope_resolution(node, source);

        let context_scope = get_context_scope(node, source).join(&constant_scope);

        let mut file_scope = self.ruby_filename_converter.path_to_scope(file).unwrap_or(Scope::new(vec![]));
        file_scope.remove_last();
        let file_scope = file_scope.join(&constant_scope);

        let symbols = self.symbols.iter().filter(|s| s.kind.is_classlike() || s.kind == RSymbolKind::Constant);

        let results = if constant_scope.is_global() {
            info!("Global scope, searching for {constant_scope}");
            symbols.filter(|s| s.scope == constant_scope).cloned().collect()
        } else {
            info!("Searching for {context_scope} or {file_scope} or {constant_scope} in the same file");
            // search in contexts first
            let found_symbols: Vec<Arc<RSymbolV2>> = symbols
                .clone()
                .filter(|s| {
                    let name = &s.scope.join(&(&s.name).into());
                    name == &context_scope || name == &file_scope || (name == &constant_scope && s.file == file)
                })
                .cloned()
                .collect();

            // then global
            if found_symbols.is_empty() {
                info!("Haven't found anything, searching for global {constant_scope}");
                symbols.clone().filter(|s| s.scope == constant_scope).cloned().collect()
            } else {
                found_symbols
            }
        };

        debug!("Found {} results", results.len());

        results
    }
}
