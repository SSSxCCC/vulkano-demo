#![no_std]

use spirv_std::{
    glam::{vec3, vec4, Vec2, Vec3, Vec4},
    spirv,
};

#[spirv(vertex)]
#[no_mangle]
pub fn main_vs(
    #[spirv(vertex_index)] vert_id: i32,
    #[spirv(position, invariant)] out_pos: &mut Vec4,
    position: Vec2,
    color: &mut Vec3,
) {
    *out_pos = position.extend(0.0).extend(1.0);
    /**out_pos = vec4(
        (vert_id - 1) as f32,
        ((vert_id & 1) * 2 - 1) as f32,
        0.0,
        1.0,
    );*/

    if vert_id == 0 {
        *color = vec3(1.0, 0.0, 0.0);
    } else if vert_id == 1 {
        *color = vec3(0.0, 1.0, 0.0);
    } else if vert_id == 2 {
        *color = vec3(0.0, 0.0, 1.0);
    }
}

#[spirv(fragment)]
pub fn main_fs(output: &mut Vec4, color: Vec3) {
    //*output = vec4(1.0, 0.0, 0.0, 1.0);
    *output = color.extend(1.0);
}
