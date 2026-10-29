use crate::prelude::*;

#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub enum Lit {
    Num(LitNum),
    Str(LitStr),
    Bool(LitBool),
    Undef(LitUndef),
}

impl Spannable for Lit {
    fn span(&self) -> Span {
        match self {
            Lit::Num(n) => n.span(),
            Lit::Str(l) => l.span(),
            Lit::Bool(b) => b.span(),
            Lit::Undef(u) => u.span(),
        }
    }
}

impl<'a> Parseable<'a> for Lit {
    fn parser<I>() -> impl Parser<'a, I, Self, ParserError<'a>> + Clone
    where
        I: Input<'a>,
    {
        choice((
            LitNum::parser::<I>().map(Lit::Num),
            LitStr::parser::<I>().map(Lit::Str),
            LitBool::parser::<I>().map(Lit::Bool),
            LitUndef::parser::<I>().map(Lit::Undef),
        ))
        .labelled("Lit")
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct LitNum {
    literal: Literal,
    value: f64,
}

impl LitNum {
    /// Returns the numeric value.
    pub fn value(&self) -> f64 {
        self.value
    }

    /// Returns the number exactly as written in the source (e.g. `1.50`,
    /// `1e3`).
    pub fn raw(&self) -> &str {
        &self.literal.literal
    }
}

impl core::cmp::Eq for LitNum {}
impl std::hash::Hash for LitNum {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.literal.hash(state);
        self.value.to_bits().hash(state);
    }
}

#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub struct LitStr {
    literal: Literal,
}

impl LitStr {
    /// Returns the string value with escape sequences decoded.
    ///
    /// Supported escapes: `\"`, `\\`, `\n`, `\t`, `\r`. Unknown escapes are
    /// kept verbatim (backslash included), matching OpenSCAD's lenient
    /// behavior.
    pub fn value(&self) -> String {
        let raw: &str = &self.literal.literal;
        let mut out = String::with_capacity(raw.len());
        let mut chars = raw.chars();
        while let Some(c) = chars.next() {
            if c != '\\' {
                out.push(c);
                continue;
            }
            match chars.next() {
                Some('n') => out.push('\n'),
                Some('t') => out.push('\t'),
                Some('r') => out.push('\r'),
                Some('"') => out.push('"'),
                Some('\\') => out.push('\\'),
                Some(other) => {
                    out.push('\\');
                    out.push(other);
                }
                None => out.push('\\'),
            }
        }
        out
    }

    /// Returns the raw string slice as written in the source (quotes stripped,
    /// escapes not decoded).
    pub fn raw(&self) -> &str {
        &self.literal.literal
    }
}

impl Spannable for LitNum {
    fn span(&self) -> Span {
        self.literal.span
    }
}

impl<'a> Parseable<'a> for LitNum {
    fn parser<I>() -> impl Parser<'a, I, Self, ParserError<'a>> + Clone
    where
        I: Input<'a>,
    {
        select! { Tokens::Number(raw) => raw }
            .try_map(|raw: &str, span: Span| {
                raw.parse::<f64>()
                    .map(|value| LitNum {
                        literal: Literal::new(raw, span),
                        value,
                    })
                    .map_err(|err| Rich::custom(span, format!("invalid number `{raw}`: {err}")))
            })
            .labelled("LitNum")
    }
}

impl Spannable for LitStr {
    fn span(&self) -> Span {
        self.literal.span
    }
}

impl<'a> Parseable<'a> for LitStr {
    fn parser<I>() -> impl Parser<'a, I, Self, ParserError<'a>> + Clone
    where
        I: Input<'a>,
    {
        any()
            .filter(|t| matches!(t, Tokens::String(_)))
            .map_with(|token, extra| {
                if let Tokens::String(s) = token {
                    LitStr {
                        literal: Literal::new(s, extra.span()),
                    }
                } else {
                    unreachable!("This should never happen because of the filter")
                }
            })
            .labelled("LitStr")
    }
}

#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub struct Literal {
    literal: Box<str>,
    span: Span,
}

impl Literal {
    pub fn new(literal: &str, span: Span) -> Self {
        Self {
            literal: literal.into(),
            span,
        }
    }
    pub fn from_string(literal: String, span: Span) -> Self {
        Self {
            literal: literal.into(),
            span,
        }
    }
}

/// A boolean literal: `true` or `false`.
#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub struct LitBool {
    value: bool,
    span: Span,
}

impl LitBool {
    /// Returns the boolean value.
    pub fn value(&self) -> bool {
        self.value
    }
}

impl Spannable for LitBool {
    fn span(&self) -> Span {
        self.span
    }
}

impl<'a> Parseable<'a> for LitBool {
    fn parser<I>() -> impl Parser<'a, I, Self, ParserError<'a>> + Clone
    where
        I: Input<'a>,
    {
        any()
            .filter(|t| matches!(t, Tokens::True | Tokens::False))
            .map_with(|token, extra| LitBool {
                value: matches!(token, Tokens::True),
                span: extra.span(),
            })
            .labelled("LitBool")
    }
}

/// The `undef` literal.
#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub struct LitUndef {
    span: Span,
}

impl Spannable for LitUndef {
    fn span(&self) -> Span {
        self.span
    }
}

impl<'a> Parseable<'a> for LitUndef {
    fn parser<I>() -> impl Parser<'a, I, Self, ParserError<'a>> + Clone
    where
        I: Input<'a>,
    {
        just(Tokens::Undef)
            .to_span()
            .map(|span| LitUndef { span })
            .labelled("LitUndef")
    }
}

/// A path literal, used in `include <path>` and `use <path>` directives.
///
/// In OpenSCAD, paths are written unquoted between angle brackets:
/// `include <foo/bar.scad>`, `use <../lib/utils.scad>`.
///
/// The lexer emits the text between `<` and `>` verbatim as a single
/// [`Tokens::Path`], so spaces and digit runs survive unchanged.
#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub struct LitPath {
    path: Box<str>,
    span: Span,
}

impl LitPath {
    /// Returns the path string.
    pub fn value(&self) -> &str {
        &self.path
    }
}

impl Spannable for LitPath {
    fn span(&self) -> Span {
        self.span
    }
}

impl<'a> Parseable<'a> for LitPath {
    fn parser<I>() -> impl Parser<'a, I, Self, ParserError<'a>> + Clone
    where
        I: Input<'a>,
    {
        select! {
            Tokens::Path(path) = extra => LitPath {
                path: path.into(),
                span: extra.span(),
            },
        }
        .labelled("LitPath")
    }
}
