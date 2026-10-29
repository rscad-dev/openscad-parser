/* Block comment at the top.
   Spans multiple lines. */
// Numeric literal forms.
a = 1;
b = 1.5;
c = .5;
d = 1.;
e = 2e3;
f = 1.5e-2;
g = -4;
h = a-1;      // no-space subtraction must lex as `a - 1`
i = -a;
j = 3--2;     // 3 - (-2)

// Strings with escapes.
s1 = "plain";
s2 = "with \"quotes\"";
s3 = "tab\there";
s4 = "line\nbreak";
s5 = "back\\slash";

echo(a, b, c, d, e, f, g, h, i, j);
echo(s1, s2, s3, s4, s5);

cube(/* inline */ [1, /* mid-vector */ 2, 3]); // trailing comment
sphere(
    // comment inside an argument list
    r = 2
);

translate([0, 0, h /* end-of-expression comment */]) cube(1);
