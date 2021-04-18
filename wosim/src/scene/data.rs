use nalgebra::{Matrix4, Rotation3, Translation3, UnitQuaternion, Vector3, Vector4};

#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct Vertex {
    pub pos: Vector3<f32>,
    pub color: Vector3<f32>,
}

#[derive(Clone, Copy)]
pub struct Mesh {
    pub(super) _first_index: u32,
    pub(super) _index_count: u32,
    pub(super) _vertex_offset: i32,
}

#[repr(packed)]
#[derive(Clone, Copy, Default)]
pub struct SceneConstants {
    pub view: Matrix4<f32>,
    pub previous_view: Matrix4<f32>,
    pub projection: Matrix4<f32>,
    pub view_projection: Matrix4<f32>,
    pub znear: f32,
    pub zfar: f32,
    pub w: f32,
    pub h: f32,
    pub object_count: u32,
}

#[derive(Clone, Copy)]
pub struct Transform {
    pub translation: Vector3<f32>,
    pub scale: Vector3<f32>,
    pub rotation: UnitQuaternion<f32>,
}

#[derive(Clone, Copy)]
pub struct Object {
    pub transform: Transform,
    pub model: u32,
}

#[derive(Clone, Copy)]
pub struct Sphere {
    pub center: Vector3<f32>,
    pub radius: f32,
}

#[derive(Clone, Copy)]
pub struct Model {
    pub bounds: Sphere,
    pub mesh: Mesh,
}

pub struct DrawData {
    pub transform: Matrix4<f32>,
    pub color: Vector4<f32>,
}

pub struct Camera {
    pub translation: Translation3<f32>,
    pub yaw: f32,
    pub pitch: f32,
    pub roll: f32,
    pub fovy: f32,
    pub znear: f32,
    pub zfar: f32,
}

impl Camera {
    pub fn rotation(&self) -> Rotation3<f32> {
        Rotation3::from_axis_angle(&Vector3::y_axis(), self.yaw)
            * Rotation3::from_axis_angle(&Vector3::x_axis(), self.pitch)
            * Rotation3::from_axis_angle(&Vector3::z_axis(), self.roll)
    }
}

pub struct MeshData {
    pub vertices: Vec<Vertex>,
    pub indices: Vec<u32>,
}

pub struct ControlState {
    pub forward: bool,
    pub backward: bool,
    pub left: bool,
    pub right: bool,
}
