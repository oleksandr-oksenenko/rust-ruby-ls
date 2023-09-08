use std::{path::Path, sync::Arc};

use log::{error, info};
use tree_sitter::Node;

use crate::types::{NodeKind, RSymbolKind, RSymbolV2, Scope};

pub fn parse_constant(file: &Path, source: &[u8], node: &Node, parent: Option<Arc<RSymbolV2>>) -> Option<RSymbolV2> {
    if node.kind() != NodeKind::Constant && node.kind() != NodeKind::RestAssignment {
        error!("{} instead of constant in {file:?} at {:?}", node.kind(), node.range());
        return None;
    }

    let node = if node.kind() == NodeKind::RestAssignment {
        let mut cursor = node.walk();
        let mut children = node.children(&mut cursor);
        children.find(|n| n.kind() == NodeKind::Constant).unwrap()
    } else {
        *node
    };

    let parent_scope = match &parent {
        None => Scope::default(),
        Some(parent_symbol) => parent_symbol.scope.join(&(&parent_symbol.name).into()),
    };

    let name = node.utf8_text(source).unwrap().to_string();

    Some(RSymbolV2 {
        kind: RSymbolKind::Constant,
        name,
        scope: parent_scope,
        file: file.to_path_buf(),
        start: node.start_position(),
        end: node.end_position(),
        parent,
    })
}
