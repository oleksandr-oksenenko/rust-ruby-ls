use tree_sitter::Node;

use crate::parsers::types::NodeKind;

pub fn get_identifier_context<'a>(node: &Node<'a>) -> Option<Node<'a>> {
    let node_kind = node.kind().try_into();
    assert!(node_kind.is_ok());
    let node_kind: NodeKind = node_kind.unwrap();
    assert!(node_kind == NodeKind::Identifier);

    let mut parent = node.parent();
    while let Some(p) = parent {
        match p.kind().try_into() {
            Err(_) => parent = p.parent(),

            Ok(k) => match k {
                NodeKind::Call => return Some(p),
                NodeKind::Method => return Some(p),
                NodeKind::SingletonMethod => return Some(p),
                NodeKind::Class => return Some(p),
                NodeKind::Module => return Some(p),

                _ => parent = p.parent(),
            },
        }
    }

    None
}
