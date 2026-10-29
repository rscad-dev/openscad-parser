mod ast;
mod expr;
mod extra;
mod file;
mod ident;
mod item;
mod lexer;
mod literal;
mod parser;
mod span;
// Public: the exported `Token![...]` macro expands to `$crate::token::...`
// paths, so downstream crates need the module reachable.
pub mod token;

/// Re-export of the parser combinator library, so downstream crates can name
/// the `Parser` trait (e.g. `File::parser().parse(...)`) without depending on
/// the exact chumsky git revision themselves.
pub use chumsky;
pub mod prelude {
    pub use super::expr::*;
    pub use super::extra::*;
    pub use super::file::*;
    pub use super::ident::*;
    pub use super::item::*;
    pub use super::literal::*;
    pub use super::parser::*;
    pub use super::token::*;
    pub use crate::Token;
    pub use crate::ast::*;
    pub use crate::lexer::*;
    pub use crate::parser::Input;
    pub use crate::parser::Spannable;
    pub use crate::span::Span;
    pub(crate) use chumsky::prelude::*;
    // pub use crate::ident::Ident;
}
