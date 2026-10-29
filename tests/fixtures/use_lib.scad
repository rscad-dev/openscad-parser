// Library pulled in via `use <>`: only module/function definitions are
// imported; this top-level geometry must NOT appear in the consumer.
function beam_r(len) = len / 40 + 0.5;

module beam(len = 10) {
    rotate([0, 90, 0]) cylinder(h = len, r = beam_r(len));
}

// Top-level geometry that `use` consumers must ignore.
beam(100);
