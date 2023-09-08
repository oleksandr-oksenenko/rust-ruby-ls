use std::{path::Path, sync::Arc};

use log::{error, info};
use tree_sitter::Node;

use crate::types::{RSymbolV2, NodeName, RSymbolKind, Scope};

pub fn parse_call(node: &Node, file: &Path, source: &[u8], parent: Option<Arc<RSymbolV2>>) -> Option<Vec<Arc<RSymbolV2>>> {
    let method_name = node.child_by_field_name(NodeName::Method).unwrap().utf8_text(source).unwrap();
    let args = match node.child_by_field_name(NodeName::Arguments) {
        None => {
            info!("No args provided for {method_name}");
            return None
        },
        Some(a) => a
    };

    match method_name {
        "attr_accessor" | "attr_reader" | "attr_writer" | "delegate" | "belongs_to" | "has_one" | "has_many" => {
            let first_child = args.child(0).unwrap();
            let var_name = first_child.utf8_text(source).unwrap().to_owned();
            let var_name = var_name.replace(':', "");

            let scope = parent.as_ref().map(|p| p.scope.join(&(&p.name).into())).unwrap_or(Scope::default());

            let symbol = RSymbolV2 {
                kind: RSymbolKind::InstanceVariable,
                name: var_name,
                scope,
                file: file.to_path_buf(),
                start: first_child.start_position(),
                end: first_child.end_position(),
                parent,
            };

            Some(vec![Arc::new(symbol)])
        },

        // TODO: impelement include and extend
        "require" | "require_relative" | "include" | "extend" => None,

        _ => {
            error!("Unknown method call in class context: {method_name}");
            None
        }
    }
}
