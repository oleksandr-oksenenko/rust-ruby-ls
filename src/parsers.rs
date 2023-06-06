use std::{path::Path, sync::Arc};

use itertools::Itertools;
use log::{error, info, warn};
use strum::{AsRefStr, Display, EnumString, IntoStaticStr};
use tree_sitter::Node;

use crate::indexer_v2::{RClass, RConstant, RMethod, RMethodParam, RSymbol};

const SCOPE_DELIMITER: &str = "::";

#[derive(PartialEq, Eq, Debug, EnumString, AsRefStr, IntoStaticStr, Display)]
enum NodeKind {
    Class,
    Module,
    Method,
    SingletonMethod,
    Assignment,
    Program,
    Comment,
    Call,
    Constant,
    LeftAssignmentList,
    GlobalVariable,
    ScopeResolution,
    ClassVariable,
    InstanceVariable,
    Identifier,
    ElementReference,
    RestAssignment,
    OptionalParameter,
    KeywordParameter,
}

impl PartialEq<NodeKind> for &str {
    fn eq(&self, other: &NodeKind) -> bool {
        let other: &str = other.into();
        (*self).eq(other)
    }
}
impl PartialEq<&str> for NodeKind {
    fn eq(&self, other: &&str) -> bool {
        let s: &str = self.into();
        s.eq(*other)
    }
}


#[derive(PartialEq, Eq, Debug, EnumString, AsRefStr, IntoStaticStr, Display)]
enum NodeName {
    Name,
    Superclass,
    Body,
    Scope,
    Left,
    MethodParameters,
}

impl AsRef<[u8]> for NodeName {
    fn as_ref(&self) -> &[u8] {
        Into::<&str>::into(self).as_bytes()
    }
}

pub fn parse(
    file: &Path,
    source: &[u8],
    node: Node,
    parent: Option<Arc<RSymbol>>,
) -> Vec<Arc<RSymbol>> {
    match node.kind().try_into().unwrap() {
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

        NodeKind::Program => {
            info!("empty file: {:?}", file);
            vec![]
        }

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

pub fn parse_class(
    file: &Path,
    source: &[u8],
    node: Node,
    parent: Option<Arc<RSymbol>>,
) -> Vec<Arc<RSymbol>> {
    assert!(node.kind() == NodeKind::Class || node.kind() == NodeKind::Module);

    let name_node = node.child_by_field_name(NodeName::Name).unwrap();
    let scopes = get_scopes(&name_node, source);
    let name = scopes.iter().join(SCOPE_DELIMITER);
    let superclass_scopes = node
        .child_by_field_name(NodeName::Superclass)
        .map(|n| get_scopes(&n, source))
        .unwrap_or(Vec::default());

    let rclass = RClass {
        file: file.to_path_buf(),
        name,
        location: name_node.start_position(),
        scopes: get_scopes(&name_node, source),
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

pub fn parse_method(
    file: &Path,
    source: &[u8],
    node: Node,
    parent: Option<Arc<RSymbol>>,
) -> RSymbol {
    assert!(node.kind() == NodeKind::Method || node.kind() == NodeKind::SingletonMethod);

    let scopes = match &parent {
        Some(p) => match &**p {
            RSymbol::Class(c) | RSymbol::Module(c) => Some(&c.scopes),
            _ => None,
        },

        None => None,
    };

    let name_node = node.child_by_field_name(NodeName::Name).unwrap();
    let name = get_node_text(&name_node, source);
    let name = match scopes {
        Some(s) => s.iter().join(SCOPE_DELIMITER) + SCOPE_DELIMITER+ &name,
        None => name,
    };

    let mut cursor = node.walk();
    let mut params: Vec<RMethodParam> = Vec::new();
    if let Some(method_parameters) = node.child_by_field_name(NodeName::MethodParameters) {
        for param in method_parameters.children(&mut cursor) {
            let param = match param.kind().try_into().unwrap() {
                NodeKind::Identifier => RMethodParam::Regular(get_node_text(&param, source)),
                NodeKind::OptionalParameter => {
                    let name =
                        get_node_text(&param.child_by_field_name(NodeName::Name).unwrap(), source);
                    RMethodParam::Optional(name)
                }
                NodeKind::KeywordParameter => {
                    let name =
                        get_node_text(&param.child_by_field_name(NodeName::Name).unwrap(), source);
                    RMethodParam::Keyword(name)
                }

                _ => unreachable!(),
            };

            params.push(param);
        }
    }

    RSymbol::Method(RMethod {
        file: file.to_owned(),
        name,
        location: name_node.start_position(),
        parameters: params,
        parent,
    })
}

pub fn parse_singleton_method(
    file: &Path,
    source: &[u8],
    node: Node,
    parent: Option<Arc<RSymbol>>,
) -> RSymbol {
    match parse_method(file, source, node, parent) {
        RSymbol::Method(method) => RSymbol::SingletonMethod(method),
        _ => unreachable!(),
    }
}

fn parse_assignment(
    file: &Path,
    source: &[u8],
    node: Node,
    parent: Option<Arc<RSymbol>>,
) -> Option<Vec<RSymbol>> {
    assert_eq!(node.kind(), NodeKind::Assignment);

    let lhs = node.child_by_field_name(NodeName::Left).unwrap();

    match lhs.kind().try_into().unwrap() {
        NodeKind::Constant => parse_constant(file, source, &lhs, parent).map(|c| vec![c]),

        NodeKind::LeftAssignmentList => {
            // Only handle constants
            let mut cursor = lhs.walk();
            Some(
                lhs.named_children(&mut cursor)
                    .filter(|n| n.kind() == NodeKind::Constant || n.kind() ==NodeKind::RestAssignment)
                    .filter_map(|node| parse_constant(file, source, &node, parent.clone()))
                    .collect(),
            )
        }

        NodeKind::GlobalVariable => {
            // TODO: parse global variables as constants
            None
        }

        NodeKind::ScopeResolution => {
            // TODO: parse scope resolution constant assignment
            None
        }

        NodeKind::InstanceVariable | NodeKind::ClassVariable => {
            // TODO: parse instance and class variables
            None
        }

        NodeKind::Identifier => {
            // TODO: variable declaration, should parse?
            None
        }

        NodeKind::ElementReference => {
            // TODO: e.g. putting into a Hash or Array, should parse?
            None
        }

        NodeKind::Call => {
            // TODO: parse attr_accessors
            None
        }

        _ => {
            warn!(
                "Unknown assignment 'left' node kind: {}, file: {:?}, range: {:?}",
                lhs.kind(),
                file,
                lhs.range()
            );
            None
        }
    }
}

pub fn parse_constant(
    file: &Path,
    source: &[u8],
    node: &Node,
    parent: Option<Arc<RSymbol>>,
) -> Option<RSymbol> {
    if node.kind() != NodeKind::Constant && node.kind() != NodeKind::RestAssignment {
        error!(
            "{} instead of constant in {file:?} at {:?}",
            node.kind(),
            node.range()
        );
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
    let text = get_node_text(&node, source);

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

pub fn get_node_text(node: &Node, source: &[u8]) -> String {
    node.utf8_text(source).unwrap().to_owned()
}

pub fn get_node_parent_scope(node: &Node, source: &[u8]) -> Vec<String> {
    let mut scopes = Vec::new();

    let mut node = Some(*node);
    while let Some(p) = node {
        match p.kind().try_into().unwrap() {
            NodeKind::Class | NodeKind::Module => {
                let name_node = p.child_by_field_name(NodeName::Name).unwrap();
                let mut class_scopes = get_scopes(&name_node, source);
                class_scopes.reverse();
                scopes.append(&mut class_scopes);

                node = p.parent()
            }

            _ => node = p.parent(),
        }
    }

    scopes
}

pub fn get_partial_scope<'b>(node: &Node, source: &'b [u8]) -> Vec<&'b str> {
    assert!(node.kind() == NodeKind::Constant);

    let parent = node.parent().unwrap();
    if parent.kind() != NodeKind::ScopeResolution {
        // single constant without a scope
        return vec![node.utf8_text(source).unwrap()];
    }

    // determine if node is a "scope" or a "name"
    let scope_node = parent.child_by_field_name(NodeName::Scope).unwrap();
    let name_node = parent.child_by_field_name(NodeName::Name).unwrap();
    let is_scope = scope_node.range() == node.range();
    let is_name = name_node.range() == node.range();
    assert!(is_scope || is_name);

    // it's the first constant in the "scope_resolution", just return it (e.g. A in A::B::C)
    if is_scope {
        return vec![node.utf8_text(source).unwrap()];
    }

    // go down from the current node to get scopes on the left (e.g. A::B::C in A::B::C::D if
    // cursor is on C)
    let mut scopes = Vec::new();
    let parent = node.parent();
    if let Some(p) = parent {
        // if let + condition is in nightly only
        if p.kind() == NodeKind::ScopeResolution {
            let name = p.child_by_field_name(NodeName::Name).unwrap();
            scopes.push(name.utf8_text(source).unwrap());

            let mut scope = p.child_by_field_name(NodeName::Scope);
            while let Some(s) = scope {
                match s.kind().try_into().unwrap() {
                    NodeKind::ScopeResolution => {
                        let name = s.child_by_field_name(NodeName::Name).unwrap();
                        scopes.push(name.utf8_text(source).unwrap());
                        scope = s.child_by_field_name(NodeName::Scope);
                    }
                    NodeKind::Constant => {
                        scopes.push(s.utf8_text(source).unwrap());
                        break;
                    }
                    _ => unreachable!(),
                }
            }
        }
    }

    scopes
}

pub fn get_scopes(main_node: &Node, source: &[u8]) -> Vec<String> {
    let mut scopes = Vec::new();

    if main_node.kind() == NodeKind::ScopeResolution {
        let mut node = *main_node;
        while node.kind() == NodeKind::ScopeResolution {
            let name_node = node.child_by_field_name(NodeName::Name).unwrap();
            let name = name_node.utf8_text(source).unwrap().to_owned();
            scopes.push(name);

            let child = node.child_by_field_name(NodeName::Scope);
            match child {
                None => break,
                Some(n) => node = n,
            }
        }
        if node.kind() == NodeKind::Constant {
            let name = node.utf8_text(source).unwrap().to_owned();
            scopes.push(name);
        }
    }
    if main_node.kind() == NodeKind::Constant {
        let name = main_node.utf8_text(source).unwrap().to_owned();
        scopes.push(name);
    }

    let class_node = main_node.parent();
    let mut class_parent_node = class_node.and_then(|p| p.parent());
    while let Some(parent) = class_parent_node {
        if parent.kind() == NodeKind::Class || parent.kind() == NodeKind::Module {
            let parent_class_name = parent.child_by_field_name(NodeName::Name).unwrap();
            let scope = parent_class_name.utf8_text(source).unwrap().to_owned();
            scopes.push(scope);
        }
        class_parent_node = parent.parent();
    }
    scopes.reverse();

    scopes
}
