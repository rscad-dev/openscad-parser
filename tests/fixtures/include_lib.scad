// Library pulled in via `include <>`: definitions AND top-level geometry
// become part of the including file.
plate_w = 30;
plate_h = 2;

module plate() {
    cube([plate_w, plate_w, plate_h]);
}

// Top-level geometry: `include` executes this, `use` would not.
translate([-plate_w, 0, 0]) plate();
