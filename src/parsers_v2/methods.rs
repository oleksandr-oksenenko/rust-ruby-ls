use std::{path::{Path, PathBuf}, sync::Arc};

use log::{error, info, warn};
use tree_sitter::{Node, Query, QueryCursor};

use itertools::Itertools;

use crate::types::{RSymbolV2, NodeKind, RSymbolKind, NodeName, RMethodParam, MethodParam, Scope, RMethodParamV2, RMethodParamKind};

pub fn parse_method(file: &Path, source: &[u8], node: Node, parent: Option<Arc<RSymbolV2>>) -> RSymbolV2 {
    assert!(node.kind() == NodeKind::Method || node.kind() == NodeKind::SingletonMethod);

    let parent_scope = match &parent {
        None => Scope::default(),
        Some(parent_symbol) => {
            if parent_symbol.kind.is_classlike() {
                parent_symbol.scope.join(&(&parent_symbol.name).into())
            } else {
                Scope::default()
            }
        }
    };

    let name_node = node.child_by_field_name(NodeName::Name).unwrap();
    let name = name_node.utf8_text(source).unwrap().to_string();

    let mut params: Vec<RMethodParamV2> = Vec::new();

    for param in get_method_param_nodes(file, &node) {
        let param = match param.kind().try_into().unwrap() {
            NodeKind::Identifier => {
                let name = param.utf8_text(source).unwrap().to_string();

                RMethodParamV2 {
                    kind: RMethodParamKind::Regular,
                    file: file.to_path_buf(),
                    name,
                    start: param.start_position(),
                    end: param.end_position()
                }
            }

            NodeKind::OptionalParameter => {
                let name_node = param.child_by_field_name(NodeName::Name).unwrap();
                let name = name_node.utf8_text(source).unwrap().to_string();

                RMethodParamV2 {
                    kind: RMethodParamKind::Optional,
                    file: file.to_path_buf(),
                    name,
                    start: name_node.start_position(),
                    end: name_node.end_position()
                }
            }
            NodeKind::KeywordParameter => {
                let name_node = param.child_by_field_name(NodeName::Name).unwrap();
                let name = name_node.utf8_text(source).unwrap().to_string();
                RMethodParamV2 {
                    kind: RMethodParamKind::Keyword,
                    file: file.to_path_buf(),
                    name,
                    start: name_node.start_position(),
                    end: name_node.end_position()
                }
            }

            _ => unreachable!(),
        };

        params.push(param);
    }

    RSymbolV2 {
        kind: RSymbolKind::InstanceMethod { parameters: params },
        name,
        scope: parent_scope,
        file: file.to_path_buf(),
        start: name_node.start_position(),
        end: node.end_position(),
        parent,
    }
}

pub fn parse_singleton_method(file: &Path, source: &[u8], node: Node, parent: Option<Arc<RSymbolV2>>) -> RSymbolV2 {
    let mut instance_method_symbol = parse_method(file, source, node, parent);

    let parameters = match instance_method_symbol.kind {
        RSymbolKind::InstanceMethod { parameters } => parameters,
        _ => unreachable!()
    };

    instance_method_symbol.kind = RSymbolKind::SingletonMethod { parameters };
    instance_method_symbol
}

pub fn get_method_variable_definition<'a>(
    node: &Node<'a>,
    context: &Node<'a>,
    context_file: &Path,
    source: &[u8],
) -> Option<Node<'a>> {
    let variable_name = node.utf8_text(source).unwrap();

    let mut cursor = context.walk();
    if !cursor.goto_first_child() {
        error!("Context node is empty, kind: {}, start position: {:?}", context.kind(), context.start_position());
        return None;
    };

    let query = format!(
        r#"
        (assignment 
            left: (identifier) @variable (#eq? @variable {variable_name})
            right: (_)) @assignment
        "#
    );
    // TODO: handle unwrap
    let query = Query::new(tree_sitter_ruby::language(), query.as_str()).unwrap();

    let closest_assignment = QueryCursor::new()
        .matches(&query, *context, source)
        .flat_map(|m| m.captures)
        .map(|c| c.node)
        .filter(|n| n.range() < node.range())
        .sorted_by_key(|n| n.range())
        .last();
    // TODO: determine reachability from assignment to node (e.g. if assignment is not in the
    // correct if branch)

    match closest_assignment {
        Some(n) => return Some(n),

        None => {
            info!("Variable assignment for '{variable_name}' wasn't found in the method body, checking method params");

            // check method params
            for param_node in get_method_param_nodes(context_file, context) {
                info!("param_node: {param_node:?}");
                match param_node.kind().try_into().unwrap() {
                    NodeKind::Identifier => {
                        let param_name = param_node.utf8_text(source).unwrap();

                        info!("param name: {param_name}");

                        if param_name == variable_name {
                            return Some(param_node);
                        }
                    }
                    NodeKind::OptionalParameter => {
                        let name_node = param_node.child_by_field_name(NodeName::Name).unwrap();
                        let name = name_node.utf8_text(source).unwrap().to_string();

                        info!("param name: {name}");

                        if name == variable_name {
                            return Some(param_node);
                        }
                    }
                    NodeKind::KeywordParameter => {
                        let name_node = param_node.child_by_field_name(NodeName::Name).unwrap();
                        let name = name_node.utf8_text(source).unwrap().to_string();

                        info!("param name: {name}");

                        if name == variable_name {
                            return Some(param_node);
                        }
                    }

                    _ => unreachable!(),
                }
            }
        }
    };

    None
}

fn get_method_param_nodes<'a>(file: &Path, method_node: &Node<'a>) -> Vec<Node<'a>> {
    let mut params = Vec::new();

    let mut cursor = method_node.walk();
    if let Some(method_parameters) = method_node.child_by_field_name(NodeName::Parameters) {
        for param in method_parameters.children(&mut cursor) {
            match param.kind().try_into() {
                Err(_) => {}
                Ok(kind) => match kind {
                    NodeKind::Identifier | NodeKind::OptionalParameter | NodeKind::KeywordParameter => {
                        params.push(param)
                    }

                    _ => warn!(
                        "New kind of method kind in {file:?} at {:?}: {}",
                        method_node.start_position(),
                        param.kind()
                    ),
                },
            };
        }
    }

    params
}
