use std::{
    path::{Path, PathBuf},
    rc::Rc,
    sync::Arc,
    time::Instant,
};

use log::{debug, info};

use anyhow::{Context, Result};
use tree_sitter::{Node, Point};

use crate::parsers::methods::get_method_variable_definition;
use crate::parsers::scopes::{get_context_scope, get_parent_scope_resolution};
use crate::{
    parsers::{
        general::read_file_tree,
        identifiers::get_identifier_context,
        types::{NodeKind, NodeName, Scope},
    },
    ruby_filename_converter::RubyFilenameConverter,
    symbols_matcher::SymbolsMatcher,
    types::{RSymbol, RVariable},
};

pub struct Finder {
    root_dir: PathBuf,
    symbols: Rc<Vec<Arc<RSymbol>>>,
    ruby_filename_converter: Rc<RubyFilenameConverter>,
}

impl Finder {
    pub fn new(
        root_dir: &Path,
        symbols: Rc<Vec<Arc<RSymbol>>>,
        ruby_filename_converter: Rc<RubyFilenameConverter>,
    ) -> Finder {
        Finder {
            root_dir: root_dir.to_path_buf(),
            symbols,
            ruby_filename_converter,
        }
    }

    pub fn find_by_path(&self, path: &Path) -> Vec<Arc<RSymbol>> {
        self.symbols.iter().filter(|s| s.file() == path).cloned().collect()
    }

    pub fn fuzzy_find_symbol(&self, query: &str) -> Vec<Arc<RSymbol>> {
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

    pub fn find_definition(&self, file: &Path, position: Point) -> Result<Vec<Arc<RSymbol>>> {
        let (tree, source) = read_file_tree(file)?;

        let node = tree
            .root_node()
            .descendant_for_point_range(position, position)
            .ok_or(anyhow!("Failed to find node of definition"))?;

        let node_kind = node.kind().try_into().with_context(|| format!("Unknown node kind: {}", node.kind()))?;

        match node_kind {
            NodeKind::Constant => Ok(self.find_constant(&node, file, &source)),
            NodeKind::Identifier => self.find_identifier(&node, file, &source),
            NodeKind::GlobalVariable => self.find_global_variable(&node, &source),
            _ => Err(anyhow!("Find definition of {} node kind is not supported", node.kind())),
        }
    }

    fn find_identifier(&self, node: &Node, file: &Path, source: &[u8]) -> Result<Vec<Arc<RSymbol>>> {
        info!("Trying to find an identifier in {:?} at {:?}", file, node.start_position());
        let identifier = node.utf8_text(source).unwrap();

        let parent = node.parent().with_context(|| {
            format!("Failed to find parent for identifier in {:?} at {:?}", file, node.start_position())
        })?;

        let context_node = get_identifier_context(node).ok_or(anyhow!(
            "Failed to determine context of node in {:?} at {:?}",
            file,
            node.start_position()
        ))?;

        match context_node.kind().try_into()? {
            NodeKind::Call => {
                let receiver = parent.child_by_field_name(NodeName::Receiver);
                self.find_method_definition(identifier, file, receiver)
            }

            NodeKind::Method | NodeKind::SingletonMethod => {
                let variable_def = get_method_variable_definition(node, &context_node, file, source).ok_or(anyhow!(
                    "Failed to find variable definition in {:?} at {:?}",
                    file,
                    node.start_position()
                ))?;
                let symbol = Arc::new(RSymbol::Variable(RVariable {
                    file: file.to_path_buf(),
                    name: variable_def.utf8_text(source).unwrap().to_string(),
                    scope: Scope::new(vec![]),
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

        let receiver_definitions = receiver.map(|r| self.find_definition(file, r.start_position())).transpose()?;

        Ok(self
            .symbols
            .iter()
            // TODO: depends on the type of receiver, change after adding more definition types
            .filter(|s| matches!(***s, RSymbol::SingletonMethod(_)))
            .filter(|s| {
                let defs = if let Some(rd) = &receiver_definitions { rd } else { return true };
                let parent = if let Some(p) = s.parent() { p } else { return true };

                defs.contains(parent)
            })
            .filter(|s| s.full_scope().last().map(|l| l == method_name).unwrap_or(false))
            .cloned()
            .collect())
    }

    fn find_global_variable(&self, node: &Node, source: &[u8]) -> Result<Vec<Arc<RSymbol>>> {
        info!("Trying to find a global variable");

        let node_kind: NodeKind = node.kind().try_into()?;
        if node_kind != NodeKind::GlobalVariable {
            bail!("Node kind is not global variable")
        }

        let name = node.utf8_text(source).unwrap();

        Ok(self
            .symbols
            .iter()
            .filter(|s| matches!(***s, RSymbol::GlobalVariable(_) if s.name() == name))
            .cloned()
            .collect())
    }

    fn find_constant(&self, node: &Node, file: &Path, source: &[u8]) -> Vec<Arc<RSymbol>> {
        info!("Trying to find a constant");
        // traverse down till we hit the whole symbol name
        let constant_scope = get_parent_scope_resolution(node, source);

        let context_scope = get_context_scope(node, source).join(&constant_scope);

        let mut file_scope = self.ruby_filename_converter.path_to_scope(file).unwrap_or(Scope::new(vec![]));
        file_scope.remove_last();
        let file_scope = file_scope.join(&constant_scope);

        let symbols = self
            .symbols
            .iter()
            .filter(|s| matches!(***s, RSymbol::Class(_) | RSymbol::Module(_) | RSymbol::Constant(_)));

        let results = if constant_scope.is_global() {
            info!("Global scope, searching for {constant_scope}");
            symbols.filter(|s| s.full_scope() == &constant_scope).cloned().collect()
        } else {
            info!("Searching for {context_scope} or {file_scope} or {context_scope} in the same file");
            // search in contexts first
            let found_symbols: Vec<Arc<RSymbol>> = symbols
                .clone()
                .filter(|s| {
                    let name = s.full_scope();
                    name == &context_scope || name == &file_scope || (name == &constant_scope && s.file() == file)
                })
                .cloned()
                .collect();

            // then global
            if found_symbols.is_empty() {
                info!("Haven't found anything, searching for global {constant_scope}");
                symbols.clone().filter(|s| s.full_scope() == &constant_scope).cloned().collect()
            } else {
                found_symbols
            }
        };

        debug!("Found {} results", results.len());

        results
    }
}
