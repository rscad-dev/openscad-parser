use logos::SpannedIter;

use crate::lexer::Tokens;
use crate::prelude::*;

pub trait Input<'a>:
    chumsky::input::Input<'a, Token = Tokens<'a>, Span = Span> + chumsky::input::ValueInput<'a>
{
}
impl<'a, T> Input<'a> for T where
    T: chumsky::input::Input<'a, Token = Tokens<'a>, Span = Span> + chumsky::input::ValueInput<'a>
{
}
pub type ParserError<'a> = chumsky::extra::Err<Rich<'a, Tokens<'a>, Span>>;

pub trait Spannable {
    fn span(&self) -> Span;
}

pub trait Parseable<'a>: Sized + Spannable {
    fn parser<I>() -> impl Parser<'a, I, Self, ParserError<'a>> + Clone
    where
        I: Input<'a>;
}

pub type FakeIterInput = SpannedIter<'static, Tokens<'static>>;
