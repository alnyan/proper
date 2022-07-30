use bytemuck::{Pod, Zeroable};
use nalgebra::{Point3, Vector3, Point2};

pub mod context;
pub mod frame;
pub mod shader;
pub mod system;

#[repr(C)]
#[derive(Default, Clone, Copy, Zeroable, Pod)]
pub struct Vertex {
    pub v_position: Point3<f32>,
    pub v_normal: Vector3<f32>,
    pub v_tex_coord: Point2<f32>
}

#[repr(C)]
#[derive(Default, Clone, Copy, Zeroable, Pod)]
pub struct SimpleVertex {
    pub v_position: Point3<f32>
}

vulkano::impl_vertex!(Vertex, v_position, v_normal, v_tex_coord);
vulkano::impl_vertex!(SimpleVertex, v_position);
