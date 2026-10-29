use chumsky::Parser;
use openscad_parser::prelude::*;

#[test]
fn parse_bezier_file() {
    let ast = Ast::parse_ast("tests/beziers.scad").expect("failed to parse beziers.scad");
    assert!(ast.get_file(0).is_some());
}

#[track_caller]
fn test_parse_str<'a, T: Parseable<'a> + std::fmt::Debug>(input: &'a str) -> T {
    let input = Tokens::to_chumsky_input(input, 0);
    let parser = T::parser();
    parser.parse(input).unwrap()
}

#[test]
fn parse_includes() {
    let input = r#"
include <constants.scad>
use <math.scad>
use <paths.scad>
use <transforms.scad>"#;
    let out: File = test_parse_str(input);
    assert_eq!(out.stmts.len(), 4);
}

#[test]
fn parse_empty() {
    let input = "";
    let out: File = test_parse_str(input);
    assert!(out.stmts.is_empty());
}

#[test]
fn parse_all_files() {
    let files = std::fs::read_dir("tests/fixtures").unwrap();
    let mut parsed = 0;
    let mut failures = Vec::new();
    for file in files {
        let file = file.unwrap();
        if file.file_type().unwrap().is_file() {
            let path = file.path();
            match Ast::parse_ast(&path) {
                Ok(ast) => {
                    assert!(
                        ast.get_file(0).is_some(),
                        "no parsed file for {}",
                        path.display()
                    );
                    parsed += 1;
                }
                Err(e) => failures.push(format!("{}:\n{}", path.display(), e.render())),
            }
        }
    }
    assert!(
        failures.is_empty(),
        "{} fixture(s) failed to parse:\n{}",
        failures.len(),
        failures.join("\n")
    );
    assert!(parsed >= 10, "expected the fixture corpus, found {parsed}");
}

#[test]
fn parse_source_reports_errors_as_string() {
    let err = Ast::parse_source("bad.scad", "cube(".to_string())
        .expect_err("unbalanced paren must fail to parse");
    let rendered = err.render();
    assert!(
        rendered.contains("bad.scad"),
        "diagnostic should name the file: {rendered}"
    );
}

#[test]
fn parse_file_is_cached() {
    let mut ast = Ast::new();
    let a = ast.parse_file("tests/beziers.scad").unwrap();
    let b = ast.parse_file("tests/beziers.scad").unwrap();
    assert_eq!(a, b);
    assert_eq!(ast.files.len(), 1);
}
