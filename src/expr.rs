use crate::ident::Ident;
use crate::item::Assignment;
use crate::item::Block;
use crate::prelude::*;

#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub enum Expr {
    Assert(ExprAssert),
    Assign(ExprAssign),
    Binary(ExprBinary),
    Call(ExprCall),
    Echo(ExprEcho),
    Ident(Ident),
    Index(ExprIndex),
    Let(ExprLet),
    ListComp(ExprListComp),
    Lit(ExprLit),
    Member(ExprMember),
    Range(ExprRange),
    Ternary(ExprTernary),
    Unary(ExprUnary),
    Vector(ExprVector),
    Block(ExprBlock),
}

impl Spannable for Expr {
    fn span(&self) -> Span {
        match self {
            Expr::Assert(e) => e.span(),
            Expr::Assign(e) => e.span(),
            Expr::Call(e) => e.span(),
            Expr::Binary(e) => e.span(),
            Expr::Echo(e) => e.span(),
            Expr::Ident(e) => e.span(),
            Expr::Index(e) => e.span(),
            Expr::Let(e) => e.span(),
            Expr::ListComp(e) => e.span(),
            Expr::Lit(e) => e.lit.span(),
            Expr::Member(e) => e.span(),
            Expr::Range(e) => e.span(),
            Expr::Ternary(e) => e.span(),
            Expr::Unary(e) => e.span(),
            Expr::Vector(e) => e.span(),
            Expr::Block(e) => e.span(),
        }
    }
}

impl<'a> Parseable<'a> for Expr {
    fn parser<I>() -> impl Parser<'a, I, Self, ParserError<'a>> + Clone
    where
        I: Input<'a>,
    {
        use chumsky::pratt::*;

        recursive(|expr| {
            // Call arguments: (arg, arg, ...)
            let call_args = delimited_paren_with::<_, I>(punctuated_with::<_, _, I>(
                Arg::parser_with_expr::<I>(expr.clone()),
                <Token![,]>::parser::<I>(),
            ));

            // Grouped sub-expression: ( expr )  — evaluates to the inner expr
            let grouped = just(Tokens::LParen)
                .ignore_then(expr.clone())
                .then_ignore(just(Tokens::RParen));

            // Block expression built inline using the `expr` handle so we don't
            // re-enter Expr::parser() and cause a stack overflow.
            let stmt = {
                let local = Ident::parser::<I>()
                    .then(<Token![=]>::parser::<I>())
                    .then(expr.clone())
                    .then(<Token![;]>::parser::<I>())
                    .map(|(((name, eq_token), value), semicolon)| {
                        Stmt::Local(Local {
                            name,
                            eq_token,
                            value,
                            semicolon,
                        })
                    });
                let expr_stmt = expr
                    .clone()
                    .then(<Token![;]>::parser::<I>())
                    .map(|(e, sc)| Stmt::Expr(e, sc));
                // NOTE: Item (function/module decls) are not supported inside block
                // expressions to avoid re-entering Expr::parser() outside the
                // recursive boundary. Top-level items are parsed separately.
                choice((local, expr_stmt))
            };
            let block_expr = {
                let open = just(Tokens::LBrace).to_span();
                let close = just(Tokens::RBrace).to_span();
                let stmts = stmt.repeated().collect::<Vec<_>>();
                group((open, stmts, close)).map(|(open, stmts, close)| {
                    Expr::Block(ExprBlock {
                        block: Block {
                            brace_token: Brace {
                                span: DelimSpan { open, close },
                            },
                            stmts,
                        },
                    })
                })
            };

            let range_expr = ExprRange::parser_with_expr::<I>(expr.clone()).map(Expr::Range);
            let vector_expr = ExprVector::parser_with_expr::<I>(expr.clone()).map(Expr::Vector);
            let let_expr = ExprLet::parser_with_expr::<I>(expr.clone()).map(Expr::Let);
            let echo_expr = ExprEcho::parser_with_expr::<I>(expr.clone()).map(Expr::Echo);
            let assert_expr = ExprAssert::parser_with_expr::<I>(expr.clone()).map(Expr::Assert);
            let list_comp_expr =
                ExprListComp::parser_with_expr::<I>(expr.clone()).map(Expr::ListComp);

            // Atom: the non-recursive base cases.
            // ExprCall must be tried before bare Ident so that `foo(...)` is
            // consumed as a call rather than leaving the `(...)` unparsed.
            // Bracket forms are ordered range → vector → list-comp: ranges
            // need the `:`, plain expression lists become vectors, and only
            // lists containing a generator (`for`/`each`/`let`/`if`) fall
            // through to the comprehension parser.
            let atom = choice((
                Ident::parser::<I>()
                    .then(call_args)
                    .map(|(func, args)| ExprCall { func, args })
                    .map(Expr::Call),
                ExprLit::parser::<I>().map(Expr::Lit),
                let_expr,
                echo_expr,
                assert_expr,
                block_expr,
                grouped,
                range_expr,
                vector_expr,
                list_comp_expr,
                Ident::parser::<I>().map(Expr::Ident),
            ));

            // Pratt parser: atom + operator rules handle precedence/associativity
            // without any left-recursion.
            atom.pratt((
                // Assignment: right-associative, lowest precedence (1)
                infix(
                    right(1),
                    <Token![=]>::parser::<I>(),
                    |left, eq_token, right, _| {
                        Expr::Assign(ExprAssign {
                            left: Box::new(left),
                            eq_token,
                            right: Box::new(right),
                        })
                    },
                ),
                // Ternary: right-associative, precedence 2
                // Parsed as a postfix operator: `? <expr> : <expr>`
                postfix(
                    2,
                    <Token![?]>::parser::<I>()
                        .then(expr.clone())
                        .then(<Token![:]>::parser::<I>())
                        .then(expr.clone()),
                    |condition, (((question_token, if_true), colon_token), if_false), _| {
                        Expr::Ternary(ExprTernary {
                            condition: Box::new(condition),
                            question_token,
                            if_true: Box::new(if_true),
                            colon_token,
                            if_false: Box::new(if_false),
                        })
                    },
                ),
                // Logical or: left-associative, precedence 3
                infix(
                    left(3),
                    <Token![||]>::parser::<I>(),
                    |left, op, right, _| {
                        Expr::Binary(ExprBinary {
                            left: Box::new(left),
                            op: BinOp::Or(op),
                            right: Box::new(right),
                        })
                    },
                ),
                // Logical and: left-associative, precedence 4
                infix(
                    left(4),
                    <Token![&&]>::parser::<I>(),
                    |left, op, right, _| {
                        Expr::Binary(ExprBinary {
                            left: Box::new(left),
                            op: BinOp::And(op),
                            right: Box::new(right),
                        })
                    },
                ),
                // Equality: left-associative, precedence 5
                infix(
                    left(5),
                    <Token![==]>::parser::<I>(),
                    |left, op, right, _| {
                        Expr::Binary(ExprBinary {
                            left: Box::new(left),
                            op: BinOp::EqEq(op),
                            right: Box::new(right),
                        })
                    },
                ),
                infix(
                    left(5),
                    <Token![!=]>::parser::<I>(),
                    |left, op, right, _| {
                        Expr::Binary(ExprBinary {
                            left: Box::new(left),
                            op: BinOp::Ne(op),
                            right: Box::new(right),
                        })
                    },
                ),
                // Relational: left-associative, precedence 6
                // Multi-char tokens (<=, >=) must be listed before single-char (<, >)
                infix(
                    left(6),
                    <Token![<=]>::parser::<I>(),
                    |left, op, right, _| {
                        Expr::Binary(ExprBinary {
                            left: Box::new(left),
                            op: BinOp::Le(op),
                            right: Box::new(right),
                        })
                    },
                ),
                infix(
                    left(6),
                    <Token![>=]>::parser::<I>(),
                    |left, op, right, _| {
                        Expr::Binary(ExprBinary {
                            left: Box::new(left),
                            op: BinOp::Ge(op),
                            right: Box::new(right),
                        })
                    },
                ),
                infix(left(6), <Token![<]>::parser::<I>(), |left, op, right, _| {
                    Expr::Binary(ExprBinary {
                        left: Box::new(left),
                        op: BinOp::Lt(op),
                        right: Box::new(right),
                    })
                }),
                infix(left(6), <Token![>]>::parser::<I>(), |left, op, right, _| {
                    Expr::Binary(ExprBinary {
                        left: Box::new(left),
                        op: BinOp::Gt(op),
                        right: Box::new(right),
                    })
                }),
                // Additive: left-associative, precedence 7
                infix(left(7), <Token![+]>::parser::<I>(), |left, op, right, _| {
                    Expr::Binary(ExprBinary {
                        left: Box::new(left),
                        op: BinOp::Add(op),
                        right: Box::new(right),
                    })
                }),
                infix(left(7), <Token![-]>::parser::<I>(), |left, op, right, _| {
                    Expr::Binary(ExprBinary {
                        left: Box::new(left),
                        op: BinOp::Sub(op),
                        right: Box::new(right),
                    })
                }),
                // Multiplicative: left-associative, precedence 8
                infix(left(8), <Token![*]>::parser::<I>(), |left, op, right, _| {
                    Expr::Binary(ExprBinary {
                        left: Box::new(left),
                        op: BinOp::Mul(op),
                        right: Box::new(right),
                    })
                }),
                infix(left(8), <Token![/]>::parser::<I>(), |left, op, right, _| {
                    Expr::Binary(ExprBinary {
                        left: Box::new(left),
                        op: BinOp::Div(op),
                        right: Box::new(right),
                    })
                }),
                infix(left(8), <Token![%]>::parser::<I>(), |left, op, right, _| {
                    Expr::Binary(ExprBinary {
                        left: Box::new(left),
                        op: BinOp::Mod(op),
                        right: Box::new(right),
                    })
                }),
                // Unary prefix: precedence 9 (tightest)
                prefix(9, <Token![-]>::parser::<I>(), |op, expr, _| {
                    Expr::Unary(ExprUnary {
                        op: UnaryOp::Neg(op),
                        expr: Box::new(expr),
                    })
                }),
                prefix(9, <Token![!]>::parser::<I>(), |op, expr, _| {
                    Expr::Unary(ExprUnary {
                        op: UnaryOp::Not(op),
                        expr: Box::new(expr),
                    })
                }),
                // Index: postfix, precedence 10 (tightest — binds before unary)
                postfix(
                    10,
                    just(Tokens::LBracket)
                        .to_span()
                        .then(expr.clone())
                        .then(just(Tokens::RBracket).to_span()),
                    |lhs, ((open, index), close), _| {
                        Expr::Index(ExprIndex {
                            expr: Box::new(lhs),
                            bracket: Bracket {
                                span: DelimSpan { open, close },
                            },
                            index: Box::new(index),
                        })
                    },
                ),
                // Member access: postfix, precedence 10 (same as index)
                postfix(
                    10,
                    <Token![.]>::parser::<I>().then(Ident::parser::<I>()),
                    |lhs, (dot_token, member), _| {
                        Expr::Member(ExprMember {
                            expr: Box::new(lhs),
                            dot_token,
                            member,
                        })
                    },
                ),
            ))
        })
        .labelled("Expr")
    }
}

#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub struct ExprCall {
    pub func: Ident,
    pub args: Delimited<Punctuated<Arg, Token![,]>, Paren>,
}

impl Spannable for ExprCall {
    fn span(&self) -> Span {
        self.func.span().union(self.args.span())
    }
}

impl<'a> Parseable<'a> for ExprCall {
    fn parser<I>() -> impl Parser<'a, I, Self, ParserError<'a>> + Clone
    where
        I: Input<'a>,
    {
        Ident::parser::<I>()
            .then(Delimited::<Punctuated<Arg, Token![,]>, Paren>::parser::<I>())
            .map(|(func, args)| ExprCall { func, args })
            .labelled("ExprCall")
    }
}

/// A function call argument: either positional (`expr`) or named (`name = expr`).
#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub enum Arg {
    Positional(Expr),
    Named {
        name: Ident,
        eq_token: Token![=],
        value: Expr,
    },
}

impl Spannable for Arg {
    fn span(&self) -> Span {
        match self {
            Arg::Positional(expr) => expr.span(),
            Arg::Named { name, value, .. } => name.span().union(value.span()),
        }
    }
}

impl Arg {
    pub(crate) fn parser_with_expr<'a, I: Input<'a>>(
        expr: impl Parser<'a, I, Expr, ParserError<'a>> + Clone + 'a,
    ) -> impl Parser<'a, I, Self, ParserError<'a>> + Clone {
        let named = Ident::parser::<I>()
            .then(<Token![=]>::parser::<I>())
            .then(expr.clone())
            .map(|((name, eq_token), value)| Arg::Named {
                name,
                eq_token,
                value,
            });
        let positional = expr.map(Arg::Positional);
        choice((named, positional)).labelled("Arg")
    }
}

impl<'a> Parseable<'a> for Arg {
    fn parser<I>() -> impl Parser<'a, I, Self, ParserError<'a>> + Clone
    where
        I: Input<'a>,
    {
        Arg::parser_with_expr::<I>(Expr::parser::<I>())
    }
}

#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub struct ExprLit {
    pub lit: Lit,
}

impl Spannable for ExprLit {
    fn span(&self) -> Span {
        self.lit.span()
    }
}

impl<'a> Parseable<'a> for ExprLit {
    fn parser<I>() -> impl Parser<'a, I, Self, ParserError<'a>> + Clone
    where
        I: Input<'a>,
    {
        Lit::parser::<I>()
            .map(|lit| ExprLit { lit })
            .labelled("ExprLit")
    }
}

#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub struct ExprAssign {
    pub left: Box<Expr>,
    pub eq_token: Token![=],
    pub right: Box<Expr>,
}

impl Spannable for ExprAssign {
    fn span(&self) -> Span {
        self.left.span().union(self.right.span())
    }
}

impl<'a> Parseable<'a> for ExprAssign {
    fn parser<I>() -> impl Parser<'a, I, Self, ParserError<'a>> + Clone
    where
        I: Input<'a>,
    {
        Expr::parser::<I>()
            .try_map(|expr, span| match expr {
                Expr::Assign(a) => Ok(a),
                _ => Err(chumsky::error::Rich::custom(
                    span,
                    "expected assignment expression",
                )),
            })
            .labelled("ExprAssign")
    }
}

#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub struct ExprTernary {
    pub condition: Box<Expr>,
    pub question_token: Token![?],
    pub if_true: Box<Expr>,
    pub colon_token: Token![:],
    pub if_false: Box<Expr>,
}

impl Spannable for ExprTernary {
    fn span(&self) -> Span {
        self.condition.span().union(self.if_false.span())
    }
}

impl<'a> Parseable<'a> for ExprTernary {
    fn parser<I>() -> impl Parser<'a, I, Self, ParserError<'a>> + Clone
    where
        I: Input<'a>,
    {
        Expr::parser::<I>()
            .try_map(|expr, span| match expr {
                Expr::Ternary(t) => Ok(t),
                _ => Err(chumsky::error::Rich::custom(
                    span,
                    "expected ternary expression",
                )),
            })
            .labelled("ExprTernary")
    }
}

#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub struct ExprIndex {
    pub expr: Box<Expr>,
    pub bracket: Bracket,
    pub index: Box<Expr>,
}

impl Spannable for ExprIndex {
    fn span(&self) -> Span {
        self.expr.span().union(self.bracket.span())
    }
}

impl<'a> Parseable<'a> for ExprIndex {
    fn parser<I>() -> impl Parser<'a, I, Self, ParserError<'a>> + Clone
    where
        I: Input<'a>,
    {
        Expr::parser::<I>()
            .try_map(|expr, span| match expr {
                Expr::Index(i) => Ok(i),
                _ => Err(chumsky::error::Rich::custom(
                    span,
                    "expected index expression",
                )),
            })
            .labelled("ExprIndex")
    }
}

#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub struct ExprMember {
    pub expr: Box<Expr>,
    pub dot_token: Token![.],
    pub member: Ident,
}

impl Spannable for ExprMember {
    fn span(&self) -> Span {
        self.expr.span().union(self.member.span())
    }
}

impl<'a> Parseable<'a> for ExprMember {
    fn parser<I>() -> impl Parser<'a, I, Self, ParserError<'a>> + Clone
    where
        I: Input<'a>,
    {
        Expr::parser::<I>()
            .try_map(|expr, span| match expr {
                Expr::Member(m) => Ok(m),
                _ => Err(chumsky::error::Rich::custom(
                    span,
                    "expected member access expression",
                )),
            })
            .labelled("ExprMember")
    }
}

/// A vector/list literal: `[1, 2, 3]`, `[]`.
#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub struct ExprVector {
    pub bracket: Bracket,
    pub elements: Punctuated<Expr, Token![,]>,
}

impl Spannable for ExprVector {
    fn span(&self) -> Span {
        self.bracket.span()
    }
}

impl ExprVector {
    pub(crate) fn parser_with_expr<'a, I: Input<'a>>(
        expr: impl Parser<'a, I, Expr, ParserError<'a>> + Clone + 'a,
    ) -> impl Parser<'a, I, Self, ParserError<'a>> + Clone {
        delimited_bracket_with::<_, I>(punctuated_with::<_, _, I>(expr, <Token![,]>::parser::<I>()))
            .map(|delimited| ExprVector {
                bracket: Bracket {
                    span: delimited.delimiter.span.clone(),
                },
                elements: delimited.inner,
            })
            .labelled("ExprVector")
    }
}

impl<'a> Parseable<'a> for ExprVector {
    fn parser<I>() -> impl Parser<'a, I, Self, ParserError<'a>> + Clone
    where
        I: Input<'a>,
    {
        Expr::parser::<I>()
            .try_map(|expr, span| match expr {
                Expr::Vector(v) => Ok(v),
                _ => Err(chumsky::error::Rich::custom(
                    span,
                    "expected vector literal",
                )),
            })
            .labelled("ExprVector")
    }
}

/// A range expression: `[start : end]` or `[start : step : end]`.
#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub struct ExprRange {
    pub bracket: Bracket,
    pub start: Box<Expr>,
    pub colon1: Token![:],
    /// If present, `(step_expr, second_colon)`. In `[0:0.5:10]`, step is `0.5`.
    pub step: Option<(Box<Expr>, Token![:])>,
    pub end: Box<Expr>,
}

impl Spannable for ExprRange {
    fn span(&self) -> Span {
        self.bracket.span()
    }
}

impl ExprRange {
    pub(crate) fn parser_with_expr<'a, I: Input<'a>>(
        expr: impl Parser<'a, I, Expr, ParserError<'a>> + Clone + 'a,
    ) -> impl Parser<'a, I, Self, ParserError<'a>> + Clone {
        let open = just(Tokens::LBracket).to_span();
        let close = just(Tokens::RBracket).to_span();
        // [start : step : end] — three-operand range
        let three = group((
            open.clone(),
            expr.clone(),
            <Token![:]>::parser::<I>(),
            expr.clone(),
            <Token![:]>::parser::<I>(),
            expr.clone(),
            close.clone(),
        ))
        .map(
            |(open, start, colon1, step, colon2, end, close)| ExprRange {
                bracket: Bracket {
                    span: DelimSpan { open, close },
                },
                start: Box::new(start),
                colon1,
                step: Some((Box::new(step), colon2)),
                end: Box::new(end),
            },
        );
        // [start : end] — two-operand range
        let two = group((open, expr.clone(), <Token![:]>::parser::<I>(), expr, close)).map(
            |(open, start, colon1, end, close)| ExprRange {
                bracket: Bracket {
                    span: DelimSpan { open, close },
                },
                start: Box::new(start),
                colon1,
                step: None,
                end: Box::new(end),
            },
        );
        // Try three-operand first (more specific), then two-operand
        choice((three, two)).labelled("ExprRange")
    }
}

impl<'a> Parseable<'a> for ExprRange {
    fn parser<I>() -> impl Parser<'a, I, Self, ParserError<'a>> + Clone
    where
        I: Input<'a>,
    {
        Expr::parser::<I>()
            .try_map(|expr, span| match expr {
                Expr::Range(r) => Ok(r),
                _ => Err(chumsky::error::Rich::custom(
                    span,
                    "expected range expression",
                )),
            })
            .labelled("ExprRange")
    }
}

#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub struct ExprBinary {
    pub left: Box<Expr>,
    pub op: BinOp,
    pub right: Box<Expr>,
}

impl Spannable for ExprBinary {
    fn span(&self) -> Span {
        self.left.span().union(self.right.span())
    }
}

impl<'a> Parseable<'a> for ExprBinary {
    fn parser<I>() -> impl Parser<'a, I, Self, ParserError<'a>> + Clone
    where
        I: Input<'a>,
    {
        Expr::parser::<I>()
            .try_map(|expr, span| match expr {
                Expr::Binary(b) => Ok(b),
                _ => Err(chumsky::error::Rich::custom(
                    span,
                    "expected binary expression",
                )),
            })
            .labelled("ExprBinary")
    }
}

#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub enum BinOp {
    // Multiplicative (highest precedence)
    Mul(Token![*]),
    Div(Token![/]),
    Mod(Token![%]),
    // Additive
    Add(Token![+]),
    Sub(Token![-]),
    // Relational
    Lt(Token![<]),
    Gt(Token![>]),
    Le(Token![<=]),
    Ge(Token![>=]),
    // Equality
    EqEq(Token![==]),
    Ne(Token![!=]),
    // Logical
    And(Token![&&]),
    Or(Token![||]),
}

impl Spannable for BinOp {
    fn span(&self) -> Span {
        match self {
            BinOp::Mul(t) => t.span(),
            BinOp::Div(t) => t.span(),
            BinOp::Mod(t) => t.span(),
            BinOp::Add(t) => t.span(),
            BinOp::Sub(t) => t.span(),
            BinOp::Lt(t) => t.span(),
            BinOp::Gt(t) => t.span(),
            BinOp::Le(t) => t.span(),
            BinOp::Ge(t) => t.span(),
            BinOp::EqEq(t) => t.span(),
            BinOp::Ne(t) => t.span(),
            BinOp::And(t) => t.span(),
            BinOp::Or(t) => t.span(),
        }
    }
}

impl<'a> Parseable<'a> for BinOp {
    fn parser<I>() -> impl Parser<'a, I, Self, ParserError<'a>> + Clone
    where
        I: Input<'a>,
    {
        choice((
            <Token![*]>::parser::<I>().map(BinOp::Mul),
            <Token![/]>::parser::<I>().map(BinOp::Div),
            <Token![%]>::parser::<I>().map(BinOp::Mod),
            <Token![+]>::parser::<I>().map(BinOp::Add),
            <Token![-]>::parser::<I>().map(BinOp::Sub),
            <Token![<=]>::parser::<I>().map(BinOp::Le),
            <Token![>=]>::parser::<I>().map(BinOp::Ge),
            <Token![<]>::parser::<I>().map(BinOp::Lt),
            <Token![>]>::parser::<I>().map(BinOp::Gt),
            <Token![==]>::parser::<I>().map(BinOp::EqEq),
            <Token![!=]>::parser::<I>().map(BinOp::Ne),
            <Token![&&]>::parser::<I>().map(BinOp::And),
            <Token![||]>::parser::<I>().map(BinOp::Or),
        ))
        .labelled("BinOp")
    }
}

#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub enum UnaryOp {
    Neg(Token![-]),
    Not(Token![!]),
}

impl Spannable for UnaryOp {
    fn span(&self) -> Span {
        match self {
            UnaryOp::Neg(t) => t.span(),
            UnaryOp::Not(t) => t.span(),
        }
    }
}

impl<'a> Parseable<'a> for UnaryOp {
    fn parser<I>() -> impl Parser<'a, I, Self, ParserError<'a>> + Clone
    where
        I: Input<'a>,
    {
        choice((
            <Token![-]>::parser::<I>().map(UnaryOp::Neg),
            <Token![!]>::parser::<I>().map(UnaryOp::Not),
        ))
        .labelled("UnaryOp")
    }
}

#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub struct ExprUnary {
    pub op: UnaryOp,
    pub expr: Box<Expr>,
}

impl Spannable for ExprUnary {
    fn span(&self) -> Span {
        self.op.span().union(self.expr.span())
    }
}

impl<'a> Parseable<'a> for ExprUnary {
    fn parser<I>() -> impl Parser<'a, I, Self, ParserError<'a>> + Clone
    where
        I: Input<'a>,
    {
        UnaryOp::parser::<I>()
            .then(Expr::parser::<I>())
            .map(|(op, expr)| ExprUnary {
                op,
                expr: Box::new(expr),
            })
            .labelled("ExprUnary")
    }
}

#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub struct ExprBlock {
    pub(crate) block: Block,
}

impl Spannable for ExprBlock {
    fn span(&self) -> Span {
        self.block.span()
    }
}

impl<'a> Parseable<'a> for ExprBlock {
    fn parser<I>() -> impl Parser<'a, I, Self, ParserError<'a>> + Clone
    where
        I: Input<'a>,
    {
        Block::parser::<I>()
            .map(|block| ExprBlock { block })
            .labelled("ExprBlock")
    }
}

/// A let expression: `let (a = 1, b = 2) body_expr`.
///
/// Binds variables in the parenthesized assignment list, then evaluates the body
/// expression with those bindings in scope.
#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub struct ExprLet {
    pub let_token: Token![let],
    pub assignments: Delimited<Punctuated<Assignment, Token![,]>, Paren>,
    pub body: Box<Expr>,
}

impl Spannable for ExprLet {
    fn span(&self) -> Span {
        self.let_token.span().union(self.body.span())
    }
}

impl ExprLet {
    pub(crate) fn parser_with_expr<'a, I: Input<'a>>(
        expr: impl Parser<'a, I, Expr, ParserError<'a>> + Clone + 'a,
    ) -> impl Parser<'a, I, Self, ParserError<'a>> + Clone {
        let assignments = delimited_paren_with::<_, I>(punctuated_with::<_, _, I>(
            Assignment::parser_with_expr::<I>(expr.clone()),
            <Token![,]>::parser::<I>(),
        ));
        <Token![let]>::parser::<I>()
            .then(assignments)
            .then(expr)
            .map(|((let_token, assignments), body)| ExprLet {
                let_token,
                assignments,
                body: Box::new(body),
            })
            .labelled("ExprLet")
    }
}

impl<'a> Parseable<'a> for ExprLet {
    fn parser<I>() -> impl Parser<'a, I, Self, ParserError<'a>> + Clone
    where
        I: Input<'a>,
    {
        Expr::parser::<I>()
            .try_map(|expr, span| match expr {
                Expr::Let(l) => Ok(l),
                _ => Err(chumsky::error::Rich::custom(
                    span,
                    "expected let expression",
                )),
            })
            .labelled("ExprLet")
    }
}

/// An echo expression: `echo("x=", x) body_expr`.
///
/// Prints the arguments and evaluates to the body expression.
#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub struct ExprEcho {
    pub echo_token: Token![echo],
    pub args: Delimited<Punctuated<Arg, Token![,]>, Paren>,
    pub body: Box<Expr>,
}

impl Spannable for ExprEcho {
    fn span(&self) -> Span {
        self.echo_token.span().union(self.body.span())
    }
}

impl ExprEcho {
    pub(crate) fn parser_with_expr<'a, I: Input<'a>>(
        expr: impl Parser<'a, I, Expr, ParserError<'a>> + Clone + 'a,
    ) -> impl Parser<'a, I, Self, ParserError<'a>> + Clone {
        let args = delimited_paren_with::<_, I>(punctuated_with::<_, _, I>(
            Arg::parser_with_expr::<I>(expr.clone()),
            <Token![,]>::parser::<I>(),
        ));
        <Token![echo]>::parser::<I>()
            .then(args)
            .then(expr)
            .map(|((echo_token, args), body)| ExprEcho {
                echo_token,
                args,
                body: Box::new(body),
            })
            .labelled("ExprEcho")
    }
}

impl<'a> Parseable<'a> for ExprEcho {
    fn parser<I>() -> impl Parser<'a, I, Self, ParserError<'a>> + Clone
    where
        I: Input<'a>,
    {
        Expr::parser::<I>()
            .try_map(|expr, span| match expr {
                Expr::Echo(e) => Ok(e),
                _ => Err(chumsky::error::Rich::custom(
                    span,
                    "expected echo expression",
                )),
            })
            .labelled("ExprEcho")
    }
}

/// An assert expression: `assert(condition, message) body_expr`.
///
/// Checks the condition and evaluates to the body expression.
#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub struct ExprAssert {
    pub assert_token: Token![assert],
    pub args: Delimited<Punctuated<Arg, Token![,]>, Paren>,
    pub body: Box<Expr>,
}

impl Spannable for ExprAssert {
    fn span(&self) -> Span {
        self.assert_token.span().union(self.body.span())
    }
}

impl ExprAssert {
    pub(crate) fn parser_with_expr<'a, I: Input<'a>>(
        expr: impl Parser<'a, I, Expr, ParserError<'a>> + Clone + 'a,
    ) -> impl Parser<'a, I, Self, ParserError<'a>> + Clone {
        let args = delimited_paren_with::<_, I>(punctuated_with::<_, _, I>(
            Arg::parser_with_expr::<I>(expr.clone()),
            <Token![,]>::parser::<I>(),
        ));
        <Token![assert]>::parser::<I>()
            .then(args)
            .then(expr)
            .map(|((assert_token, args), body)| ExprAssert {
                assert_token,
                args,
                body: Box::new(body),
            })
            .labelled("ExprAssert")
    }
}

impl<'a> Parseable<'a> for ExprAssert {
    fn parser<I>() -> impl Parser<'a, I, Self, ParserError<'a>> + Clone
    where
        I: Input<'a>,
    {
        Expr::parser::<I>()
            .try_map(|expr, span| match expr {
                Expr::Assert(a) => Ok(a),
                _ => Err(chumsky::error::Rich::custom(
                    span,
                    "expected assert expression",
                )),
            })
            .labelled("ExprAssert")
    }
}

/// A list comprehension: `[for (i = [0:10]) i*2]`, `[each v]`, etc.
///
/// OpenSCAD list comprehensions use bracket syntax with one or more
/// comma-separated generator/expression elements: `[each a, each b, 6]`.
/// Pure-expression lists (`[1, 2, 3]`) parse as [`ExprVector`] instead —
/// this node is only produced when at least one element is a generator.
#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub struct ExprListComp {
    pub bracket: Bracket,
    pub bodies: Punctuated<ListCompBody, Token![,]>,
}

impl Spannable for ExprListComp {
    fn span(&self) -> Span {
        self.bracket.span()
    }
}

impl ExprListComp {
    pub(crate) fn parser_with_expr<'a, I: Input<'a>>(
        expr: impl Parser<'a, I, Expr, ParserError<'a>> + Clone + 'a,
    ) -> impl Parser<'a, I, Self, ParserError<'a>> + Clone {
        let comp_body = ListCompBody::parser_with_expr::<I>(expr);
        let bodies = punctuated_with::<_, _, I>(comp_body, <Token![,]>::parser::<I>());
        let open = just(Tokens::LBracket).to_span();
        let close = just(Tokens::RBracket).to_span();
        group((open, bodies, close))
            .try_map(|(open, bodies, close), span| {
                if bodies.is_empty() {
                    return Err(chumsky::error::Rich::custom(
                        span,
                        "expected at least one comprehension element",
                    ));
                }
                Ok(ExprListComp {
                    bracket: Bracket {
                        span: DelimSpan { open, close },
                    },
                    bodies,
                })
            })
            .labelled("ExprListComp")
    }
}

impl<'a> Parseable<'a> for ExprListComp {
    fn parser<I>() -> impl Parser<'a, I, Self, ParserError<'a>> + Clone
    where
        I: Input<'a>,
    {
        Expr::parser::<I>()
            .try_map(|expr, span| match expr {
                Expr::ListComp(lc) => Ok(lc),
                _ => Err(chumsky::error::Rich::custom(
                    span,
                    "expected list comprehension",
                )),
            })
            .labelled("ExprListComp")
    }
}

/// The body/generator of a list comprehension.
///
/// These can be nested: `[for (i = [0:10]) if (i > 5) i]` has a `For` containing
/// an `If` containing an `Expr`.
#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub enum ListCompBody {
    /// `for (assignments) body` — iterate and generate elements.
    For {
        for_token: Token![for],
        assignments: Delimited<Punctuated<Assignment, Token![,]>, Paren>,
        body: Box<ListCompBody>,
    },
    /// `if (condition) then_body` or `if (condition) then_body else else_body`.
    If {
        if_token: Token![if],
        condition: Delimited<Expr, Paren>,
        then_body: Box<ListCompBody>,
        else_body: Option<(Token![else], Box<ListCompBody>)>,
    },
    /// `let (assignments) body` — bind variables inside comprehension.
    Let {
        let_token: Token![let],
        assignments: Delimited<Punctuated<Assignment, Token![,]>, Paren>,
        body: Box<ListCompBody>,
    },
    /// `each expr` — flatten an expression into the parent list.
    Each {
        each_token: Token![each],
        expr: Box<Expr>,
    },
    /// A plain expression that produces a single element.
    Expr(Box<Expr>),
}

impl ListCompBody {
    pub(crate) fn parser_with_expr<'a, I: Input<'a>>(
        expr: impl Parser<'a, I, Expr, ParserError<'a>> + Clone + 'a,
    ) -> impl Parser<'a, I, Self, ParserError<'a>> + Clone {
        recursive(|comp_body| {
            let for_comp = group((
                <Token![for]>::parser::<I>(),
                delimited_paren_with::<_, I>(punctuated_with::<_, _, I>(
                    Assignment::parser_with_expr::<I>(expr.clone()),
                    <Token![,]>::parser::<I>(),
                )),
                comp_body.clone(),
            ))
            .map(|(for_token, assignments, body)| ListCompBody::For {
                for_token,
                assignments,
                body: Box::new(body),
            });

            let if_comp = group((
                <Token![if]>::parser::<I>(),
                delimited_paren_with::<_, I>(expr.clone()),
                comp_body.clone(),
                group((<Token![else]>::parser::<I>(), comp_body.clone()))
                    .map(|(else_token, body)| (else_token, Box::new(body)))
                    .or_not(),
            ))
            .map(
                |(if_token, condition, then_body, else_body)| ListCompBody::If {
                    if_token,
                    condition,
                    then_body: Box::new(then_body),
                    else_body,
                },
            );

            let let_comp = group((
                <Token![let]>::parser::<I>(),
                delimited_paren_with::<_, I>(punctuated_with::<_, _, I>(
                    Assignment::parser_with_expr::<I>(expr.clone()),
                    <Token![,]>::parser::<I>(),
                )),
                comp_body,
            ))
            .map(|(let_token, assignments, body)| ListCompBody::Let {
                let_token,
                assignments,
                body: Box::new(body),
            });

            let each_comp =
                group((<Token![each]>::parser::<I>(), expr.clone())).map(|(each_token, expr)| {
                    ListCompBody::Each {
                        each_token,
                        expr: Box::new(expr),
                    }
                });

            let expr_comp = expr.clone().map(|e| ListCompBody::Expr(Box::new(e)));

            choice((for_comp, if_comp, let_comp, each_comp, expr_comp)).labelled("ListCompBody")
        })
    }
}

impl Spannable for ListCompBody {
    fn span(&self) -> Span {
        match self {
            ListCompBody::For {
                for_token, body, ..
            } => for_token.span().union(body.span()),
            ListCompBody::If {
                if_token,
                then_body,
                else_body,
                ..
            } => {
                let end = else_body
                    .as_ref()
                    .map(|(_, b)| b.span())
                    .unwrap_or_else(|| then_body.span());
                if_token.span().union(end)
            }
            ListCompBody::Let {
                let_token, body, ..
            } => let_token.span().union(body.span()),
            ListCompBody::Each { each_token, expr } => each_token.span().union(expr.span()),
            ListCompBody::Expr(expr) => expr.span(),
        }
    }
}

#[cfg(test)]
mod tests {
    use core::time::Duration;

    use super::*;
    const DEFAULT_TIMEOUT: Duration = Duration::from_millis(100);

    #[track_caller]
    fn timeout(timeout: Duration, call: impl FnOnce() + Send + 'static) {
        better_panic::install();
        let handle = std::thread::spawn(call);
        std::thread::sleep(timeout);
        if !handle.is_finished() {
            panic!("Test timed out after {:?}", timeout);
        } else {
            handle.join().expect("Test panicked");
        }
    }

    #[track_caller]
    fn test_parse_expr(input: &str) -> Expr {
        use logos::Logos;
        let lexer = crate::lexer::Tokens::lexer(input);
        let tokens = lexer.collect::<Vec<_>>();
        println!("Tokens: {:#?}", tokens);
        let input = crate::lexer::Tokens::to_chumsky_input(input, 0);
        let parser = Expr::parser();
        let res = parser.parse(input).unwrap();
        dbg!(res)
    }

    #[test]
    pub fn parse_expr_fn_call() {
        timeout(DEFAULT_TIMEOUT, || {
            test_parse_expr("foo()");
        });
        timeout(DEFAULT_TIMEOUT, || {
            test_parse_expr("foo(1, 2, 3)");
        });
    }

    #[test]
    pub fn parse_expr_binary() {
        timeout(DEFAULT_TIMEOUT, || {
            test_parse_expr("1 + 2");
        });
    }

    #[test]
    pub fn parse_expr_assign() {
        timeout(DEFAULT_TIMEOUT, || {
            test_parse_expr("a = 5");
        });
    }

    #[test]
    pub fn parse_expr_nested() {
        timeout(DEFAULT_TIMEOUT, || {
            test_parse_expr("a = foo(1 + 2, 3 * 4)");
        });
    }

    #[test]
    pub fn parse_expr_precedence() {
        timeout(DEFAULT_TIMEOUT, || {
            test_parse_expr("1 + 2 * 3");
        });
    }

    #[test]
    pub fn parse_expr_unary_neg() {
        timeout(DEFAULT_TIMEOUT, || {
            // `-x` produces Expr::Unary(Neg) — the lexer emits Minus then Ident
            let expr = test_parse_expr("-x");
            assert!(
                matches!(
                    expr,
                    Expr::Unary(ExprUnary {
                        op: UnaryOp::Neg(_),
                        ..
                    })
                ),
                "expected Unary(Neg), got {expr:?}"
            );
        });
        timeout(DEFAULT_TIMEOUT, || {
            // `-1` lexes as Minus + Number so `x-1` stays a subtraction;
            // the parser folds it into Unary(Neg, Lit).
            let expr = test_parse_expr("-1");
            assert!(
                matches!(
                    expr,
                    Expr::Unary(ExprUnary {
                        op: UnaryOp::Neg(_),
                        ..
                    })
                ),
                "expected Unary(Neg) for -1, got {expr:?}"
            );
        });
        timeout(DEFAULT_TIMEOUT, || {
            // `x-1` without spaces must parse as a binary subtraction.
            let expr = test_parse_expr("x-1");
            assert!(
                matches!(
                    expr,
                    Expr::Binary(ExprBinary {
                        op: BinOp::Sub(_),
                        ..
                    })
                ),
                "expected Binary(Sub) for x-1, got {expr:?}"
            );
        });
    }

    #[test]
    pub fn parse_expr_unary_not() {
        timeout(DEFAULT_TIMEOUT, || {
            let expr = test_parse_expr("!x");
            assert!(matches!(
                expr,
                Expr::Unary(ExprUnary {
                    op: UnaryOp::Not(_),
                    ..
                })
            ));
        });
    }

    #[test]
    pub fn parse_expr_unary_precedence() {
        // -x + 2  should parse as  (-x) + 2,  not  -(x + 2)
        timeout(DEFAULT_TIMEOUT, || {
            let expr = test_parse_expr("-x + 2");
            assert!(
                matches!(
                    expr,
                    Expr::Binary(ExprBinary {
                        op: BinOp::Add(_),
                        ..
                    })
                ),
                "expected top-level Add, got {expr:?}"
            );
        });
    }

    #[test]
    pub fn parse_expr_grouped() {
        // (1 + 2) * 3  — grouped paren changes precedence
        timeout(DEFAULT_TIMEOUT, || {
            let expr = test_parse_expr("(1 + 2) * 3");
            assert!(
                matches!(
                    expr,
                    Expr::Binary(ExprBinary {
                        op: BinOp::Mul(_),
                        ..
                    })
                ),
                "expected top-level Mul, got {expr:?}"
            );
        });
    }

    #[test]
    pub fn parse_expr_comparison() {
        timeout(DEFAULT_TIMEOUT, || {
            test_parse_expr("a < b");
            test_parse_expr("a > b");
            test_parse_expr("a <= b");
            test_parse_expr("a >= b");
            test_parse_expr("a == b");
            test_parse_expr("a != b");
        });
    }

    #[test]
    pub fn parse_expr_logical() {
        timeout(DEFAULT_TIMEOUT, || {
            test_parse_expr("a && b");
            test_parse_expr("a || b");
            // &&  binds tighter than ||
            let expr = test_parse_expr("a || b && c");
            assert!(
                matches!(
                    expr,
                    Expr::Binary(ExprBinary {
                        op: BinOp::Or(_),
                        ..
                    })
                ),
                "expected top-level Or, got {expr:?}"
            );
        });
    }

    #[test]
    pub fn parse_expr_ternary() {
        // Simple ternary
        timeout(DEFAULT_TIMEOUT, || {
            let expr = test_parse_expr("a ? 1 : 2");
            assert!(
                matches!(expr, Expr::Ternary(_)),
                "expected Ternary, got {expr:?}"
            );
        });
    }

    #[test]
    pub fn parse_expr_ternary_with_binary_condition() {
        // Ternary with binary condition: binaries bind tighter than ?:
        timeout(DEFAULT_TIMEOUT, || {
            let expr = test_parse_expr("a > b ? 1 : 2");
            assert!(
                matches!(expr, Expr::Ternary(ExprTernary { ref condition, .. }) if matches!(**condition, Expr::Binary(ExprBinary { op: BinOp::Gt(_), .. }))),
                "expected Ternary with Gt condition, got {expr:?}"
            );
        });
    }

    #[test]
    pub fn parse_expr_ternary_nested() {
        // Nested ternary: a ? 1 : b ? 2 : 3  should be  a ? 1 : (b ? 2 : 3)
        // (right-associative)
        timeout(DEFAULT_TIMEOUT, || {
            let expr = test_parse_expr("a ? 1 : b ? 2 : 3");
            assert!(
                matches!(expr, Expr::Ternary(ExprTernary { ref if_false, .. }) if matches!(**if_false, Expr::Ternary(_))),
                "expected nested Ternary in if_false branch, got {expr:?}"
            );
        });
    }

    #[test]
    pub fn parse_expr_ternary_precedence() {
        // Assignment has lower precedence than ternary:
        // x = a ? 1 : 2  should parse as  x = (a ? 1 : 2)
        timeout(DEFAULT_TIMEOUT, || {
            let expr = test_parse_expr("x = a ? 1 : 2");
            assert!(
                matches!(expr, Expr::Assign(ExprAssign { ref right, .. }) if matches!(**right, Expr::Ternary(_))),
                "expected Assign with Ternary rhs, got {expr:?}"
            );
        });
    }

    #[test]
    pub fn parse_expr_index() {
        // Simple index access
        timeout(DEFAULT_TIMEOUT, || {
            let expr = test_parse_expr("curve[0]");
            assert!(
                matches!(expr, Expr::Index(_)),
                "expected Index, got {expr:?}"
            );
        });
    }

    #[test]
    pub fn parse_expr_index_expr() {
        // Index with an expression inside brackets
        timeout(DEFAULT_TIMEOUT, || {
            let expr = test_parse_expr("arr[i + 1]");
            assert!(
                matches!(expr, Expr::Index(ExprIndex { ref index, .. }) if matches!(**index, Expr::Binary(ExprBinary { op: BinOp::Add(_), .. }))),
                "expected Index with Add index expr, got {expr:?}"
            );
        });
    }

    #[test]
    pub fn parse_expr_index_chained() {
        // Chained index: matrix[0][1]  should parse as  (matrix[0])[1]
        timeout(DEFAULT_TIMEOUT, || {
            let expr = test_parse_expr("matrix[0][1]");
            assert!(
                matches!(expr, Expr::Index(ExprIndex { ref expr, .. }) if matches!(**expr, Expr::Index(_))),
                "expected nested Index, got {expr:?}"
            );
        });
    }

    #[test]
    pub fn parse_expr_index_after_call() {
        // Index after a function call: foo()[0]
        timeout(DEFAULT_TIMEOUT, || {
            let expr = test_parse_expr("foo()[0]");
            assert!(
                matches!(expr, Expr::Index(ExprIndex { ref expr, .. }) if matches!(**expr, Expr::Call(_))),
                "expected Index on Call, got {expr:?}"
            );
        });
    }

    #[test]
    pub fn parse_expr_index_precedence() {
        // Index binds tighter than binary: arr[0] + 1  should be  (arr[0]) + 1
        timeout(DEFAULT_TIMEOUT, || {
            let expr = test_parse_expr("arr[0] + 1");
            assert!(
                matches!(
                    expr,
                    Expr::Binary(ExprBinary {
                        op: BinOp::Add(_),
                        ref left,
                        ..
                    }) if matches!(**left, Expr::Index(_))
                ),
                "expected Add with Index lhs, got {expr:?}"
            );
        });
    }

    #[test]
    pub fn parse_expr_bool_true() {
        timeout(DEFAULT_TIMEOUT, || {
            let expr = test_parse_expr("true");
            assert!(
                matches!(expr, Expr::Lit(ExprLit { lit: Lit::Bool(ref b) }) if b.value()),
                "expected Lit(Bool(true)), got {expr:?}"
            );
        });
    }

    #[test]
    pub fn parse_expr_bool_false() {
        timeout(DEFAULT_TIMEOUT, || {
            let expr = test_parse_expr("false");
            assert!(
                matches!(expr, Expr::Lit(ExprLit { lit: Lit::Bool(ref b) }) if !b.value()),
                "expected Lit(Bool(false)), got {expr:?}"
            );
        });
    }

    #[test]
    pub fn parse_expr_undef() {
        timeout(DEFAULT_TIMEOUT, || {
            let expr = test_parse_expr("undef");
            assert!(
                matches!(expr, Expr::Lit(ExprLit { lit: Lit::Undef(_) })),
                "expected Lit(Undef), got {expr:?}"
            );
        });
    }

    #[test]
    pub fn parse_expr_bool_in_ternary() {
        // true ? 1 : 0 — boolean as ternary condition
        timeout(DEFAULT_TIMEOUT, || {
            let expr = test_parse_expr("true ? 1 : 0");
            assert!(
                matches!(expr, Expr::Ternary(ExprTernary { ref condition, .. }) if matches!(**condition, Expr::Lit(ExprLit { lit: Lit::Bool(_) }))),
                "expected Ternary with Bool condition, got {expr:?}"
            );
        });
    }

    #[test]
    pub fn parse_expr_special_var() {
        // $fn — special variable
        timeout(DEFAULT_TIMEOUT, || {
            let expr = test_parse_expr("$fn");
            assert!(
                matches!(expr, Expr::Ident(ref id) if id.name() == "$fn"),
                "expected Ident($fn), got {expr:?}"
            );
        });
    }

    #[test]
    pub fn parse_expr_special_var_assignment() {
        // $fn = 32 — assignment to special variable
        timeout(DEFAULT_TIMEOUT, || {
            let expr = test_parse_expr("$fn = 32");
            assert!(
                matches!(expr, Expr::Assign(ExprAssign { ref left, .. }) if matches!(**left, Expr::Ident(ref id) if id.name() == "$fn")),
                "expected Assign to $fn, got {expr:?}"
            );
        });
    }

    #[test]
    pub fn parse_expr_member_access() {
        // v.x — member access
        timeout(DEFAULT_TIMEOUT, || {
            let expr = test_parse_expr("v.x");
            assert!(
                matches!(expr, Expr::Member(ExprMember { ref member, .. }) if member.name() == "x"),
                "expected Member access .x, got {expr:?}"
            );
        });
    }

    #[test]
    pub fn parse_expr_member_chained() {
        // a.b.c — chained member access should be (a.b).c
        timeout(DEFAULT_TIMEOUT, || {
            let expr = test_parse_expr("a.b.c");
            assert!(
                matches!(expr, Expr::Member(ExprMember { ref expr, ref member, .. }) if member.name() == "c" && matches!(**expr, Expr::Member(_))),
                "expected chained Member access, got {expr:?}"
            );
        });
    }

    #[test]
    pub fn parse_expr_member_with_index() {
        // v.x[0] — member access then index
        timeout(DEFAULT_TIMEOUT, || {
            let expr = test_parse_expr("v.x[0]");
            assert!(
                matches!(expr, Expr::Index(ExprIndex { ref expr, .. }) if matches!(**expr, Expr::Member(_))),
                "expected Index on Member, got {expr:?}"
            );
        });
    }

    #[test]
    pub fn parse_expr_member_precedence() {
        // v.x + 1 — member binds tighter than binary
        timeout(DEFAULT_TIMEOUT, || {
            let expr = test_parse_expr("v.x + 1");
            assert!(
                matches!(
                    expr,
                    Expr::Binary(ExprBinary {
                        op: BinOp::Add(_),
                        ref left,
                        ..
                    }) if matches!(**left, Expr::Member(_))
                ),
                "expected Add with Member lhs, got {expr:?}"
            );
        });
    }

    #[test]
    pub fn parse_expr_vector_empty() {
        // [] — empty vector
        timeout(DEFAULT_TIMEOUT, || {
            let expr = test_parse_expr("[]");
            assert!(
                matches!(expr, Expr::Vector(ExprVector { ref elements, .. }) if elements.is_empty()),
                "expected empty Vector, got {expr:?}"
            );
        });
    }

    #[test]
    pub fn parse_expr_vector_simple() {
        // [1, 2, 3] — simple vector
        timeout(DEFAULT_TIMEOUT, || {
            let expr = test_parse_expr("[1, 2, 3]");
            assert!(
                matches!(expr, Expr::Vector(ExprVector { ref elements, .. }) if elements.len() == 3),
                "expected Vector with 3 elements, got {expr:?}"
            );
        });
    }

    #[test]
    pub fn parse_expr_vector_nested() {
        // [[1, 2], [3, 4]] — nested vectors
        timeout(DEFAULT_TIMEOUT, || {
            let expr = test_parse_expr("[[1, 2], [3, 4]]");
            assert!(
                matches!(expr, Expr::Vector(ExprVector { ref elements, .. }) if elements.len() == 2),
                "expected Vector with 2 elements, got {expr:?}"
            );
        });
    }

    #[test]
    pub fn parse_expr_vector_trailing_comma() {
        // [1, 2,] — trailing comma
        timeout(DEFAULT_TIMEOUT, || {
            let expr = test_parse_expr("[1, 2,]");
            assert!(
                matches!(expr, Expr::Vector(ExprVector { ref elements, .. }) if elements.len() == 2),
                "expected Vector with 2 elements (trailing comma), got {expr:?}"
            );
        });
    }

    #[test]
    pub fn parse_expr_range_two() {
        // [0:10] — two-operand range
        timeout(DEFAULT_TIMEOUT, || {
            let expr = test_parse_expr("[0:10]");
            assert!(
                matches!(expr, Expr::Range(ExprRange { ref step, .. }) if step.is_none()),
                "expected Range without step, got {expr:?}"
            );
        });
    }

    #[test]
    pub fn parse_expr_range_three() {
        // [0:0.5:10] — three-operand range
        timeout(DEFAULT_TIMEOUT, || {
            let expr = test_parse_expr("[0:0.5:10]");
            assert!(
                matches!(expr, Expr::Range(ExprRange { ref step, .. }) if step.is_some()),
                "expected Range with step, got {expr:?}"
            );
        });
    }

    #[test]
    pub fn parse_expr_range_with_exprs() {
        // [a:b+1] — range with expression operands
        timeout(DEFAULT_TIMEOUT, || {
            let expr = test_parse_expr("[a:b+1]");
            assert!(
                matches!(expr, Expr::Range(_)),
                "expected Range, got {expr:?}"
            );
        });
    }

    #[test]
    pub fn parse_expr_vector_index() {
        // [1,2,3][0] — index into a vector literal
        timeout(DEFAULT_TIMEOUT, || {
            let expr = test_parse_expr("[1,2,3][0]");
            assert!(
                matches!(expr, Expr::Index(ExprIndex { ref expr, .. }) if matches!(**expr, Expr::Vector(_))),
                "expected Index on Vector, got {expr:?}"
            );
        });
    }

    #[test]
    pub fn parse_expr_list_comp_for() {
        // [for (i = [0:10]) i] — basic for list comprehension
        timeout(DEFAULT_TIMEOUT, || {
            let expr = test_parse_expr("[for (i = [0:10]) i]");
            assert!(
                matches!(expr, Expr::ListComp(_)),
                "expected ListComp, got {expr:?}"
            );
        });
    }

    #[test]
    pub fn parse_expr_list_comp_trailing_comma() {
        // [for (i = [0:10]) i,] — list comprehension with trailing comma
        timeout(DEFAULT_TIMEOUT, || {
            let expr = test_parse_expr("[for (i = [0:10]) i,]");
            assert!(
                matches!(expr, Expr::ListComp(_)),
                "expected ListComp, got {expr:?}"
            );
        });
    }

    #[test]
    pub fn parse_expr_list_comp_if() {
        // [for (i = [0:10]) if (i > 5) i] — for with if filter
        timeout(DEFAULT_TIMEOUT, || {
            let expr = test_parse_expr("[for (i = [0:10]) if (i > 5) i]");
            assert!(
                matches!(expr, Expr::ListComp(_)),
                "expected ListComp, got {expr:?}"
            );
        });
    }

    #[test]
    pub fn parse_expr_list_comp_each() {
        // [each v] — each comprehension
        timeout(DEFAULT_TIMEOUT, || {
            let expr = test_parse_expr("[each v]");
            assert!(
                matches!(expr, Expr::ListComp(_)),
                "expected ListComp, got {expr:?}"
            );
        });
    }

    #[test]
    pub fn parse_expr_list_comp_let() {
        // [for (i = [0:5]) let (x = i * 2) x] — for with let binding
        timeout(DEFAULT_TIMEOUT, || {
            let expr = test_parse_expr("[for (i = [0:5]) let (x = i * 2) x]");
            assert!(
                matches!(expr, Expr::ListComp(_)),
                "expected ListComp, got {expr:?}"
            );
        });
    }

    #[test]
    pub fn parse_expr_let_simple() {
        // let (a = 1) a — simplest let expression
        timeout(DEFAULT_TIMEOUT, || {
            let expr = test_parse_expr("let (a = 1) a");
            assert!(
                matches!(expr, Expr::Let(ExprLet { ref assignments, ref body, .. })
                    if assignments.inner.len() == 1 && matches!(**body, Expr::Ident(_))),
                "expected Let with 1 assignment and Ident body, got {expr:?}"
            );
        });
    }

    #[test]
    pub fn parse_expr_let_multi_bindings() {
        // let (a = 1, b = 2) a + b — multiple bindings
        timeout(DEFAULT_TIMEOUT, || {
            let expr = test_parse_expr("let (a = 1, b = 2) a + b");
            assert!(
                matches!(expr, Expr::Let(ExprLet { ref assignments, ref body, .. })
                    if assignments.inner.len() == 2 && matches!(**body, Expr::Binary(_))),
                "expected Let with 2 assignments and Binary body, got {expr:?}"
            );
        });
    }

    #[test]
    pub fn parse_expr_let_nested() {
        // let (a = 1) let (b = 2) a + b — nested let
        timeout(DEFAULT_TIMEOUT, || {
            let expr = test_parse_expr("let (a = 1) let (b = 2) a + b");
            assert!(
                matches!(expr, Expr::Let(ExprLet { ref body, .. })
                    if matches!(**body, Expr::Let(_))),
                "expected nested Let, got {expr:?}"
            );
        });
    }

    #[test]
    pub fn parse_expr_let_with_call() {
        // let (r = 5) circle(r) — let with function call body
        timeout(DEFAULT_TIMEOUT, || {
            let expr = test_parse_expr("let (r = 5) circle(r)");
            assert!(
                matches!(expr, Expr::Let(ExprLet { ref body, .. })
                    if matches!(**body, Expr::Call(_))),
                "expected Let with Call body, got {expr:?}"
            );
        });
    }

    #[test]
    pub fn parse_expr_echo_simple() {
        // echo("hello") 1 — echo expression with body
        timeout(DEFAULT_TIMEOUT, || {
            let expr = test_parse_expr("echo(\"hello\") 1");
            assert!(
                matches!(expr, Expr::Echo(ExprEcho { ref args, ref body, .. })
                    if args.inner.len() == 1 && matches!(**body, Expr::Lit(_))),
                "expected Echo with 1 arg and Lit body, got {expr:?}"
            );
        });
    }

    #[test]
    pub fn parse_expr_echo_multi_args() {
        // echo("x=", x) x — echo with multiple args
        timeout(DEFAULT_TIMEOUT, || {
            let expr = test_parse_expr("echo(\"x=\", x) x");
            assert!(
                matches!(expr, Expr::Echo(ExprEcho { ref args, ref body, .. })
                    if args.inner.len() == 2 && matches!(**body, Expr::Ident(_))),
                "expected Echo with 2 args and Ident body, got {expr:?}"
            );
        });
    }

    #[test]
    pub fn parse_expr_assert_simple() {
        // assert(x > 0) x — simple assert expression
        timeout(DEFAULT_TIMEOUT, || {
            let expr = test_parse_expr("assert(x > 0) x");
            assert!(
                matches!(expr, Expr::Assert(ExprAssert { ref args, ref body, .. })
                    if args.inner.len() == 1 && matches!(**body, Expr::Ident(_))),
                "expected Assert with 1 arg and Ident body, got {expr:?}"
            );
        });
    }

    #[test]
    pub fn parse_expr_assert_with_message() {
        // assert(x > 0, "must be positive") x — assert with message
        timeout(DEFAULT_TIMEOUT, || {
            let expr = test_parse_expr("assert(x > 0, \"must be positive\") x");
            assert!(
                matches!(expr, Expr::Assert(ExprAssert { ref args, ref body, .. })
                    if args.inner.len() == 2 && matches!(**body, Expr::Ident(_))),
                "expected Assert with 2 args and Ident body, got {expr:?}"
            );
        });
    }

    #[test]
    pub fn parse_expr_echo_assert_chained() {
        // echo("checking") assert(x > 0) x — chained echo + assert
        timeout(DEFAULT_TIMEOUT, || {
            let expr = test_parse_expr("echo(\"checking\") assert(x > 0) x");
            assert!(
                matches!(expr, Expr::Echo(ExprEcho { ref body, .. })
                    if matches!(**body, Expr::Assert(_))),
                "expected Echo with Assert body, got {expr:?}"
            );
        });
    }
}
