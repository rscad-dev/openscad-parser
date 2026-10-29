// List comprehension shapes.
squares = [for (i = [0 : 9]) i * i];
evens = [for (i = [0 : 20]) if (i % 2 == 0) i];
signs = [for (i = [-3 : 3]) if (i < 0) -1 else if (i > 0) 1 else 0];
scaled = [for (i = [0 : 5]) let (x = i * 2) x + 1];
flat = [each [1, 2, 3], each [4, 5], 6];
pairs = [for (x = [0 : 2], y = [0 : 2]) [x, y]];
nested = [for (row = [0 : 3]) [for (col = [0 : 3]) row * 4 + col]];
trailing = [for (i = [0 : 3]) i,];

echo(squares, evens, signs, scaled, flat, pairs, nested, trailing);

points = [for (a = [0 : 30 : 359]) [10 * cos(a), 10 * sin(a)]];
polygon(points);
