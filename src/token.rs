use crate::prelude::*;
macro_rules! create_tokens {
    ($($name:ident => $symbol:expr),*) => {
        $(
            #[derive(Debug, Clone, PartialEq, Eq, Hash)]
            pub struct $name {
                span: Span
            }
            impl Spannable for $name {
                fn span(&self) -> Span {
                    self.span.clone()
                }
            }
            impl<'a> Parseable<'a> for $name {
                fn parser<I>() -> impl Parser<'a, I, Self, ParserError<'a>> + Clone
                where
                    I: Input<'a>,
                {
                    just($symbol).to_span().map(|span| Self { span }).labelled(format!("'{}'",$symbol))
                }
            }
        )*
    };
}
create_tokens!(
    Eq => Tokens::Equals,
    Question => Tokens::Question,
    Comma => Tokens::Comma,
    Semicolon => Tokens::Semicolon,
    Colon => Tokens::Colon,
    Plus => Tokens::Plus,
    Minus => Tokens::Minus,
    Star => Tokens::Star,
    Slash => Tokens::Slash,
    Percent => Tokens::Percent,
    Bang => Tokens::Bang,
    Lt => Tokens::Lt,
    Gt => Tokens::Gt,
    Le => Tokens::Le,
    Ge => Tokens::Ge,
    EqEq => Tokens::EqEq,
    Ne => Tokens::Ne,
    And => Tokens::And,
    Or => Tokens::Or,
    Dot => Tokens::Dot,
    DotDot => Tokens::DotDot,
    DotDotDot => Tokens::DotDotDot,
    Hash => Tokens::Hash,
    Module => Tokens::Module,
    Function => Tokens::Function,
    Include => Tokens::Include,
    Use => Tokens::Use,
    If => Tokens::If,
    Else => Tokens::Else,
    For => Tokens::For,
    Let => Tokens::Let,
    Assert => Tokens::Assert,
    Echo => Tokens::Echo,
    Each => Tokens::Each
);

#[macro_export]
macro_rules! Token {
    [=] => {$crate::token::Eq };
    [?] => {$crate::token::Question };
    [,] => {$crate::token::Comma };
    [;] => {$crate::token::Semicolon };
    [:] => {$crate::token::Colon };
    [+] => {$crate::token::Plus };
    [-] => {$crate::token::Minus };
    [*] => {$crate::token::Star };
    [/] => {$crate::token::Slash };
    [%] => {$crate::token::Percent };
    [!] => {$crate::token::Bang };
    [<] => {$crate::token::Lt };
    [>] => {$crate::token::Gt };
    [<=] => {$crate::token::Le };
    [>=] => {$crate::token::Ge };
    [==] => {$crate::token::EqEq };
    [!=] => {$crate::token::Ne };
    [&&] => {$crate::token::And };
    [||] => {$crate::token::Or };
    [.] => {$crate::token::Dot };
    [..] => {$crate::token::DotDot };
    [#] => {$crate::token::Hash };
    [Module] => {$crate::token::Module };
    [mod] => {$crate::token::Module };
    [()] => {$crate::extra::Paren };
    [[]] => {$crate::extra::Brace };
    [Function] => {$crate::token::Function};
    [fn] => {$crate::token::Function};
    [Include] => {$crate::token::Include};
    [include] => {$crate::token::Include};
    [Use] => {$crate::token::Use};
    [use] => {$crate::token::Use};
    [if] => {$crate::token::If};
    [else] => {$crate::token::Else};
    [for] => {$crate::token::For};
    [let] => {$crate::token::Let};
    [assert] => {$crate::token::Assert};
    [echo] => {$crate::token::Echo};
    [each] => {$crate::token::Each};
}
