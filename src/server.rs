use std::{
    path::{Path, PathBuf},
    rc::Rc,
    sync::Arc,
    time::Instant,
};

use anyhow::Result;

use crossbeam_channel::Sender;
use log::{error, info};
use lsp_server::{Connection, Message, RequestId, Response};
use lsp_types::{
    request::{DocumentSymbolRequest, GotoDefinition, WorkspaceSymbolRequest},
    DocumentSymbolParams, GotoDefinitionParams, GotoDefinitionResponse, Location, Position, Range, SymbolInformation,
    SymbolKind, Url, WorkspaceSymbolParams,
};
use serde::de::DeserializeOwned;
use tree_sitter::Point;

use crate::{
    finder::Finder,
    indexer::Indexer,
    progress_reporter::ProgressReporter,
    ruby_env_provider::RubyEnvProvider,
    ruby_filename_converter::RubyFilenameConverter,
    types::{RSymbolKind, RSymbolV2},
};

pub struct Server<'a> {
    root_dir: PathBuf,
    indexer: Indexer<'a>,
    pub finder: Finder,
    symbols: Rc<Vec<Arc<RSymbolV2>>>,
    ruby_env_provider: Rc<RubyEnvProvider>,
    ruby_filename_converter: Rc<RubyFilenameConverter>,
    progress_reporter: Rc<ProgressReporter<'a>>,
}

trait Handler<P: DeserializeOwned> {
    fn handle<R>(&self, sender: &Sender<Message>, request: (RequestId, P)) -> Result<()>;
}

impl<'a> Server<'a> {
    pub fn new(root_dir: &Path, sender: &'a Sender<Message>) -> Result<Server<'a>> {
        let root_dir = root_dir.to_path_buf();

        let progress_reporter = Rc::new(ProgressReporter::new(sender));
        let ruby_env_provider = Rc::new(RubyEnvProvider::new(&root_dir));
        let ruby_filename_converter = Rc::new(RubyFilenameConverter::new(&root_dir, &ruby_env_provider)?);
        let mut indexer = Indexer::new(
            &root_dir,
            progress_reporter.clone(),
            ruby_env_provider.clone(),
            ruby_filename_converter.clone(),
        );

        let symbols = Rc::new(indexer.index()?);
        let finder = Finder::new(&root_dir, symbols.clone(), ruby_filename_converter.clone());

        Ok(Server {
            root_dir,
            indexer,
            finder,
            symbols,
            ruby_filename_converter,
            ruby_env_provider,
            progress_reporter,
        })
    }

    pub fn handle_request(&self, connection: &Connection, request: lsp_server::Request) -> Result<()> {
        use lsp_types::request::Request;

        let sender = &connection.sender;
        match request.method.as_str() {
            WorkspaceSymbolRequest::METHOD => self.handle::<WorkspaceSymbolParams>(
                sender,
                request.extract::<WorkspaceSymbolParams>(WorkspaceSymbolRequest::METHOD)?,
            ),

            DocumentSymbolRequest::METHOD => self.handle::<DocumentSymbolRequest>(
                sender,
                request.extract::<DocumentSymbolParams>(DocumentSymbolRequest::METHOD)?,
            ),

            GotoDefinition::METHOD => {
                self.handle::<GotoDefinition>(sender, request.extract::<GotoDefinitionParams>(GotoDefinition::METHOD)?)
            }

            _ => Err(anyhow!("Method {} is not supported", request.method)),
        }
    }

    fn send_response<T: serde::Serialize>(sender: &Sender<Message>, id: RequestId, response: T) -> Result<()> {
        let result = serde_json::to_value(response).unwrap();
        let resp = Response {
            id,
            result: Some(result),
            error: None,
        };
        sender.send(Message::Response(resp))?;

        Ok(())
    }

    fn convert_to_lsp_sym_info(rsymbol: impl AsRef<RSymbolV2>) -> SymbolInformation {
        let rsymbol = rsymbol.as_ref();
        let path = &rsymbol.file;
        let file_path_str = path.to_str().unwrap();
        let url = Url::parse(&format!("file:///{}", file_path_str)).unwrap();

        let location = rsymbol.start;
        let line: u32 = location.row.try_into().unwrap();
        let character: u32 = location.column.try_into().unwrap();

        let name = &rsymbol.name;
        let name_len: u32 = name.len().try_into().unwrap();

        let range = Range {
            start: Position::new(line, character),
            end: Position::new(line, character + name_len),
        };

        let kind = match rsymbol.kind {
            RSymbolKind::Class { .. } => SymbolKind::CLASS,
            RSymbolKind::Module { .. } => SymbolKind::MODULE,
            RSymbolKind::InstanceMethod { .. } => SymbolKind::METHOD,
            RSymbolKind::SingletonMethod { .. } => SymbolKind::METHOD,
            RSymbolKind::Constant => SymbolKind::CONSTANT,
            RSymbolKind::GlobalVariable => SymbolKind::CONSTANT,
            RSymbolKind::InstanceVariable => SymbolKind::FIELD,
            RSymbolKind::ClassVariable => SymbolKind::FIELD,
            _ => SymbolKind::NULL,
        };

        #[allow(deprecated)]
        SymbolInformation {
            name: name.to_string(),
            kind,
            tags: None,
            deprecated: None,
            location: Location {
                uri: url,
                range,
            },
            container_name: None,
        }
    }
}

impl<'a> Handler<WorkspaceSymbolParams> for Server<'a> {
    fn handle<R>(&self, sender: &Sender<Message>, request: (RequestId, WorkspaceSymbolParams)) -> Result<()> {
        let (id, params) = request;

        info!("got workspace/symbol request #{id}: {params:?}");

        let start = Instant::now();

        let symbols: Vec<SymbolInformation> =
            self.finder.fuzzy_find_symbol(&params.query).iter().map(Self::convert_to_lsp_sym_info).collect();

        Self::send_response(sender, id, symbols)?;

        let duration = start.elapsed();

        info!("workspace/symbool took {:?}", duration);

        Ok(())
    }
}

impl<'a> Handler<DocumentSymbolParams> for Server<'a> {
    fn handle<R>(&self, sender: &Sender<Message>, request: (RequestId, DocumentSymbolParams)) -> Result<()> {
        let start = Instant::now();

        let (id, params) = request;

        info!("[#{id}] Got document/symbol request, params = {params:?}");

        let path = params.text_document.uri.to_file_path().unwrap();
        let symbols: Vec<SymbolInformation> =
            self.finder.find_by_path(&path).iter().map(Self::convert_to_lsp_sym_info).collect();

        let result = serde_json::to_value(symbols).unwrap();

        info!("[#{id}] document/symbol took {:?}", start.elapsed());

        let resp = Response {
            id,
            result: Some(result),
            error: None,
        };
        sender.send(Message::Response(resp))?;

        Ok(())
    }
}

impl<'a> Handler<GotoDefinitionParams> for Server<'a> {
    fn handle<R>(&self, sender: &Sender<Message>, request: (RequestId, GotoDefinitionParams)) -> Result<()> {
        let (id, params) = request;

        info!("got textDocument/definition request #{id}: {params:?}");

        let start = Instant::now();

        let file = params.text_document_position_params.text_document.uri.to_file_path().unwrap();
        let position = params.text_document_position_params.position;
        let position = Point {
            row: position.line.try_into()?,
            column: position.character.try_into()?,
        };

        let definitions = match self.finder.find_definition(file.as_path(), position) {
            Ok(defs) => defs,
            Err(e) => {
                error!("Failed to find definitions: {e:?}");
                vec![]
            }
        };

        let symbols: Vec<Location> =
            definitions.iter().map(Self::convert_to_lsp_sym_info).map(|s| s.location).collect();

        info!("textDocument/definition found {} symbols", symbols.len());

        let result = GotoDefinitionResponse::Array(symbols);
        let result = serde_json::to_value(result).unwrap();
        let resp = Response {
            id,
            result: Some(result),
            error: None,
        };
        sender.send(Message::Response(resp))?;

        let duration = start.elapsed();

        info!("textDocument/definition took {:?}", duration);

        Ok(())
    }
}
