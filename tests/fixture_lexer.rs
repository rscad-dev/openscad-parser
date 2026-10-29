use logos::Logos as _;
use openscad_parser::prelude::Tokens;

macro_rules! fixture_lex_test {
    ($name:ident, $path:literal) => {
        #[test]
        fn $name() {
            let src = include_str!($path);
            let errors: Vec<_> = Tokens::lexer(src)
                .spanned()
                .filter_map(|(tok, span)| tok.err().map(|e| (e, span)))
                .collect();
            assert!(errors.is_empty(), "lexer errors in {}: {:?}", $path, errors);
        }
    };
}

fixture_lex_test!(lex_primitives, "fixtures/primitives.scad");
fixture_lex_test!(lex_transforms, "fixtures/transforms.scad");
fixture_lex_test!(lex_booleans, "fixtures/booleans.scad");
fixture_lex_test!(lex_modules_children, "fixtures/modules_children.scad");
fixture_lex_test!(lex_functions_recursion, "fixtures/functions_recursion.scad");
fixture_lex_test!(lex_control_flow, "fixtures/control_flow.scad");
fixture_lex_test!(lex_list_comprehensions, "fixtures/list_comprehensions.scad");
fixture_lex_test!(lex_modifiers, "fixtures/modifiers.scad");
fixture_lex_test!(lex_extrude_2d, "fixtures/extrude_2d.scad");
fixture_lex_test!(
    lex_numbers_strings_comments,
    "fixtures/numbers_strings_comments.scad"
);
fixture_lex_test!(lex_include_main, "fixtures/include_main.scad");
fixture_lex_test!(lex_include_lib, "fixtures/include_lib.scad");
fixture_lex_test!(lex_use_lib, "fixtures/use_lib.scad");
fixture_lex_test!(lex_beziers, "../tests/beziers.scad");

#[test]
fn lex_no_space_subtraction() {
    // `a-1` must produce Identifier, Minus, Number — not Identifier, Number(-1).
    let tokens: Vec<_> = Tokens::lexer("a-1")
        .map(|t| t.expect("lex error"))
        .collect();
    assert_eq!(
        tokens,
        vec![Tokens::Identifier("a"), Tokens::Minus, Tokens::Number(1.0)]
    );
}

#[test]
fn lex_number_forms() {
    for (src, expected) in [
        ("1", 1.0),
        ("1.5", 1.5),
        (".5", 0.5),
        ("1.", 1.0),
        ("2e3", 2000.0),
        ("1.5e-2", 0.015),
    ] {
        let tokens: Vec<_> = Tokens::lexer(src).map(|t| t.expect("lex error")).collect();
        assert_eq!(tokens, vec![Tokens::Number(expected)], "lexing {src:?}");
    }
}
