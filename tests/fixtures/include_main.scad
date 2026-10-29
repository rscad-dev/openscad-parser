// Multi-file directives. The referenced files live next to this one.
include <include_lib.scad>
use <use_lib.scad>

// Geometry that relies on the included/used definitions.
plate();
translate([0, 0, plate_h]) beam(20);
