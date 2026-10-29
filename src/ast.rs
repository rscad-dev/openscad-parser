use ariadne::Report;
use ariadne::{Cache, Source};
use std::{
    collections::BTreeMap,
    fmt,
    path::{Path, PathBuf},
};

use crate::prelude::*;

/// Errors that can occur during parsing.
pub enum ParseError {
    /// An I/O error occurred while reading a source file.
    Io {
        path: PathBuf,
        source: std::io::Error,
    },
    /// The file was not found in the AST cache after being added.
    FileNotFound { file_id: usize },
    /// One or more lex/parse errors occurred.
    Parse(Ast, Vec<ariadne::Report<Span>>),
}

impl ParseError {
    /// Render the error (including all ariadne reports) to a plain string.
    ///
    /// No side effects — safe to call from GUI/log contexts.
    pub fn render(&self) -> String {
        match self {
            ParseError::Io { path, source } => {
                format!("failed to read '{}': {}", path.display(), source)
            }
            ParseError::FileNotFound { file_id } => {
                format!("file with id {file_id} not found in cache")
            }
            ParseError::Parse(ast, reports) => ast.render_reports(reports),
        }
    }
}

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.render())
    }
}

impl core::fmt::Debug for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        <ParseError as fmt::Display>::fmt(self, f)
    }
}

impl std::error::Error for ParseError {}

/// A multi-file AST cache.
///
/// Source files are registered (from disk or memory) and parsed on demand;
/// parsed [`File`]s are cached by file id. `Ast` also implements
/// [`ariadne::Cache`] so diagnostics can render source snippets.
#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub struct Ast {
    pub file: BTreeMap<PathBuf, usize>,
    pub unparsed: BTreeMap<PathBuf, Source>,
    pub files: BTreeMap<usize, File>,
}

impl Default for Ast {
    fn default() -> Self {
        Self::new()
    }
}

impl Ast {
    pub fn new() -> Self {
        Ast {
            file: BTreeMap::new(),
            unparsed: BTreeMap::new(),
            files: BTreeMap::new(),
        }
    }

    /// Register a file's content from disk without parsing it.
    ///
    /// Returns the file id (existing id if already registered).
    pub fn add_file(&mut self, path: impl AsRef<Path>) -> Result<usize, ParseError> {
        let path = path.as_ref().to_path_buf();
        if let Some(file_id) = self.file.get(&path) {
            return Ok(*file_id);
        }
        let content = std::fs::read_to_string(&path).map_err(|source| ParseError::Io {
            path: path.clone(),
            source,
        })?;
        Ok(self.insert_source(path, content))
    }

    /// Register in-memory source content without parsing it.
    ///
    /// Returns the file id (existing id if the path is already registered).
    pub fn insert_source(&mut self, path: PathBuf, content: String) -> usize {
        if let Some(file_id) = self.file.get(&path) {
            return *file_id;
        }
        self.unparsed.insert(path.clone(), Source::from(content));
        let file_id = self.file.len();
        self.file.insert(path, file_id);
        file_id
    }

    /// Parse a single file into a fresh AST (convenience wrapper).
    pub fn parse_ast(path: impl AsRef<Path>) -> Result<Self, ParseError> {
        let mut ast = Ast::new();
        ast.parse_file(path)?;
        Ok(ast)
    }

    /// Parse in-memory source into a fresh AST (convenience wrapper, useful
    /// for tests and virtual files).
    pub fn parse_source(path: impl Into<PathBuf>, content: String) -> Result<Self, ParseError> {
        let mut ast = Ast::new();
        let id = ast.insert_source(path.into(), content);
        ast.parse_id(id)?;
        Ok(ast)
    }

    /// Register (if needed) and parse a file, returning its file id.
    ///
    /// Already-parsed files are returned from cache without re-parsing.
    pub fn parse_file(&mut self, path: impl AsRef<Path>) -> Result<usize, ParseError> {
        let id = self.add_file(&path)?;
        if self.files.contains_key(&id) {
            return Ok(id);
        }
        self.parse_id(id)?;
        Ok(id)
    }

    /// The parsed [`File`] for a file id, if it has been parsed.
    pub fn get_file(&self, id: usize) -> Option<&File> {
        self.files.get(&id)
    }

    /// The registered path for a file id.
    pub fn path_of(&self, id: usize) -> Option<&Path> {
        self.file
            .iter()
            .find_map(|(path, file_id)| (*file_id == id).then_some(path.as_path()))
    }

    /// Render a set of reports to a plain string using this AST as the
    /// source cache.
    pub fn render_reports(&self, reports: &[Report<Span>]) -> String {
        let mut buf = Vec::new();
        for report in reports {
            let _ = report.write(self.clone(), &mut buf);
            buf.push(b'\n');
        }
        String::from_utf8_lossy(&buf).into_owned()
    }

    /// Lex + parse an already-registered file id, storing the parsed
    /// [`File`] in the cache.
    fn parse_id(&mut self, id: usize) -> Result<(), ParseError> {
        let content = self
            .fetch(&id)
            .map_err(|_| ParseError::FileNotFound { file_id: id })?
            .clone();
        let content = content.text();

        // Reports render without ANSI color: the output is consumed by GUI
        // logs and error types, not only terminals.
        let config = ariadne::Config::default().with_color(false);

        let (input, lex_errors) = Tokens::to_chumsky_input_with_errors(content, id);
        let mut reports: Vec<Report<Span>> = lex_errors
            .into_iter()
            .map(|(err, span)| {
                Report::build(ariadne::ReportKind::Error, span)
                    .with_config(config)
                    .with_message("Failed to lex input")
                    .with_label(ariadne::Label::new(span).with_message(err.to_string()))
                    .finish()
            })
            .collect();

        let parser = File::parser();
        let parsed = parser.parse(input).into_result();
        match parsed {
            Ok(file) if reports.is_empty() => {
                self.files.insert(id, file);
                Ok(())
            }
            result => {
                let mut colors = ariadne::ColorGenerator::new();
                reports.extend(result.err().into_iter().flatten().map(|err| {
                    let color_next = colors.next();
                    let span = *err.span();
                    let span = Span {
                        start: span.start,
                        end: span.end,
                        context: id,
                    };
                    let mut report = Report::build(ariadne::ReportKind::Error, span)
                        .with_config(config)
                        .with_message("Failed to parse file");

                    if err.found().is_some() {
                        match err.reason() {
                            chumsky::error::RichReason::ExpectedFound { expected, .. } => {
                                report = report.with_message("Failed to parse input").with_label(
                                    ariadne::Label::new(span)
                                        .with_message(format!(
                                            "Expected one of {}",
                                            expected
                                                .iter()
                                                .rev()
                                                .map(|t| t.to_string())
                                                .collect::<Vec<_>>()
                                                .join(", ")
                                        ))
                                        .with_color(color_next),
                                );
                            }
                            chumsky::error::RichReason::Custom(msg) => {
                                report = report.with_label(
                                    ariadne::Label::new(span)
                                        .with_message(msg)
                                        .with_color(color_next),
                                );
                            }
                        };
                    } else {
                        report = report.with_label(
                            ariadne::Label::new(span)
                                .with_message("Unexpected end of input")
                                .with_color(color_next),
                        );
                    }

                    report.finish()
                }));
                Err(ParseError::Parse(self.clone(), reports))
            }
        }
    }
}

impl Cache<usize> for Ast {
    type Storage = String;
    fn fetch(
        &mut self,
        id: &usize,
    ) -> Result<&ariadne::Source<Self::Storage>, impl core::fmt::Debug> {
        let path = self
            .file
            .iter()
            .find_map(
                |(path, file_id)| {
                    if file_id == id { Some(path) } else { None }
                },
            )
            .ok_or_else(|| format!("File with id {} not found", id))?;
        self.unparsed
            .get(path)
            .ok_or_else(|| format!("File with id {} not found", id))
    }

    fn display<'a>(&self, id: &'a usize) -> Option<impl std::fmt::Display + 'a> {
        let path = self.path_of(*id)?;
        Some(path.display().to_string())
    }
}
