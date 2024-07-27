#![no_std]

use spirv_std::{
    glam::{vec4, Vec2, Vec4},
    spirv,
};

#[spirv(vertex)]
pub fn main_vs(#[spirv(position, invariant)] out_pos: &mut Vec4, position: Vec2) {
    *out_pos = position.extend(0.0).extend(1.0);
}

#[spirv(fragment)]
pub fn main_fs(output: &mut Vec4) {
    *output = vec4(1.0, 0.0, 0.0, 1.0);
}
