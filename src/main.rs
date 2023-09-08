#[cfg(not(target_env = "msvc"))]
use jemallocator::Jemalloc;
use log::info;

#[cfg(not(target_env = "msvc"))]
#[global_allocator]
static GLOBAL: Jemalloc = Jemalloc;

#[macro_use]
extern crate anyhow;

use anyhow::Result;

use lsp_server::{Connection, Message};
use lsp_types::{InitializeParams, OneOf, ServerCapabilities};

mod finder;
mod indexer;
mod parsers;
mod parsers_v2;
mod progress_reporter;
mod ruby_env_provider;
mod ruby_filename_converter;
mod server;
mod symbols_matcher;
mod types;

use crate::server::Server;

fn main() -> Result<()> {
    let file = log4rs::append::file::FileAppender::builder()
        .encoder(Box::new(log4rs::encode::pattern::PatternEncoder::new("{d} - {m}{n}")))
        .build("/Users/oleksandr.oksenenko/code/rust-ruby-ls/lsp.log")
        .unwrap();
    let config = log4rs::Config::builder()
        .appender(log4rs::config::Appender::builder().build("file", Box::new(file)))
        .build(log4rs::config::Root::builder().appender("file").build(log::LevelFilter::Info))
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

    let server = Server::new(&path, &connection.sender)?;

    for msg in &connection.receiver {
        match msg {
            Message::Request(req) => {
                if connection.handle_shutdown(&req)? {
                    return Ok(());
                }

                server.handle_request(&connection, req)?;
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
