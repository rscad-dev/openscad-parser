// CSG boolean nesting.
union() {
    cube(10);
    translate([5, 5, 5]) sphere(4);
}

difference() {
    cube(10, center = true);
    sphere(6);
    cylinder(h = 20, r = 2, center = true);
}

intersection() {
    cube(10, center = true);
    sphere(6.5);
}

// Implicit union via a bare block statement.
{
    cube(1);
    translate([2, 0, 0]) cube(1);
}

difference() {
    union() {
        cube([20, 20, 5]);
        translate([10, 10, 5]) cylinder(h = 10, r = 5);
    }
    translate([10, 10, -1]) cylinder(h = 20, r = 2);
}

hull() {
    translate([0, 0, 0]) circle(2);
    translate([10, 0, 0]) circle(3);
}

minkowski() {
    cube([10, 10, 1]);
    cylinder(r = 2, h = 1);
}
