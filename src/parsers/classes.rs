use std::{path::Path, sync::Arc};

use log::debug;
use tree_sitter::Node;

use itertools::Itertools;

use crate::{parsers::{types::{NodeKind, NodeName, SCOPE_DELIMITER}, scopes::get_full_and_context_scope, general::parse}, types::{RSymbol, RClass}};

pub fn parse_class(file: &Path, source: &[u8], node: Node, parent: Option<Arc<RSymbol>>) -> Vec<Arc<RSymbol>> {
    debug!("Parsing {:?} at {:?}", file, node.start_position());

    assert!(node.kind() == NodeKind::Class || node.kind() == NodeKind::Module);

    let name_node = node.child_by_field_name(NodeName::Name).unwrap();
    let scopes = get_full_and_context_scope(&name_node, source);
    let name = scopes.iter().join(SCOPE_DELIMITER);
    let superclass_scopes = node
        .child_by_field_name(NodeName::Superclass)
        .and_then(|n| n.child_by_field_name(NodeName::Name))
        .map(|n| get_full_and_context_scope(&n, source))
        .map(|s| s.into_iter().map(|s| s.to_string()).collect())
        .unwrap_or(Vec::default());

    let rclass = RClass {
        file: file.to_path_buf(),
        name,
        location: name_node.start_position(),
        scopes: scopes.into_iter().map(|s| s.to_string()).collect(),
        superclass_scopes,
        parent,
    };

    let parent_symbol = if node.kind() == NodeKind::Class {
        Arc::new(RSymbol::Class(rclass))
    } else {
        Arc::new(RSymbol::Module(rclass))
    };

    let mut result: Vec<Arc<RSymbol>> = Vec::new();
    if let Some(body_node) = node.child_by_field_name(NodeName::Body) {
        let mut cursor = body_node.walk();
        cursor.goto_first_child();
        let mut node = cursor.node();
        loop {
            let mut parsed = parse(file, source, node, Some(parent_symbol.clone()));
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

