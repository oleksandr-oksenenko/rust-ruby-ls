use std::{path::Path, sync::Arc};

use itertools::Itertools;
use log::{error, info, warn, debug};
use strum::{AsRefStr, Display, EnumString, IntoStaticStr};
use tree_sitter::Node;

use crate::indexer::{RClass, RConstant, RMethod, RMethodParam, RSymbol};

pub const SCOPE_DELIMITER: &str = "::";

pub const GLOBAL_SCOPE_VALUE: &str = "$GLOBAL";

#[derive(PartialEq, Eq, Debug, EnumString, AsRefStr, IntoStaticStr, Display)]
#[strum(serialize_all = "snake_case")]
pub enum NodeKind {
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

#[derive(PartialEq, Eq, Debug, EnumString, AsRefStr, IntoStaticStr, Display)]
#[strum(serialize_all = "snake_case")]
pub enum NodeName {
    Name,
    Superclass,
    Body,
    Scope,
    Left,
    MethodParameters,
    Receiver,
    Method
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
    let node_kind = match node.kind().try_into() {
        Ok(k) => k,
        Err(_) => return vec![],
    };

    match node_kind {
        NodeKind::Program => {
            info!("empty file: {:?}", file);
            vec![]
        },

        NodeKind::Class | NodeKind::Module => parse_class(file, source, node, parent),

        NodeKind::Method => {
            vec![Arc::new(parse_method(file, source, node, parent))]
        },

        NodeKind::SingletonMethod => {
            vec![Arc::new(parse_singleton_method(file, source, node, parent))]
        },

        NodeKind::Assignment => parse_assignment(file, source, node, parent)
            .unwrap_or(Vec::new())
            .into_iter()
            .map(Arc::new)
            .collect(),

        NodeKind::Comment | NodeKind::Call => {
            // TODO: Implement
            vec![]
        },

        _ => {
            // warn!( "Unknown node kind: {}", node.kind());
            vec![]
        },
    }
}

pub fn parse_class(
    file: &Path,
    source: &[u8],
    node: Node,
    parent: Option<Arc<RSymbol>>,
) -> Vec<Arc<RSymbol>> {
    debug!("Parsing {:?} at {:?}", file, node.start_position());

    assert!(node.kind() == NodeKind::Class || node.kind() == NodeKind::Module);

    let name_node = node.child_by_field_name(NodeName::Name).unwrap();
    let scopes = get_full_and_context_scope(&name_node, source);
    let name = scopes.iter().join(SCOPE_DELIMITER);
    let superclass_scopes = node
        .child_by_field_name(NodeName::Superclass)
        .and_then(|n| n.child_by_field_name(NodeName::Name))
        .map(|n| get_full_and_context_scope(&n, source))
        .map(|s| s.into_iter().map(|s| s.to_string()).collect())
        .unwrap_or(Vec::default());

    let rclass = RClass {
        file: file.to_path_buf(),
        name,
        location: name_node.start_position(),
        scopes: scopes.into_iter().map(|s| s.to_string()).collect(),
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
    let name = name_node.utf8_text(source).unwrap().to_string();
    let name = match scopes {
        Some(s) => s.iter().join(SCOPE_DELIMITER) + SCOPE_DELIMITER + &name,
        None => name,
    };

    let mut cursor = node.walk();
    let mut params: Vec<RMethodParam> = Vec::new();
    if let Some(method_parameters) = node.child_by_field_name(NodeName::MethodParameters) {
        for param in method_parameters.children(&mut cursor) {
            let param = match param.kind().try_into().unwrap() {
                NodeKind::Identifier => {
                    RMethodParam::Regular(param.utf8_text(source).unwrap().to_string())
                }
                NodeKind::OptionalParameter => {
                    let name_node = param.child_by_field_name(NodeName::Name).unwrap();
                    let name = name_node.utf8_text(source).unwrap().to_string();
                    RMethodParam::Optional(name)
                }
                NodeKind::KeywordParameter => {
                    let name_node = param.child_by_field_name(NodeName::Name).unwrap();
                    let name = name_node.utf8_text(source).unwrap().to_string();
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
                    .filter(|n| {
                        n.kind() == NodeKind::Constant || n.kind() == NodeKind::RestAssignment
                    })
                    .filter_map(|node| parse_constant(file, source, &node, parent.clone()))
                    .collect(),
            )
        }

        NodeKind::GlobalVariable => {
            let name = lhs.utf8_text(source).unwrap().to_string();
            Some(vec![RSymbol::GlobalVariable(crate::indexer::RVariable {
                file: file.to_path_buf(),
                name,
                location: node.start_position(),
                parent: None
            })])
        }

        NodeKind::ScopeResolution => {
            info!("Scope resolution assignment: {}, file: {:?}, range: {:?}", node.to_sexp(), file, node.range());
            // TODO: parse scope resolution constant assignment
            None
        }

        NodeKind::InstanceVariable | NodeKind::ClassVariable => {
            info!("Instance/class variable assignment: {}, file: {:?}, range: {:?}", node.to_sexp(), file, node.range());
            // TODO: parse instance and class variables
            None
        }

        NodeKind::Identifier => {
            info!("Identifier assignment: {}, file: {:?}, range: {:?}", node.to_sexp(), file, node.range());
            // TODO: variable declaration, should parse?
            None
        }

        NodeKind::Call => {
            info!("Call assignment: {}, file: {:?}, range: {:?}", node.to_sexp(), file, node.range());
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

/*
 * Gets the scope of the enclosing classes and modules.
 * */
pub fn get_context_scope<'a>(node: &Node, source: &'a [u8]) -> Vec<&'a str> {
    let mut scopes = Vec::new();

    // find out if node is part of the class/module scope resolution
    let mut parent = node.parent();
    while let Some(p) = parent {
        match p.kind().try_into() {
            Err(_) => break,

            Ok(nk) => match nk {
                NodeKind::ScopeResolution => parent = p.parent(),
                NodeKind::Class | NodeKind::Module => {
                    parent = p.parent();
                    break;
                }

                _ => break,
            },
        }
    }

    // traverse all parents and write down all scopes found along the way
    while let Some(p) = parent {
        match p.kind().try_into() {
            Err(_) => parent = p.parent(),

            Ok(nk) => match nk {
                NodeKind::Class | NodeKind::Module => {
                    let class_name_node = p.child_by_field_name(NodeName::Name).unwrap();
                    let class_scopes = get_full_scope_resolution(&class_name_node, source);

                    scopes.push(class_scopes);

                    parent = p.parent()
                }

                _ => parent = p.parent(),
            },
        }
    }

    scopes.into_iter().rev().flatten().collect()
}

/*
 * Get the scope prior to the constant, e.g. if node is B in A::B::C the function will return [B, A].
 */
pub fn get_parent_scope_resolution<'b>(node: &Node, source: &'b [u8]) -> Vec<&'b str> {
    let node = if node.kind() == NodeKind::ScopeResolution {
        node.child_by_field_name(NodeName::Name).unwrap()
    } else {
        *node
    };

    assert!(node.kind() == NodeKind::Constant);

    let parent = node.parent().unwrap();
    if parent.kind() != NodeKind::ScopeResolution {
        // single constant without a scope
        return vec![node.utf8_text(source).unwrap()];
    }

    let scope_node = parent.child_by_field_name(NodeName::Scope);
    let name_node = parent.child_by_field_name(NodeName::Name).unwrap();
    let is_scope = scope_node
        .map(|n| n.range() == node.range())
        .unwrap_or(false);
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
                        let new_scope = s.child_by_field_name(NodeName::Scope);

                        if new_scope.is_none() {
                            scopes.push(GLOBAL_SCOPE_VALUE);
                        }

                        scope = new_scope
                    },

                    NodeKind::Constant => {
                        scopes.push(s.utf8_text(source).unwrap());
                        break;
                    },

                    // weird module definitions with variables
                    NodeKind::ClassVariable | NodeKind::InstanceVariable => {
                        warn!("Couldn't get parent scope resolution for definition: {}", node.utf8_text(source).unwrap());
                        return vec![]
                    },

                    _ => panic!("Impossible kind in scope resolution: {}: {}", p.to_sexp(), p.utf8_text(source).unwrap()),
                }
            }
        }
    }

    scopes.reverse();
    scopes
}

/*
 * Get the scope after the constant, e.g. if node is B in A::B::C the function will return [B, C].
 */
pub fn get_child_scope_resolution<'a>(node: &Node, source: &'a [u8]) -> Vec<&'a str> {
    let node = if node.kind() == NodeKind::ScopeResolution {
        node.child_by_field_name(NodeName::Name).unwrap()
    } else {
        *node
    };
    assert!(node.kind() == NodeKind::Constant);

    let parent = node.parent().unwrap();
    if parent.kind() != NodeKind::ScopeResolution {
        // single constant without a scope
        return vec![node.utf8_text(source).unwrap()];
    }

    let scope_node = parent.child_by_field_name(NodeName::Scope);
    let name_node = parent.child_by_field_name(NodeName::Name).unwrap();
    let is_scope = scope_node
        .map(|n| n.range() == node.range())
        .unwrap_or(false);
    let is_name = name_node.range() == node.range();
    assert!(is_scope || is_name);

    let mut scopes = Vec::new();
    if is_scope {
        scopes.push(node.utf8_text(source).unwrap());
    }

    let mut parent = node.parent();
    while let Some(p) = parent {
        if p.kind() != NodeKind::ScopeResolution {
            break;
        } else {
            let name = p.child_by_field_name(NodeName::Name).unwrap();
            scopes.push(name.utf8_text(source).unwrap());
            parent = p.parent();
        }
    }

    scopes
}

/*
 * Get both parent and child scopes of the scope resolution.
 */
pub fn get_full_scope_resolution<'a>(node: &Node, source: &'a [u8]) -> Vec<&'a str> {
    let child_scopes = get_child_scope_resolution(node, source);
    let parent_scopes = get_parent_scope_resolution(node, source);

    parent_scopes
        .into_iter()
        .chain(child_scopes.into_iter().skip(1))
        .collect()
}

/*
 * Get combined context scope and full scope resolution.
 */
pub fn get_full_and_context_scope<'a>(node: &Node, source: &'a [u8]) -> Vec<&'a str> {
    let full_scope = get_full_scope_resolution(node, source);

    if full_scope
        .first()
        .map(|s| *s == GLOBAL_SCOPE_VALUE)
        .unwrap_or(false)
    {
        return full_scope;
    }

    let context_scope = get_context_scope(node, source);

    context_scope.into_iter().chain(full_scope).collect()
}

#[cfg(test)]
mod tests {
    use tree_sitter::{Node, Parser, Point, Tree};

    use super::*;

    const SOURCE: &str = r#"
class A::B < X::Y::Z
    module C::D
        module E::F::G
            class H::I::J
                CONSTANT = ""
                V::C = 10
                include ::G::S
            end
        end
    end
end
"#;

    #[cfg(test)]
    mod get_child_scope_resolution_tests {
        use super::*;

        #[test]
        fn get_child_scope_resolution_test() {
            let point = Point { row: 3, column: 18 };
            let expected_scopes = vec!["F", "G"];

            test(SOURCE, &point, &expected_scopes, |n| {
                get_child_scope_resolution(n, SOURCE.as_bytes())
            })
        }

        #[test]
        fn get_child_scope_resolution_test_2() {
            let point = Point { row: 6, column: 16 };
            let expected_scopes = vec!["V", "C"];

            test(SOURCE, &point, &expected_scopes, |n| {
                get_child_scope_resolution(n, SOURCE.as_bytes())
            })
        }

        #[test]
        fn get_child_scope_resolution_test_3() {
            let point = Point { row: 1, column: 13 };
            let expected_scopes = vec!["X", "Y", "Z"];

            test(SOURCE, &point, &expected_scopes, |n| {
                get_child_scope_resolution(n, SOURCE.as_bytes())
            })
        }
    }

    #[cfg(test)]
    mod get_parent_scope_resolution_tests {
        use super::*;

        #[test]
        fn get_parent_scope_resolution_test() {
            let point = Point { row: 3, column: 18 };
            let expected_scopes = vec!["E", "F"];

            test(SOURCE, &point, &expected_scopes, |n| {
                get_parent_scope_resolution(n, SOURCE.as_bytes())
            })
        }

        #[test]
        fn get_parent_scope_resolution_test_2() {
            let point = Point { row: 6, column: 19 };
            let expected_scopes = vec!["V", "C"];

            test(SOURCE, &point, &expected_scopes, |n| {
                get_parent_scope_resolution(n, SOURCE.as_bytes())
            })
        }

        #[test]
        fn get_parent_scope_resolution_test_3() {
            let point = Point { row: 1, column: 19 };
            let expected_scopes = vec!["X", "Y", "Z"];

            test(SOURCE, &point, &expected_scopes, |n| {
                get_parent_scope_resolution(n, SOURCE.as_bytes())
            })
        }

        #[test]
        fn get_parent_scope_resolution_test_4() {
            let point = Point { row: 7, column: 29 };
            let expected_scopes = vec!["$GLOBAL", "G", "S"];

            test(SOURCE, &point, &expected_scopes, |n| {
                get_parent_scope_resolution(n, SOURCE.as_bytes())
            })
        }
    }

    #[cfg(test)]
    mod get_full_scope_resulution_tests {
        use super::*;

        #[test]
        fn get_full_scope_resolution_test() {
            let points = [Point { row: 1, column: 6 }, Point { row: 1, column: 9 }];
            let expected_scopes = vec!["A", "B"];

            for point in points {
                test(SOURCE, &point, &expected_scopes, |n| {
                    get_full_scope_resolution(n, SOURCE.as_bytes())
                })
            }
        }

        #[test]
        fn get_full_scope_resolution_test_2() {
            let points = [Point { row: 2, column: 11 }, Point { row: 2, column: 14 }];
            let expected_scopes = vec!["C", "D"];

            for point in points {
                test(SOURCE, &point, &expected_scopes, |n| {
                    get_full_scope_resolution(n, SOURCE.as_bytes())
                })
            }
        }

        #[test]
        fn get_full_scope_resolution_test_3() {
            let points = [
                Point { row: 3, column: 15 },
                Point { row: 3, column: 18 },
                Point { row: 3, column: 21 },
            ];
            let expected_scopes = vec!["E", "F", "G"];

            for point in points {
                test(SOURCE, &point, &expected_scopes, |n| {
                    get_full_scope_resolution(n, SOURCE.as_bytes())
                })
            }
        }

        #[test]
        fn get_full_scope_resolution_test_4() {
            let points = [
                Point { row: 4, column: 18 },
                Point { row: 4, column: 21 },
                Point { row: 4, column: 24 },
            ];
            let expected_scopes = vec!["H", "I", "J"];

            for point in points {
                test(SOURCE, &point, &expected_scopes, |n| {
                    get_full_scope_resolution(n, SOURCE.as_bytes())
                })
            }
        }
    }

    #[cfg(test)]
    mod get_context_scope_tests {
        use super::*;

        #[test]
        fn get_context_scope_test() {
            let points = [Point { row: 2, column: 11 }, Point { row: 2, column: 14 }];
            let expected_scopes = vec!["A", "B"];

            for point in points {
                test(SOURCE, &point, &expected_scopes, |n| {
                    get_context_scope(n, SOURCE.as_bytes())
                })
            }
        }

        #[test]
        fn get_context_scope_test_2() {
            let points = [
                Point { row: 3, column: 15 },
                Point { row: 3, column: 18 },
                Point { row: 3, column: 21 },
            ];
            let expected_scopes = vec!["A", "B", "C", "D"];

            for point in points {
                test(SOURCE, &point, &expected_scopes, |n| {
                    get_context_scope(n, SOURCE.as_bytes())
                })
            }
        }

        #[test]
        fn get_context_scope_test_3() {
            let points = [
                Point { row: 4, column: 18 },
                Point { row: 4, column: 21 },
                Point { row: 4, column: 24 },
            ];
            let expected_scopes = vec!["A", "B", "C", "D", "E", "F", "G"];

            for point in points {
                test(SOURCE, &point, &expected_scopes, |n| {
                    get_context_scope(n, SOURCE.as_bytes())
                })
            }
        }

        #[test]
        fn get_context_scope_test_4() {
            let points = [Point { row: 5, column: 18 }, Point { row: 5, column: 21 }];
            let expected_scopes = vec!["A", "B", "C", "D", "E", "F", "G", "H", "I", "J"];

            for point in points {
                test(SOURCE, &point, &expected_scopes, |n| {
                    get_context_scope(n, SOURCE.as_bytes())
                })
            }
        }
    }

    #[cfg(test)]
    mod get_full_and_context_scope_tests {
        use super::*;

        #[test]
        fn get_full_and_context_scope_test() {
            let point = Point { row: 6, column: 16 };
            let expected_scopes = vec!["A", "B", "C", "D", "E", "F", "G", "H", "I", "J", "V", "C"];

            test(SOURCE, &point, &expected_scopes, |n| {
                get_full_and_context_scope(n, SOURCE.as_bytes())
            })
        }

        #[test]
        fn get_full_and_context_scope_test_2() {
            let point = Point { row: 2, column: 11 };
            let expected_scopes = vec!["A", "B", "C", "D"];

            test(SOURCE, &point, &expected_scopes, |n| {
                get_full_and_context_scope(n, SOURCE.as_bytes())
            })
        }

        #[test]
        fn get_full_and_context_scope_test_3() {
            let point = Point { row: 7, column: 29 };
            let expected_scopes = vec![GLOBAL_SCOPE_VALUE, "G", "S"];

            test(SOURCE, &point, &expected_scopes, |n| {
                get_full_and_context_scope(n, SOURCE.as_bytes())
            })
        }
    }

    fn test<'a, F>(source: &str, point: &Point, expected_values: &[&'a str], f: F)
    where
        F: FnOnce(&Node) -> Vec<&'a str>,
    {
        let parsed = parse_source(source);
        let node = parsed
            .root_node()
            .descendant_for_point_range(*point, *point)
            .unwrap();

        let actual = f(&node);

        assert_eq!(expected_values, actual);
    }

    fn parse_source(source: &str) -> Tree {
        let language = tree_sitter_ruby::language();
        let mut parser = Parser::new();
        parser.set_language(language).unwrap();
        parser.parse(source.as_bytes(), None).unwrap()
    }
}
