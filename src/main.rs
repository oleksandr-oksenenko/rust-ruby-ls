#[cfg(not(target_env = "msvc"))]
use jemallocator::Jemalloc;

#[cfg(not(target_env = "msvc"))]
#[global_allocator]
static GLOBAL: Jemalloc = Jemalloc;

#[macro_use]
extern crate anyhow;

use std::time::Instant;

use anyhow::Result;

use lsp_server::{Connection, ExtractError, Message, Request, RequestId, Response};
use lsp_types::{
    request::WorkspaceSymbolRequest,
    InitializeParams, OneOf, ServerCapabilities, WorkspaceSymbolParams, SymbolInformation
};

mod indexer;
use indexer::*;

mod symbols_matcher;
use symbols_matcher::SymbolsMatcher;

mod progress_reporter;
use progress_reporter::ProgressReporter;

fn main() -> Result<()> {
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

    // TODO: fix unwraps
    let path = params.root_uri.unwrap().to_file_path().unwrap();

    let progess_reporter = ProgressReporter::new(&connection.sender);
    let indexer = Indexer::index_folder(path.as_path(), progess_reporter)?;

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
                        handle_workspace_symbols_request(&indexer, id, params, &connection)?;
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


fn handle_workspace_symbols_request(
    indexer: &Indexer,
    id: RequestId,
    params: WorkspaceSymbolParams,
    connection: &Connection,
) -> Result<()> {
    eprintln!("got workspace/symbol request #{id}: {params:?}");

    let start = Instant::now();

    let symbol_information: Vec<&SymbolInformation> = if !params.query.is_empty() {
        let refs = indexer.symbols.iter();

        SymbolsMatcher::new(indexer.root_path.as_path())
            .match_symbols(&params.query, refs)
    } else {
        indexer.symbols.iter().collect()
    };

    let result = serde_json::to_value(symbol_information).unwrap();
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
