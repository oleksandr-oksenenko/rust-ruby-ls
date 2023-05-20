#[cfg(not(target_env = "msvc"))]
use jemallocator::Jemalloc;

#[cfg(not(target_env = "msvc"))]
#[global_allocator]
static GLOBAL: Jemalloc = Jemalloc;

#[macro_use]
extern crate anyhow;

use std::ffi::OsStr;
use std::fs;
use std::path::{Path, PathBuf};
use std::str;
use std::time::{Duration, Instant};

use anyhow::Result;

use std::error::Error;

use env_logger::Env;
use log::{debug, error, info};

use tree_sitter::*;

use walkdir::WalkDir;

use rayon::prelude::*;

use itertools::Itertools;

use lsp_server::{Connection, ExtractError, Message, Request, RequestId, Response};
use lsp_types::OneOf;
use lsp_types::{
    request::WorkspaceSymbolRequest, request::WorkspaceFoldersRequest,
    InitializeParams, Location, Position, Range,
    ServerCapabilities, SymbolInformation, SymbolKind, Url,
};

fn main() -> Result<()> {
    env_logger::Builder::from_env(Env::default().default_filter_or("info")).init();

    eprintln!("start ruby language server");

    let (connection, io_threads) = Connection::stdio();

    let server_capabilities = serde_json::to_value(ServerCapabilities {
        workspace_symbol_provider: Some(OneOf::Left(true)),
        ..Default::default()
    })
    .unwrap();

    let initialization_params = connection.initialize(server_capabilities)?;
    main_loop(connection, initialization_params)?;
    io_threads.join()?;

    eprintln!("shutting down the server");

    Ok(())
}

fn main_loop(connection: Connection, params: serde_json::Value) -> Result<()> {
    let params: InitializeParams = serde_json::from_value(params).unwrap();

    eprintln!("start main loop");
    eprintln!("params: {:?}", params);

    // TODO: fix unwraps
    let path = params.root_uri.unwrap().to_file_path().unwrap();

    let indexer = Indexer::index_folder(path.as_path())?;

    for msg in &connection.receiver {
        eprintln!("got msg: {msg:?}");

        match msg {
            Message::Request(req) => {
                if connection.handle_shutdown(&req)? {
                    return Ok(());
                }

                eprintln!("got request: {req:?}");

                match cast::<WorkspaceSymbolRequest>(req) {
                    Ok((id, params)) => {
                        eprintln!("got workspace/symbol request #{id}: {params:?}");

                        let start = Instant::now();
                        let symbol_information = &indexer.symbols;

                        let result = serde_json::to_value(symbol_information).unwrap();
                        let resp = Response {
                            id,
                            result: Some(result),
                            error: None,
                        };
                        connection.sender.send(Message::Response(resp))?;

                        let duration = start.elapsed();

                        eprintln!("workspace/symbool took {:?}", duration);

                        continue;
                    }

                    Err(err @ ExtractError::JsonError { .. }) => panic!("{err:?}"),
                    Err(ExtractError::MethodMismatch(req)) => req,
                };
            }

            Message::Response(resp) => {
                eprintln!("got response: {resp:?}")
            }

            Message::Notification(not) => {
                eprintln!("got notification: {not:?}")
            }
        }
    }

    Ok(())
}

fn cast<R>(req: Request) -> std::result::Result<(RequestId, R::Params), ExtractError<Request>>
where
    R: lsp_types::request::Request,
    R::Params: serde::de::DeserializeOwned,
{
    req.extract(R::METHOD)
}

struct Method {
    name: String,
    params_count: u8,
    location: Point,
}

struct Class {
    name: String,
    scopes: Vec<String>,
    superclass: Option<Box<Class>>,
    path: Option<PathBuf>,
    location: Option<Point>,
    methods: Vec<Method>,
    singleton_methods: Vec<Method>,
}

struct Indexer {
    sources: Vec<PathBuf>,
    pub classes: Vec<Class>,

    pub symbols: Vec<SymbolInformation>,

    parser: Parser,
    class_query: Query,
    method_query: Query,
    singleton_method_query: Query,
}

impl Indexer {
    pub fn index_folder(folder: &Path) -> Result<Indexer> {
        let start = Instant::now();
        eprintln!("Started indexing");
        let mut indexer = Indexer::new()?;
        indexer.recursively_index_folder(folder)?;

        indexer.convert_to_symbol_info()?;

        eprintln!("Indexing done in {:?}", start.elapsed());

        Ok(indexer)
    }

    pub fn new() -> Result<Indexer> {
        let language = tree_sitter_ruby::language();
        let class_query = Self::create_query(
            r#"(class
                name: [(constant) (scope_resolution)] @class_name
                superclass: [(constant) (scope_resolution)]? @superclass_name)"#
            )?;
        let method_query = Self::create_query(
            r#"(method
            name: (identifier) @method_name
            parameters: (method_parameters
                            (identifier) @method_parameter)?)"#,
        )?;
        let singleton_method_query = Self::create_query(
            r#"(singleton_method
            name: (identifier) @method_name
            parameters: (method_parameters
                         (identifier) @method_parameters)?)"#,
        )?;

        let mut parser = Parser::new();
        parser.set_language(language).unwrap();

        Ok(Indexer {
            sources: Vec::new(),
            classes: Vec::new(),
            symbols: Vec::new(),
            class_query,
            method_query,
            singleton_method_query,
            parser,
        })
    }

    fn create_query(q: &str) -> Result<Query> {
        let language = tree_sitter_ruby::language();
        Ok(Query::new(language, q)?)
    }

    pub fn recursively_index_folder(&mut self, folder: &Path) -> Result<()> {
        WalkDir::new(folder)
            .into_iter()
            .filter_map(Result::ok)
            .filter(|e| !e.file_type().is_dir())
            .filter(|e| "rb" == e.path().extension().and_then(OsStr::to_str).unwrap_or(""))
            .for_each(|entry| {
                let path = entry.path();

                Self::index_file(
                    &mut self.parser,
                    &self.class_query,
                    &self.method_query,
                    &self.singleton_method_query,
                    &mut self.classes,
                    path,
                    ).unwrap();

                self.sources.push(path.to_path_buf());
            });

        Ok(())
    }

    fn convert_to_symbol_info(&mut self) -> Result<()> {
        self.symbols = self
            .classes
            .iter()
            .flat_map(|c| {
                let path = c.path.as_ref().unwrap();
                let file_path_str = path.to_str().unwrap();
                let url = Url::parse(&format!("file:///{}", file_path_str)).unwrap();
                let url_clone = url.clone();

                let location = c.location.unwrap();
                let line: u32 = location.row.try_into().unwrap();
                let character: u32 = location.column.try_into().unwrap();

                let class_name_len: u32 = c.name.len().try_into().unwrap();

                let range = Range {
                    start: Position::new(line, character),
                    end: Position::new(line, character + class_name_len),
                };

                let scopes = c.scopes.iter().join("::");
                let name = if scopes.is_empty() { c.name.clone() } else { scopes + "::" + c.name.as_str() };
                let class_info = SymbolInformation {
                    name,
                    kind: SymbolKind::CLASS,
                    tags: None,
                    deprecated: None,
                    location: Location { uri: url, range },
                    container_name: None,
                };

                let mut methods_info = c
                    .methods
                    .iter()
                    .map(|m| {
                        let line: u32 = m.location.row.try_into().unwrap();
                        let character: u32 = m.location.column.try_into().unwrap();

                        let name_len: u32 = m.name.len().try_into().unwrap();
                        let range = Range {
                            start: Position::new(line, character),
                            end: Position::new(line, character + name_len),
                        };

                        SymbolInformation {
                            name: m.name.clone(),
                            kind: SymbolKind::METHOD,
                            tags: None,
                            deprecated: None,
                            location: Location {
                                uri: url_clone.clone(),
                                range,
                            },
                            container_name: None,
                        }
                    })
                    .collect::<Vec<SymbolInformation>>();

                let mut symbols = Vec::with_capacity(methods_info.len() + 1);
                symbols.push(class_info);
                symbols.append(&mut methods_info);
                symbols
            })
            .collect::<Vec<SymbolInformation>>();

        Ok(())
    }

    pub fn index_file(
        parser: &mut Parser,
        class_query: &Query,
        method_query: &Query,
        singleton_method_query: &Query,
        classes: &mut Vec<Class>,
        file: &Path,
    ) -> Result<()> {
        eprintln!("Indexing source: {:?}", file);
        let source_str = fs::read_to_string(file)?;
        let source = source_str.as_bytes();
        let parsed = parser.parse(source, None).unwrap();

        let mut query_cursor = QueryCursor::new();

        for query_match in query_cursor.matches(class_query, parsed.root_node(), source) {
            if query_match.captures.is_empty() {
                return Err(anyhow!("No matches found in {file:?}"));
            }

            let class_name_node = query_match.captures[0].node;
            let class_scopes = Self::get_scopes(&class_name_node, source)?;

            let class_location = class_name_node.start_position();

            let methods = Self::get_methods(class_name_node.parent().unwrap(), method_query, source)?;

            let superclass = if query_match.captures.len() > 1 {
                let superclass_name_node = query_match.captures[1].node;
                let superclass_scopes = Self::get_scopes(&superclass_name_node, source)?;

                let mut iter = superclass_scopes.into_iter().rev();
                let name = iter.next().unwrap();
                let scopes = iter.rev().collect_vec();
                eprintln!("Superclass scopes: {:?}", scopes);
                Some(Box::new(Class {
                    name,
                    scopes,
                    superclass: None,
                    path: None,
                    location: None,
                    methods: Vec::new(),
                    singleton_methods: Vec::new()
                }))
            } else {
                None
            };

            eprintln!("All Class scopes: {:?}", class_scopes);
            let mut iter = class_scopes.into_iter().rev();
            let name = iter.next().unwrap();
            let scopes = iter.rev().collect_vec();
            eprintln!("Class scopes: {:?}", scopes);
            let class = Class {
                name,
                scopes,
                superclass,
                path: Some(file.to_owned()),
                location: Some(class_location),
                methods,
                singleton_methods: Vec::new(),
            };

            classes.push(class);
        }

        Ok(())
    }

    fn get_scopes(main_node: &Node, source: &[u8]) -> Result<Vec<String>> {
        let mut scopes = Vec::new();

        if main_node.kind() == "scope_resolution" {
            let mut node = *main_node;
            while node.kind() == "scope_resolution" {
                let name_node = node.child_by_field_name("name").unwrap();
                let name = name_node.utf8_text(source)?.to_owned();
                eprintln!("Pushing scope resolution name: {}", name);
                scopes.push(name);

                let child = node.child_by_field_name("scope");
                match child {
                    None => break,
                    Some(n) => node = n
                }
            }
            if node.kind() == "constant" {
                let name = node.utf8_text(source)?.to_owned();
                eprintln!("Pushing constant name: {}", name);
                scopes.push(name);
            }
        }
        if main_node.kind() == "constant" {
            let name = main_node.utf8_text(source)?.to_owned();
            eprintln!("Pushing main node constant name: {}", name);
            scopes.push(name);
        }

        let class_node = main_node.parent();
        let mut class_parent_node = class_node.and_then(|p| p.parent());
        while let Some(parent) = class_parent_node {
            if parent.kind() == "class" || parent.kind() == "module" {
                let parent_class_name = parent.child_by_field_name("name").unwrap();
                let scope = parent_class_name.utf8_text(source)?.to_owned();
                scopes.push(scope);
            }
            class_parent_node = parent.parent();
        }
        scopes.reverse();

        Ok(scopes)
    }

    fn get_methods(class_node: Node, method_query: &Query, source: &[u8]) -> Result<Vec<Method>> {
        let mut methods: Vec<Method> = Vec::new();

        let mut query_cursor = QueryCursor::new();

        for query_match in query_cursor.matches(method_query, class_node, source) {
            let method = query_match.captures.first().unwrap();
            let method_name = method.node.utf8_text(source)?.to_owned();
            let method_location = method.node.start_position();

            let params_count = query_match.captures.iter().skip(1).count();
            methods.push(Method {
                name: method_name,
                params_count: params_count.try_into()?,
                location: method_location,
            });
        }

        Ok(methods)
    }
}
