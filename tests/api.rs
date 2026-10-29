//! Behaviour of the public API: source registration, diagnostics, and the
//! values downstream consumers read back out of the AST.

use std::path::{Path, PathBuf};

use openscad_parser::prelude::*;

#[track_caller]
fn parse(src: &str) -> File {
    let ast = Ast::parse_source("test.scad", src.to_string())
        .unwrap_or_else(|e| panic!("failed to parse {src:?}:\n{e}"));
    ast.get_file(0).expect("parsed file is cached").clone()
}

#[track_caller]
fn parse_err(src: &str) -> String {
    Ast::parse_source("test.scad", src.to_string())
        .expect_err("source must be rejected")
        .render()
}

#[track_caller]
fn single_stmt(src: &str) -> Stmt {
    let mut stmts = parse(src).stmts;
    assert_eq!(stmts.len(), 1, "expected one statement in {src:?}");
    stmts.remove(0)
}

#[track_caller]
fn local_value(src: &str) -> Expr {
    match single_stmt(src) {
        Stmt::Local(local) => local.value().clone(),
        other => panic!("expected an assignment, got {other:?}"),
    }
}

#[track_caller]
fn import_path(src: &str) -> String {
    match single_stmt(src) {
        Stmt::Item(Item::Include(i)) => i.path().value().to_string(),
        Stmt::Item(Item::Use(u)) => u.path().value().to_string(),
        other => panic!("expected include/use, got {other:?}"),
    }
}

// --- literals ---------------------------------------------------------------

#[test]
fn string_escapes_are_decoded() {
    let Expr::Lit(ExprLit { lit: Lit::Str(s) }) = local_value(r#"x = "q\"b\\s\nn\tt\rr\zz";"#)
    else {
        panic!("expected a string literal");
    };
    assert_eq!(s.value(), "q\"b\\s\nn\tt\rr\\zz");
    assert_eq!(s.raw(), r#"q\"b\\s\nn\tt\rr\zz"#);
}

#[test]
fn string_containing_comment_markers_is_not_a_comment() {
    let Expr::Lit(ExprLit { lit: Lit::Str(s) }) = local_value(r#"x = "http://a /* b */";"#) else {
        panic!("expected a string literal");
    };
    assert_eq!(s.value(), "http://a /* b */");
}

#[test]
fn number_literal_values() {
    for (src, expected) in [
        ("x = 42;", 42.0),
        ("x = 1.5e-2;", 0.015),
        ("x = .25;", 0.25),
    ] {
        let Expr::Lit(ExprLit { lit: Lit::Num(n) }) = local_value(src) else {
            panic!("expected a number literal in {src:?}");
        };
        assert_eq!(n.value(), expected, "value of {src:?}");
    }
}

#[test]
fn keyword_prefixed_identifiers_are_identifiers() {
    for name in [
        "modules",
        "iffy",
        "format",
        "useful",
        "included",
        "undefined",
    ] {
        match single_stmt(&format!("{name} = 1;")) {
            Stmt::Local(local) => assert_eq!(local.name().name(), name),
            other => panic!("{name:?} should be an assignment target, got {other:?}"),
        }
    }
}

#[test]
fn special_variables_keep_dollar_prefix() {
    let Expr::Ident(ident) = local_value("x = $fn;") else {
        panic!("expected an identifier");
    };
    assert_eq!(ident.name(), "$fn");
    assert!(ident.is_special());
    assert_eq!((ident.span().start, ident.span().end), (4, 7));

    let Expr::Ident(plain) = local_value("x = fn_;") else {
        panic!("expected an identifier");
    };
    assert!(!plain.is_special());
}

#[test]
fn literal_spans_cover_the_source_text() {
    for (src, lit) in [
        ("x = 1.5;", "1.5"),
        ("x = \"ab\";", "\"ab\""),
        ("x = true;", "true"),
        ("x = undef;", "undef"),
    ] {
        let span = local_value(src).span();
        assert_eq!(
            &src[span.start..span.end],
            lit,
            "span of literal in {src:?}"
        );
    }
}

// --- include / use ----------------------------------------------------------

#[test]
fn include_and_use_paths() {
    assert_eq!(import_path("include <lib/utils.scad>"), "lib/utils.scad");
    assert_eq!(
        import_path("use <../lib/my-lib.scad>"),
        "../lib/my-lib.scad"
    );
    assert_eq!(import_path("use <MCAD/stepper.scad>"), "MCAD/stepper.scad");
    assert_eq!(import_path("use <parts/12/x.scad>"), "parts/12/x.scad");
}

#[test]
fn include_path_span_covers_the_path() {
    let src = "include <lib/utils.scad>";
    let Stmt::Item(Item::Include(i)) = single_stmt(src) else {
        panic!("expected include");
    };
    let span = i.path().span();
    assert_eq!(&src[span.start..span.end], "lib/utils.scad");
}

#[test]
#[ignore = "bug: LitPath rebuilds the path from tokens, losing whitespace and number spelling"]
fn include_paths_preserve_source_text() {
    for path in [
        "01_base.scad",
        "a/2.scad",
        "lib/v1.0/x.scad",
        "my file.scad",
    ] {
        assert_eq!(import_path(&format!("include <{path}>")), path);
    }
}

#[test]
#[ignore = "bug: empty statement `;` (valid OpenSCAD) is rejected"]
fn empty_statements_are_accepted() {
    parse("include <a.scad>;\ncube();;");
}

// --- statements -------------------------------------------------------------

#[test]
fn comments_are_allowed_anywhere() {
    let file = parse("/* head */ cube([1, /* inner */ 2, // eol\n 3]); // tail");
    assert_eq!(file.stmts.len(), 1);
}

#[test]
fn show_only_modifier() {
    let Stmt::ModuleCall(call) = single_stmt("!cube();") else {
        panic!("expected a module call");
    };
    assert!(matches!(call.modifier, Some(Modifier::ShowOnly(_))));
    assert_eq!(call.call.func.name(), "cube");
}

#[test]
fn module_body_may_be_a_single_statement() {
    let Stmt::Item(Item::Module(m)) = single_stmt("module m() cube();") else {
        panic!("expected a module definition");
    };
    assert!(matches!(m.body(), Stmt::ModuleCall(_)));
}

#[test]
fn else_if_chains_nest_in_else_branch() {
    let Stmt::If(outer) = single_stmt("if (a) x(); else if (b) y(); else z();") else {
        panic!("expected an if statement");
    };
    let (_, else_branch) = outer.else_branch.expect("outer else");
    let Stmt::If(inner) = *else_branch else {
        panic!("else-if should nest an if statement");
    };
    assert!(
        inner.else_branch.is_some(),
        "final else belongs to inner if"
    );
}

#[test]
fn file_span_covers_first_to_last_statement() {
    let src = "  a = 1;\nb = 2;  ";
    let span = parse(src).span();
    assert_eq!((span.start, span.end), (2, src.trim_end().len()));
}

// --- diagnostics ------------------------------------------------------------

#[test]
fn lex_errors_are_reported_with_location() {
    let rendered = parse_err("cube(); @");
    assert!(rendered.contains("Unknown token: @"), "{rendered}");
    assert!(rendered.contains("test.scad:1:9"), "{rendered}");
}

#[test]
fn unterminated_string_is_rejected() {
    let rendered = parse_err("x = \"oops;");
    assert!(rendered.contains("Failed to lex input"), "{rendered}");
}

#[test]
fn parse_errors_point_at_offending_token() {
    let rendered = parse_err("a = 1;\nb = ;");
    assert!(rendered.contains("test.scad:2:5"), "{rendered}");
}

#[test]
fn errors_name_the_file_they_occurred_in() {
    let mut ast = Ast::new();
    ast.insert_source(PathBuf::from("good.scad"), "cube();".into());
    let bad = ast.insert_source(PathBuf::from("bad.scad"), "cube(".into());
    assert_eq!(bad, 1);
    ast.parse_file("good.scad").expect("good file parses");
    let rendered = ast
        .parse_file("bad.scad")
        .expect_err("bad file fails")
        .render();
    assert!(rendered.contains("bad.scad"), "{rendered}");
    assert!(!rendered.contains("good.scad"), "{rendered}");
}

// --- Ast cache --------------------------------------------------------------

#[test]
fn insert_source_assigns_stable_ids() {
    let mut ast = Ast::new();
    let a = ast.insert_source(PathBuf::from("a.scad"), "a = 1;".into());
    let b = ast.insert_source(PathBuf::from("b.scad"), "b = 2;".into());
    let a_again = ast.insert_source(PathBuf::from("a.scad"), "ignored".into());
    assert_eq!((a, b, a_again), (0, 1, 0));
    assert_eq!(ast.path_of(b), Some(Path::new("b.scad")));
    assert_eq!(ast.path_of(7), None);
    assert!(ast.get_file(a).is_none(), "registering must not parse");
}

#[test]
fn in_memory_sources_parse_without_touching_disk() {
    let mut ast = Ast::new();
    ast.insert_source(PathBuf::from("virtual/x.scad"), "x = 1;".into());
    let id = ast
        .parse_file("virtual/x.scad")
        .expect("parses from memory");
    assert_eq!(ast.get_file(id).map(|f| f.stmts.len()), Some(1));
}

#[test]
fn missing_file_is_an_io_error() {
    let err = Ast::parse_ast("does/not/exist.scad").expect_err("missing file");
    assert!(matches!(err, ParseError::Io { .. }));
    assert!(err.render().contains("does/not/exist.scad"));
    // `Display` and `Debug` (hit by `unwrap`) must show the same diagnostic.
    assert_eq!(err.to_string(), err.render());
    assert_eq!(format!("{err:?}"), err.render());
}
