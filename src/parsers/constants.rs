use std::{path::Path, sync::Arc};

use log::error;
use tree_sitter::Node;

use crate::types::{RSymbol, RConstant};

use super::types::{NodeKind, SCOPE_DELIMITER};

pub fn parse_constant(file: &Path, source: &[u8], node: &Node, parent: Option<Arc<RSymbol>>) -> Option<RSymbol> {
    if node.kind() != NodeKind::Constant && node.kind() != NodeKind::RestAssignment {
        error!("{} instead of constant in {file:?} at {:?}", node.kind(), node.range());
    }

    let node = if node.kind() == NodeKind::RestAssignment {
        node.child(0).unwrap()
    } else {
        *node
    };

    let scope = match &parent {
        Some(p) => match &**p {
            RSymbol::Class(c) | RSymbol::Module(c) => Some(&c.scope),
            _ => None,
        },

        None => None,
    };
    let text = node.utf8_text(source).unwrap().to_string();

    let name = match scope {
        Some(s) => s.to_string() + SCOPE_DELIMITER + &text,
        None => text,
    };

    let scope = scope.map(|s| s.join(&(&name).into())).unwrap_or_default();

    Some(RSymbol::Constant(RConstant {
        file: file.to_owned(),
        name,
        scope,
        location: node.start_position(),
        parent,
    }))
}

