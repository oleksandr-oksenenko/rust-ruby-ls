use tree_sitter::{Node, Point, Query, QueryCursor};

use itertools::Itertools;

use crate::types::{NodeKind, NodeName};

pub fn find_first_assingment_with_lhs_text<'a>(
    identifier_node: &'a Node,
    scope_start: &Point,
    source: &[u8],
) -> Option<Node<'a>> {
    let identifier_text = identifier_node.utf8_text(source).unwrap();

    let mut parent = identifier_node.parent();
    while let Some(p) = parent {
        if p.start_position() < *scope_start {
            return None;
        }

        let mut cursor = p.walk();
        let assignment_left_node = p
            .children(&mut cursor)
            .filter(|n| n.start_position() < identifier_node.start_position())
            .filter(|n| n.kind() == NodeKind::Assignment)
            .map(|n| n.child_by_field_name(NodeName::Left).unwrap())
            .filter(|n| {
                let node_text = n.utf8_text(source).unwrap();
                node_text == identifier_text
            })
            .sorted_by_key(|n| identifier_node.start_position().row - n.start_position().row)
            .next();

        if assignment_left_node.is_some() {
            return assignment_left_node;
        } else {
            parent = p.parent();
        }
    }

    None
}

pub fn find_first_assignment_with_lhs_text_with_query<'a>(
    identifier_node: &'a Node,
    context_node: &'a Node,
    source: &[u8],
) -> Option<Node<'a>> {
    let identifier_text = identifier_node.utf8_text(source).unwrap();
    let query_str = format!(r#"(assignment left: (identifier) @lhs (#eq @lhs {}) right: (_))"#, identifier_text);
    let query = Query::new(tree_sitter_ruby::language(), &query_str).unwrap();

    let mut query_cursor = QueryCursor::new();
    let matches = query_cursor.matches(&query, *context_node, source);

    let node = matches.flat_map(|qm| qm.captures)
        .map(|qc| qc.node)
        .filter(|n| n.start_position() < identifier_node.start_position())
        .sorted_by_key(|n| n.start_position())
        .next();

    node
}
