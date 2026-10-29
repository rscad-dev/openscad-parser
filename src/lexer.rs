use crate::span::Span;
use logos::Logos;
use std::fmt;
use std::num::ParseFloatError;

/// Custom error type for the OpenSCAD lexer
#[derive(Debug, Clone, PartialEq)]
pub enum LexingError<'l> {
    /// Invalid number literal
    InvalidNumber(ParseFloatError),
    /// Invalid string literal (e.g., unclosed string)
    InvalidString,
    /// Unexpected character
    UnexpectedCharacter(char),
    /// Unknown token
    Unknown(&'l str),
}

impl Default for LexingError<'_> {
    fn default() -> Self {
        // hopefully this variant is never used and we get the callback that provides the actual unknown token
        LexingError::Unknown("")
    }
}

impl fmt::Display for LexingError<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            LexingError::InvalidNumber(s) => write!(f, "Invalid number literal: {}", s),
            LexingError::InvalidString => write!(f, "Invalid string literal"),
            LexingError::UnexpectedCharacter(c) => write!(f, "Unexpected character: '{}'", c),
            LexingError::Unknown(token) => write!(f, "Unknown token: {}", token),
        }
    }
}

impl std::error::Error for LexingError<'_> {}

/// Convert ParseFloatError to LexingError
impl From<ParseFloatError> for LexingError<'_> {
    fn from(err: ParseFloatError) -> Self {
        LexingError::InvalidNumber(err)
    }
}

/// Callback for creating LexingError from lexer context
impl<'l> LexingError<'l> {
    fn from_lexer(lex: &mut logos::Lexer<'l, Tokens<'l>>) -> Self {
        LexingError::Unknown(lex.slice())
    }
}

#[derive(Debug, Clone, Logos, PartialEq)]
#[logos(skip r"[ \t\n\r]+")]
#[logos(error(LexingError<'s>, LexingError::from_lexer))]
pub enum Tokens<'t> {
    // Comments (must come before Slash operator to match // and /*)
    #[regex(r"//[^\n]*", |lex| lex.slice(), allow_greedy = true)]
    LineComment(&'t str),

    // Standard C-style block comment. The naive `/\*([^*]|\*[^/])*\*/` cannot
    // match a body ending in `*` (e.g. `/***/`, `/* a **/`, banner comments),
    // because the final `*` before `*/` fits neither `[^*]` nor `\*[^/]`.
    #[regex(r"/\*[^*]*\*+([^/*][^*]*\*+)*/", |lex| lex.slice())]
    BlockComment(&'t str),

    // Keywords
    #[token("module")]
    Module,
    #[token("function")]
    Function,
    #[token("for")]
    For,
    #[token("if")]
    If,
    #[token("else")]
    Else,
    #[token("let")]
    Let,
    #[token("assert")]
    Assert,
    #[token("echo")]
    Echo,
    #[token("each")]
    Each,
    #[token("use")]
    Use,
    #[token("include")]
    Include,
    #[token("true")]
    True,
    #[token("false")]
    False,
    #[token("undef")]
    Undef,

    // Operators - Multi-character (must come before single-character)
    #[token("==")]
    EqEq,
    #[token("!=")]
    Ne,
    #[token("<=")]
    Le,
    #[token(">=")]
    Ge,
    #[token("&&")]
    And,
    #[token("||")]
    Or,
    #[token("...")]
    DotDotDot,
    #[token("..")]
    DotDot,

    // Operators - Single-character
    #[token("=")]
    Equals,
    #[token("+")]
    Plus,
    #[token("-")]
    Minus,
    #[token("*")]
    Star,
    #[token("/")]
    Slash,
    #[token("%")]
    Percent,
    #[token("!")]
    Bang,
    #[token("<")]
    Lt,
    #[token(">")]
    Gt,
    #[token("?")]
    Question,
    #[token(":")]
    Colon,
    #[token(".")]
    Dot,

    // Delimiters
    #[token("(")]
    LParen,
    #[token(")")]
    RParen,
    #[token("[")]
    LBracket,
    #[token("]")]
    RBracket,
    #[token("{")]
    LBrace,
    #[token("}")]
    RBrace,
    #[token(",")]
    Comma,
    #[token(";")]
    Semicolon,
    #[token("#")]
    Hash,

    // Literals
    //
    // No leading sign: `x-1` must lex as `x`, `-`, `1` so the parser sees a
    // binary subtraction. Negative literals are handled by unary `-` in the
    // Pratt parser. Leading-dot (`.5`), trailing-dot (`1.`), and exponent
    // forms are all valid OpenSCAD numbers.
    #[regex(r"[0-9]+\.?[0-9]*([eE][+-]?[0-9]+)?", |lex| lex.slice().parse::<f64>().ok())]
    #[regex(r"\.[0-9]+([eE][+-]?[0-9]+)?", |lex| lex.slice().parse::<f64>().ok())]
    Number(f64),

    #[regex(r#""([^"\\]|\\.)*""#, |lex| {
        let s = lex.slice();
        &s[1..s.len()-1]
    })]
    String(&'t str),

    // Identifiers (must come after keywords)
    #[regex(r"[a-zA-Z_][a-zA-Z0-9_]*", |lex| lex.slice())]
    Identifier(&'t str),

    // Special literals
    #[token("$")]
    Dollar,
}

/// Tokens paired with their source spans.
pub type SpannedTokens<'t> = Vec<(Tokens<'t>, Span)>;
/// Lex errors paired with their source spans.
pub type SpannedLexErrors<'t> = Vec<(LexingError<'t>, Span)>;

impl fmt::Display for Tokens<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            // Comments
            Self::LineComment(s) => write!(f, "{s}"),
            Self::BlockComment(s) => write!(f, "{s}"),

            // Keywords
            Self::Module => f.write_str("module"),
            Self::Function => f.write_str("function"),
            Self::For => f.write_str("for"),
            Self::If => f.write_str("if"),
            Self::Else => f.write_str("else"),
            Self::Let => f.write_str("let"),
            Self::Assert => f.write_str("assert"),
            Self::Echo => f.write_str("echo"),
            Self::Each => f.write_str("each"),
            Self::Use => f.write_str("use"),
            Self::Include => f.write_str("include"),
            Self::True => f.write_str("true"),
            Self::False => f.write_str("false"),
            Self::Undef => f.write_str("undef"),

            // Operators - Multi-character
            Self::EqEq => f.write_str("=="),
            Self::Ne => f.write_str("!="),
            Self::Le => f.write_str("<="),
            Self::Ge => f.write_str(">="),
            Self::And => f.write_str("&&"),
            Self::Or => f.write_str("||"),
            Self::DotDotDot => f.write_str("..."),
            Self::DotDot => f.write_str(".."),

            // Operators - Single-character
            Self::Equals => f.write_str("="),
            Self::Plus => f.write_str("+"),
            Self::Minus => f.write_str("-"),
            Self::Star => f.write_str("*"),
            Self::Slash => f.write_str("/"),
            Self::Percent => f.write_str("%"),
            Self::Bang => f.write_str("!"),
            Self::Lt => f.write_str("<"),
            Self::Gt => f.write_str(">"),
            Self::Question => f.write_str("?"),
            Self::Colon => f.write_str(":"),
            Self::Dot => f.write_str("."),

            // Delimiters
            Self::LParen => f.write_str("("),
            Self::RParen => f.write_str(")"),
            Self::LBracket => f.write_str("["),
            Self::RBracket => f.write_str("]"),
            Self::LBrace => f.write_str("{"),
            Self::RBrace => f.write_str("}"),
            Self::Comma => f.write_str(","),
            Self::Semicolon => f.write_str(";"),
            Self::Hash => f.write_str("#"),

            // Literals
            Self::Number(n) => write!(f, "{n}"),
            Self::String(s) => write!(f, "\"{s}\""),
            Self::Identifier(s) => write!(f, "{s}"),

            // Special
            Self::Dollar => f.write_str("$"),
        }
    }
}

impl<'t> Tokens<'t> {
    /// Returns `true` if the tokens is [`LineComment`].
    ///
    /// [`LineComment`]: Tokens::LineComment
    #[must_use]
    pub fn is_line_comment(&self) -> bool {
        matches!(self, Self::LineComment(..))
    }

    /// Returns `true` if the tokens is [`BlockComment`].
    ///
    /// [`BlockComment`]: Tokens::BlockComment
    #[must_use]
    pub fn is_block_comment(&self) -> bool {
        matches!(self, Self::BlockComment(..))
    }

    /// Returns `true` if the tokens is either a line comment or a block comment.
    /// This is a convenience method for filtering out comments from the token stream.
    #[must_use]
    pub fn is_comment(&self) -> bool {
        self.is_line_comment() || self.is_block_comment()
    }

    /// Lex `source` into a chumsky input, silently dropping comments and lex
    /// errors.
    ///
    /// Comments are filtered out so they can appear anywhere (inside argument
    /// lists, vectors, expressions) without tripping the grammar. Use
    /// [`Tokens::to_chumsky_input_with_errors`] when lex errors must be
    /// surfaced as diagnostics.
    pub fn to_chumsky_input<'a>(source: &'a str, context: usize) -> impl crate::parser::Input<'a> {
        chumsky::input::IterInput::new(
            Tokens::lexer(source)
                .spanned()
                .filter_map(move |(token, span)| {
                    let span = Span {
                        start: span.start,
                        end: span.end,
                        context,
                    };
                    match token {
                        Ok(t) if t.is_comment() => None,
                        Ok(t) => Some((t, span)),
                        Err(_) => None,
                    }
                }),
            Span {
                start: source.len(),
                end: source.len(),
                context,
            },
        )
    }

    /// Lex `source` into a chumsky input, collecting lex errors instead of
    /// dropping them. Comments are filtered out (see [`Tokens::to_chumsky_input`]).
    pub fn to_chumsky_input_with_errors<'a>(
        source: &'a str,
        context: usize,
    ) -> (impl crate::parser::Input<'a>, SpannedLexErrors<'a>) {
        let (tokens, errors) = Tokens::lexer_with_errors(source, context);
        let input = chumsky::input::IterInput::new(
            tokens.into_iter().filter(|(t, _)| !t.is_comment()),
            Span {
                start: source.len(),
                end: source.len(),
                context,
            },
        );
        (input, errors)
    }

    /// Lex the source, returning valid tokens and collecting errors separately.
    ///
    /// Returns `(tokens, errors)` where `tokens` is a `Vec<(Tokens, Span)>` and
    /// `errors` is a `Vec<(LexingError, Span)>`.
    pub fn lexer_with_errors(
        source: &'t str,
        id: usize,
    ) -> (SpannedTokens<'t>, SpannedLexErrors<'t>) {
        let mut tokens = Vec::new();
        let mut errors = Vec::new();
        for (token, span) in Tokens::lexer(source).spanned() {
            let span = Span {
                start: span.start,
                end: span.end,
                context: id,
            };
            match token {
                Ok(t) => tokens.push((t, span)),
                Err(e) => errors.push((e, span)),
            }
        }
        (tokens, errors)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Lex `source` into a plain vector of tokens, panicking on any lex error.
    #[track_caller]
    fn lex(source: &str) -> Vec<Tokens<'_>> {
        Tokens::lexer(source)
            .map(|res| res.expect("BUG: unexpected lex error"))
            .collect()
    }

    /// A block comment must be consumed as a single `BlockComment`, never
    /// decaying into `Star`/`Slash` operator tokens.
    #[track_caller]
    fn assert_single_block_comment(source: &str) {
        let tokens = lex(source);
        assert_eq!(
            tokens.len(),
            1,
            "expected a single token for {source:?}, got {tokens:?}"
        );
        assert!(
            matches!(tokens[0], Tokens::BlockComment(_)),
            "expected a BlockComment for {source:?}, got {tokens:?}"
        );
        assert!(
            !tokens
                .iter()
                .any(|t| matches!(t, Tokens::Star | Tokens::Slash)),
            "block comment {source:?} decayed into operator tokens: {tokens:?}"
        );
    }

    #[test]
    fn block_comment_empty() {
        assert_single_block_comment("/**/");
    }

    #[test]
    fn block_comment_only_stars() {
        assert_single_block_comment("/***/");
    }

    #[test]
    fn block_comment_body_ends_in_star() {
        assert_single_block_comment("/* a **/");
    }

    #[test]
    fn block_comment_banner() {
        assert_single_block_comment("/**** banner ****/");
    }

    #[test]
    fn block_comment_multiline() {
        assert_single_block_comment("/* multi\nline */");
    }
}
