#[cfg(not(target_env = "msvc"))]
use jemallocator::Jemalloc;
use log::{info, error};

#[cfg(not(target_env = "msvc"))]
#[global_allocator]
static GLOBAL: Jemalloc = Jemalloc;

#[macro_use]
extern crate anyhow;

use std::time::Instant;

use anyhow::Result;

use lsp_server::{Connection, ExtractError, Message, Request, RequestId, Response};
use lsp_types::{
    request::{DocumentSymbolRequest, WorkspaceSymbolRequest },
    InitializeParams, Location, OneOf, Position, Range, ServerCapabilities, SymbolInformation,
    SymbolKind, Url, WorkspaceSymbolParams,
};

mod indexer_v2;
use indexer_v2::*;

mod symbols_matcher;

mod progress_reporter;
use progress_reporter::ProgressReporter;

fn main() -> Result<()> {
    let file = log4rs::append::file::FileAppender::builder()
        .encoder(Box::new(log4rs::encode::pattern::PatternEncoder::new(
            "{d} - {m}{n}",
        )))
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

    // TODO: fix unwraps
    let path = params.root_uri.unwrap().to_file_path().unwrap();

    let progess_reporter = ProgressReporter::new(&connection.sender);
    let mut indexer = IndexerV2::new(path.as_path(), progess_reporter);
    indexer.index()?;

    for msg in &connection.receiver {
        eprintln!("got msg: {msg:?}");

        match msg {
            Message::Request(req) => {
                use lsp_types::request::Request;
                if connection.handle_shutdown(&req)? {
                    return Ok(());
                }

                eprintln!("got request: {req:?}");

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
                    }

                    m => error!("Unknown method: {}", m)
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

fn handle_document_symbols_request(
    indexer: &IndexerV2,
    id: RequestId,
    params: lsp_types::DocumentSymbolParams,
    connection: &Connection,
) -> Result<()> {
    let start = Instant::now();

    info!("[#{id}] Got document/symbol request, params = {params:?}");

    let path = params.text_document.uri.to_file_path().unwrap();
    let symbols: Vec<SymbolInformation> = indexer.file_symbols(path.as_path()).unwrap_or(&Vec::new())
        .iter()
        .map(|s| convert_to_lsp_sym_info(s))
        .collect();

    let result = serde_json::to_value(symbols).unwrap();

    info!("[#{id}] document/symbol took {:?}", start.elapsed());

    let resp = Response {
        id,
        result: Some(result),
        error: None
    };
    connection.sender.send(Message::Response(resp))?;

    Ok(())
}

fn handle_workspace_symbols_request(
    indexer_v2: &IndexerV2,
    id: RequestId,
    params: WorkspaceSymbolParams,
    connection: &Connection,
) -> Result<()> {
    eprintln!("got workspace/symbol request #{id}: {params:?}");

    let start = Instant::now();

    let symbols: Vec<SymbolInformation> = indexer_v2
        .fuzzy_find_symbol(&params.query)
        .iter()
        .map(|s| convert_to_lsp_sym_info(s))
        .collect();

    let result = serde_json::to_value(symbols).unwrap();
    let resp = Response {
        id,
        result: Some(result),
        error: None,
    };
    connection.sender.send(Message::Response(resp))?;

    let duration = start.elapsed();

    eprintln!("workspace/symbool took {:?}", duration);

    Ok(())
}

fn convert_to_lsp_sym_info(rsymbol: &RSymbol) -> SymbolInformation {
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

    SymbolInformation {
        name: name.to_string(),
        kind,
        tags: None,
        deprecated: None,
        location: Location { uri: url, range },
        container_name: None,
    }
}
