// 2D profiles and extrusions.
linear_extrude(height = 10) circle(5);
linear_extrude(height = 10, center = true, twist = 90, slices = 20, scale = 0.5) square([4, 2]);
linear_extrude(5) {
    difference() {
        circle(10);
        circle(8);
    }
}

rotate_extrude() translate([10, 0]) circle(2);
rotate_extrude(angle = 90, $fn = 64) polygon([[5, 0], [8, 0], [8, 4], [5, 4]]);

offset(r = 1) square(10);
offset(delta = -0.5) circle(5);

projection(cut = true) translate([0, 0, -1]) cube(4, center = true);

text("rscad", size = 8, font = "Liberation Sans");

polyhedron(
    points = [[0, 0, 0], [10, 0, 0], [10, 10, 0], [0, 10, 0], [5, 5, 10]],
    faces = [[0, 1, 4], [1, 2, 4], [2, 3, 4], [3, 0, 4], [3, 2, 1, 0]]
);
