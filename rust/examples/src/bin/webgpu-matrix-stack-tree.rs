use wgpu_fun::{App, Frame, RenderMode};

#[rustfmt::skip]
fn create_cube_vertices() -> (Vec<f32>, u32) {
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

    (vertex_data, num_vertices)
}

mod vec3 {
    #![allow(dead_code)]

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

        dst[4] = d * ((tmp1 * m10 + tmp2 * m20 + tmp5 * m30) - (tmp0 * m10 + tmp3 * m20 + tmp4 * m30));
        dst[5] = d * ((tmp0 * m00 + tmp7 * m20 + tmp8 * m30) - (tmp1 * m00 + tmp6 * m20 + tmp9 * m30));
        dst[6] = d * ((tmp3 * m00 + tmp6 * m10 + tmp11 * m30) - (tmp2 * m00 + tmp7 * m10 + tmp10 * m30));
        dst[7] = d * ((tmp4 * m00 + tmp9 * m10 + tmp10 * m20) - (tmp5 * m00 + tmp8 * m10 + tmp11 * m20));

        dst[8] = d * ((tmp12 * m13 + tmp15 * m23 + tmp16 * m33) - (tmp13 * m13 + tmp14 * m23 + tmp17 * m33));
        dst[9] = d * ((tmp13 * m03 + tmp18 * m23 + tmp21 * m33) - (tmp12 * m03 + tmp19 * m23 + tmp20 * m33));
        dst[10] = d * ((tmp14 * m03 + tmp19 * m13 + tmp22 * m33) - (tmp15 * m03 + tmp18 * m13 + tmp23 * m33));
        dst[11] = d * ((tmp17 * m03 + tmp20 * m13 + tmp23 * m23) - (tmp16 * m03 + tmp21 * m13 + tmp22 * m23));

        dst[12] = d * ((tmp14 * m22 + tmp17 * m32 + tmp13 * m12) - (tmp16 * m32 + tmp12 * m12 + tmp15 * m22));
        dst[13] = d * ((tmp20 * m32 + tmp12 * m02 + tmp19 * m22) - (tmp18 * m22 + tmp21 * m32 + tmp13 * m02));
        dst[14] = d * ((tmp18 * m12 + tmp23 * m32 + tmp15 * m02) - (tmp22 * m32 + tmp14 * m02 + tmp19 * m12));
        dst[15] = d * ((tmp22 * m22 + tmp16 * m02 + tmp21 * m12) - (tmp20 * m12 + tmp23 * m22 + tmp17 * m02));

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

struct MatrixStack {
    matrix: [f32; 16],
    stack: Vec<[f32; 16]>,
}

#[allow(dead_code)]
impl MatrixStack {
    fn new() -> Self {
        MatrixStack {
            matrix: m4::identity(),
            stack: Vec::new(),
        }
    }
    fn reset(&mut self) -> &mut Self {
        self.matrix = m4::identity();
        self.stack.clear();
        self
    }
    fn save(&mut self) -> &mut Self {
        // [f32; 16] is Copy so pushing copies the current matrix
        self.stack.push(self.matrix);
        self
    }
    fn restore(&mut self) -> &mut Self {
        self.matrix = self.stack.pop().unwrap();
        self
    }
    fn get(&self) -> [f32; 16] {
        self.matrix
    }
    fn set(&mut self, matrix: [f32; 16]) -> &mut Self {
        self.matrix = matrix;
        self
    }
    fn translate(&mut self, translation: [f32; 3]) -> &mut Self {
        self.matrix = m4::translate(&self.matrix, translation);
        self
    }
    fn rotate_x(&mut self, angle: f32) -> &mut Self {
        self.matrix = m4::rotate_x(&self.matrix, angle);
        self
    }
    fn rotate_y(&mut self, angle: f32) -> &mut Self {
        self.matrix = m4::rotate_y(&self.matrix, angle);
        self
    }
    fn rotate_z(&mut self, angle: f32) -> &mut Self {
        self.matrix = m4::rotate_z(&self.matrix, angle);
        self
    }
    fn scale(&mut self, scale: [f32; 3]) -> &mut Self {
        self.matrix = m4::scale(&self.matrix, scale);
        self
    }
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

// In JavaScript `drawObject` was a function that captured `device`,
// `pipeline`, `objectInfos`, `objectNdx` and `numVertices` from the
// enclosing scope. In Rust we pass those in via the context.
#[derive(Clone, Copy)]
struct Settings {
    scale: f32,
    rotation_x: f32,
    rotation_y: f32,
}

struct Ctx<'a, 'b> {
    pass: &'a mut wgpu::RenderPass<'b>,
    stack: &'a mut MatrixStack,
    settings: Settings,
    view_projection_matrix: [f32; 16],
    device: &'a wgpu::Device,
    queue: &'a wgpu::Queue,
    pipeline: &'a wgpu::RenderPipeline,
    object_infos: &'a mut Vec<ObjectInfo>,
    object_ndx: usize,
    num_vertices: u32,
}

fn draw_object(ctx: &mut Ctx, matrix: [f32; 16], color: [f32; 4]) {
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

    ctx.pass.set_bind_group(0, &object_info.bind_group, &[]);
    ctx.pass.draw(0..ctx.num_vertices, 0..1);
}

const K_TREE_DEPTH: usize = 6;
const K_HEIGHT: usize = 1;
// Moves the 1 unit cube so it's center above the origin so that when it scales
// it scales out in x and z and up (y) from the origin
const K_BRANCH_POSITION: [f32; 3] = [-0.5, 0.0, 0.5];
const K_BRANCH_SIZE: [f32; 3] = [20.0, 150.0, 20.0];

const K_WHITE: [f32; 4] = [1.0, 1.0, 1.0, 1.0];

fn draw_branch(ctx: &mut Ctx) {
    ctx.stack
        .save()
        .scale(K_BRANCH_SIZE)
        .translate(K_BRANCH_POSITION);
    draw_object(ctx, ctx.stack.get(), K_WHITE);
    ctx.stack.restore();
}

fn draw_tree_level(ctx: &mut Ctx, offset: f32, tree_depth: usize) {
    let s = if offset != 0.0 { ctx.settings.scale } else { 1.0 };
    let y = if offset != 0.0 {
        K_BRANCH_SIZE[K_HEIGHT]
    } else {
        0.0
    };
    ctx.stack
        .save()
        .translate([0.0, y, 0.0])
        .rotate_z(offset * ctx.settings.rotation_x)
        .rotate_y(offset.abs() * ctx.settings.rotation_y)
        .scale([s, s, s]);

    draw_branch(ctx);

    if tree_depth > 0 {
        draw_tree_level(ctx, -1.0, tree_depth - 1);
        draw_tree_level(ctx, 1.0, tree_depth - 1);
    }

    ctx.stack.restore();
}

async fn run() {
    let mut app = App::new("WebGPU Matrix Stack - Tree").await;
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

    let mut object_infos: Vec<ObjectInfo> = Vec::new();

    let (vertex_data, num_vertices) = create_cube_vertices();
    let vertex_buffer = app.device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("vertex buffer vertices"),
        size: (vertex_data.len() * 4) as u64,
        usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });
    app.queue
        .write_buffer(&vertex_buffer, 0, bytemuck::cast_slice(&vertex_data));

    let mut stack = MatrixStack::new();

    let mut depth_texture: Option<wgpu::Texture> = None;

    app.run(RenderMode::Once, move |frame: &Frame| {
        // If we don't have a depth texture OR if its size is different
        // from the canvasTexture when make a new depth texture
        if depth_texture
            .as_ref()
            .is_none_or(|t| t.width() != frame.width || t.height() != frame.height)
        {
            if let Some(texture) = depth_texture.take() {
                texture.destroy();
            }
            depth_texture = Some(frame.device.create_texture(&wgpu::TextureDescriptor {
                label: None,
                size: wgpu::Extent3d {
                    width: frame.width,
                    height: frame.height,
                    depth_or_array_layers: 1,
                },
                format: wgpu::TextureFormat::Depth24Plus,
                usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                view_formats: &[],
            }));
        }
        let depth_view = depth_texture
            .as_ref()
            .unwrap()
            .create_view(&Default::default());

        let mut encoder = frame
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
        {
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
            pass.set_vertex_buffer(0, vertex_buffer.slice(..));

            let base_rotation = wgpu_fun::setting_f64("baseRotation", 0.0) as f32;
            let settings = Settings {
                scale: wgpu_fun::setting_f64("scale", 0.9) as f32,
                rotation_x: wgpu_fun::setting_f64("rotationX", 20.0f64.to_radians()) as f32,
                rotation_y: wgpu_fun::setting_f64("rotationY", 10.0f64.to_radians()) as f32,
            };

            let aspect = frame.width as f32 / frame.height as f32;
            let projection = m4::perspective(
                60.0f32.to_radians(), // fieldOfView,
                aspect,
                1.0,    // zNear
                2000.0, // zFar
            );

            let eye = [0.0, 450.0, 1000.0];
            let target = [0.0, 450.0, 0.0];
            let up = [0.0, 1.0, 0.0];

            // Compute a view matrix
            let view_matrix = m4::look_at(eye, target, up);

            // combine the view and projection matrixes
            let view_projection_matrix = m4::multiply(&projection, &view_matrix);

            let mut ctx = Ctx {
                pass: &mut pass,
                stack: &mut stack,
                settings,
                view_projection_matrix,
                device: frame.device,
                queue: frame.queue,
                pipeline: &pipeline,
                object_infos: &mut object_infos,
                object_ndx: 0,
                num_vertices,
            };
            ctx.stack.save();
            ctx.stack.rotate_y(base_rotation);
            draw_tree_level(&mut ctx, 0.0, K_TREE_DEPTH);
            ctx.stack.restore();
        }

        let command_buffer = encoder.finish();
        frame.queue.submit([command_buffer]);
    });
}

fn main() {
    wgpu_fun::start(run());
}
