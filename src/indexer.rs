use std::ffi::OsStr;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::Instant;

use anyhow::Result;

use itertools::Itertools;

use walkdir::WalkDir;

use rayon::prelude::*;

use tree_sitter::*;

use lsp_types::{Location, Position, Range, SymbolInformation, SymbolKind, Url};

use crate::progress_reporter::ProgressReporter;

pub struct Method {
    name: String,
    params_count: u8,
    location: Point,
}

pub struct Class {
    name: String,
    scopes: Vec<String>,
    superclass: Option<Box<Class>>,
    path: Option<PathBuf>,
    location: Option<Point>,
    methods: Vec<Method>,
    singleton_methods: Vec<Method>,
}

pub struct Indexer<'a> {
    pub root_path: PathBuf,
    pub classes: Vec<Class>,

    pub external_classes: Vec<Class>,

    pub symbols: Vec<SymbolInformation>,
    pub external_symbols: Vec<SymbolInformation>,

    progress_reporter: ProgressReporter<'a>,

    language: Language,
    class_query: Query,
    method_query: Query,
    singleton_method_query: Query,
}

impl<'a> Indexer<'a> {
    pub fn index_folder(folder: &Path, progress_reporter: ProgressReporter<'a>) -> Result<Indexer<'a>> {
        let start = Instant::now();
        eprintln!("Started indexing");

        let ruby_version_file = folder.join(".ruby-version");
        let ruby_version = if ruby_version_file.exists() {
            Some(fs::read_to_string(ruby_version_file)?.trim().to_owned())
        } else {
            None
        };

        let gemset_file = folder.join(".ruby-gemset");
        let gemset = if gemset_file.exists() {
            Some(fs::read_to_string(gemset_file)?.trim().to_owned())
        } else {
            None
        };

        let mut indexer = Indexer::new(folder, progress_reporter)?;

        if let Some(dir) = Self::choose_stubs_dir(&ruby_version) {
            eprintln!("Stubs dir: {:?}", dir);
            let token = indexer.progress_reporter.send_progress_begin(format!("Starting indexing {}", dir.to_str().unwrap()).as_str(), "", 0)?;
            let mut stub_classes = indexer.recursively_index_folder(&dir)?;
            indexer.external_classes.append(&mut stub_classes);
            indexer.progress_reporter.send_progress_end(token, format!("Indexing {} done", dir.to_str().unwrap()).as_str())?;
        }
        if let Some(dir) = Self::choose_gems_dir(&ruby_version, &gemset) {
            eprintln!("Gems dir: {:?}", dir);
            let token = indexer.progress_reporter.send_progress_begin(format!("Starting indexing {}", dir.to_str().unwrap()).as_str(), "", 0)?;
            let mut gem_classes = indexer.recursively_index_folder(&dir)?;
            indexer.external_classes.append(&mut gem_classes);
            indexer.progress_reporter.send_progress_end(token, format!("Indexing {} done", dir.to_str().unwrap()).as_str())?;
        }

        let token = indexer.progress_reporter.send_progress_begin(format!("Starting indexing {}", folder.to_str().unwrap()).as_str(), "", 0)?;
        let mut classes = indexer.recursively_index_folder(folder)?;
        indexer.external_classes.append(&mut classes);
        indexer.progress_reporter.send_progress_end(token, format!("Indexing {} done", folder.to_str().unwrap()).as_str())?;

        let token = indexer.progress_reporter.send_progress_begin("Converting to symbols", "", 0)?;
        indexer.convert_to_symbol_info()?;
        indexer.progress_reporter.send_progress_end(token, "Done")?;

        eprintln!("Indexing done in {:?}", start.elapsed());

        Ok(indexer)
    }

    pub fn new(root_path: &Path, progress_reporter: ProgressReporter<'a>) -> Result<Indexer<'a>> {
        let language = tree_sitter_ruby::language();
        let class_query = Self::create_query(
            r#"(class
                name: [(constant) (scope_resolution)] @class_name
                superclass: [(constant) (scope_resolution)]? @superclass_name)"#,
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
            root_path: root_path.to_path_buf(),
            classes: Vec::new(),
            external_classes: Vec::new(),
            symbols: Vec::new(),
            external_symbols: Vec::new(),
            progress_reporter,
            class_query,
            method_query,
            singleton_method_query,
            language,
        })
    }

    fn choose_stubs_dir(ruby_version: &Option<String>) -> Option<PathBuf> {
        let ruby_version = match ruby_version {
            None => return None,
            Some(version) => version,
        };

        let segments = ruby_version.split('.').collect_vec();
        let major = segments[0];
        let minor = segments[1];

        let path = "/Users/oleksandr.oksenenko/code/rust-ruby-ls/stubs/rubystubs".to_owned()
            + major
            + minor;

        Some(PathBuf::from(path))
    }

    fn choose_gems_dir(ruby_version: &Option<String>, gemset: &Option<String>) -> Option<PathBuf> {
        let ruby_version = match ruby_version {
            None => return None,
            Some(version) => version,
        };

        let path = "/Users/oleksandr.oksenenko/.rvm/gems/ruby-".to_owned() + ruby_version;
        match gemset {
            None => Some(PathBuf::from(path)),
            Some(gemset) => Some(PathBuf::from(path + "@" + gemset)),
        }
    }

    fn create_query(q: &str) -> Result<Query> {
        let language = tree_sitter_ruby::language();
        Ok(Query::new(language, q)?)
    }

    pub fn recursively_index_folder(&mut self, folder: &Path) -> Result<Vec<Class>> {
        let class_query = &self.class_query;
        let method_query = &self.method_query;
        let singleton_method_query = &self.singleton_method_query;

        let classes: Vec<Class> = WalkDir::new(folder)
            .into_iter()
            .par_bridge()
            .filter_map(Result::ok)
            .filter(|e| !e.file_type().is_dir())
            .filter(|e| "rb" == e.path().extension().and_then(OsStr::to_str).unwrap_or(""))
            .flat_map(|entry| {
                Self::index_file(
                    self.language,
                    class_query,
                    method_query,
                    singleton_method_query,
                    entry.path().to_path_buf(),
                )
                .unwrap()
            })
            .collect();

        eprintln!("Found {} classes", classes.len());

        eprintln!("Total {} classes", self.classes.len());

        Ok(classes)
    }

    fn convert_to_symbol_info(&mut self) -> Result<()> {
        self.symbols = self.classes
            .par_iter()
            .map(Self::convert_class_to_symbol)
            .collect::<Vec<SymbolInformation>>();

        self.external_symbols = self.external_classes
            .par_iter()
            .map(Self::convert_class_to_symbol)
            .collect();

        Ok(())
    }

    fn convert_class_to_symbol(class: &Class) -> SymbolInformation {
        let path = class.path.as_ref().unwrap();
        let file_path_str = path.to_str().unwrap();
        let url = Url::parse(&format!("file:///{}", file_path_str)).unwrap();

        let location = class.location.unwrap();
        let line: u32 = location.row.try_into().unwrap();
        let character: u32 = location.column.try_into().unwrap();

        let class_name_len: u32 = class.name.len().try_into().unwrap();

        let range = Range {
            start: Position::new(line, character),
            end: Position::new(line, character + class_name_len),
        };

        let container_name = if class.scopes.is_empty() {
            None
        } else {
            class.scopes.last().cloned()
        };
        let all_scopes = class.scopes.iter().join("::");
        let name = if all_scopes.is_empty() {
            class.name.clone()
        } else {
            class.scopes.iter().join("::") + "::" + &class.name
        };

        SymbolInformation {
            name,
            kind: SymbolKind::CLASS,
            tags: None,
            deprecated: None,
            location: Location { uri: url, range },
            container_name
        }
    }

    pub fn index_file(
        language: Language,
        class_query: &Query,
        method_query: &Query,
        singleton_method_query: &Query,
        file: PathBuf,
    ) -> Result<Vec<Class>> {
        let mut parser = Parser::new();
        parser.set_language(language).unwrap();

        let source = &fs::read(&file)?[..];

        let parsed = parser.parse(source, None).unwrap();

        let mut query_cursor = QueryCursor::new();

        let mut classes = Vec::new();
        for query_match in query_cursor.matches(class_query, parsed.root_node(), source) {
            if query_match.captures.is_empty() {
                return Err(anyhow!("No matches found in {file:?}"));
            }

            let class_name_node = query_match.captures[0].node;
            let class_scopes = Self::get_scopes(&class_name_node, source)?;

            let class_location = class_name_node.start_position();

            let methods =
                Self::get_methods(class_name_node.parent().unwrap(), method_query, source)?;

            let superclass = if query_match.captures.len() > 1 {
                let superclass_name_node = query_match.captures[1].node;
                let superclass_scopes = Self::get_scopes(&superclass_name_node, source)?;

                let mut iter = superclass_scopes.into_iter().rev();
                let name = iter.next().unwrap();
                let scopes = iter.rev().collect_vec();
                Some(Box::new(Class {
                    name,
                    scopes,
                    superclass: None,
                    path: None,
                    location: None,
                    methods: Vec::new(),
                    singleton_methods: Vec::new(),
                }))
            } else {
                None
            };

            let mut iter = class_scopes.into_iter().rev();
            let name = iter.next().unwrap();
            let scopes = iter.rev().collect_vec();
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

        Ok(classes)
    }

    fn get_scopes(main_node: &Node, source: &[u8]) -> Result<Vec<String>> {
        let mut scopes = Vec::new();

        if main_node.kind() == "scope_resolution" {
            let mut node = *main_node;
            while node.kind() == "scope_resolution" {
                let name_node = node.child_by_field_name("name").unwrap();
                let name = name_node.utf8_text(source)?.to_owned();
                scopes.push(name);

                let child = node.child_by_field_name("scope");
                match child {
                    None => break,
                    Some(n) => node = n,
                }
            }
            if node.kind() == "constant" {
                let name = node.utf8_text(source)?.to_owned();
                scopes.push(name);
            }
        }
        if main_node.kind() == "constant" {
            let name = main_node.utf8_text(source)?.to_owned();
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
