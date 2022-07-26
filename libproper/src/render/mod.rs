use bytemuck::{Pod, Zeroable};
use nalgebra::Point3;

pub mod context;
pub mod frame;
pub mod gui;
pub mod scene;
pub mod shader;
pub mod system;

#[repr(C)]
#[derive(Default, Clone, Copy, Zeroable, Pod)]
pub struct Vertex {
    pub v_position: Point3<f32>,
}

vulkano::impl_vertex!(Vertex, v_position);
