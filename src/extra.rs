use crate::prelude::*;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Delimited<T, D> {
    pub delimiter: D,
    pub inner: T,
}

impl<T> Spannable for Delimited<T, Paren> {
    fn span(&self) -> Span {
        self.delimiter.span.join()
    }
}

impl<T> Spannable for Delimited<T, Brace> {
    fn span(&self) -> Span {
        self.delimiter.span.join()
    }
}

impl<T> Spannable for Delimited<T, Bracket> {
    fn span(&self) -> Span {
        self.delimiter.span.join()
    }
}

impl<'a, T> Parseable<'a> for Delimited<T, Paren>
where
    T: Parseable<'a>,
{
    fn parser<I>() -> impl Parser<'a, I, Self, ParserError<'a>> + Clone
    where
        I: Input<'a>,
    {
        let open = just(Tokens::LParen).to_span();
        let close = just(Tokens::RParen).to_span();
        let inner = T::parser::<I>();
        group((open, inner, close))
            .map(|(open, inner, close)| Delimited {
                delimiter: Paren {
                    span: DelimSpan { open, close },
                },
                inner,
            })
            .labelled("Delimited<Paren>")
    }
}

impl<'a, T> Parseable<'a> for Delimited<T, Brace>
where
    T: Parseable<'a>,
{
    fn parser<I>() -> impl Parser<'a, I, Self, ParserError<'a>> + Clone
    where
        I: Input<'a>,
    {
        let open = just(Tokens::LBrace).to_span();
        let close = just(Tokens::RBrace).to_span();
        let inner = T::parser::<I>();
        group((open, inner, close))
            .map(|(open, inner, close)| Delimited {
                delimiter: Brace {
                    span: DelimSpan { open, close },
                },
                inner,
            })
            .labelled("Delimited<Brace>")
    }
}

impl<'a, T> Parseable<'a> for Delimited<T, Bracket>
where
    T: Parseable<'a>,
{
    fn parser<I>() -> impl Parser<'a, I, Self, ParserError<'a>> + Clone
    where
        I: Input<'a>,
    {
        let open = just(Tokens::LBracket).to_span();
        let close = just(Tokens::RBracket).to_span();
        let inner = T::parser::<I>();
        group((open, inner, close))
            .map(|(open, inner, close)| Delimited {
                delimiter: Bracket {
                    span: DelimSpan { open, close },
                },
                inner,
            })
            .labelled("Delimited<Bracket>")
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
/// A pair of parentheses.
/// ( and )
pub struct Paren {
    pub(crate) span: DelimSpan,
}

impl Spannable for Paren {
    fn span(&self) -> Span {
        self.span.join()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
/// A pair of square brackets.
/// [ and ]
pub struct Brace {
    pub(crate) span: DelimSpan,
}

impl Spannable for Brace {
    fn span(&self) -> Span {
        self.span.join()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
/// A pair of square brackets.
/// [ and ]
pub struct Bracket {
    pub(crate) span: DelimSpan,
}

impl Spannable for Bracket {
    fn span(&self) -> Span {
        self.span.join()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct DelimSpan {
    pub(crate) open: Span,
    pub(crate) close: Span,
}

impl DelimSpan {
    pub fn join(&self) -> Span {
        use chumsky::span::Span as _;
        self.open.union(self.close)
    }
}

#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub struct Punctuated<T, P> {
    pub(crate) inner: Vec<(T, P)>,
    pub(crate) last: Option<Box<T>>,
}

impl<T, P> Default for Punctuated<T, P> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T, P> Punctuated<T, P> {
    pub const fn new() -> Self {
        Punctuated {
            inner: Vec::new(),
            last: None,
        }
    }
    pub fn is_empty(&self) -> bool {
        self.inner.len() == 0 && self.last.is_none()
    }

    pub fn len(&self) -> usize {
        self.inner.len() + if self.last.is_some() { 1 } else { 0 }
    }

    /// Iterate over the values, skipping punctuation.
    pub fn iter(&self) -> impl Iterator<Item = &T> {
        self.inner
            .iter()
            .map(|(t, _)| t)
            .chain(self.last.as_deref())
    }

    pub fn opt_span(&self) -> Option<Span>
    where
        T: Spannable,
        P: Spannable,
    {
        let first = self.inner.first().map(|(t, _)| t.span());
        let last = self
            .last
            .as_ref()
            .map(|t| t.span())
            .or_else(|| self.inner.last().map(|(_, p)| p.span()));
        match (first, last) {
            (Some(f), Some(l)) => Some(f.union(l)),
            (Some(f), None) => Some(f),
            (None, Some(l)) => Some(l),
            (None, None) => None,
        }
    }
}

impl<T: Spannable, P: Spannable> Spannable for Punctuated<T, P> {
    fn span(&self) -> Span {
        self.opt_span().unwrap_or_default()
    }
}

impl<'a, T, P> Parseable<'a> for Punctuated<T, P>
where
    T: Parseable<'a>,
    P: Parseable<'a>,
{
    fn parser<I>() -> impl Parser<'a, I, Self, ParserError<'a>> + Clone
    where
        I: Input<'a>,
    {
        T::parser::<I>()
            .then(P::parser::<I>())
            .repeated()
            .collect::<Vec<_>>()
            .then(T::parser::<I>().or_not())
            .map(|(parsed, last)| Punctuated {
                inner: parsed,
                last: last.map(Box::new),
            })
            .labelled("Punctuated")
    }
}

/// Build a `Punctuated<T, P>` parser from custom item and punctuation parsers.
///
/// This is useful when you can't call `Punctuated::<T, P>::parser()` because
/// doing so would re-enter a recursive parser boundary (e.g., `Expr::parser()`
/// from inside `recursive(|expr| ...)`).
pub(crate) fn punctuated_with<'a, T, P, I>(
    item: impl Parser<'a, I, T, ParserError<'a>> + Clone + 'a,
    punct: impl Parser<'a, I, P, ParserError<'a>> + Clone + 'a,
) -> impl Parser<'a, I, Punctuated<T, P>, ParserError<'a>> + Clone
where
    I: Input<'a>,
{
    item.clone()
        .then(punct)
        .repeated()
        .collect::<Vec<_>>()
        .then(item.or_not())
        .map(|(parsed, last)| Punctuated {
            inner: parsed,
            last: last.map(Box::new),
        })
}

/// Build a `Delimited<T, Paren>` parser from a custom inner parser.
///
/// Wraps the inner parser with `(` and `)` tokens.
pub(crate) fn delimited_paren_with<'a, T, I>(
    inner: impl Parser<'a, I, T, ParserError<'a>> + Clone + 'a,
) -> impl Parser<'a, I, Delimited<T, Paren>, ParserError<'a>> + Clone
where
    I: Input<'a>,
{
    let open = just(Tokens::LParen).to_span();
    let close = just(Tokens::RParen).to_span();
    group((open, inner, close)).map(|(open, inner, close)| Delimited {
        delimiter: Paren {
            span: DelimSpan { open, close },
        },
        inner,
    })
}

/// Build a `Delimited<T, Bracket>` parser from a custom inner parser.
///
/// Wraps the inner parser with `[` and `]` tokens.
pub(crate) fn delimited_bracket_with<'a, T, I>(
    inner: impl Parser<'a, I, T, ParserError<'a>> + Clone + 'a,
) -> impl Parser<'a, I, Delimited<T, Bracket>, ParserError<'a>> + Clone
where
    I: Input<'a>,
{
    let open = just(Tokens::LBracket).to_span();
    let close = just(Tokens::RBracket).to_span();
    group((open, inner, close)).map(|(open, inner, close)| Delimited {
        delimiter: Bracket {
            span: DelimSpan { open, close },
        },
        inner,
    })
}

impl<T: Spannable> Spannable for Vec<T> {
    fn span(&self) -> Span {
        if let Some(first) = self.first()
            && let Some(last) = self.last()
        {
            return first.span().union(last.span());
        }
        Span::default()
    }
}

impl<'a, T> Parseable<'a> for Vec<T>
where
    T: Parseable<'a>,
{
    fn parser<I>() -> impl Parser<'a, I, Self, ParserError<'a>> + Clone
    where
        I: Input<'a>,
    {
        T::parser::<I>().repeated().collect().labelled("Vec")
    }
}
