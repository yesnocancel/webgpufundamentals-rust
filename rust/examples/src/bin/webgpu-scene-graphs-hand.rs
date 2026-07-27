use wgpu_fun::{App, Frame, RenderMode};

#[rustfmt::skip]
fn create_cube_vertices() -> (Vec<f32>, u32) {
    let positions: Vec<f32> = vec![
        // left
        -0.5, 0.0,  0.5,
        -0.5, 0.0, -0.5,
        -0.5, 1.0,  0.5,
        -0.5, 1.0, -0.5,

        // right
         0.5, 0.0,  0.5,
         0.5, 0.0, -0.5,
         0.5, 1.0,  0.5,
         0.5, 1.0, -0.5,
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
}

fn create_vertices(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    (vertex_data, num_vertices): (Vec<f32>, u32),
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
    }
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

const K_CUBE_VERTICES: usize = 0;

const K_WHITE: [f32; 4] = [1.0, 1.0, 1.0, 1.0];

fn add_finger(
    scene: &mut SceneGraph,
    meshes: &mut Vec<Mesh>,
    name: &str,
    parent: NodeNdx,
    segments: usize,
    segment_height: f32,
    trs: TRS,
) -> Vec<NodeNdx> {
    let mut nodes = Vec::new();
    let base_name = name;
    let mut name = name.to_string();
    let mut parent = parent;
    let mut trs = trs;
    for i in 0..segments {
        let node = add_trs_scene_graph_node(scene, &name, Some(parent), trs);
        nodes.push(node);
        let mesh_node = add_trs_scene_graph_node(
            scene,
            &format!("{name}-mesh"),
            Some(node),
            TRS {
                scale: [10.0, segment_height, 10.0],
                ..Default::default()
            },
        );
        add_mesh(meshes, mesh_node, K_CUBE_VERTICES, K_WHITE);
        parent = node;
        name = format!("{base_name}-{}", i + 1);
        trs = TRS {
            translation: [0.0, segment_height, 0.0],
            rotation: [15.0f32.to_radians(), 0.0, 0.0],
            ..Default::default()
        };
    }
    nodes
}

fn lerp(a: f32, b: f32, t: f32) -> f32 {
    a + (b - a) * t
}

fn animate(time: f64, scene: &mut SceneGraph, anim_nodes: &[NodeNdx]) {
    for (i, &node) in anim_nodes.iter().enumerate() {
        let source = scene.nodes[node].source.as_mut().unwrap();
        let t = time + i as f64 * 0.1;
        let l = (t.sin() * 0.5 + 0.5) as f32;
        source.rotation[0] = lerp(0.0, std::f32::consts::PI * 0.25, l);
    }
}

async fn run() {
    let mut app = App::new("WebGPU Scene Graph - Hand").await;
    app.auto_resize = true;
    app.alpha_mode = wgpu::CompositeAlphaMode::PreMultiplied;

    let module = app
        .device
        .create_shader_module(wgpu::ShaderModuleDescriptor {
            label: None,
            source: wgpu::ShaderSource::Wgsl(
                /* wgsl */ r#"
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

    let vertex_sets = vec![create_vertices(
        &app.device,
        &app.queue,
        create_cube_vertices(),
        "cube",
    )];

    let mut scene = SceneGraph::new();
    let mut meshes: Vec<Mesh> = Vec::new();

    let root = scene.add_node("root", None);
    let wrist = add_trs_scene_graph_node(&mut scene, "wrist", Some(root), TRS::default());
    let palm = add_trs_scene_graph_node(
        &mut scene,
        "palm",
        Some(wrist),
        TRS {
            translation: [0.0, 100.0, 0.0],
            ..Default::default()
        },
    );
    let palm_mesh = add_trs_scene_graph_node(
        &mut scene,
        "palm-mesh",
        Some(wrist),
        TRS {
            scale: [100.0, 100.0, 10.0],
            ..Default::default()
        },
    );
    add_mesh(&mut meshes, palm_mesh, K_CUBE_VERTICES, K_WHITE);
    let rotation = [15.0f32.to_radians(), 0.0, 0.0];
    #[rustfmt::skip]
    let mut anim_nodes: Vec<NodeNdx> = vec![
        wrist,
        palm,
    ];
    #[rustfmt::skip]
    {
        anim_nodes.extend(add_finger(&mut scene, &mut meshes, "thumb",         palm, 2, 20.0, TRS { translation: [-50.0, 0.0, 0.0], rotation, ..Default::default() }));
        anim_nodes.extend(add_finger(&mut scene, &mut meshes, "index finger",  palm, 3, 30.0, TRS { translation: [-25.0, 0.0, 0.0], rotation, ..Default::default() }));
        anim_nodes.extend(add_finger(&mut scene, &mut meshes, "middle finger", palm, 3, 35.0, TRS { translation: [ -0.0, 0.0, 0.0], rotation, ..Default::default() }));
        anim_nodes.extend(add_finger(&mut scene, &mut meshes, "ring finger",   palm, 3, 33.0, TRS { translation: [ 25.0, 0.0, 0.0], rotation, ..Default::default() }));
        anim_nodes.extend(add_finger(&mut scene, &mut meshes, "pinky",         palm, 3, 25.0, TRS { translation: [ 45.0, 0.0, 0.0], rotation, ..Default::default() }));
    };

    let mut depth_texture: Option<wgpu::Texture> = None;

    // id of the last TRS edit we applied from the page's GUI
    let mut last_trs_edit_id = 0.0f64;

    // clock for the animation; it only advances while animating
    let mut then = 0.0f64;
    let mut time = 0.0f64;
    let mut was_running = false;

    app.run(RenderMode::Continuous, move |frame: &Frame| {
        // The page's GUI selects a node (`nodeNdx`) and sends TRS edits as a
        // "id axis value" string; axis 0-2 is translation, 3-5 rotation,
        // 6-8 scale. Apply each edit once, to the selected node's TRS.
        let node_ndx = wgpu_fun::setting_f64("nodeNdx", 1.0) as usize;
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

        // The animation clock only advances while "animate" is checked.
        let settings_animate = wgpu_fun::setting_bool("animate", false);
        let is_running = settings_animate;
        let now = frame.time;
        let delta_time = if was_running { now - then } else { 0.0 };
        then = now;

        if is_running {
            time += delta_time;
        }
        was_running = is_running;

        if settings_animate {
            animate(time, &mut scene, &anim_nodes);
        }

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

            let camera_rotation =
                wgpu_fun::setting_f64("cameraRotation", (-45.0f64).to_radians()) as f32;

            let aspect = frame.width as f32 / frame.height as f32;
            let projection = m4::perspective(
                60.0f32.to_radians(), // fieldOfView,
                aspect,
                1.0,    // zNear
                2000.0, // zFar
            );

            // Compute a camera matrix
            let mut camera_matrix = m4::identity();
            camera_matrix = m4::translate(&camera_matrix, [0.0, 100.0, 0.0]);
            camera_matrix = m4::rotate_y(&camera_matrix, camera_rotation);
            camera_matrix = m4::translate(&camera_matrix, [100.0, 0.0, 300.0]);

            // Compute a view matrix
            let view_matrix = m4::inverse(&camera_matrix);

            // combine the view and projection matrixes
            let view_projection_matrix = m4::multiply(&projection, &view_matrix);

            let mut ctx = Ctx {
                pass: &mut pass,
                view_projection_matrix,
                device: frame.device,
                queue: frame.queue,
                pipeline: &pipeline,
                object_infos: &mut object_infos,
                object_ndx: 0,
            };
            scene.update_world_matrix(root);
            for mesh in &meshes {
                draw_mesh(&mut ctx, mesh, &scene, &vertex_sets);
            }
        }

        let command_buffer = encoder.finish();
        frame.queue.submit([command_buffer]);
    });
}

fn main() {
    wgpu_fun::start(run());
}
