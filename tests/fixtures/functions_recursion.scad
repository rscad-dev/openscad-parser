// Function definitions, recursion, ternary chains.
function fact(n) = n <= 1 ? 1 : n * fact(n - 1);
function fib(n) = n < 2 ? n : fib(n - 1) + fib(n - 2);
function clamp(x, lo = 0, hi = 1) = x < lo ? lo : x > hi ? hi : x;
function lerp(a, b, t) = a + (b - a) * t;
function mag2(v) = v[0] * v[0] + v[1] * v[1];

echo(fact(5));
echo(fib(10));
echo(clamp(1.5));
echo(lerp(0, 10, 0.25));
echo(mag2([3, 4]));

function deg_points(n, r) = [for (i = [0 : n - 1]) [r * cos(360 * i / n), r * sin(360 * i / n)]];

polygon(deg_points(6, 10));

cube(fact(3));
