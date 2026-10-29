use crate::span::Span;
use logos::Logos;
use std::fmt;
use std::ops::Range;

/// Custom error type for the OpenSCAD lexer
#[derive(Debug, Clone, PartialEq)]
pub enum LexingError<'l> {
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
            LexingError::InvalidString => write!(f, "Invalid string literal"),
            LexingError::UnexpectedCharacter(c) => write!(f, "Unexpected character: '{}'", c),
            LexingError::Unknown(token) => write!(f, "Unknown token: {}", token),
        }
    }
}

impl std::error::Error for LexingError<'_> {}

/// Callback for creating LexingError from lexer context
impl<'l> LexingError<'l> {
    fn from_lexer(lex: &mut logos::Lexer<'l, Tokens<'l>>) -> Self {
        LexingError::Unknown(lex.slice())
    }
}

#[derive(Debug, Clone, Logos, PartialEq, Eq, Hash)]
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
    //
    // The source text is kept as written; [`LitNum`](crate::prelude::LitNum)
    // converts it to `f64`, so the raw spelling (`1.50`, `1e3`) survives.
    #[regex(r"[0-9]+\.?[0-9]*([eE][+-]?[0-9]+)?", |lex| lex.slice())]
    #[regex(r"\.[0-9]+([eE][+-]?[0-9]+)?", |lex| lex.slice())]
    Number(&'t str),

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

    /// The raw text between `<` and `>` of an `include` / `use` directive.
    /// Never produced by the raw [`Tokens::lexer`]; see
    /// [`Tokens::lexer_with_errors`].
    Path(&'t str),
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
            Self::Path(s) => write!(f, "{s}"),
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
            Tokens::spanned(source, context).filter_map(|(token, span)| match token {
                Ok(t) if t.is_comment() => None,
                Ok(t) => Some((t, span)),
                Err(_) => None,
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
        for (token, span) in Tokens::spanned(source, id) {
            match token {
                Ok(t) => tokens.push((t, span)),
                Err(e) => errors.push((e, span)),
            }
        }
        (tokens, errors)
    }

    /// Lex `source`, pairing each token (or lex error) with its span.
    ///
    /// Unlike the raw [`Tokens::lexer`], the `<path>` following `include` /
    /// `use` is taken verbatim from the source and emitted as
    /// [`Lt`](Tokens::Lt), [`Path`](Tokens::Path), [`Gt`](Tokens::Gt) — the
    /// same special case OpenSCAD's own lexer makes. Tokenising the path
    /// instead would mangle it (`01` → `1`, `2.` → `2`, spaces dropped).
    fn spanned(
        source: &'t str,
        context: usize,
    ) -> impl Iterator<Item = (Result<Tokens<'t>, LexingError<'t>>, Span)> + Clone {
        let mut lexer = Tokens::lexer(source);
        std::iter::from_fn(move || {
            let token = lexer.next()?;
            let keyword = lexer.span();
            let import = matches!(token, Ok(Tokens::Include | Tokens::Use))
                .then(|| ImportPath::find(lexer.remainder(), keyword.end))
                .flatten()
                .map(|import| {
                    lexer.bump(import.gt.end - keyword.end);
                    [
                        (Ok(Tokens::Lt), import.lt),
                        (Ok(Tokens::Path(&source[import.path.clone()])), import.path),
                        (Ok(Tokens::Gt), import.gt),
                    ]
                });
            Some(std::iter::once((token, keyword)).chain(import.into_iter().flatten()))
        })
        .flatten()
        .map(move |(token, range)| {
            let span = Span {
                start: range.start,
                end: range.end,
                context,
            };
            (token, span)
        })
    }
}

/// Byte ranges of the `<`, path text, and `>` of an `include` / `use` path.
struct ImportPath {
    lt: Range<usize>,
    path: Range<usize>,
    gt: Range<usize>,
}

impl ImportPath {
    /// Locate `<path>` at the start of `rest` (after optional whitespace),
    /// where `rest` begins at byte `offset` of the source. Mirrors OpenSCAD's
    /// `include[ \t\r\n]*<[^\t\r\n>]*>` rule: the path may not span lines.
    fn find(rest: &str, offset: usize) -> Option<Self> {
        let body = rest.trim_start_matches([' ', '\t', '\r', '\n']);
        let lt = offset + (rest.len() - body.len());
        let body = body.strip_prefix('<')?;
        let len = body.find(['>', '\t', '\r', '\n'])?;
        body[len..].starts_with('>').then(|| {
            let path = lt + 1..lt + 1 + len;
            Self {
                lt: lt..lt + 1,
                gt: path.end..path.end + 1,
                path,
            }
        })
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
