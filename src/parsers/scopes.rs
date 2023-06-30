use log::warn;
use tree_sitter::Node;

use crate::parsers::types::GLOBAL_SCOPE_VALUE;

use super::types::{NodeKind, NodeName};

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
    let is_scope = scope_node.map(|n| n.range() == node.range()).unwrap_or(false);
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
                    }

                    NodeKind::Constant => {
                        scopes.push(s.utf8_text(source).unwrap());
                        break;
                    }

                    // weird module definitions with variables
                    NodeKind::ClassVariable | NodeKind::InstanceVariable => {
                        warn!(
                            "Couldn't get parent scope resolution for definition: {}",
                            node.utf8_text(source).unwrap()
                        );
                        return vec![];
                    }

                    _ => panic!(
                        "Impossible kind in scope resolution: {}: {}",
                        p.to_sexp(),
                        p.utf8_text(source).unwrap()
                    ),
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
    let is_scope = scope_node.map(|n| n.range() == node.range()).unwrap_or(false);
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

    if full_scope.first().map(|s| *s == GLOBAL_SCOPE_VALUE).unwrap_or(false) {
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
        let node = parsed.root_node().descendant_for_point_range(*point, *point).unwrap();

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

