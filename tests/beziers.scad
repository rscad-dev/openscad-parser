// Cubic bezier sampling — exercises functions, list comprehensions and
// vector arithmetic together.
function bez1(p0, p1, t) = p0 * (1 - t) + p1 * t;
function bez2(p0, p1, p2, t) = bez1(bez1(p0, p1, t), bez1(p1, p2, t), t);
function bez3(p0, p1, p2, p3, t) = bez1(bez2(p0, p1, p2, t), bez2(p1, p2, p3, t), t);

function bezier(p0, p1, p2, p3, steps = 16) =
    [for (i = [0 : steps]) bez3(p0, p1, p2, p3, i / steps)];

curve = bezier([0, 0], [5, 15], [15, -5], [20, 10]);
base = [[20, -10], [0, -10]];

polygon(concat(curve, base));

echo("first point:", curve[0]);
echo("length:", len(curve));
