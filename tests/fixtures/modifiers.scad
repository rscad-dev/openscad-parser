// Modifier characters on module instantiations.
difference() {
    cube(10, center = true);
    #sphere(6);
}

%translate([0, 0, 10]) cube(2, center = true);

*cylinder(h = 100, r = 1);

union() {
    cube(1);
    !translate([5, 0, 0]) sphere(2);
}

#translate([0, 0, 5]) rotate([45, 0, 0]) cube(2);
