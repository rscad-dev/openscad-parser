use crate::prelude::*;
#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub struct Ident {
    name: Box<str>,
    span: Span,
}

impl Ident {
    /// Returns the identifier name (including `$` prefix for special variables).
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Returns `true` if this is a special variable (starts with `$`).
    pub fn is_special(&self) -> bool {
        self.name.starts_with('$')
    }
}

impl Spannable for Ident {
    fn span(&self) -> Span {
        self.span
    }
}

impl<'a> Parseable<'a> for Ident {
    fn parser<I>() -> impl Parser<'a, I, Self, ParserError<'a>> + Clone
    where
        I: Input<'a>,
    {
        // Regular identifier
        let regular = any()
            .filter(|t| matches!(t, Tokens::Identifier(_)))
            .map_with(|token, extra| {
                if let Tokens::Identifier(s) = token {
                    Ident {
                        name: s.into(),
                        span: extra.span(),
                    }
                } else {
                    unreachable!("BUG: filter guarantees Identifier variant")
                }
            });

        // Special variable: $ followed immediately by an identifier (e.g., $fn, $fa, $fs, $t)
        let special = just(Tokens::Dollar)
            .to_span()
            .then(
                any()
                    .filter(|t| matches!(t, Tokens::Identifier(_)))
                    .map_with(|token, extra| {
                        if let Tokens::Identifier(s) = token {
                            (s.to_string(), extra.span())
                        } else {
                            unreachable!("BUG: filter guarantees Identifier variant")
                        }
                    }),
            )
            .map(|(dollar_span, (name, name_span)): (Span, (String, Span))| {
                let span = dollar_span.union(name_span);
                Ident {
                    name: format!("${name}").into(),
                    span,
                }
            });

        choice((special, regular)).labelled("Ident")
    }
}
