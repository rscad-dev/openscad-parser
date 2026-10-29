// Loops, conditionals, let statements.
for (i = [0 : 4]) {
    translate([i * 3, 0, 0]) cube(2);
}

for (i = [0 : 2 : 10]) translate([i, 0, 0]) sphere(0.5);

for (x = [0 : 2], y = [0 : 2]) {
    translate([x * 5, y * 5, 0]) cylinder(h = 1, r = 1);
}

for (p = [[0, 0], [5, 2], [10, 8]]) {
    translate([p[0], p[1], 0]) cube(1);
}

count = 5;
if (count > 3) {
    cube(3);
} else {
    sphere(3);
}

if (count % 2 == 0) cube(1);
else if (count % 3 == 0) sphere(1);
else cylinder(h = 1, r = 0.5);

let (r = 4, h = r * 2) {
    cylinder(h = h, r = r);
}

intersection_for(n = [1 : 6]) {
    rotate([0, 0, n * 60]) translate([5, 0, 0]) circle(8);
}
