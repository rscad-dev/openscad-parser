use crate::prelude::*;

#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub struct File {
    pub stmts: Vec<Stmt>,
}

impl Spannable for File {
    fn span(&self) -> Span {
        if let Some(first) = self.stmts.first()
            && let Some(last) = self.stmts.last()
        {
            return first.span().union(last.span());
        }
        Span::default()
    }
}

impl<'a> Parseable<'a> for File {
    fn parser<I>() -> impl Parser<'a, I, Self, ParserError<'a>> + Clone
    where
        I: Input<'a>,
    {
        Stmt::parser::<I>()
            .repeated()
            .collect()
            .then_ignore(end())
            .map(|stmts| File { stmts })
            .labelled("File")
    }
}
