//! Extremely fast, lossless, and error tolerant JSON Parser.

use crate::parser::JsonParser;
use crate::syntax::parse_root;
use rome_diagnostics::FileId;
use rome_json_factory::JsonSyntaxFactory;
use rome_json_syntax::{JsonLanguage, JsonRoot, JsonSyntaxNode};
pub use rome_parser::prelude::*;
use rome_parser::tree_sink::LosslessTreeSink;
use rome_parser::AnyParse;
use rome_rowan::AstNode;

mod lexer;
mod parser;
mod prelude;
mod syntax;
mod token_source;

pub(crate) type JsonLosslessTreeSink<'source> =
    LosslessTreeSink<'source, JsonLanguage, JsonSyntaxFactory>;

pub fn parse_json(source: &str, file_id: FileId) -> JsonParse {
    tracing::debug_span!("parse", file_id = ?file_id).in_scope(move || {
        let mut parser = JsonParser::new(source, file_id);

        parse_root(&mut parser);

        let (events, diagnostics, trivia) = parser.finish();

        let mut tree_sink = JsonLosslessTreeSink::new(source, &trivia);
        rome_parser::event::process(&mut tree_sink, events, diagnostics);
        let (green, diagnostics) = tree_sink.finish();

        JsonParse::new(green, diagnostics)
    })
}

/// A utility struct for managing the result of a parser job
#[derive(Debug)]
pub struct JsonParse {
    root: JsonSyntaxNode,
    diagnostics: Vec<ParseDiagnostic>,
}

impl JsonParse {
    pub fn new(root: JsonSyntaxNode, diagnostics: Vec<ParseDiagnostic>) -> JsonParse {
        JsonParse { root, diagnostics }
    }

    /// The syntax node represented by this Parse result
    ///
    /// ```
    /// # use rome_json_parser::parse_json;
    /// # use rome_json_syntax::JsonSyntaxKind;
    /// # use rome_rowan::{AstNode, AstNodeList, SyntaxError};
    /// # use rome_diagnostics::location::FileId;
    ///
    /// # fn main() -> Result<(), SyntaxError> {
    /// use rome_json_syntax::JsonSyntaxKind;
    /// let parse = parse_json(r#"["a", 1]"#, FileId::zero());
    ///
    /// // Get the root value
    /// let root_value = parse.tree().value()?;
    ///
    /// assert_eq!(root_value.syntax().kind(), JsonSyntaxKind::JSON_ARRAY_VALUE);
    ///
    /// # Ok(())
    /// # }
    /// ```
    pub fn syntax(&self) -> JsonSyntaxNode {
        self.root.clone()
    }

    /// Get the diagnostics which occurred when parsing
    pub fn diagnostics(&self) -> &[ParseDiagnostic] {
        &self.diagnostics
    }

    /// Get the diagnostics which occurred when parsing
    pub fn into_diagnostics(self) -> Vec<ParseDiagnostic> {
        self.diagnostics
    }

    /// Returns [true] if the parser encountered some errors during the parsing.
    pub fn has_errors(&self) -> bool {
        self.diagnostics
            .iter()
            .any(|diagnostic| diagnostic.is_error())
    }

    /// Convert this parse result into a typed AST node.
    ///
    /// # Panics
    /// Panics if the node represented by this parse result mismatches.
    pub fn tree(&self) -> JsonRoot {
        JsonRoot::unwrap_cast(self.syntax())
    }
}

impl From<JsonParse> for AnyParse {
    fn from(parse: JsonParse) -> Self {
        let root = parse.syntax();
        let diagnostics = parse.into_diagnostics();

        AnyParse::new(
            // SAFETY: the parser should always return a root node
            root.as_send().unwrap(),
            diagnostics,
        )
    }
}
