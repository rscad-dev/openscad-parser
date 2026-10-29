// Transform chains in the forms OpenSCAD users actually write.
translate([1, 0, 0]) cube(1);
translate([0, -2, 0.5]) sphere(1);

rotate(45) square(2);
rotate([90, 0, 0]) cylinder(h = 5, r = 1);
rotate(a = 30, v = [1, 1, 0]) cube(2);

scale(2) sphere(1);
scale([1, 2, 0.5]) cube(1);

mirror([1, 0, 0]) translate([5, 0, 0]) cube(1);

color("red") cube(1);
color("steelblue", 0.5) sphere(1);
color("#ff8800") cylinder(h = 2, r = 1);
color([0.2, 0.4, 0.8]) cube(1);
color([0.2, 0.4, 0.8, 0.5]) cube(1);

resize([10, 0, 0]) sphere(3);
multmatrix([[1, 0, 0, 5], [0, 1, 0, 0], [0, 0, 1, 0], [0, 0, 0, 1]]) cube(1);
