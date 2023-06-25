#[cfg(not(target_env = "msvc"))]
use jemallocator::Jemalloc;
use log::{error, info};
use tree_sitter::Point;

#[cfg(not(target_env = "msvc"))]
#[global_allocator]
static GLOBAL: Jemalloc = Jemalloc;

#[macro_use]
extern crate anyhow;

use std::time::Instant;

use anyhow::Result;

use lsp_server::{Connection, ExtractError, Message, Request, RequestId, Response};
use lsp_types::{
    request::{DocumentSymbolRequest, GotoDefinition, WorkspaceSymbolRequest},
    GotoDefinitionParams, GotoDefinitionResponse, InitializeParams, Location, OneOf, Position, Range,
    ServerCapabilities, SymbolInformation, SymbolKind, Url, WorkspaceSymbolParams,
};

mod indexer;
mod parsers;
mod progress_reporter;
mod ruby_env_provider;
mod ruby_filename_converter;
mod symbols_matcher;

use indexer::*;
use progress_reporter::ProgressReporter;

fn main() -> Result<()> {
    let file = log4rs::append::file::FileAppender::builder()
        .encoder(Box::new(log4rs::encode::pattern::PatternEncoder::new("{d} - {m}{n}")))
        .build("/Users/oleksandr.oksenenko/code/rust-ruby-ls/lsp.log")
        .unwrap();
    let config = log4rs::Config::builder()
        .appender(log4rs::config::Appender::builder().build("file", Box::new(file)))
        .build(
            log4rs::config::Root::builder()
                .appender("file")
                .build(log::LevelFilter::Info),
        )
        .unwrap();
    log4rs::init_config(config).unwrap();

    let (connection, io_threads) = Connection::stdio();

    let server_capabilities = serde_json::to_value(ServerCapabilities {
        workspace_symbol_provider: Some(OneOf::Left(true)),
        document_symbol_provider: Some(OneOf::Left(true)),
        definition_provider: Some(OneOf::Left(true)),
        ..Default::default()
    })
    .unwrap();

    let initialization_params = connection.initialize(server_capabilities)?;
    main_loop(connection, initialization_params)?;
    io_threads.join()?;

    info!("shutting down the server");

    Ok(())
}

fn main_loop(connection: Connection, params: serde_json::Value) -> Result<()> {
    let params: InitializeParams = serde_json::from_value(params).unwrap();

    info!("start main loop");

    // TODO: fix unwraps
    let path = params.root_uri.unwrap().to_file_path().unwrap();

    let progess_reporter = ProgressReporter::new(&connection.sender);
    let mut indexer = Indexer::new(path.as_path(), progess_reporter);
    indexer.index()?;

    for msg in &connection.receiver {
        match msg {
            Message::Request(req) => {
                use lsp_types::request::Request;
                if connection.handle_shutdown(&req)? {
                    return Ok(());
                }

                match req.method.as_str() {
                    WorkspaceSymbolRequest::METHOD => match cast::<WorkspaceSymbolRequest>(req) {
                        Ok((id, params)) => {
                            handle_workspace_symbols_request(&indexer, id, params, &connection)?;
                            continue;
                        }

                        Err(ExtractError::JsonError { .. }) => error!("JsonError"),
                        Err(ExtractError::MethodMismatch(_)) => error!("MethodMismatch"),
                    },

                    DocumentSymbolRequest::METHOD => match cast::<DocumentSymbolRequest>(req) {
                        Ok((id, params)) => {
                            handle_document_symbols_request(&indexer, id, params, &connection)?;
                            continue;
                        }

                        Err(ExtractError::JsonError { .. }) => error!("JsonError"),
                        Err(ExtractError::MethodMismatch(_)) => error!("MethodMismatch"),
                    },

                    GotoDefinition::METHOD => match cast::<GotoDefinition>(req) {
                        Ok((id, params)) => {
                            handle_goto_definition_request(&indexer, id, params, &connection)?;
                            continue;
                        }

                        Err(ExtractError::JsonError { .. }) => error!("JsonError"),
                        Err(ExtractError::MethodMismatch(_)) => error!("MethodMismatch"),
                    },

                    m => error!("Unknown method: {}", m),
                };
            }

            Message::Response(resp) => {
                info!("got response: {resp:?}")
            }

            Message::Notification(not) => {
                info!("got notification: {not:?}")
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

fn handle_document_symbols_request(
    indexer: &Indexer,
    id: RequestId,
    params: lsp_types::DocumentSymbolParams,
    connection: &Connection,
) -> Result<()> {
    let start = Instant::now();

    info!("[#{id}] Got document/symbol request, params = {params:?}");

    let path = params.text_document.uri.to_file_path().unwrap();
    let symbols: Vec<SymbolInformation> = indexer
        .file_symbols(path.as_path())
        .unwrap_or(&Vec::new())
        .iter()
        .map(convert_to_lsp_sym_info)
        .collect();

    let result = serde_json::to_value(symbols).unwrap();

    info!("[#{id}] document/symbol took {:?}", start.elapsed());

    let resp = Response {
        id,
        result: Some(result),
        error: None,
    };
    connection.sender.send(Message::Response(resp))?;

    Ok(())
}

fn handle_workspace_symbols_request(
    indexer_v2: &Indexer,
    id: RequestId,
    params: WorkspaceSymbolParams,
    connection: &Connection,
) -> Result<()> {
    info!("got workspace/symbol request #{id}: {params:?}");

    let start = Instant::now();

    let symbols: Vec<SymbolInformation> = indexer_v2
        .fuzzy_find_symbol(&params.query)
        .iter()
        .map(convert_to_lsp_sym_info)
        .collect();

    let result = serde_json::to_value(symbols).unwrap();
    let resp = Response {
        id,
        result: Some(result),
        error: None,
    };
    connection.sender.send(Message::Response(resp))?;

    let duration = start.elapsed();

    info!("workspace/symbool took {:?}", duration);

    Ok(())
}

fn handle_goto_definition_request(
    indexer: &Indexer,
    id: RequestId,
    params: GotoDefinitionParams,
    connection: &Connection,
) -> Result<()> {
    info!("got textDocument/definition request #{id}: {params:?}");

    let start = Instant::now();

    let file = params
        .text_document_position_params
        .text_document
        .uri
        .to_file_path()
        .unwrap();
    let position = params.text_document_position_params.position;
    let position = Point {
        row: position.line.try_into()?,
        column: position.character.try_into()?,
    };

    let symbols: Vec<Location> = indexer
        .find_definition(file.as_path(), position)?
        .iter()
        .map(convert_to_lsp_sym_info)
        .map(|s| s.location)
        .collect();

    info!("textDocument/definition found {} symbols", symbols.len());

    let result = GotoDefinitionResponse::Array(symbols);
    let result = serde_json::to_value(result).unwrap();
    let resp = Response {
        id,
        result: Some(result),
        error: None,
    };
    connection.sender.send(Message::Response(resp))?;

    let duration = start.elapsed();

    info!("textDocument/definition took {:?}", duration);

    Ok(())
}

fn convert_to_lsp_sym_info(rsymbol: impl AsRef<RSymbol>) -> SymbolInformation {
    let rsymbol = rsymbol.as_ref();
    let path = rsymbol.file();
    let file_path_str = path.to_str().unwrap();
    let url = Url::parse(&format!("file:///{}", file_path_str)).unwrap();

    let location = rsymbol.location();
    let line: u32 = location.row.try_into().unwrap();
    let character: u32 = location.column.try_into().unwrap();

    let name = rsymbol.name();
    let name_len: u32 = name.len().try_into().unwrap();

    let range = Range {
        start: Position::new(line, character),
        end: Position::new(line, character + name_len),
    };

    let kind = match rsymbol {
        RSymbol::Class(_) => SymbolKind::CLASS,
        RSymbol::Module(_) => SymbolKind::MODULE,
        RSymbol::Method(_) => SymbolKind::METHOD,
        RSymbol::SingletonMethod(_) => SymbolKind::METHOD,
        RSymbol::Constant(_) => SymbolKind::CONSTANT,
        _ => SymbolKind::NULL,
    };

    #[allow(deprecated)]
    SymbolInformation {
        name: name.to_string(),
        kind,
        tags: None,
        deprecated: None,
        location: Location { uri: url, range },
        container_name: None,
    }
}
