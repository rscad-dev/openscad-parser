// Basic 3D and 2D primitives with the common argument spellings.
$fn = 32;
$fa = 6;
$fs = 0.5;

cube(10);
cube([10, 20, 30]);
cube(size = [5, 5, 5], center = true);

sphere(5);
sphere(r = 5);
sphere(d = 10, $fn = 64);

cylinder(h = 10, r = 4);
cylinder(h = 10, r1 = 4, r2 = 0);
cylinder(h = 10, d1 = 8, d2 = 4, center = true);
cylinder(10, 4, 4);

circle(5);
circle(r = 2.5, $fn = 16);
circle(d = .5);

square(4);
square([4, 8], center = true);

polygon([[0, 0], [10, 0], [5, 8.66]]);
polygon(
    points = [[0, 0], [10, 0], [10, 10], [0, 10], [2, 2], [8, 2], [8, 8], [2, 8]],
    paths = [[0, 1, 2, 3], [4, 5, 6, 7]]
);
