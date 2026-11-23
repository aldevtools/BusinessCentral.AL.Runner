//! AL syntax: lexer, preprocessor and lossless parser.
//!
//! Crate root of the AL syntax: lexer, parser, tree, AST views.

pub mod ast;
mod builder;
mod grammar;
pub mod lexer;
mod parser;
pub mod preprocessor;
pub mod scan;
mod splice;
pub mod syntax_kind;

pub use builder::TreeCache;
pub use lexer::{Token, lex};
pub use parser::{Parse, ParseError, debug_tree, parse, parse_type, parse_with_cache};
pub use splice::splice_routines;
pub use syntax_kind::{AlLanguage, SyntaxKind, SyntaxNode, SyntaxToken};
