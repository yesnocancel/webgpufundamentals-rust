use wgpu_fun::{App, Frame, PointerEvent, RenderMode};

struct Aabb {
    min: [f32; 3],
    max: [f32; 3],
}

#[rustfmt::skip]
fn create_cube_vertices() -> (Vec<f32>, u32, Aabb) {
    let positions: Vec<f32> = vec![
        // left
        0.0, 0.0,  0.0,
        0.0, 0.0, -1.0,
        0.0, 1.0,  0.0,
        0.0, 1.0, -1.0,

        // right
        1.0, 0.0,  0.0,
        1.0, 0.0, -1.0,
        1.0, 1.0,  0.0,
        1.0, 1.0, -1.0,
    ];

    let indices: Vec<u32> = vec![
         0,  2,  1,    2,  3,  1,   // left
         4,  5,  6,    6,  5,  7,   // right
         0,  4,  2,    2,  4,  6,   // front
         1,  3,  5,    5,  3,  7,   // back
         0,  1,  4,    4,  1,  5,   // bottom
         2,  6,  3,    3,  6,  7,   // top
    ];

    let quad_colors: Vec<u8> = vec![
        200,  70, 120,  // left column front
         80,  70, 200,  // left column back
         70, 200, 210,  // top
        160, 160, 220,  // top rung right
         90, 130, 110,  // top rung bottom
        200, 200,  70,  // between top and middle rung
    ];

    let num_vertices = indices.len() as u32;
    let mut vertex_data = vec![0.0f32; indices.len() * 4]; // xyz + color
    for (i, index) in indices.iter().enumerate() {
        let position_ndx = (index * 3) as usize;
        let position = &positions[position_ndx..position_ndx + 3];
        vertex_data[i * 4..i * 4 + 3].copy_from_slice(position);

        let quad_ndx = (i / 6) * 3;
        let color = &quad_colors[quad_ndx..quad_ndx + 3];
        // set RGB in the first 3 bytes of the 4th float, set A to 255
        vertex_data[i * 4 + 3] = f32::from_ne_bytes([color[0], color[1], color[2], 255]);
    }

    (vertex_data, num_vertices, Aabb {
        min: [ 0.0,  0.0, -1.0],
        max: [ 1.0,  1.0,  0.0],
    })
}

mod vec3 {
    #![allow(dead_code)]

    pub fn create() -> [f32; 3] {
        [0.0; 3]
    }

    pub fn add(a: [f32; 3], b: [f32; 3]) -> [f32; 3] {
        let mut dst = [0.0; 3];

        dst[0] = a[0] + b[0];
        dst[1] = a[1] + b[1];
        dst[2] = a[2] + b[2];

        dst
    }

    pub fn transform_mat3(v: [f32; 3], m: &[f32; 16]) -> [f32; 3] {
        let mut dst = [0.0; 3];

        let x = v[0];
        let y = v[1];
        let z = v[2];

        dst[0] = x * m[0] + y * m[4] + z * m[8];
        dst[1] = x * m[1] + y * m[5] + z * m[9];
        dst[2] = x * m[2] + y * m[6] + z * m[10];

        dst
    }

    pub fn transform_mat4(v: [f32; 3], m: &[f32; 16]) -> [f32; 3] {
        let mut dst = [0.0; 3];

        let x = v[0];
        let y = v[1];
        let z = v[2];
        let w = m[3] * x + m[7] * y + m[11] * z + m[15];
        let w = if w == 0.0 { 1.0 } else { w }; // the JS version's `|| 1`

        dst[0] = (m[0] * x + m[4] * y + m[8] * z + m[12]) / w;
        dst[1] = (m[1] * x + m[5] * y + m[9] * z + m[13]) / w;
        dst[2] = (m[2] * x + m[6] * y + m[10] * z + m[14]) / w;

        dst
    }

    pub fn add_scaled(a: [f32; 3], b: [f32; 3], scale: f32) -> [f32; 3] {
        let mut dst = [0.0; 3];

        dst[0] = a[0] + b[0] * scale;
        dst[1] = a[1] + b[1] * scale;
        dst[2] = a[2] + b[2] * scale;

        dst
    }

    pub fn min(a: [f32; 3], b: [f32; 3]) -> [f32; 3] {
        let mut dst = [0.0; 3];

        dst[0] = a[0].min(b[0]);
        dst[1] = a[1].min(b[1]);
        dst[2] = a[2].min(b[2]);

        dst
    }

    pub fn max(a: [f32; 3], b: [f32; 3]) -> [f32; 3] {
        let mut dst = [0.0; 3];

        dst[0] = a[0].max(b[0]);
        dst[1] = a[1].max(b[1]);
        dst[2] = a[2].max(b[2]);

        dst
    }

    pub fn dot(a: [f32; 3], b: [f32; 3]) -> f32 {
        a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
    }

    pub fn distance(a: [f32; 3], b: [f32; 3]) -> f32 {
        let dx = a[0] - b[0];
        let dy = a[1] - b[1];
        let dz = a[2] - b[2];
        (dx * dx + dy * dy + dz * dz).sqrt()
    }

    pub fn cross(a: [f32; 3], b: [f32; 3]) -> [f32; 3] {
        let mut dst = [0.0; 3];

        let t0 = a[1] * b[2] - a[2] * b[1];
        let t1 = a[2] * b[0] - a[0] * b[2];
        let t2 = a[0] * b[1] - a[1] * b[0];

        dst[0] = t0;
        dst[1] = t1;
        dst[2] = t2;

        dst
    }

    pub fn subtract(a: [f32; 3], b: [f32; 3]) -> [f32; 3] {
        let mut dst = [0.0; 3];

        dst[0] = a[0] - b[0];
        dst[1] = a[1] - b[1];
        dst[2] = a[2] - b[2];

        dst
    }

    pub fn normalize(v: [f32; 3]) -> [f32; 3] {
        let mut dst = [0.0; 3];

        let length = (v[0] * v[0] + v[1] * v[1] + v[2] * v[2]).sqrt();
        // make sure we don't divide by 0.
        if length > 0.00001 {
            dst[0] = v[0] / length;
            dst[1] = v[1] / length;
            dst[2] = v[2] / length;
        } else {
            dst[0] = 0.0;
            dst[1] = 0.0;
            dst[2] = 0.0;
        }

        dst
    }

    pub fn get_translation(m: &[f32; 16]) -> [f32; 3] {
        let mut dst = [0.0; 3];

        dst[0] = m[12];
        dst[1] = m[13];
        dst[2] = m[14];

        dst
    }
}

mod m4 {
    #![allow(dead_code)]

    use super::vec3;

    pub fn projection(width: f32, height: f32, depth: f32) -> [f32; 16] {
        // Note: This matrix flips the Y axis so that 0 is at the top.
        ortho(0.0, width, height, 0.0, depth, -depth)
    }

    pub fn perspective(
        field_of_view_y_in_radians: f32,
        aspect: f32,
        z_near: f32,
        z_far: f32,
    ) -> [f32; 16] {
        let mut dst = [0.0; 16];

        let f = (std::f32::consts::PI * 0.5 - 0.5 * field_of_view_y_in_radians).tan();
        let range_inv = 1.0 / (z_near - z_far);

        dst[0] = f / aspect;
        dst[1] = 0.0;
        dst[2] = 0.0;
        dst[3] = 0.0;

        dst[4] = 0.0;
        dst[5] = f;
        dst[6] = 0.0;
        dst[7] = 0.0;

        dst[8] = 0.0;
        dst[9] = 0.0;
        dst[10] = z_far * range_inv;
        dst[11] = -1.0;

        dst[12] = 0.0;
        dst[13] = 0.0;
        dst[14] = z_near * z_far * range_inv;
        dst[15] = 0.0;

        dst
    }

    pub fn ortho(left: f32, right: f32, bottom: f32, top: f32, near: f32, far: f32) -> [f32; 16] {
        let mut dst = [0.0; 16];

        dst[0] = 2.0 / (right - left);
        dst[1] = 0.0;
        dst[2] = 0.0;
        dst[3] = 0.0;

        dst[4] = 0.0;
        dst[5] = 2.0 / (top - bottom);
        dst[6] = 0.0;
        dst[7] = 0.0;

        dst[8] = 0.0;
        dst[9] = 0.0;
        dst[10] = 1.0 / (near - far);
        dst[11] = 0.0;

        dst[12] = (right + left) / (left - right);
        dst[13] = (top + bottom) / (bottom - top);
        dst[14] = near / (near - far);
        dst[15] = 1.0;

        dst
    }

    #[rustfmt::skip]
    pub fn identity() -> [f32; 16] {
        let mut dst = [0.0; 16];
        dst[ 0] = 1.0;  dst[ 1] = 0.0;  dst[ 2] = 0.0;  dst[ 3] = 0.0;
        dst[ 4] = 0.0;  dst[ 5] = 1.0;  dst[ 6] = 0.0;  dst[ 7] = 0.0;
        dst[ 8] = 0.0;  dst[ 9] = 0.0;  dst[10] = 1.0;  dst[11] = 0.0;
        dst[12] = 0.0;  dst[13] = 0.0;  dst[14] = 0.0;  dst[15] = 1.0;
        dst
    }

    pub fn multiply(a: &[f32; 16], b: &[f32; 16]) -> [f32; 16] {
        let mut dst = [0.0; 16];
        let b00 = b[0 * 4 + 0];
        let b01 = b[0 * 4 + 1];
        let b02 = b[0 * 4 + 2];
        let b03 = b[0 * 4 + 3];
        let b10 = b[1 * 4 + 0];
        let b11 = b[1 * 4 + 1];
        let b12 = b[1 * 4 + 2];
        let b13 = b[1 * 4 + 3];
        let b20 = b[2 * 4 + 0];
        let b21 = b[2 * 4 + 1];
        let b22 = b[2 * 4 + 2];
        let b23 = b[2 * 4 + 3];
        let b30 = b[3 * 4 + 0];
        let b31 = b[3 * 4 + 1];
        let b32 = b[3 * 4 + 2];
        let b33 = b[3 * 4 + 3];
        let a00 = a[0 * 4 + 0];
        let a01 = a[0 * 4 + 1];
        let a02 = a[0 * 4 + 2];
        let a03 = a[0 * 4 + 3];
        let a10 = a[1 * 4 + 0];
        let a11 = a[1 * 4 + 1];
        let a12 = a[1 * 4 + 2];
        let a13 = a[1 * 4 + 3];
        let a20 = a[2 * 4 + 0];
        let a21 = a[2 * 4 + 1];
        let a22 = a[2 * 4 + 2];
        let a23 = a[2 * 4 + 3];
        let a30 = a[3 * 4 + 0];
        let a31 = a[3 * 4 + 1];
        let a32 = a[3 * 4 + 2];
        let a33 = a[3 * 4 + 3];

        dst[0] = b00 * a00 + b01 * a10 + b02 * a20 + b03 * a30;
        dst[1] = b00 * a01 + b01 * a11 + b02 * a21 + b03 * a31;
        dst[2] = b00 * a02 + b01 * a12 + b02 * a22 + b03 * a32;
        dst[3] = b00 * a03 + b01 * a13 + b02 * a23 + b03 * a33;

        dst[4] = b10 * a00 + b11 * a10 + b12 * a20 + b13 * a30;
        dst[5] = b10 * a01 + b11 * a11 + b12 * a21 + b13 * a31;
        dst[6] = b10 * a02 + b11 * a12 + b12 * a22 + b13 * a32;
        dst[7] = b10 * a03 + b11 * a13 + b12 * a23 + b13 * a33;

        dst[8] = b20 * a00 + b21 * a10 + b22 * a20 + b23 * a30;
        dst[9] = b20 * a01 + b21 * a11 + b22 * a21 + b23 * a31;
        dst[10] = b20 * a02 + b21 * a12 + b22 * a22 + b23 * a32;
        dst[11] = b20 * a03 + b21 * a13 + b22 * a23 + b23 * a33;

        dst[12] = b30 * a00 + b31 * a10 + b32 * a20 + b33 * a30;
        dst[13] = b30 * a01 + b31 * a11 + b32 * a21 + b33 * a31;
        dst[14] = b30 * a02 + b31 * a12 + b32 * a22 + b33 * a32;
        dst[15] = b30 * a03 + b31 * a13 + b32 * a23 + b33 * a33;

        dst
    }

    pub fn inverse(m: &[f32; 16]) -> [f32; 16] {
        let mut dst = [0.0; 16];

        let m00 = m[0 * 4 + 0];
        let m01 = m[0 * 4 + 1];
        let m02 = m[0 * 4 + 2];
        let m03 = m[0 * 4 + 3];
        let m10 = m[1 * 4 + 0];
        let m11 = m[1 * 4 + 1];
        let m12 = m[1 * 4 + 2];
        let m13 = m[1 * 4 + 3];
        let m20 = m[2 * 4 + 0];
        let m21 = m[2 * 4 + 1];
        let m22 = m[2 * 4 + 2];
        let m23 = m[2 * 4 + 3];
        let m30 = m[3 * 4 + 0];
        let m31 = m[3 * 4 + 1];
        let m32 = m[3 * 4 + 2];
        let m33 = m[3 * 4 + 3];

        let tmp0 = m22 * m33;
        let tmp1 = m32 * m23;
        let tmp2 = m12 * m33;
        let tmp3 = m32 * m13;
        let tmp4 = m12 * m23;
        let tmp5 = m22 * m13;
        let tmp6 = m02 * m33;
        let tmp7 = m32 * m03;
        let tmp8 = m02 * m23;
        let tmp9 = m22 * m03;
        let tmp10 = m02 * m13;
        let tmp11 = m12 * m03;
        let tmp12 = m20 * m31;
        let tmp13 = m30 * m21;
        let tmp14 = m10 * m31;
        let tmp15 = m30 * m11;
        let tmp16 = m10 * m21;
        let tmp17 = m20 * m11;
        let tmp18 = m00 * m31;
        let tmp19 = m30 * m01;
        let tmp20 = m00 * m21;
        let tmp21 = m20 * m01;
        let tmp22 = m00 * m11;
        let tmp23 = m10 * m01;

        let t0 = (tmp0 * m11 + tmp3 * m21 + tmp4 * m31) - (tmp1 * m11 + tmp2 * m21 + tmp5 * m31);
        let t1 = (tmp1 * m01 + tmp6 * m21 + tmp9 * m31) - (tmp0 * m01 + tmp7 * m21 + tmp8 * m31);
        let t2 = (tmp2 * m01 + tmp7 * m11 + tmp10 * m31) - (tmp3 * m01 + tmp6 * m11 + tmp11 * m31);
        let t3 = (tmp5 * m01 + tmp8 * m11 + tmp11 * m21) - (tmp4 * m01 + tmp9 * m11 + tmp10 * m21);

        let d = 1.0 / (m00 * t0 + m10 * t1 + m20 * t2 + m30 * t3);

        dst[0] = d * t0;
        dst[1] = d * t1;
        dst[2] = d * t2;
        dst[3] = d * t3;

        dst[4] =
            d * ((tmp1 * m10 + tmp2 * m20 + tmp5 * m30) - (tmp0 * m10 + tmp3 * m20 + tmp4 * m30));
        dst[5] =
            d * ((tmp0 * m00 + tmp7 * m20 + tmp8 * m30) - (tmp1 * m00 + tmp6 * m20 + tmp9 * m30));
        dst[6] =
            d * ((tmp3 * m00 + tmp6 * m10 + tmp11 * m30) - (tmp2 * m00 + tmp7 * m10 + tmp10 * m30));
        dst[7] =
            d * ((tmp4 * m00 + tmp9 * m10 + tmp10 * m20) - (tmp5 * m00 + tmp8 * m10 + tmp11 * m20));

        dst[8] = d
            * ((tmp12 * m13 + tmp15 * m23 + tmp16 * m33)
                - (tmp13 * m13 + tmp14 * m23 + tmp17 * m33));
        dst[9] = d
            * ((tmp13 * m03 + tmp18 * m23 + tmp21 * m33)
                - (tmp12 * m03 + tmp19 * m23 + tmp20 * m33));
        dst[10] = d
            * ((tmp14 * m03 + tmp19 * m13 + tmp22 * m33)
                - (tmp15 * m03 + tmp18 * m13 + tmp23 * m33));
        dst[11] = d
            * ((tmp17 * m03 + tmp20 * m13 + tmp23 * m23)
                - (tmp16 * m03 + tmp21 * m13 + tmp22 * m23));

        dst[12] = d
            * ((tmp14 * m22 + tmp17 * m32 + tmp13 * m12)
                - (tmp16 * m32 + tmp12 * m12 + tmp15 * m22));
        dst[13] = d
            * ((tmp20 * m32 + tmp12 * m02 + tmp19 * m22)
                - (tmp18 * m22 + tmp21 * m32 + tmp13 * m02));
        dst[14] = d
            * ((tmp18 * m12 + tmp23 * m32 + tmp15 * m02)
                - (tmp22 * m32 + tmp14 * m02 + tmp19 * m12));
        dst[15] = d
            * ((tmp22 * m22 + tmp16 * m02 + tmp21 * m12)
                - (tmp20 * m12 + tmp23 * m22 + tmp17 * m02));

        dst
    }

    #[rustfmt::skip]
    pub fn aim(eye: [f32; 3], target: [f32; 3], up: [f32; 3]) -> [f32; 16] {
        let mut dst = [0.0; 16];

        let z_axis = vec3::normalize(vec3::subtract(target, eye));
        let x_axis = vec3::normalize(vec3::cross(up, z_axis));
        let y_axis = vec3::normalize(vec3::cross(z_axis, x_axis));

        dst[ 0] = x_axis[0];  dst[ 1] = x_axis[1];  dst[ 2] = x_axis[2];  dst[ 3] = 0.0;
        dst[ 4] = y_axis[0];  dst[ 5] = y_axis[1];  dst[ 6] = y_axis[2];  dst[ 7] = 0.0;
        dst[ 8] = z_axis[0];  dst[ 9] = z_axis[1];  dst[10] = z_axis[2];  dst[11] = 0.0;
        dst[12] = eye[0];     dst[13] = eye[1];     dst[14] = eye[2];     dst[15] = 1.0;

        dst
    }

    #[rustfmt::skip]
    pub fn camera_aim(eye: [f32; 3], target: [f32; 3], up: [f32; 3]) -> [f32; 16] {
        let mut dst = [0.0; 16];

        let z_axis = vec3::normalize(vec3::subtract(eye, target));
        let x_axis = vec3::normalize(vec3::cross(up, z_axis));
        let y_axis = vec3::normalize(vec3::cross(z_axis, x_axis));

        dst[ 0] = x_axis[0];  dst[ 1] = x_axis[1];  dst[ 2] = x_axis[2];  dst[ 3] = 0.0;
        dst[ 4] = y_axis[0];  dst[ 5] = y_axis[1];  dst[ 6] = y_axis[2];  dst[ 7] = 0.0;
        dst[ 8] = z_axis[0];  dst[ 9] = z_axis[1];  dst[10] = z_axis[2];  dst[11] = 0.0;
        dst[12] = eye[0];     dst[13] = eye[1];     dst[14] = eye[2];     dst[15] = 1.0;

        dst
    }

    pub fn look_at(eye: [f32; 3], target: [f32; 3], up: [f32; 3]) -> [f32; 16] {
        inverse(&camera_aim(eye, target, up))
    }

    #[rustfmt::skip]
    pub fn translation([tx, ty, tz]: [f32; 3]) -> [f32; 16] {
        let mut dst = [0.0; 16];
        dst[ 0] = 1.0;  dst[ 1] = 0.0;  dst[ 2] = 0.0;  dst[ 3] = 0.0;
        dst[ 4] = 0.0;  dst[ 5] = 1.0;  dst[ 6] = 0.0;  dst[ 7] = 0.0;
        dst[ 8] = 0.0;  dst[ 9] = 0.0;  dst[10] = 1.0;  dst[11] = 0.0;
        dst[12] = tx;   dst[13] = ty;   dst[14] = tz;   dst[15] = 1.0;
        dst
    }

    #[rustfmt::skip]
    pub fn rotation_x(angle_in_radians: f32) -> [f32; 16] {
        let c = angle_in_radians.cos();
        let s = angle_in_radians.sin();
        let mut dst = [0.0; 16];
        dst[ 0] = 1.0;  dst[ 1] = 0.0;  dst[ 2] = 0.0;  dst[ 3] = 0.0;
        dst[ 4] = 0.0;  dst[ 5] = c;    dst[ 6] = s;    dst[ 7] = 0.0;
        dst[ 8] = 0.0;  dst[ 9] = -s;   dst[10] = c;    dst[11] = 0.0;
        dst[12] = 0.0;  dst[13] = 0.0;  dst[14] = 0.0;  dst[15] = 1.0;
        dst
    }

    #[rustfmt::skip]
    pub fn rotation_y(angle_in_radians: f32) -> [f32; 16] {
        let c = angle_in_radians.cos();
        let s = angle_in_radians.sin();
        let mut dst = [0.0; 16];
        dst[ 0] = c;    dst[ 1] = 0.0;  dst[ 2] = -s;   dst[ 3] = 0.0;
        dst[ 4] = 0.0;  dst[ 5] = 1.0;  dst[ 6] = 0.0;  dst[ 7] = 0.0;
        dst[ 8] = s;    dst[ 9] = 0.0;  dst[10] = c;    dst[11] = 0.0;
        dst[12] = 0.0;  dst[13] = 0.0;  dst[14] = 0.0;  dst[15] = 1.0;
        dst
    }

    #[rustfmt::skip]
    pub fn rotation_z(angle_in_radians: f32) -> [f32; 16] {
        let c = angle_in_radians.cos();
        let s = angle_in_radians.sin();
        let mut dst = [0.0; 16];
        dst[ 0] = c;    dst[ 1] = s;    dst[ 2] = 0.0;  dst[ 3] = 0.0;
        dst[ 4] = -s;   dst[ 5] = c;    dst[ 6] = 0.0;  dst[ 7] = 0.0;
        dst[ 8] = 0.0;  dst[ 9] = 0.0;  dst[10] = 1.0;  dst[11] = 0.0;
        dst[12] = 0.0;  dst[13] = 0.0;  dst[14] = 0.0;  dst[15] = 1.0;
        dst
    }

    #[rustfmt::skip]
    pub fn scaling([sx, sy, sz]: [f32; 3]) -> [f32; 16] {
        let mut dst = [0.0; 16];
        dst[ 0] = sx;   dst[ 1] = 0.0;  dst[ 2] = 0.0;  dst[ 3] = 0.0;
        dst[ 4] = 0.0;  dst[ 5] = sy;   dst[ 6] = 0.0;  dst[ 7] = 0.0;
        dst[ 8] = 0.0;  dst[ 9] = 0.0;  dst[10] = sz;   dst[11] = 0.0;
        dst[12] = 0.0;  dst[13] = 0.0;  dst[14] = 0.0;  dst[15] = 1.0;
        dst
    }

    pub fn translate(m: &[f32; 16], translation: [f32; 3]) -> [f32; 16] {
        multiply(m, &self::translation(translation))
    }

    pub fn rotate_x(m: &[f32; 16], angle_in_radians: f32) -> [f32; 16] {
        multiply(m, &rotation_x(angle_in_radians))
    }

    pub fn rotate_y(m: &[f32; 16], angle_in_radians: f32) -> [f32; 16] {
        multiply(m, &rotation_y(angle_in_radians))
    }

    pub fn rotate_z(m: &[f32; 16], angle_in_radians: f32) -> [f32; 16] {
        multiply(m, &rotation_z(angle_in_radians))
    }

    pub fn scale(m: &[f32; 16], scale: [f32; 3]) -> [f32; 16] {
        multiply(m, &scaling(scale))
    }
}

// A node in the graph is identified by its index in the SceneGraph's Vec of
// nodes. In JavaScript nodes held direct references to their parent and
// children; in Rust we use indices into an arena instead.
type NodeNdx = usize;

struct SceneGraphNode {
    #[allow(dead_code)] // shown in the page's GUI; used by find() later
    name: String,
    children: Vec<NodeNdx>,
    parent: Option<NodeNdx>,
    local_matrix: [f32; 16],
    world_matrix: [f32; 16],
    source: Option<TRS>,
}

struct SceneGraph {
    nodes: Vec<SceneGraphNode>,
}

#[allow(dead_code)]
impl SceneGraph {
    fn new() -> Self {
        SceneGraph { nodes: Vec::new() }
    }

    // the JS version's `new SceneGraphNode(name, source)`
    fn add_node(&mut self, name: &str, source: Option<TRS>) -> NodeNdx {
        self.nodes.push(SceneGraphNode {
            name: name.to_string(),
            children: Vec::new(),
            parent: None,
            local_matrix: m4::identity(),
            world_matrix: m4::identity(),
            source,
        });
        self.nodes.len() - 1
    }

    fn add_child(&mut self, parent: NodeNdx, child: NodeNdx) {
        self.set_parent(child, Some(parent));
    }

    fn remove_child(&mut self, _parent: NodeNdx, child: NodeNdx) {
        self.set_parent(child, None);
    }

    fn set_parent(&mut self, node: NodeNdx, parent: Option<NodeNdx>) {
        // remove us from our parent
        if let Some(old_parent) = self.nodes[node].parent {
            let children = &mut self.nodes[old_parent].children;
            if let Some(ndx) = children.iter().position(|&c| c == node) {
                children.remove(ndx);
            }
        }

        // Add us to our new parent
        if let Some(parent) = parent {
            self.nodes[parent].children.push(node);
        }
        self.nodes[node].parent = parent;
    }

    fn update_world_matrix(&mut self, node: NodeNdx) {
        // update the local matrix from its source if it has one.
        if let Some(source) = &self.nodes[node].source {
            self.nodes[node].local_matrix = source.get_matrix();
        }

        if let Some(parent) = self.nodes[node].parent {
            // we have a parent so do the math
            self.nodes[node].world_matrix = m4::multiply(
                &self.nodes[parent].world_matrix,
                &self.nodes[node].local_matrix,
            );
        } else {
            // we have no parent so just copy local to world
            self.nodes[node].world_matrix = self.nodes[node].local_matrix;
        }

        // now process all the children
        for i in 0..self.nodes[node].children.len() {
            let child = self.nodes[node].children[i];
            self.update_world_matrix(child);
        }
    }
}

#[derive(Clone, Copy)]
struct TRS {
    translation: [f32; 3],
    rotation: [f32; 3],
    scale: [f32; 3],
}

impl Default for TRS {
    fn default() -> Self {
        TRS {
            translation: [0.0, 0.0, 0.0],
            rotation: [0.0, 0.0, 0.0],
            scale: [1.0, 1.0, 1.0],
        }
    }
}

impl TRS {
    fn get_matrix(&self) -> [f32; 16] {
        let mut dst = m4::translation(self.translation);
        dst = m4::rotate_x(&dst, self.rotation[0]);
        dst = m4::rotate_y(&dst, self.rotation[1]);
        dst = m4::rotate_z(&dst, self.rotation[2]);
        m4::scale(&dst, self.scale)
    }
}

fn add_trs_scene_graph_node(
    scene: &mut SceneGraph,
    name: &str,
    parent: Option<NodeNdx>,
    trs: TRS,
) -> NodeNdx {
    let node = scene.add_node(name, Some(trs));
    if let Some(parent) = parent {
        scene.set_parent(node, Some(parent));
    }
    node
}

// The camera rig. In JavaScript this was a class holding direct references
// to its rig nodes and a `nodeToUISettings` map for the page's GUI (the map
// stays page-side in this port); here the rig holds node indices and its
// methods take the SceneGraph.
struct OrbitCamera {
    cam_target: NodeNdx,
    cam_pan: NodeNdx,
    cam_tilt: NodeNdx,
    cam_extend: NodeNdx,
    cam: NodeNdx,
}

#[allow(dead_code)]
impl OrbitCamera {
    fn new(scene: &mut SceneGraph) -> Self {
        // Create Camera Rig
        let cam_target = add_trs_scene_graph_node(scene, "cam-target", None, TRS::default());
        let cam_pan = add_trs_scene_graph_node(scene, "cam-pan", Some(cam_target), TRS::default());
        let cam_tilt = add_trs_scene_graph_node(scene, "cam-tilt", Some(cam_pan), TRS::default());
        let cam_extend =
            add_trs_scene_graph_node(scene, "cam-extend", Some(cam_tilt), TRS::default());
        let cam = add_trs_scene_graph_node(scene, "cam", Some(cam_extend), TRS::default());

        OrbitCamera {
            cam_target,
            cam_pan,
            cam_tilt,
            cam_extend,
            cam,
        }
    }

    fn set_parent(&self, scene: &mut SceneGraph, parent: NodeNdx) {
        scene.set_parent(self.cam_target, Some(parent));
    }

    fn get_camera_matrix(&self, scene: &SceneGraph) -> [f32; 16] {
        scene.nodes[self.cam].world_matrix
    }

    fn pan(&self, scene: &SceneGraph) -> f32 {
        scene.nodes[self.cam_pan].source.as_ref().unwrap().rotation[1]
    }
    fn set_pan(&self, scene: &mut SceneGraph, v: f32) {
        scene.nodes[self.cam_pan].source.as_mut().unwrap().rotation[1] = v;
    }
    fn tilt(&self, scene: &SceneGraph) -> f32 {
        scene.nodes[self.cam_tilt].source.as_ref().unwrap().rotation[0]
    }
    fn set_tilt(&self, scene: &mut SceneGraph, v: f32) {
        scene.nodes[self.cam_tilt].source.as_mut().unwrap().rotation[0] = v;
    }
    fn radius(&self, scene: &SceneGraph) -> f32 {
        scene.nodes[self.cam_extend].source.as_ref().unwrap().translation[2]
    }
    fn set_radius(&self, scene: &mut SceneGraph, v: f32) {
        scene.nodes[self.cam_extend].source.as_mut().unwrap().translation[2] = v;
    }
    fn target(&self, scene: &SceneGraph) -> [f32; 3] {
        scene.nodes[self.cam_target].source.as_ref().unwrap().translation
    }
    fn set_target(&self, scene: &mut SceneGraph, world_position: [f32; 3]) {
        // this.#camTarget.parent?.worldMatrix ?? mat4.identity()
        let parent_world_matrix = match scene.nodes[self.cam_target].parent {
            Some(parent) => scene.nodes[parent].world_matrix,
            None => m4::identity(),
        };
        let inv = m4::inverse(&parent_world_matrix);
        scene.nodes[self.cam_target].source.as_mut().unwrap().translation =
            vec3::transform_mat4(world_position, &inv);
    }

    fn get_update_helper(&self, scene: &SceneGraph) -> UpdateHelper {
        UpdateHelper {
            start_tilt: self.tilt(scene),
            start_pan: self.pan(scene),
            start_radius: self.radius(scene),
            start_camera_matrix: self.get_camera_matrix(scene),
            start_target: self.target(scene),
        }
    }
}

// In JavaScript `getUpdateHelper` returned an object of closures that
// captured the starting camera state; in Rust it's a struct of the starting
// values with methods that take the camera and scene.
struct UpdateHelper {
    start_tilt: f32,
    start_pan: f32,
    start_radius: f32,
    start_camera_matrix: [f32; 16],
    start_target: [f32; 3],
}

impl UpdateHelper {
    fn pan_and_tilt(&self, cam: &OrbitCamera, scene: &mut SceneGraph, delta_pan: f32, delta_tilt: f32) {
        cam.set_tilt(scene, self.start_tilt - delta_tilt);
        cam.set_pan(scene, self.start_pan - delta_pan);
    }

    fn track(&self, cam: &OrbitCamera, scene: &mut SceneGraph, delta_x: f32, delta_y: f32) {
        let world_direction =
            vec3::transform_mat3([delta_x, delta_y, 0.0], &self.start_camera_matrix);
        // this.#camTarget.parent?.worldMatrix ?? mat4.identity()
        let parent_world_matrix = match scene.nodes[cam.cam_target].parent {
            Some(parent) => scene.nodes[parent].world_matrix,
            None => m4::identity(),
        };
        let inv = m4::inverse(&parent_world_matrix);
        let camera_direction = vec3::transform_mat3(world_direction, &inv);
        scene.nodes[cam.cam_target].source.as_mut().unwrap().translation =
            vec3::add(self.start_target, camera_direction);
    }

    fn dolly(&self, cam: &OrbitCamera, scene: &mut SceneGraph, delta: f32) {
        cam.set_radius(scene, self.start_radius + delta);
    }
}

// The JS version uses strings for the modes ('track', 'panAndTilt', ...);
// in Rust an enum is the natural fit.
#[derive(Clone, Copy, PartialEq)]
enum Mode {
    Undefined,
    DoubleTapZoom,
    Track,
    PanAndTilt,
}

// Like the nodes, each mesh refers to its vertices by index (into a Vec of
// Vertices) instead of holding a direct reference.
struct Mesh {
    node: NodeNdx,
    vertices: usize,
    color: [f32; 4],
}

fn add_mesh(meshes: &mut Vec<Mesh>, node: NodeNdx, vertices: usize, color: [f32; 4]) {
    meshes.push(Mesh {
        node,
        vertices,
        color,
    });
}

fn add_cube_node(
    scene: &mut SceneGraph,
    meshes: &mut Vec<Mesh>,
    name: &str,
    parent: NodeNdx,
    trs: TRS,
    color: [f32; 4],
) {
    let node = add_trs_scene_graph_node(scene, name, Some(parent), trs);
    add_mesh(meshes, node, K_CUBE_VERTICES, color);
}

// matrix and color
const UNIFORM_BUFFER_SIZE: u64 = (16 + 4) * 4;

// offsets to the various uniform values in float32 indices
const K_MATRIX_OFFSET: usize = 0;
const K_COLOR_OFFSET: usize = 16;

struct ObjectInfo {
    uniform_buffer: wgpu::Buffer,
    uniform_values: [f32; UNIFORM_BUFFER_SIZE as usize / 4],
    bind_group: wgpu::BindGroup,
}

fn create_object_info(device: &wgpu::Device, pipeline: &wgpu::RenderPipeline) -> ObjectInfo {
    let uniform_buffer = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("uniforms"),
        size: UNIFORM_BUFFER_SIZE,
        usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });

    let uniform_values = [0.0f32; UNIFORM_BUFFER_SIZE as usize / 4];

    let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("bind group for object"),
        layout: &pipeline.get_bind_group_layout(0),
        entries: &[wgpu::BindGroupEntry {
            binding: 0,
            resource: uniform_buffer.as_entire_binding(),
        }],
    });

    ObjectInfo {
        uniform_buffer,
        uniform_values,
        bind_group,
    }
}

struct Vertices {
    vertex_buffer: wgpu::Buffer,
    num_vertices: u32,
    aabb: Aabb,
    vertex_data: Vec<f32>,
}

fn create_vertices(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    (vertex_data, num_vertices, aabb): (Vec<f32>, u32, Aabb),
    name: &str,
) -> Vertices {
    let vertex_buffer = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some(&format!("{name}: vertex buffer vertices")),
        size: (vertex_data.len() * 4) as u64,
        usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });
    queue.write_buffer(&vertex_buffer, 0, bytemuck::cast_slice(&vertex_data));
    Vertices {
        vertex_buffer,
        num_vertices,
        aabb,
        vertex_data,
    }
}

// https://en.wikipedia.org/wiki/M%C3%B6ller%E2%80%93Trumbore_intersection_algorithm
fn intersect_line_segment_and_triangle(
    p0: [f32; 3],
    p1: [f32; 3],
    v0: [f32; 3],
    v1: [f32; 3],
    v2: [f32; 3],
) -> Option<[f32; 3]> {
    let edge1 = vec3::subtract(v1, v0);
    let edge2 = vec3::subtract(v2, v0);
    let dir = vec3::subtract(p1, p0); // Line segment direction

    let h = vec3::cross(dir, edge2);
    let a = vec3::dot(edge1, h);

    // If 'a' is near zero, the line is parallel
    // to the triangle's plane
    if a.abs() < 0.00001 {
        return None;
    }

    let f = 1.0 / a;
    let s = vec3::subtract(p0, v0);
    let u = f * vec3::dot(s, h);

    // Check if the intersection point is outside
    // the triangle's U parameter range [0, 1]
    if !(0.0..=1.0).contains(&u) {
        return None;
    }

    let q = vec3::cross(s, edge1);
    let v = f * vec3::dot(dir, q);

    // Check if the intersection point is outside
    // the triangle's V parameter range [0, 1] or S+T range [0, 1]
    if v < 0.0 || u + v > 1.0 {
        return None;
    }

    // At this stage, the intersection point lies on
    // the infinite line and within the triangle
    let t = f * vec3::dot(edge2, q);

    // Check if the intersection point lies within
    // the line segment's T parameter range [0, 1]
    if !(0.0..=1.0).contains(&t) {
        return None;
    }

    // Return the intersection point
    Some(vec3::add_scaled(p0, dir, t))
}

// the result of an intersection: where it hit (in clip space) and the
// index of the mesh that was hit
struct IntersectingMesh {
    position: [f32; 3],
    mesh: usize,
}

fn get_intersecting_meshes(
    clip_x: f32,
    clip_y: f32,
    view_projection: &[f32; 16],
    meshes: &[Mesh],
    scene: &SceneGraph,
    vertex_sets: &[Vertices],
) -> Vec<IntersectingMesh> {
    let clip_near = [clip_x, clip_y, 0.0];
    let clip_far = [clip_x, clip_y, 1.0];

    let mut verts = [[0.0f32; 3]; 3];

    let mut intersecting_meshes = Vec::new();
    for (mesh_ndx, mesh) in meshes.iter().enumerate() {
        // put mat in model space (the space of the vertex data)
        let world_view_projection =
            m4::multiply(view_projection, &scene.nodes[mesh.node].world_matrix);

        // invert it so putting in clip space coords will transform them
        // to model space.
        let mat = m4::inverse(&world_view_projection);

        // now transform the clip space coords to model space
        // so we can compare them to the model vertices and AABB
        let near = vec3::transform_mat4(clip_near, &mat);
        let far = vec3::transform_mat4(clip_far, &mat);

        let Vertices {
            vertex_data,
            num_vertices,
            ..
        } = &vertex_sets[mesh.vertices];

        let num_triangles = num_vertices / 3;
        let mut closest: Option<[f32; 3]> = None;
        for t in 0..num_triangles {
            for (i, v) in verts.iter_mut().enumerate() {
                // get the 3 positions for the triangle
                let offset = (t as usize * 3 + i) * 4;
                v[0] = vertex_data[offset];
                v[1] = vertex_data[offset + 1];
                v[2] = vertex_data[offset + 2];
            }

            let result =
                intersect_line_segment_and_triangle(near, far, verts[0], verts[1], verts[2]);
            if let Some(result) = result {
                // Convert back to clip space so we can check Z to keep
                // the closest hit.
                let result = vec3::transform_mat4(result, &world_view_projection);
                if closest.is_none_or(|closest| result[2] < closest[2]) {
                    closest = Some(result);
                }
            }
        }

        if let Some(closest) = closest {
            intersecting_meshes.push(IntersectingMesh {
                position: closest,
                mesh: mesh_ndx,
            });
        }
    }

    intersecting_meshes
}

fn get_view_projection_matrix(
    cam: &OrbitCamera,
    scene: &SceneGraph,
    field_of_view: f32,
    width: u32,
    height: u32,
) -> [f32; 16] {
    let aspect = width as f32 / height as f32;
    let projection = m4::perspective(
        field_of_view,
        aspect,
        1.0,    // zNear
        2000.0, // zFar
    );

    let view_matrix = m4::inverse(&cam.get_camera_matrix(scene));

    // combine the view and projection matrixes
    m4::multiply(&projection, &view_matrix)
}

// pickMeshes(e, cam) in the JS version. Returns the picked node, if any.
#[allow(clippy::too_many_arguments)]
fn pick_meshes(
    x: f32,
    y: f32,
    width: u32,
    height: u32,
    cam: &OrbitCamera,
    scene: &SceneGraph,
    meshes: &[Mesh],
    vertex_sets: &[Vertices],
    field_of_view: f32,
) -> Option<NodeNdx> {
    let clip_x = x / width as f32 * 2.0 - 1.0;
    let clip_y = y / height as f32 * -2.0 + 1.0;

    let view_projection_value = get_view_projection_matrix(cam, scene, field_of_view, width, height);
    let mut intersecting_meshes =
        get_intersecting_meshes(clip_x, clip_y, &view_projection_value, meshes, scene, vertex_sets);

    // sort the results by their z
    intersecting_meshes.sort_by(|a, b| a.position[2].total_cmp(&b.position[2]));

    // pick the first one
    if !intersecting_meshes.is_empty() {
        let mut node = meshes[intersecting_meshes[0].mesh].node;
        if !wgpu_fun::setting_bool("showMeshNodes", false) {
            while scene.nodes[node].name.contains("mesh") {
                node = scene.nodes[node].parent.unwrap();
            }
        }
        Some(node)
    } else {
        None
    }
}

fn compute_aabb_for_mesh(mesh: &Mesh, scene: &SceneGraph, vertex_sets: &[Vertices]) -> Aabb {
    let mat = &scene.nodes[mesh.node].world_matrix;
    let p0 = vertex_sets[mesh.vertices].aabb.min;
    let p1 = vertex_sets[mesh.vertices].aabb.max;
    let mut min = [0.0; 3];
    let mut max = [0.0; 3];
    for i in 0..8 {
        let p = [
            if i & 1 != 0 { p0[0] } else { p1[0] },
            if i & 2 != 0 { p0[1] } else { p1[1] },
            if i & 4 != 0 { p0[2] } else { p1[2] },
        ];
        let p = vec3::transform_mat4(p, mat);
        if i == 0 {
            min = p;
            max = p;
        } else {
            min = vec3::min(min, p);
            max = vec3::max(max, p);
        }
    }
    Aabb { min, max }
}

fn expand_aabb_in_place(aabb: &mut Aabb, other_aabb: &Aabb) {
    aabb.min = vec3::min(aabb.min, other_aabb.min);
    aabb.max = vec3::max(aabb.max, other_aabb.max);
}

fn get_aabb_for_selected_meshes(
    selected_meshes: &[&Mesh],
    scene: &SceneGraph,
    vertex_sets: &[Vertices],
) -> Option<Aabb> {
    if selected_meshes.is_empty() {
        return None;
    }
    let mut aabb = compute_aabb_for_mesh(selected_meshes[0], scene, vertex_sets);
    for mesh in &selected_meshes[1..] {
        expand_aabb_in_place(
            &mut aabb,
            &compute_aabb_for_mesh(mesh, scene, vertex_sets),
        );
    }
    Some(aabb)
}

// In JavaScript `drawObject` was a function that captured `device`,
// `pipeline`, `objectInfos` and `objectNdx` from the enclosing scope.
// In Rust we pass those in via the context.
struct Ctx<'a, 'b> {
    pass: &'a mut wgpu::RenderPass<'b>,
    view_projection_matrix: [f32; 16],
    device: &'a wgpu::Device,
    queue: &'a wgpu::Queue,
    pipeline: &'a wgpu::RenderPipeline,
    object_infos: &'a mut Vec<ObjectInfo>,
    object_ndx: usize,
}

fn draw_object(ctx: &mut Ctx, vertices: &Vertices, matrix: [f32; 16], color: [f32; 4]) {
    let Vertices {
        vertex_buffer,
        num_vertices,
        ..
    } = vertices;
    if ctx.object_ndx == ctx.object_infos.len() {
        ctx.object_infos
            .push(create_object_info(ctx.device, ctx.pipeline));
    }
    let object_info = &mut ctx.object_infos[ctx.object_ndx];
    ctx.object_ndx += 1;

    let matrix_value = m4::multiply(&ctx.view_projection_matrix, &matrix);
    object_info.uniform_values[K_MATRIX_OFFSET..K_MATRIX_OFFSET + 16]
        .copy_from_slice(&matrix_value);
    object_info.uniform_values[K_COLOR_OFFSET..K_COLOR_OFFSET + 4].copy_from_slice(&color);

    // upload the uniform values to the uniform buffer
    ctx.queue.write_buffer(
        &object_info.uniform_buffer,
        0,
        bytemuck::cast_slice(&object_info.uniform_values),
    );

    ctx.pass.set_vertex_buffer(0, vertex_buffer.slice(..));
    ctx.pass.set_bind_group(0, &object_info.bind_group, &[]);
    ctx.pass.draw(0..*num_vertices, 0..1);
}

fn draw_mesh(ctx: &mut Ctx, mesh: &Mesh, scene: &SceneGraph, vertex_sets: &[Vertices]) {
    let Mesh {
        node,
        vertices,
        color,
    } = mesh;
    draw_object(
        ctx,
        &vertex_sets[*vertices],
        scene.nodes[*node].world_matrix,
        *color,
    );
}

fn mesh_uses_node(mesh: &Mesh, scene: &SceneGraph, node: NodeNdx) -> bool {
    if mesh.node == node {
        return true;
    }
    for &child in &scene.nodes[node].children {
        if mesh_uses_node(mesh, scene, child) {
            return true;
        }
    }
    false
}

fn make_new_texture_if_size_different(
    device: &wgpu::Device,
    texture: Option<wgpu::Texture>,
    (width, height): (u32, u32),
    format: wgpu::TextureFormat,
    usage: wgpu::TextureUsages,
) -> wgpu::Texture {
    if let Some(texture) = texture {
        if texture.width() == width && texture.height() == height {
            return texture;
        }
        texture.destroy();
    }
    device.create_texture(&wgpu::TextureDescriptor {
        label: None,
        size: wgpu::Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        },
        format,
        usage,
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        view_formats: &[],
    })
}

const K_CUBE_VERTICES: usize = 0;

const K_HANDLE_COLOR: [f32; 4] = [0.5, 0.5, 0.5, 1.0];
const K_DRAWER_COLOR: [f32; 4] = [1.0, 1.0, 1.0, 1.0];
const K_CABINET_COLOR: [f32; 4] = [0.75, 0.75, 0.75, 0.75];
const K_NUM_DRAWERS_PER_CABINET: usize = 4;
const K_NUM_CABINETS: usize = 5;

const K_DRAWER_SIZE: [f32; 3] = [40.0, 30.0, 50.0];
const K_HANDLE_SIZE: [f32; 3] = [10.0, 2.0, 2.0];

const K_WIDTH: usize = 0;
const K_HEIGHT: usize = 1;
const K_DEPTH: usize = 2;

const K_HANDLE_POSITION: [f32; 3] = [
    (K_DRAWER_SIZE[K_WIDTH] - K_HANDLE_SIZE[K_WIDTH]) / 2.0,
    K_DRAWER_SIZE[K_HEIGHT] * 2.0 / 3.0,
    K_HANDLE_SIZE[K_DEPTH],
];

const K_DRAWER_SPACING: f32 = K_DRAWER_SIZE[K_HEIGHT] + 3.0;
const K_CABINET_SPACING: f32 = K_DRAWER_SIZE[K_WIDTH] + 10.0;

fn add_drawer(scene: &mut SceneGraph, meshes: &mut Vec<Mesh>, parent: NodeNdx, drawer_ndx: usize) {
    let drawer_name = format!("drawer{drawer_ndx}");

    // add a node for the entire drawer
    let drawer = add_trs_scene_graph_node(
        scene,
        &drawer_name,
        Some(parent),
        TRS {
            translation: [3.0, drawer_ndx as f32 * K_DRAWER_SPACING + 5.0, 1.0],
            ..Default::default()
        },
    );

    // add a node with a cube for the drawer cube.
    add_cube_node(
        scene,
        meshes,
        &format!("{drawer_name}-drawer-mesh"),
        drawer,
        TRS {
            scale: K_DRAWER_SIZE,
            ..Default::default()
        },
        K_DRAWER_COLOR,
    );

    // add a node with a cube for the handle
    add_cube_node(
        scene,
        meshes,
        &format!("{drawer_name}-handle-mesh"),
        drawer,
        TRS {
            translation: K_HANDLE_POSITION,
            scale: K_HANDLE_SIZE,
            ..Default::default()
        },
        K_HANDLE_COLOR,
    );
}

fn add_cabinet(
    scene: &mut SceneGraph,
    meshes: &mut Vec<Mesh>,
    parent: NodeNdx,
    cabinet_ndx: usize,
) {
    let cabinet_name = format!("cabinet{cabinet_ndx}");

    // add a node for the entire cabinet
    let cabinet = add_trs_scene_graph_node(
        scene,
        &cabinet_name,
        Some(parent),
        TRS {
            translation: [cabinet_ndx as f32 * K_CABINET_SPACING, 0.0, 0.0],
            ..Default::default()
        },
    );

    // add a node with a cube for the cabinet
    let k_cabinet_size = [
        K_DRAWER_SIZE[K_WIDTH] + 6.0,
        K_DRAWER_SPACING * K_NUM_DRAWERS_PER_CABINET as f32 + 6.0,
        K_DRAWER_SIZE[K_DEPTH] + 4.0,
    ];
    add_cube_node(
        scene,
        meshes,
        &format!("{cabinet_name}-mesh"),
        cabinet,
        TRS {
            scale: k_cabinet_size,
            ..Default::default()
        },
        K_CABINET_COLOR,
    );

    // Add the drawers
    for drawer_ndx in 0..K_NUM_DRAWERS_PER_CABINET {
        add_drawer(scene, meshes, cabinet, drawer_ndx);
    }
}

async fn run() {
    let mut app = App::new("WebGPU Picking - CPU - Select 1").await;
    app.auto_resize = true;
    app.alpha_mode = wgpu::CompositeAlphaMode::PreMultiplied;

    let module = app
        .device
        .create_shader_module(wgpu::ShaderModuleDescriptor {
            label: None,
            source: wgpu::ShaderSource::Wgsl(
                r#"
      struct Uniforms {
        matrix: mat4x4f,
        color: vec4f,
      };

      struct Vertex {
        @location(0) position: vec4f,
        @location(1) color: vec4f,
      };

      struct VSOutput {
        @builtin(position) position: vec4f,
        @location(0) color: vec4f,
      };

      @group(0) @binding(0) var<uniform> uni: Uniforms;

      @vertex fn vs(vert: Vertex) -> VSOutput {
        var vsOut: VSOutput;
        vsOut.position = uni.matrix * vert.position;
        vsOut.color = vert.color;
        return vsOut;
      }

      @fragment fn fs(vsOut: VSOutput) -> @location(0) vec4f {
        return vsOut.color * uni.color;
      }
    "#
                .into(),
            ),
        });

    let pipeline = app
        .device
        .create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("2 attributes with color"),
            layout: None,
            vertex: wgpu::VertexState {
                module: &module,
                entry_point: None,
                compilation_options: Default::default(),
                buffers: &[Some(wgpu::VertexBufferLayout {
                    array_stride: (4) * 4, // (3) floats 4 bytes each + one 4 byte color
                    step_mode: wgpu::VertexStepMode::Vertex,
                    attributes: &[
                        // position
                        wgpu::VertexAttribute {
                            shader_location: 0,
                            offset: 0,
                            format: wgpu::VertexFormat::Float32x3,
                        },
                        // color
                        wgpu::VertexAttribute {
                            shader_location: 1,
                            offset: 12,
                            format: wgpu::VertexFormat::Unorm8x4,
                        },
                    ],
                })],
            },
            fragment: Some(wgpu::FragmentState {
                module: &module,
                entry_point: None,
                compilation_options: Default::default(),
                targets: &[Some(app.format.into())],
            }),
            primitive: wgpu::PrimitiveState {
                cull_mode: Some(wgpu::Face::Back),
                ..Default::default()
            },
            depth_stencil: Some(wgpu::DepthStencilState {
                depth_write_enabled: Some(true),
                depth_compare: Some(wgpu::CompareFunction::Less),
                format: wgpu::TextureFormat::Depth24Plus,
                stencil: Default::default(),
                bias: Default::default(),
            }),
            multisample: Default::default(),
            multiview_mask: None,
            cache: None,
        });

    let post_process_module = app
        .device
        .create_shader_module(wgpu::ShaderModuleDescriptor {
            label: None,
            source: wgpu::ShaderSource::Wgsl(
                r#"
      struct VSOutput {
        @builtin(position) position: vec4f,
        @location(0) texcoord: vec2f,
      };

      @vertex fn vs(
        @builtin(vertex_index) vertexIndex : u32,
      ) -> VSOutput {
        var pos = array(
          vec2f(-1.0, -1.0),
          vec2f(-1.0,  3.0),
          vec2f( 3.0, -1.0),
        );

        var vsOutput: VSOutput;
        let xy = pos[vertexIndex];
        vsOutput.position = vec4f(xy, 0.0, 1.0);
        vsOutput.texcoord = xy * vec2f(0.5, -0.5) + vec2f(0.5);
        return vsOutput;
      }

      @group(0) @binding(0) var mask: texture_2d<f32>;

      fn isOnEdge(pos: vec2i) -> bool {
        // Note: we need to make sure we don't use out of bounds
        // texel coordinates with textureLoad as that returns
        // different results on different GPUs
        let size = vec2i(textureDimensions(mask, 0));
        let start = max(pos - 2, vec2i(0));
        let end = min(pos + 2, size);

        for (var y = start.y; y <= end.y; y++) {
          for (var x = start.x; x <= end.x; x++) {
            let s = textureLoad(mask, vec2i(x, y), 0).a;
            if (s > 0) {
              return true;
            }
          }
        }
        return false;
      };

      @fragment fn fs2d(fsInput: VSOutput) -> @location(0) vec4f {
        let pos = vec2i(fsInput.position.xy);

        // get the current. If it's not 0 we're inside the selected objects
        let s = textureLoad(mask, pos, 0).a;
        if (s > 0) {
          discard;
        }

        let hit = isOnEdge(pos);
        if (!hit) {
          discard;
        }
        return vec4f(1, 0.5, 0, 1);
      }
    "#
                .into(),
            ),
        });

    let post_process_pipeline = app
        .device
        .create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: None,
            layout: None,
            vertex: wgpu::VertexState {
                module: &post_process_module,
                entry_point: None,
                compilation_options: Default::default(),
                buffers: &[],
            },
            fragment: Some(wgpu::FragmentState {
                module: &post_process_module,
                entry_point: None,
                compilation_options: Default::default(),
                targets: &[Some(app.format.into())],
            }),
            primitive: Default::default(),
            depth_stencil: None,
            multisample: Default::default(),
            multiview_mask: None,
            cache: None,
        });

    let mut object_infos: Vec<ObjectInfo> = Vec::new();

    let vertex_sets = vec![create_vertices(
        &app.device,
        &app.queue,
        create_cube_vertices(),
        "cube",
    )];

    let mut scene = SceneGraph::new();
    let mut meshes: Vec<Mesh> = Vec::new();

    let root = scene.add_node("root", None);

    let orbit_camera = OrbitCamera::new(&mut scene);
    let extra_rot = add_trs_scene_graph_node(&mut scene, "extra-rot", Some(root), TRS::default());
    let extra_mov =
        add_trs_scene_graph_node(&mut scene, "extra-mov", Some(extra_rot), TRS::default());
    orbit_camera.set_parent(&mut scene, extra_mov);
    orbit_camera.set_target(&mut scene, [120.0, 80.0, 0.0]);
    orbit_camera.set_tilt(&mut scene, std::f32::consts::PI * -0.2);
    orbit_camera.set_radius(&mut scene, 300.0);

    let cabinets = add_trs_scene_graph_node(&mut scene, "cabinets", Some(root), TRS::default());
    // Add cabinets
    for cabinet_ndx in 0..K_NUM_CABINETS {
        add_cabinet(&mut scene, &mut meshes, cabinets, cabinet_ndx);
    }

    let mut depth_texture: Option<wgpu::Texture> = None;
    let mut post_texture: Option<wgpu::Texture> = None;
    let mut post_process_bind_group: Option<wgpu::BindGroup> = None;

    let field_of_view = 60.0f32.to_radians();

    // id of the last TRS edit we applied from the page's GUI
    let mut last_trs_edit_id = 0.0f64;
    // id of the last "frame selected" button press we handled
    let mut last_frame_selected_id = 0.0f64;

    // The selected node. The page's GUI changes it via the `selectNode`
    // setting; picking (below) changes it directly. This is the state
    // behind the JS version's `setCurrentSceneGraphNode(node)`.
    let mut selected_node: Option<NodeNdx> = Some(23); // cabinets.children[1]
    // id of the last selection the page sent
    let mut last_select_id = 0.0f64;

    // (native test mode) there's no pointer, so simulate one click in the
    // center of the canvas on the first frame to exercise the pick path
    #[cfg(not(target_arch = "wasm32"))]
    let mut test_pick_pending = std::env::var("WGPU_FUN_TEST").is_ok();

    // state for the pointer events (addOrbitCameraEventListeners in the
    // JS version)
    let mut start_x = 0.0f32;
    let mut start_y = 0.0f32;
    let mut moved = false;
    let mut last_mode: Option<Mode> = None;
    // Some(...) while a drag is in progress; this stands in for the JS
    // version's pointer capture check.
    let mut cam_helper: Option<UpdateHelper> = None;
    // wgpu_fun's event queue doesn't carry keyboard modifiers, so where the
    // JS version checks `e.shiftKey || (e.buttons & 4) !== 0` we check
    // which button started the drag (1 = middle).
    let mut drag_button = 0u32;
    // The JS version keeps a Map of pointer id -> last position so it can
    // compute the distance between 2 fingers (a pinch). wgpu_fun's event
    // queue merges all pointers into one stream with no ids, so we can only
    // count them (the JS version's pointerToLastPosition.size) and give up
    // on 2 or more, like the JS gives up on 3 or more.
    let mut pointer_count = 0i32;
    let mut double_tap_mode = false;
    // performance.now() in the JS version; we use the frame time instead.
    let mut last_single_tap_time = f64::NEG_INFINITY;

    app.run(RenderMode::Once, move |frame: &Frame| {
        // The page's GUI selects nodes by sending an "id ndx" string (ndx
        // -1 = none); the id makes each click apply exactly once, since
        // picking below also changes the selection.
        let select_node = wgpu_fun::setting_str("selectNode", "");
        let parts: Vec<f64> = select_node
            .split_whitespace()
            .filter_map(|v| v.parse().ok())
            .collect();
        if let [id, ndx] = parts[..] {
            if id != last_select_id {
                last_select_id = id;
                selected_node = (ndx >= 0.0).then(|| ndx as usize);
            }
        }

        // The page's GUI sends TRS edits as an "id axis value" string for
        // the node its own UI has selected (`nodeNdx`); axis 0-2 is
        // translation, 3-5 rotation, 6-8 scale. Apply each edit once.
        let node_ndx = wgpu_fun::setting_f64("nodeNdx", 23.0) as usize;
        let trs_edit = wgpu_fun::setting_str("trsEdit", "");
        let parts: Vec<f64> = trs_edit
            .split_whitespace()
            .filter_map(|v| v.parse().ok())
            .collect();
        if let [id, axis, value] = parts[..] {
            if id != last_trs_edit_id {
                last_trs_edit_id = id;
                if let Some(trs) = scene
                    .nodes
                    .get_mut(node_ndx)
                    .and_then(|node| node.source.as_mut())
                {
                    let (axis, value) = (axis as usize, value as f32);
                    match axis {
                        0..=2 => trs.translation[axis] = value,
                        3..=5 => trs.rotation[axis - 3] = value,
                        _ => trs.scale[axis - 6] = value,
                    }
                }
            }
        }

        // The JS version attaches pointerdown/pointermove/pointerup
        // listeners to the canvas; here we drain wgpu_fun's pointer event
        // queue instead (coordinates are in device pixels).
        for event in wgpu_fun::drain_pointer_events() {
            match event {
                PointerEvent::Down { x, y, button } => {
                    const K_DOUBLE_CLICK_TIME_MS: f64 = 300.0;
                    // canvas.setPointerCapture(e.pointerId);
                    pointer_count += 1;
                    drag_button = button;
                    if pointer_count == 1 {
                        moved = false;
                        if !double_tap_mode {
                            let now = frame.time;
                            let delta_time = (now - last_single_tap_time) * 1000.0;
                            if delta_time < K_DOUBLE_CLICK_TIME_MS {
                                double_tap_mode = true;
                            }
                            last_single_tap_time = now;
                        }
                    } else {
                        double_tap_mode = false;
                    }
                    // updateStartPosition(e);
                    start_x = x;
                    start_y = y;
                    cam_helper = Some(orbit_camera.get_update_helper(&scene));
                }
                PointerEvent::Move { x, y } => {
                    // if (!canvas.hasPointerCapture(e.pointerId)) return;
                    if cam_helper.is_none() {
                        continue;
                    }

                    let mode = if pointer_count >= 2 {
                        // more than one pointer; without pointer ids we
                        // can't compute a pinch distance, so give up.
                        Mode::Undefined
                    } else if double_tap_mode {
                        Mode::DoubleTapZoom
                    } else if drag_button == 1 {
                        Mode::Track
                    } else {
                        Mode::PanAndTilt
                    };

                    if Some(mode) != last_mode {
                        last_mode = Some(mode);
                        // updateStartPosition(e);
                        start_x = x;
                        start_y = y;
                        cam_helper = Some(orbit_camera.get_update_helper(&scene));
                    }

                    let delta_x = x - start_x;
                    let delta_y = y - start_y;

                    if pointer_count == 1 && delta_x.hypot(delta_y) > 1.0 {
                        moved = true;
                    }

                    let helper = cam_helper.as_ref().unwrap();
                    match mode {
                        Mode::Undefined => {}
                        Mode::Track => {
                            let s = orbit_camera.radius(&scene) * 0.001;
                            helper.track(&orbit_camera, &mut scene, -delta_x * s, delta_y * s);
                        }
                        Mode::PanAndTilt => {
                            helper.pan_and_tilt(
                                &orbit_camera,
                                &mut scene,
                                delta_x * 0.01,
                                delta_y * 0.01,
                            );
                        }
                        Mode::DoubleTapZoom => {
                            let radius = orbit_camera.radius(&scene);
                            helper.dolly(&orbit_camera, &mut scene, radius * 0.002 * delta_y);
                        }
                    }
                }
                PointerEvent::Up { x, y, .. } => {
                    let num_pointers = pointer_count;
                    // pointerToLastPosition.delete(e.pointerId);
                    pointer_count = (pointer_count - 1).max(0);
                    // canvas.releasePointerCapture(e.pointerId);
                    cam_helper = None;
                    if num_pointers == 1 && pointer_count == 0 {
                        double_tap_mode = false;
                        if !moved {
                            // pickMeshes(e, cam). The world matrices are up
                            // to date from the previous render in the JS
                            // version; make sure they are here too.
                            scene.update_world_matrix(root);
                            if let Some(node) = pick_meshes(
                                x,
                                y,
                                frame.width,
                                frame.height,
                                &orbit_camera,
                                &scene,
                                &meshes,
                                &vertex_sets,
                                field_of_view,
                            ) {
                                // setCurrentSceneGraphNode(node)
                                selected_node = Some(node);
                            }
                        }
                    }
                }
                // Dolly when the user uses the wheel
                PointerEvent::Wheel { delta_y, .. } => {
                    // (e.preventDefault() happens inside wgpu_fun)
                    let helper = orbit_camera.get_update_helper(&scene);
                    let radius = orbit_camera.radius(&scene);
                    helper.dolly(&orbit_camera, &mut scene, radius * 0.001 * delta_y);
                }
            }
        }

        // (native test mode) simulate one click in the center of the
        // canvas and print what got picked so the pick path can be
        // verified headlessly.
        #[cfg(not(target_arch = "wasm32"))]
        if test_pick_pending {
            test_pick_pending = false;
            let (x, y) = (frame.width as f32 / 2.0, frame.height as f32 / 2.0);
            scene.update_world_matrix(root);
            let picked = pick_meshes(
                x,
                y,
                frame.width,
                frame.height,
                &orbit_camera,
                &scene,
                &meshes,
                &vertex_sets,
                field_of_view,
            );
            wgpu_fun::print(&format!(
                "test pick at center: {}",
                picked.map_or("--none--".to_string(), |node| scene.nodes[node].name.clone())
            ));
            if let Some(node) = picked {
                selected_node = Some(node);
            }
        }

        // Gather the meshes the selected node (or any of its children)
        // uses. This is the JS version's `selectedMeshes = meshes.filter(...)`
        // from `setCurrentSceneGraphNode`.
        let selected_meshes: Vec<&Mesh> = meshes
            .iter()
            .filter(|mesh| selected_node.is_some_and(|node| mesh_uses_node(mesh, &scene, node)))
            .collect();

        // The page's "frame selected" button bumps the `frameSelected`
        // setting; run the JS version's frameSelected() once per press.
        let frame_selected_id = wgpu_fun::setting_f64("frameSelected", 0.0);
        if frame_selected_id != last_frame_selected_id {
            last_frame_selected_id = frame_selected_id;
            if !selected_meshes.is_empty() {
                // In the JS version the world matrices are up to date from
                // the previous render; make sure they are here too.
                scene.update_world_matrix(root);

                // get aabb bounds for the selected objects.
                let aabb =
                    get_aabb_for_selected_meshes(&selected_meshes, &scene, &vertex_sets).unwrap();

                let extent = vec3::subtract(aabb.max, aabb.min);
                let diameter = vec3::distance(aabb.min, aabb.max);

                // compute how far we need to set the radius for the selected
                // objects to be framed.
                let aspect = frame.width as f32 / frame.height as f32;
                let field_of_view_h = 2.0 * (field_of_view.tan() * aspect).atan();
                let fov = field_of_view_h.min(field_of_view);
                let zoom_scale = 1.5; // make it 1.5 times as large for some padding.
                let half_size = diameter * zoom_scale * 0.5;
                let distance = half_size / (fov * 0.5).tan();

                orbit_camera.set_radius(&mut scene, distance);

                // point the camera at the center
                let center = vec3::add_scaled(aabb.min, extent, 0.5);
                orbit_camera.set_target(&mut scene, center);
            }
        }

        // If we don't have a depth texture OR if its size is different
        // from the canvasTexture when make a new depth texture
        depth_texture = Some(make_new_texture_if_size_different(
            frame.device,
            depth_texture.take(),
            (frame.width, frame.height), // for size
            wgpu::TextureFormat::Depth24Plus,
            wgpu::TextureUsages::RENDER_ATTACHMENT,
        ));
        let depth_view = depth_texture
            .as_ref()
            .unwrap()
            .create_view(&Default::default());

        scene.update_world_matrix(root);
        let view_projection_matrix = get_view_projection_matrix(
            &orbit_camera,
            &scene,
            field_of_view,
            frame.width,
            frame.height,
        );

        let mut encoder = frame
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
        let object_ndx = {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("our basic canvas renderPass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: frame.view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::TRANSPARENT),
                        store: wgpu::StoreOp::Store,
                    },
                    depth_slice: None,
                })],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: &depth_view,
                    depth_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Clear(1.0),
                        store: wgpu::StoreOp::Store,
                    }),
                    stencil_ops: None,
                }),
                ..Default::default()
            });
            pass.set_pipeline(&pipeline);

            let mut ctx = Ctx {
                pass: &mut pass,
                view_projection_matrix,
                device: frame.device,
                queue: frame.queue,
                pipeline: &pipeline,
                object_infos: &mut object_infos,
                object_ndx: 0,
            };
            for mesh in &meshes {
                draw_mesh(&mut ctx, mesh, &scene, &vertex_sets);
            }
            ctx.object_ndx
        };

        // draw selected objects to postTexture
        {
            let size_changed = post_texture
                .as_ref()
                .is_none_or(|t| t.width() != frame.width || t.height() != frame.height);
            post_texture = Some(make_new_texture_if_size_different(
                frame.device,
                post_texture.take(),
                (frame.width, frame.height), // for size
                frame.format,
                wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
            ));
            // setupPostProcess in the JS version: if the texture changed,
            // remake the bind group.
            if post_process_bind_group.is_none() || size_changed {
                post_process_bind_group =
                    Some(frame.device.create_bind_group(&wgpu::BindGroupDescriptor {
                        label: None,
                        layout: &post_process_pipeline.get_bind_group_layout(0),
                        entries: &[wgpu::BindGroupEntry {
                            binding: 0,
                            resource: wgpu::BindingResource::TextureView(
                                &post_texture.as_ref().unwrap().create_view(&Default::default()),
                            ),
                        }],
                    }));
            }

            let post_texture_view = post_texture
                .as_ref()
                .unwrap()
                .create_view(&Default::default());
            {
                let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                    label: Some("our basic canvas renderPass"),
                    color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                        view: &post_texture_view,
                        resolve_target: None,
                        ops: wgpu::Operations {
                            load: wgpu::LoadOp::Clear(wgpu::Color::TRANSPARENT),
                            store: wgpu::StoreOp::Store,
                        },
                        depth_slice: None,
                    })],
                    depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                        view: &depth_view,
                        depth_ops: Some(wgpu::Operations {
                            load: wgpu::LoadOp::Clear(1.0),
                            store: wgpu::StoreOp::Store,
                        }),
                        stencil_ops: None,
                    }),
                    ..Default::default()
                });
                pass.set_pipeline(&pipeline);

                let mut ctx = Ctx {
                    pass: &mut pass,
                    view_projection_matrix,
                    device: frame.device,
                    queue: frame.queue,
                    pipeline: &pipeline,
                    object_infos: &mut object_infos,
                    object_ndx,
                };
                for mesh in &selected_meshes {
                    draw_mesh(&mut ctx, mesh, &scene, &vertex_sets);
                }
            }

            // Draw outline based on alpha of postTexture
            // on to the canvasTexture
            {
                let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                    label: Some("post process render pass"),
                    color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                        view: frame.view,
                        resolve_target: None,
                        ops: wgpu::Operations {
                            load: wgpu::LoadOp::Load,
                            store: wgpu::StoreOp::Store,
                        },
                        depth_slice: None,
                    })],
                    ..Default::default()
                });
                pass.set_pipeline(&post_process_pipeline);
                pass.set_bind_group(0, post_process_bind_group.as_ref().unwrap(), &[]);
                pass.draw(0..3, 0..1);
            }
        }

        let command_buffer = encoder.finish();
        frame.queue.submit([command_buffer]);
    });
}

fn main() {
    wgpu_fun::start(run());
}
