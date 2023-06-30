use std::{path::Path, sync::Arc};

use tree_sitter::Node;

use crate::{parsers::{types::{NodeKind, NodeName}, constants::parse_constant}, types::{RSymbol, RVariable}};

pub fn parse_assignment(file: &Path, source: &[u8], node: Node, parent: Option<Arc<RSymbol>>) -> Option<Vec<RSymbol>> {
    assert_eq!(node.kind(), NodeKind::Assignment);

    let lhs = node.child_by_field_name(NodeName::Left).unwrap();

    let node_kind: NodeKind = match lhs.kind().try_into() {
        Err(_) => return None,
        Ok(nk) => nk,
    };
    match node_kind {
        NodeKind::Constant => parse_constant(file, source, &lhs, parent).map(|c| vec![c]),

        NodeKind::LeftAssignmentList => {
            // Only handle constants
            let mut cursor = lhs.walk();
            Some(
                lhs.named_children(&mut cursor)
                    .filter(|n| n.kind() == NodeKind::Constant || n.kind() == NodeKind::RestAssignment)
                    .filter_map(|node| parse_constant(file, source, &node, parent.clone()))
                    .collect(),
            )
        }

        NodeKind::GlobalVariable => {
            let name = lhs.utf8_text(source).unwrap().to_string();
            Some(vec![RSymbol::GlobalVariable(RVariable {
                file: file.to_path_buf(),
                name,
                location: node.start_position(),
                parent: None,
            })])
        }

        NodeKind::ScopeResolution => {
            // info!("Scope resolution assignment: {}, file: {:?}, range: {:?}", node.to_sexp(), file, node.range());
            // TODO: parse scope resolution constant assignment
            None
        }

        NodeKind::InstanceVariable | NodeKind::ClassVariable => {
            // info!("Instance/class variable assignment: {}, file: {:?}, range: {:?}", node.to_sexp(), file, node.range());
            // TODO: parse instance and class variables
            None
        }

        NodeKind::Identifier => {
            // info!("Identifier assignment: {}, file: {:?}, range: {:?}", node.to_sexp(), file, node.range());
            // TODO: variable declaration, should parse?
            None
        }

        NodeKind::Call => {
            // info!("Call assignment: {}, file: {:?}, range: {:?}", node.to_sexp(), file, node.range());
            // TODO: parse attr_accessors
            None
        }

        _ => {
            // warn!("Unknown assignment 'left' node kind: {}, file: {:?}, range: {:?}", lhs.kind(), file, lhs.range());
            None
        }
    }
}


