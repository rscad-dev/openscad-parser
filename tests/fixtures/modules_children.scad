// User modules, default parameters, children().
module ring(r = 10, thickness = 1) {
    difference() {
        circle(r);
        circle(r - thickness);
    }
}

ring();
ring(5);
ring(r = 8, thickness = 0.5);

module spread(count = 3, gap = 5) {
    for (i = [0 : count - 1]) {
        translate([i * gap, 0, 0]) children();
    }
}

spread(count = 4) cube(2);
spread() { sphere(1); }

module pair() {
    children(0);
    translate([5, 0, 0]) children(1);
}

pair() {
    cube(1);
    sphere(1);
}

module all_kids() {
    echo("children:", $children);
    children([0 : $children - 1]);
}

all_kids() {
    cube(1);
    sphere(1);
    cylinder(h = 1, r = 1);
}

module outer() {
    module inner(s) {
        cube(s);
    }
    inner(2);
}

outer();
