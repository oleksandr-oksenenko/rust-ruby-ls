use std::{path::Path, sync::Arc};

use log::info;
use tree_sitter::Node;

use crate::types::RSymbol;

use super::{types::NodeKind, methods::{parse_method, parse_singleton_method}, assignments::parse_assignment, classes::parse_class};

pub fn parse(file: &Path, source: &[u8], node: Node, parent: Option<Arc<RSymbol>>) -> Vec<Arc<RSymbol>> {
    let node_kind = match node.kind().try_into() {
        Ok(k) => k,
        Err(_) => return vec![],
    };

    match node_kind {
        NodeKind::Program => {
            info!("empty file: {:?}", file);
            vec![]
        }

        NodeKind::Class | NodeKind::Module => parse_class(file, source, node, parent),

        NodeKind::Method => {
            vec![Arc::new(parse_method(file, source, node, parent))]
        }

        NodeKind::SingletonMethod => {
            vec![Arc::new(parse_singleton_method(file, source, node, parent))]
        }

        NodeKind::Assignment => parse_assignment(file, source, node, parent)
            .unwrap_or(Vec::new())
            .into_iter()
            .map(Arc::new)
            .collect(),

        NodeKind::Comment | NodeKind::Call => {
            // TODO: Implement
            vec![]
        }

        _ => {
            // warn!( "Unknown node kind: {}", node.kind());
            vec![]
        }
    }
}

