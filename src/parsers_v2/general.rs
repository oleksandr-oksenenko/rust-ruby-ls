use std::{fs, path::Path, sync::Arc};

use anyhow::Result;
use log::info;
use tree_sitter::{Node, Parser, Tree};
use tree_sitter_ruby::language;

use crate::types::{NodeKind, RSymbolV2};

use super::{
    assignments::parse_assignment,
    classes::parse_class,
    methods::{parse_method, parse_singleton_method}, calls::parse_call,
};

pub fn parse(file: &Path, source: &[u8], node: Node, parent: Option<Arc<RSymbolV2>>) -> Vec<Arc<RSymbolV2>> {
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

        NodeKind::Method => vec![Arc::new(parse_method(file, source, node, parent))],

        NodeKind::SingletonMethod => vec![Arc::new(parse_singleton_method(file, source, node, parent))],

        NodeKind::Assignment => {
            parse_assignment(file, source, node, parent).unwrap_or(Vec::new()).into_iter().map(Arc::new).collect()
        }

        NodeKind::Call => {
            parse_call(&node, file, source, parent).unwrap_or(Vec::default())
        }

        NodeKind::Comment => {
            // TODO: Implement
            vec![]
        }

        _ => {
            // warn!( "Unknown node kind: {}", node.kind());
            vec![]
        }
    }
}

pub fn read_file_tree(path: &Path) -> Result<(Tree, Vec<u8>)> {
    let source = fs::read(path)?;

    let mut parser = Parser::new();
    parser.set_language(language())?;
    let tree = parser.parse(&source[..], None).unwrap();

    Ok((tree, source))
}
