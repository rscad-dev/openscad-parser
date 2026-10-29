use crate::prelude::*;

#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub enum Item {
    Fn(ItemFn),
    Module(ItemModule),
    Include(ItemInclude),
    Use(ItemUse),
    Comment(ItemComment),
}

#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub enum ItemComment {
    LineComment(LineComment, Span),
    BlockComment(BlockComment, Span),
}

impl<'a> Parseable<'a> for ItemComment {
    fn parser<I>() -> impl Parser<'a, I, Self, ParserError<'a>> + Clone
    where
        I: Input<'a>,
    {
        choice((
            any()
                .filter(|t| matches!(t, Tokens::LineComment(_)))
                .map_with(|token, extra| {
                    if let Tokens::LineComment(s) = token {
                        ItemComment::LineComment(LineComment { content: s.into() }, extra.span())
                    } else {
                        unreachable!("This should never happen because of the filter")
                    }
                })
                .labelled("LineComment"),
            any()
                .filter(|t| matches!(t, Tokens::BlockComment(_)))
                .map_with(|token, extra| {
                    if let Tokens::BlockComment(s) = token {
                        ItemComment::BlockComment(BlockComment { content: s.into() }, extra.span())
                    } else {
                        unreachable!("This should never happen because of the filter")
                    }
                })
                .labelled("BlockComment"),
        ))
        .labelled("ItemComment")
    }
}

impl Spannable for ItemComment {
    fn span(&self) -> Span {
        match self {
            ItemComment::LineComment(_, span) | ItemComment::BlockComment(_, span) => *span,
        }
    }
}

#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub struct LineComment {
    content: Box<str>,
}

#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub struct BlockComment {
    content: Box<str>,
}

/// A parameter in a function or module definition, with optional default value.
///
/// Examples: `x`, `size = 10`, `$fn = 32`
#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub struct Param {
    pub name: Ident,
    pub default: Option<(Token![=], Expr)>,
}

impl Spannable for Param {
    fn span(&self) -> Span {
        match &self.default {
            Some((_, expr)) => self.name.span().union(expr.span()),
            None => self.name.span(),
        }
    }
}

impl<'a> Parseable<'a> for Param {
    fn parser<I>() -> impl Parser<'a, I, Self, ParserError<'a>> + Clone
    where
        I: Input<'a>,
    {
        Ident::parser::<I>()
            .then(
                <Token![=]>::parser::<I>()
                    .then(Expr::parser::<I>())
                    .or_not(),
            )
            .map(|(name, default)| Param { name, default })
            .labelled("Param")
    }
}

#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub struct ItemFn {
    function_token: Token![fn],
    name: Ident,
    params: Punctuated<Param, Token![,]>,
    eq_token: Token![=],
    body: Expr,
    semicolon: Token![;],
}

impl ItemFn {
    /// The function name.
    pub fn name(&self) -> &Ident {
        &self.name
    }

    /// The declared parameters.
    pub fn params(&self) -> &Punctuated<Param, Token![,]> {
        &self.params
    }

    /// The body expression (`function f(...) = <body>;`).
    pub fn body(&self) -> &Expr {
        &self.body
    }
}

impl Spannable for ItemFn {
    fn span(&self) -> Span {
        self.function_token.span().union(self.body.span())
    }
}

impl<'a> Parseable<'a> for ItemFn {
    fn parser<I>() -> impl Parser<'a, I, Self, ParserError<'a>> + Clone
    where
        I: Input<'a>,
    {
        group((
            <Token![fn]>::parser::<I>(),
            Ident::parser::<I>(),
            Delimited::<Punctuated<Param, Token![,]>, Paren>::parser::<I>(),
            <Token![=]>::parser::<I>(),
            Expr::parser::<I>(),
            <Token![;]>::parser::<I>(),
        ))
        .map(
            |(function_token, name, params, eq_token, body, semicolon)| Self {
                function_token,
                name,
                params: params.inner,
                eq_token,
                body,
                semicolon,
            },
        )
        .labelled("ItemFn")
    }
}

#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub struct ItemModule {
    module_token: Token![mod],
    name: Ident,
    params: Punctuated<Param, Token![,]>,
    body: Box<Stmt>,
}

impl ItemModule {
    /// The module name.
    pub fn name(&self) -> &Ident {
        &self.name
    }

    /// The declared parameters.
    pub fn params(&self) -> &Punctuated<Param, Token![,]> {
        &self.params
    }

    /// The module body: a brace block ([`Stmt::Block`]) or — matching
    /// OpenSCAD's `module_definition: MODULE ID '(' parameters ')' statement`
    /// grammar — any single statement.
    pub fn body(&self) -> &Stmt {
        &self.body
    }

    pub(crate) fn parser_with_stmt<'a, I: Input<'a>>(
        stmt: impl Parser<'a, I, Stmt, ParserError<'a>> + Clone + 'a,
    ) -> impl Parser<'a, I, Self, ParserError<'a>> + Clone {
        group((
            <Token![mod]>::parser::<I>(),
            Ident::parser::<I>(),
            Delimited::<Punctuated<Param, Token![,]>, Paren>::parser::<I>(),
            stmt.map(Box::new),
        ))
        .map(|(module_token, name, params, body)| Self {
            module_token,
            name,
            params: params.inner,
            body,
        })
        .labelled("ItemModuleWithStmt")
    }
}

impl Spannable for ItemModule {
    fn span(&self) -> Span {
        self.module_token.span().union(self.body.span())
    }
}

impl<'a> Parseable<'a> for ItemModule {
    fn parser<I>() -> impl Parser<'a, I, Self, ParserError<'a>> + Clone
    where
        I: Input<'a>,
    {
        ItemModule::parser_with_stmt::<I>(Stmt::parser::<I>()).labelled("ItemModule")
    }
}

#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub struct ItemInclude {
    include_token: Token![include],
    lt_token: Token![<],
    path: LitPath,
    gt_token: Token![>],
}

impl ItemInclude {
    /// The included path (`include <path>`).
    pub fn path(&self) -> &LitPath {
        &self.path
    }
}

impl Spannable for ItemInclude {
    fn span(&self) -> Span {
        self.include_token
            .span()
            .union(self.path.span())
            .union(self.gt_token.span())
    }
}

impl<'a> Parseable<'a> for ItemInclude {
    fn parser<I>() -> impl Parser<'a, I, Self, ParserError<'a>> + Clone
    where
        I: Input<'a>,
    {
        group((
            <Token![include]>::parser::<I>(),
            <Token![<]>::parser::<I>(),
            LitPath::parser::<I>(),
            <Token![>]>::parser::<I>(),
        ))
        .map(|(include_token, lt_token, path, gt_token)| Self {
            include_token,
            lt_token,
            path,
            gt_token,
        })
        .labelled("ItemInclude")
    }
}

#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub struct ItemUse {
    use_token: Token![use],
    lt_token: Token![<],
    path: LitPath,
    gt_token: Token![>],
}

impl ItemUse {
    /// The used path (`use <path>`).
    pub fn path(&self) -> &LitPath {
        &self.path
    }
}

impl Spannable for ItemUse {
    fn span(&self) -> Span {
        self.use_token
            .span()
            .union(self.path.span())
            .union(self.gt_token.span())
    }
}

impl<'a> Parseable<'a> for ItemUse {
    fn parser<I>() -> impl Parser<'a, I, Self, ParserError<'a>> + Clone
    where
        I: Input<'a>,
    {
        group((
            <Token![use]>::parser::<I>(),
            <Token![<]>::parser::<I>(),
            LitPath::parser::<I>(),
            <Token![>]>::parser::<I>(),
        ))
        .map(|(use_token, lt_token, path, gt_token)| Self {
            use_token,
            lt_token,
            path,
            gt_token,
        })
        .labelled("ItemUse")
    }
}

#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub struct Local {
    pub(crate) name: Ident,
    pub(crate) eq_token: Token![=],
    pub(crate) value: Expr,
    pub(crate) semicolon: Token![;],
}

impl Local {
    /// The assigned variable name.
    pub fn name(&self) -> &Ident {
        &self.name
    }

    /// The assigned value expression.
    pub fn value(&self) -> &Expr {
        &self.value
    }
}

#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub enum Stmt {
    Local(Local),
    Expr(Expr, Token![;]),
    Item(Item),
    If(StmtIf),
    For(StmtFor),
    /// A let statement scoping bindings over a child statement:
    /// `let (r = 4) { cylinder(r = r); }`
    Let(StmtLet),
    Block(Block),
    /// A module instantiation with a child statement body.
    ///
    /// Examples:
    /// - `translate([1,0,0]) cube();`
    /// - `difference() { cube(); sphere(); }`
    /// - `color("red") translate([1,0,0]) cube();`
    ModuleCall(StmtModuleCall),
    /// A standalone echo statement: `echo("x=", x);`
    Echo(StmtEcho),
    /// A standalone assert statement: `assert(x > 0, "must be positive");`
    Assert(StmtAssert),
}

/// Modifier characters that can prefix module instantiation statements.
///
/// - `*` — disable (ignore this subtree)
/// - `!` — show only (root modifier)
/// - `#` — debug/highlight (transparent)
/// - `%` — background (transparent gray)
#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub enum Modifier {
    Disable(Token![*]),
    ShowOnly(Token![!]),
    Debug(Token![#]),
    Background(Token![%]),
}

impl Spannable for Modifier {
    fn span(&self) -> Span {
        match self {
            Modifier::Disable(t) => t.span(),
            Modifier::ShowOnly(t) => t.span(),
            Modifier::Debug(t) => t.span(),
            Modifier::Background(t) => t.span(),
        }
    }
}

impl<'a> Parseable<'a> for Modifier {
    fn parser<I>() -> impl Parser<'a, I, Self, ParserError<'a>> + Clone
    where
        I: Input<'a>,
    {
        choice((
            <Token![*]>::parser::<I>().map(Modifier::Disable),
            <Token![!]>::parser::<I>().map(Modifier::ShowOnly),
            <Token![#]>::parser::<I>().map(Modifier::Debug),
            <Token![%]>::parser::<I>().map(Modifier::Background),
        ))
        .labelled("Modifier")
    }
}

/// A module instantiation with optional modifier and children.
///
/// This represents the OpenSCAD pattern of calling a module with an optional
/// child statement (or block) as its body — the core CSG pattern.
///
/// - `cube();` — call with no children (semicolon terminates)
/// - `difference() { cube(); sphere(); }` — call with a block child
/// - `translate([1,0,0]) cube();` — call with a single statement child
/// - `color("red") translate([1,0,0]) cube();` — chained calls
/// - `#cube();` — with a debug modifier
/// - `*difference() { ... }` — disabled subtree
#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub struct StmtModuleCall {
    pub modifier: Option<Modifier>,
    pub call: ExprCall,
    pub child: ModuleChild,
}

/// The child body of a module instantiation.
#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub enum ModuleChild {
    /// No child body, terminated by semicolon: `cube();`
    Empty(Token![;]),
    /// A child statement: `translate([1,0,0]) cube();` or `difference() { ... }`
    Stmt(Box<Stmt>),
}

impl Spannable for ModuleChild {
    fn span(&self) -> Span {
        match self {
            ModuleChild::Empty(sc) => sc.span(),
            ModuleChild::Stmt(stmt) => stmt.span(),
        }
    }
}

impl Spannable for StmtModuleCall {
    fn span(&self) -> Span {
        let start = self
            .modifier
            .as_ref()
            .map(|m| m.span())
            .unwrap_or_else(|| self.call.span());
        start.union(self.child.span())
    }
}

impl StmtModuleCall {
    pub(crate) fn parser_with_stmt<'a, I: Input<'a>>(
        stmt: impl Parser<'a, I, Stmt, ParserError<'a>> + Clone + 'a,
    ) -> impl Parser<'a, I, Self, ParserError<'a>> + Clone {
        let child = choice((
            <Token![;]>::parser::<I>().map(ModuleChild::Empty),
            stmt.map(Box::new).map(ModuleChild::Stmt),
        ));
        Modifier::parser::<I>()
            .or_not()
            .then(ExprCall::parser::<I>())
            .then(child)
            .map(|((modifier, call), child)| StmtModuleCall {
                modifier,
                call,
                child,
            })
            .labelled("StmtModuleCall")
    }
}

/// An if/else statement.
///
/// Examples:
/// - `if (x > 0) cube();`
/// - `if (x > 0) cube(); else sphere();`
/// - `if (x > 0) { cube(); sphere(); }`
#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub struct StmtIf {
    pub if_token: Token![if],
    pub condition: Delimited<Expr, Paren>,
    pub then_branch: Box<Stmt>,
    pub else_branch: Option<(Token![else], Box<Stmt>)>,
}

impl Spannable for StmtIf {
    fn span(&self) -> Span {
        let end = self
            .else_branch
            .as_ref()
            .map(|(_, stmt)| stmt.span())
            .unwrap_or_else(|| self.then_branch.span());
        self.if_token.span().union(end)
    }
}

impl StmtIf {
    pub(crate) fn parser_with_stmt<'a, I: Input<'a>>(
        stmt: impl Parser<'a, I, Stmt, ParserError<'a>> + Clone + 'a,
    ) -> impl Parser<'a, I, Self, ParserError<'a>> + Clone {
        <Token![if]>::parser::<I>()
            .then(Delimited::<Expr, Paren>::parser::<I>())
            .then(stmt.clone().map(Box::new))
            .then(
                <Token![else]>::parser::<I>()
                    .then(stmt.map(Box::new))
                    .or_not(),
            )
            .map(
                |(((if_token, condition), then_branch), else_branch)| StmtIf {
                    if_token,
                    condition,
                    then_branch,
                    else_branch,
                },
            )
            .labelled("StmtIf")
    }
}

/// A for loop statement.
///
/// Examples:
/// - `for (i = [0:10]) cube(i);`
/// - `for (i = [0:10], j = [0:10]) translate([i, j]) cube();`
#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub struct StmtFor {
    pub for_token: Token![for],
    pub assignments: Delimited<Punctuated<Assignment, Token![,]>, Paren>,
    pub body: Box<Stmt>,
}

/// A single assignment in a for loop: `i = [0:10]`.
#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub struct Assignment {
    pub name: Ident,
    pub eq_token: Token![=],
    pub value: Expr,
}

impl Spannable for Assignment {
    fn span(&self) -> Span {
        self.name.span().union(self.value.span())
    }
}

impl Assignment {
    pub(crate) fn parser_with_expr<'a, I: Input<'a>>(
        expr: impl Parser<'a, I, Expr, ParserError<'a>> + Clone + 'a,
    ) -> impl Parser<'a, I, Self, ParserError<'a>> + Clone {
        Ident::parser::<I>()
            .then(<Token![=]>::parser::<I>())
            .then(expr)
            .map(|((name, eq_token), value)| Assignment {
                name,
                eq_token,
                value,
            })
            .labelled("Assignment")
    }
}

impl<'a> Parseable<'a> for Assignment {
    fn parser<I>() -> impl Parser<'a, I, Self, ParserError<'a>> + Clone
    where
        I: Input<'a>,
    {
        Assignment::parser_with_expr::<I>(Expr::parser::<I>())
    }
}

impl Spannable for StmtFor {
    fn span(&self) -> Span {
        self.for_token.span().union(self.body.span())
    }
}

impl StmtFor {
    pub(crate) fn parser_with_stmt<'a, I: Input<'a>>(
        stmt: impl Parser<'a, I, Stmt, ParserError<'a>> + Clone + 'a,
    ) -> impl Parser<'a, I, Self, ParserError<'a>> + Clone {
        <Token![for]>::parser::<I>()
            .then(Delimited::<Punctuated<Assignment, Token![,]>, Paren>::parser::<I>())
            .then(stmt.map(Box::new))
            .map(|((for_token, assignments), body)| StmtFor {
                for_token,
                assignments,
                body,
            })
            .labelled("StmtFor")
    }
}

/// A let statement: `let (r = 4, h = 8) { cylinder(h = h, r = r); }`.
///
/// Binds variables in a fresh scope for the child statement. Unlike
/// [`crate::prelude::ExprLet`] this is a geometry statement (no trailing
/// semicolon required after a block child).
#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub struct StmtLet {
    pub let_token: Token![let],
    pub assignments: Delimited<Punctuated<Assignment, Token![,]>, Paren>,
    pub body: Box<Stmt>,
}

impl Spannable for StmtLet {
    fn span(&self) -> Span {
        self.let_token.span().union(self.body.span())
    }
}

impl StmtLet {
    pub(crate) fn parser_with_stmt<'a, I: Input<'a>>(
        stmt: impl Parser<'a, I, Stmt, ParserError<'a>> + Clone + 'a,
    ) -> impl Parser<'a, I, Self, ParserError<'a>> + Clone {
        <Token![let]>::parser::<I>()
            .then(Delimited::<Punctuated<Assignment, Token![,]>, Paren>::parser::<I>())
            .then(stmt.map(Box::new))
            .map(|((let_token, assignments), body)| StmtLet {
                let_token,
                assignments,
                body,
            })
            .labelled("StmtLet")
    }
}

/// A standalone echo statement: `echo("x=", x);`
///
/// Unlike `ExprEcho` (which has a body expression), this is a statement terminated
/// by a semicolon.
#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub struct StmtEcho {
    pub echo_token: Token![echo],
    pub args: Delimited<Punctuated<Arg, Token![,]>, Paren>,
    pub semicolon: Token![;],
}

impl Spannable for StmtEcho {
    fn span(&self) -> Span {
        self.echo_token.span().union(self.semicolon.span())
    }
}

/// A standalone assert statement: `assert(x > 0, "must be positive");`
///
/// Unlike `ExprAssert` (which has a body expression), this is a statement
/// terminated by a semicolon.
#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub struct StmtAssert {
    pub assert_token: Token![assert],
    pub args: Delimited<Punctuated<Arg, Token![,]>, Paren>,
    pub semicolon: Token![;],
}

impl Spannable for StmtAssert {
    fn span(&self) -> Span {
        self.assert_token.span().union(self.semicolon.span())
    }
}

#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub struct Block {
    pub(crate) brace_token: Brace,
    pub(crate) stmts: Vec<Stmt>,
}

impl Block {
    /// The statements inside the braces.
    pub fn stmts(&self) -> &[Stmt] {
        &self.stmts
    }

    /// Build a `Block` parser given an already-constructed `Stmt` parser.
    ///
    /// This avoids re-entering `Stmt::parser()` (and therefore `Block::parser()`)
    /// during parser *construction*, which would cause infinite recursion / stack
    /// overflow because of the mutual dependency:
    ///
    /// ```text
    /// Block::parser()
    ///   → Vec<Stmt>::parser()
    ///       → Stmt::parser()
    ///           → Item::parser()
    ///               → ItemModule::parser()
    ///                   → Block::parser()   ← infinite loop
    /// ```
    pub(crate) fn parser_with_stmt<'a, I: Input<'a>>(
        stmt: impl Parser<'a, I, Stmt, ParserError<'a>> + Clone + 'a,
    ) -> impl Parser<'a, I, Self, ParserError<'a>> + Clone {
        let open = just(Tokens::LBrace).to_span();
        let close = just(Tokens::RBrace).to_span();
        let stmts = stmt.repeated().collect::<Vec<_>>();
        group((open, stmts, close)).map(|(open, stmts, close)| Self {
            brace_token: Brace {
                span: DelimSpan { open, close },
            },
            stmts,
        })
    }
}

impl Spannable for Block {
    fn span(&self) -> Span {
        self.brace_token.span.join()
    }
}

impl<'a> Parseable<'a> for Block {
    fn parser<I>() -> impl Parser<'a, I, Self, ParserError<'a>> + Clone
    where
        I: Input<'a>,
    {
        // Delegate to the recursive Stmt parser to break the construction cycle.
        Block::parser_with_stmt(Stmt::parser::<I>()).labelled("Block")
    }
}

impl Spannable for Local {
    fn span(&self) -> Span {
        self.name.span().union(self.semicolon.span())
    }
}

impl<'a> Parseable<'a> for Local {
    fn parser<I>() -> impl Parser<'a, I, Self, ParserError<'a>> + Clone
    where
        I: Input<'a>,
    {
        Ident::parser::<I>()
            .then(<Token![=]>::parser::<I>())
            .then(Expr::parser::<I>())
            .then(<Token![;]>::parser::<I>())
            .map(|(((name, eq_token), value), semicolon)| Self {
                name,
                eq_token,
                value,
                semicolon,
            })
            .labelled("Local")
    }
}

impl Spannable for Stmt {
    fn span(&self) -> Span {
        match self {
            Stmt::Local(local) => local.span(),
            Stmt::Expr(expr, _) => expr.span(),
            Stmt::Item(item) => item.span(),
            Stmt::If(stmt_if) => stmt_if.span(),
            Stmt::For(stmt_for) => stmt_for.span(),
            Stmt::Let(stmt_let) => stmt_let.span(),
            Stmt::Block(block) => block.span(),
            Stmt::ModuleCall(mc) => mc.span(),
            Stmt::Echo(echo) => echo.span(),
            Stmt::Assert(assert_stmt) => assert_stmt.span(),
        }
    }
}

impl<'a> Parseable<'a> for Stmt {
    fn parser<I>() -> impl Parser<'a, I, Self, ParserError<'a>> + Clone
    where
        I: Input<'a>,
    {
        // `recursive` is required here because Stmt is mutually recursive with
        // Block (via Item → ItemModule → Block → Vec<Stmt> → Stmt).  We thread
        // the `stmt` handle through `Item::parser_with_stmt` and
        // `Block::parser_with_stmt` so that the `recursive` cell is actually
        // used and construction terminates.
        recursive(|stmt| {
            let local = Ident::parser::<I>()
                .then(<Token![=]>::parser::<I>())
                .then(Expr::parser::<I>())
                .then(<Token![;]>::parser::<I>())
                .map(|(((name, eq_token), value), semicolon)| {
                    Stmt::Local(Local {
                        name,
                        eq_token,
                        value,
                        semicolon,
                    })
                })
                .labelled("Local");
            let expr_stmt = Expr::parser::<I>()
                .then(<Token![;]>::parser::<I>())
                .map(|(expr, sc)| Stmt::Expr(expr, sc))
                .labelled("Expr");
            let if_stmt = StmtIf::parser_with_stmt::<I>(stmt.clone()).map(Stmt::If);
            let for_stmt = StmtFor::parser_with_stmt::<I>(stmt.clone()).map(Stmt::For);
            let let_stmt = StmtLet::parser_with_stmt::<I>(stmt.clone()).map(Stmt::Let);
            let block_stmt = Block::parser_with_stmt(stmt.clone()).map(Stmt::Block);
            let module_call =
                StmtModuleCall::parser_with_stmt::<I>(stmt.clone()).map(Stmt::ModuleCall);
            let echo_stmt = <Token![echo]>::parser::<I>()
                .then(Delimited::<Punctuated<Arg, Token![,]>, Paren>::parser::<I>())
                .then(<Token![;]>::parser::<I>())
                .map(|((echo_token, args), semicolon)| {
                    Stmt::Echo(StmtEcho {
                        echo_token,
                        args,
                        semicolon,
                    })
                })
                .labelled("StmtEcho");
            let assert_stmt = <Token![assert]>::parser::<I>()
                .then(Delimited::<Punctuated<Arg, Token![,]>, Paren>::parser::<I>())
                .then(<Token![;]>::parser::<I>())
                .map(|((assert_token, args), semicolon)| {
                    Stmt::Assert(StmtAssert {
                        assert_token,
                        args,
                        semicolon,
                    })
                })
                .labelled("StmtAssert");
            choice((
                local,
                if_stmt,
                for_stmt,
                let_stmt,
                echo_stmt,
                assert_stmt,
                block_stmt,
                module_call,
                Item::parser_with_stmt::<I>(stmt).map(Stmt::Item),
                expr_stmt,
            ))
        })
        .labelled("Stmt")
    }
}

impl Item {
    pub(crate) fn parser_with_stmt<'a, I: Input<'a>>(
        stmt: impl Parser<'a, I, Stmt, ParserError<'a>> + Clone + 'a,
    ) -> impl Parser<'a, I, Self, ParserError<'a>> + Clone {
        choice((
            ItemFn::parser::<I>().map(Item::Fn),
            ItemModule::parser_with_stmt::<I>(stmt).map(Item::Module),
            ItemInclude::parser::<I>().map(Item::Include),
            ItemUse::parser::<I>().map(Item::Use),
            ItemComment::parser::<I>().map(Item::Comment),
        ))
    }
}

impl Spannable for Item {
    fn span(&self) -> Span {
        match self {
            Item::Fn(func) => func.span(),
            Item::Module(module) => module.span(),
            Item::Include(include) => include.span(),
            Item::Use(use_) => use_.span(),
            Item::Comment(comment) => match comment {
                ItemComment::LineComment(_, span) | ItemComment::BlockComment(_, span) => *span,
            },
        }
    }
}

impl<'a> Parseable<'a> for Item {
    fn parser<I>() -> impl Parser<'a, I, Self, ParserError<'a>> + Clone
    where
        I: Input<'a>,
    {
        choice((
            ItemFn::parser::<I>().map(Item::Fn),
            ItemModule::parser::<I>().map(Item::Module),
            ItemInclude::parser::<I>().map(Item::Include),
            ItemUse::parser::<I>().map(Item::Use),
            ItemComment::parser::<I>().map(Item::Comment),
        ))
        .labelled("Item")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

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
    fn test_parse_item_fn(input: &str) -> ItemFn {
        use logos::Logos;
        let lexer = crate::lexer::Tokens::lexer(input);
        let tokens = lexer.collect::<Vec<_>>();
        println!("Tokens: {:#?}", tokens);
        let input = crate::lexer::Tokens::to_chumsky_input(input, 0);
        let parser = ItemFn::parser();
        let res = parser.parse(input).unwrap();
        dbg!(res)
    }

    #[track_caller]
    fn test_parse_item_module(input: &str) -> ItemModule {
        use logos::Logos;
        let lexer = crate::lexer::Tokens::lexer(input);
        let tokens = lexer.collect::<Vec<_>>();
        println!("Tokens: {:#?}", tokens);
        let input = crate::lexer::Tokens::to_chumsky_input(input, 0);
        let parser = ItemModule::parser();
        let res = parser.parse(input).unwrap();
        dbg!(res)
    }

    #[test]
    fn parse_fn_no_params() {
        timeout(DEFAULT_TIMEOUT, || {
            let f = test_parse_item_fn("function f() = 1;");
            assert_eq!(f.params.len(), 0);
        });
    }

    #[test]
    fn parse_fn_params_no_defaults() {
        timeout(DEFAULT_TIMEOUT, || {
            let f = test_parse_item_fn("function f(x, y, z) = x + y + z;");
            assert_eq!(f.params.len(), 3);
            for param in f.params.iter() {
                assert!(param.default.is_none(), "expected no default for param");
            }
        });
    }

    #[test]
    fn parse_fn_params_with_defaults() {
        timeout(DEFAULT_TIMEOUT, || {
            let f = test_parse_item_fn("function f(x, y = 10, z = 20) = x + y + z;");
            assert_eq!(f.params.len(), 3);
            assert!(f.params.iter().next().unwrap().default.is_none());
            assert!(f.params.iter().nth(1).unwrap().default.is_some());
            assert!(f.params.iter().nth(2).unwrap().default.is_some());
        });
    }

    #[test]
    fn parse_module_params_with_defaults() {
        timeout(DEFAULT_TIMEOUT, || {
            let m = test_parse_item_module("module m(size = 10, $fn = 32) { }");
            assert_eq!(m.params.len(), 2);
            assert!(m.params.iter().next().unwrap().default.is_some());
            assert!(m.params.iter().nth(1).unwrap().default.is_some());
            assert_eq!(m.params.iter().nth(1).unwrap().name.name(), "$fn");
        });
    }

    #[track_caller]
    fn test_parse_stmt(input: &str) -> Stmt {
        use logos::Logos;
        let lexer = crate::lexer::Tokens::lexer(input);
        let tokens = lexer.collect::<Vec<_>>();
        println!("Tokens: {:#?}", tokens);
        let input = crate::lexer::Tokens::to_chumsky_input(input, 0);
        let parser = Stmt::parser();
        let res = parser.parse(input).unwrap();
        dbg!(res)
    }

    #[test]
    fn parse_if_simple() {
        timeout(DEFAULT_TIMEOUT, || {
            let stmt = test_parse_stmt("if (x) y;");
            assert!(
                matches!(
                    stmt,
                    Stmt::If(StmtIf {
                        else_branch: None,
                        ..
                    })
                ),
                "expected If without else, got {stmt:?}"
            );
        });
    }

    #[test]
    fn parse_if_else() {
        timeout(DEFAULT_TIMEOUT, || {
            let stmt = test_parse_stmt("if (x) y; else z;");
            assert!(
                matches!(
                    stmt,
                    Stmt::If(StmtIf {
                        else_branch: Some(_),
                        ..
                    })
                ),
                "expected If with else, got {stmt:?}"
            );
        });
    }

    #[test]
    fn parse_if_block() {
        timeout(DEFAULT_TIMEOUT, || {
            let stmt = test_parse_stmt("if (x > 0) { a; b; }");
            assert!(matches!(stmt, Stmt::If(_)), "expected If, got {stmt:?}");
        });
    }

    #[test]
    fn parse_for_simple() {
        timeout(DEFAULT_TIMEOUT, || {
            let stmt = test_parse_stmt("for (i = [0:10]) x;");
            assert!(
                matches!(stmt, Stmt::For(StmtFor { ref assignments, .. }) if assignments.inner.len() == 1),
                "expected For with 1 assignment, got {stmt:?}"
            );
        });
    }

    #[test]
    fn parse_for_multi_assignment() {
        timeout(DEFAULT_TIMEOUT, || {
            let stmt = test_parse_stmt("for (i = [0:10], j = [0:5]) x;");
            assert!(
                matches!(stmt, Stmt::For(StmtFor { ref assignments, .. }) if assignments.inner.len() == 2),
                "expected For with 2 assignments, got {stmt:?}"
            );
        });
    }

    #[test]
    fn parse_module_call_with_block() {
        // difference() { cube(); sphere(); }
        timeout(DEFAULT_TIMEOUT, || {
            let stmt = test_parse_stmt("difference() { cube(); sphere(); }");
            assert!(
                matches!(&stmt, Stmt::ModuleCall(StmtModuleCall { call, child, .. })
                    if call.func.name() == "difference" && matches!(child, ModuleChild::Stmt(s) if matches!(&**s, Stmt::Block(_)))),
                "expected ModuleCall with block child, got {stmt:?}"
            );
        });
    }

    #[test]
    fn parse_module_call_chained() {
        // translate([1,0,0]) cube();
        timeout(DEFAULT_TIMEOUT, || {
            let stmt = test_parse_stmt("translate([1, 0, 0]) cube();");
            assert!(
                matches!(&stmt, Stmt::ModuleCall(StmtModuleCall { call, child, .. })
                    if call.func.name() == "translate" && matches!(child, ModuleChild::Stmt(s) if matches!(&**s, Stmt::ModuleCall(_)))),
                "expected ModuleCall(translate) with ModuleCall(cube) child, got {stmt:?}"
            );
        });
    }

    #[test]
    fn parse_module_call_nested() {
        // color("red") translate([1,0,0]) cube();
        timeout(DEFAULT_TIMEOUT, || {
            let stmt = test_parse_stmt("color(\"red\") translate([1, 0, 0]) cube();");
            assert!(
                matches!(&stmt, Stmt::ModuleCall(StmtModuleCall { call, child, .. })
                    if call.func.name() == "color" && matches!(child, ModuleChild::Stmt(s) if matches!(&**s, Stmt::ModuleCall(_)))),
                "expected nested ModuleCall, got {stmt:?}"
            );
        });
    }

    #[test]
    fn parse_module_call_simple() {
        // cube(); — now a ModuleCall with Empty child
        timeout(DEFAULT_TIMEOUT, || {
            let stmt = test_parse_stmt("cube();");
            assert!(
                matches!(
                    stmt,
                    Stmt::ModuleCall(StmtModuleCall {
                        modifier: None,
                        ref call,
                        child: ModuleChild::Empty(_),
                    }) if call.func.name() == "cube"
                ),
                "expected ModuleCall with Empty child, got {stmt:?}"
            );
        });
    }

    #[test]
    fn parse_modifier_debug() {
        // #cube(); — debug modifier on a module call
        timeout(DEFAULT_TIMEOUT, || {
            let stmt = test_parse_stmt("#cube();");
            assert!(
                matches!(
                    stmt,
                    Stmt::ModuleCall(StmtModuleCall {
                        modifier: Some(Modifier::Debug(_)),
                        ref call,
                        ..
                    }) if call.func.name() == "cube"
                ),
                "expected ModuleCall with Debug modifier, got {stmt:?}"
            );
        });
    }

    #[test]
    fn parse_modifier_disable() {
        // *difference() { cube(); sphere(); }
        timeout(DEFAULT_TIMEOUT, || {
            let stmt = test_parse_stmt("*difference() { cube(); sphere(); }");
            assert!(
                matches!(
                    stmt,
                    Stmt::ModuleCall(StmtModuleCall {
                        modifier: Some(Modifier::Disable(_)),
                        ..
                    })
                ),
                "expected ModuleCall with Disable modifier, got {stmt:?}"
            );
        });
    }

    #[test]
    fn parse_modifier_background() {
        // %cube();
        timeout(DEFAULT_TIMEOUT, || {
            let stmt = test_parse_stmt("%cube();");
            assert!(
                matches!(
                    stmt,
                    Stmt::ModuleCall(StmtModuleCall {
                        modifier: Some(Modifier::Background(_)),
                        ..
                    })
                ),
                "expected ModuleCall with Background modifier, got {stmt:?}"
            );
        });
    }

    #[test]
    fn parse_module_call_named_range_arg() {
        // intersection_for-style: named argument whose value is a range.
        timeout(DEFAULT_TIMEOUT, || {
            let stmt = test_parse_stmt("foo(n = [1 : 6]) circle(1);");
            assert!(
                matches!(&stmt, Stmt::ModuleCall(StmtModuleCall { call, .. })
                    if call.func.name() == "foo"),
                "expected ModuleCall with named range arg, got {stmt:?}"
            );
        });
        timeout(DEFAULT_TIMEOUT, || {
            let stmt = test_parse_stmt("intersection_for(n = [1 : 6]) { circle(8); }");
            assert!(
                matches!(&stmt, Stmt::ModuleCall(StmtModuleCall { call, .. })
                    if call.func.name() == "intersection_for"),
                "expected ModuleCall(intersection_for), got {stmt:?}"
            );
        });
    }

    #[test]
    fn parse_echo_stmt() {
        // echo("hello");
        timeout(DEFAULT_TIMEOUT, || {
            let stmt = test_parse_stmt("echo(\"hello\");");
            assert!(
                matches!(stmt, Stmt::Echo(StmtEcho { ref args, .. }) if args.inner.len() == 1),
                "expected Echo stmt with 1 arg, got {stmt:?}"
            );
        });
    }

    #[test]
    fn parse_echo_stmt_multi_args() {
        // echo("x=", 42);
        timeout(DEFAULT_TIMEOUT, || {
            let stmt = test_parse_stmt("echo(\"x=\", 42);");
            assert!(
                matches!(stmt, Stmt::Echo(StmtEcho { ref args, .. }) if args.inner.len() == 2),
                "expected Echo stmt with 2 args, got {stmt:?}"
            );
        });
    }

    #[test]
    fn parse_assert_stmt() {
        // assert(x > 0);
        timeout(DEFAULT_TIMEOUT, || {
            let stmt = test_parse_stmt("assert(x > 0);");
            assert!(
                matches!(stmt, Stmt::Assert(StmtAssert { ref args, .. }) if args.inner.len() == 1),
                "expected Assert stmt with 1 arg, got {stmt:?}"
            );
        });
    }

    #[test]
    fn parse_assert_stmt_with_message() {
        // assert(x > 0, "must be positive");
        timeout(DEFAULT_TIMEOUT, || {
            let stmt = test_parse_stmt("assert(x > 0, \"must be positive\");");
            assert!(
                matches!(stmt, Stmt::Assert(StmtAssert { ref args, .. }) if args.inner.len() == 2),
                "expected Assert stmt with 2 args, got {stmt:?}"
            );
        });
    }
}
