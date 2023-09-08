use std::{path::Path, sync::Arc};

use log::debug;
use tree_sitter::Node;

use crate::{
    parsers_v2::{
        general::parse,
        scopes::get_full_and_context_scope,
    }, types::{RSymbolV2, RSymbolKind, NodeKind, NodeName, Scope},
};

pub fn parse_class(file: &Path, source: &[u8], node: Node, parent: Option<Arc<RSymbolV2>>) -> Vec<Arc<RSymbolV2>> {
    debug!("Parsing {:?} at {:?}", file, node.start_position());

    assert!(node.kind() == NodeKind::Class || node.kind() == NodeKind::Module);

    let name_node = node.child_by_field_name(NodeName::Name).unwrap();
    let scope = get_full_and_context_scope(&name_node, source);
    let name = name_node.utf8_text(source).unwrap().to_string();
    let superclass_scope = node
        .child_by_field_name(NodeName::Superclass)
        .and_then(|n| n.child_by_field_name(NodeName::Name))
        .map(|n| get_full_and_context_scope(&n, source))
        .unwrap_or(Scope::default());

    let kind = if node.kind() == NodeKind::Class {
        RSymbolKind::Class { superclass_scope }
    } else {
        RSymbolKind::Module { superclass_scope }
    };

    let symbol = Arc::new(RSymbolV2 {
        kind,
        name,
        scope,
        file: file.to_path_buf(),
        start: name_node.start_position(),
        end: node.end_position(),
        parent,
    });

    let mut result: Vec<Arc<RSymbolV2>> = Vec::new();
    if let Some(body_node) = node.child_by_field_name(NodeName::Body) {
        let mut cursor = body_node.walk();
        cursor.goto_first_child();
        let mut node = cursor.node();
        loop {
            let mut parsed = parse(file, source, node, Some(symbol.clone()));
            result.append(&mut parsed);

            node = match node.next_sibling() {
                None => break,
                Some(n) => n,
            }
        }
    }
    result.push(symbol);

    result
}
