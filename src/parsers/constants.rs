use std::{path::Path, sync::Arc};

use log::error;
use tree_sitter::Node;

use itertools::Itertools;

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

    let scopes = match &parent {
        Some(p) => match &**p {
            RSymbol::Class(c) | RSymbol::Module(c) => Some(&c.scopes),
            _ => None,
        },

        None => None,
    };
    let text = node.utf8_text(source).unwrap().to_string();

    let name = match scopes {
        Some(s) => s.iter().join(SCOPE_DELIMITER) + SCOPE_DELIMITER + &text,
        None => text,
    };

    Some(RSymbol::Constant(RConstant {
        file: file.to_owned(),
        name,
        location: node.start_position(),
        parent,
    }))
}

