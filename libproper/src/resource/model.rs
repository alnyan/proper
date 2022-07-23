use std::sync::Arc;

use nalgebra::Point3;
use vulkano::{
    buffer::{BufferUsage, ImmutableBuffer},
    device::Queue,
    sync::GpuFuture,
};

use crate::{error::Error, render::Vertex};

use super::material::MaterialTemplateId;

pub enum Storage {
    Host(String),
    Device(Arc<ImmutableBuffer<[Vertex]>>),
}

pub struct Model {
    #[allow(dead_code)]
    gfx_queue: Arc<Queue>,
    data: Storage,
    material_template_id: MaterialTemplateId,
}

impl Model {
    pub fn new<I>(
        gfx_queue: Arc<Queue>,
        vertices: I,
        material_template_id: MaterialTemplateId,
    ) -> Result<Self, Error>
    where
        I: IntoIterator<Item = Vertex>,
        I::IntoIter: ExactSizeIterator,
    {
        let (buffer, init) =
            ImmutableBuffer::from_iter(vertices, BufferUsage::vertex_buffer(), gfx_queue.clone())?;

        init.then_signal_fence_and_flush()?.wait(None).unwrap();

        Ok(Self {
            data: Storage::Device(buffer),
            gfx_queue,
            material_template_id,
        })
    }

    pub fn host(
        gfx_queue: Arc<Queue>,
        path: &str,
        material_template_id: MaterialTemplateId,
    ) -> Self {
        Self {
            data: Storage::Host(path.to_owned()),
            gfx_queue,
            material_template_id,
        }
    }

    #[inline]
    pub const fn is_loaded(&self) -> bool {
        matches!(self.data, Storage::Device(_))
    }

    #[inline]
    pub const fn data(&self) -> Option<&Arc<ImmutableBuffer<[Vertex]>>> {
        if let Storage::Device(data) = &self.data {
            Some(data)
        } else {
            None
        }
    }

    #[inline]
    pub const fn material_template_id(&self) -> MaterialTemplateId {
        self.material_template_id
    }

    pub fn load(&mut self) {
        if let Storage::Host(_path) = &self.data {
            todo!()
        }
    }

    // Helper constructors for debugging
    pub fn triangle(
        gfx_queue: Arc<Queue>,
        material_template_id: MaterialTemplateId,
    ) -> Result<Self, Error> {
        let vertices = vec![
            Vertex {
                v_position: Point3::new(-0.5, -0.5, 0.0),
            },
            Vertex {
                v_position: Point3::new(0.0, 0.5, 0.0),
            },
            Vertex {
                v_position: Point3::new(0.5, -0.5, 0.0),
            },
        ];

        Self::new(gfx_queue, vertices, material_template_id)
    }

    pub fn cube(
        gfx_queue: Arc<Queue>,
        material_template_id: MaterialTemplateId,
    ) -> Result<Self, Error> {
        let vertices = vec![
            // Front
            Vertex {
                v_position: Point3::new(-0.5, -0.5, -0.5),
            },
            Vertex {
                v_position: Point3::new(0.5, -0.5, -0.5),
            },
            Vertex {
                v_position: Point3::new(0.5, 0.5, -0.5),
            },
            Vertex {
                v_position: Point3::new(0.5, 0.5, -0.5),
            },
            Vertex {
                v_position: Point3::new(-0.5, 0.5, -0.5),
            },
            Vertex {
                v_position: Point3::new(-0.5, -0.5, -0.5),
            },
            // Right
            Vertex {
                v_position: Point3::new(0.5, -0.5, -0.5),
            },
            Vertex {
                v_position: Point3::new(0.5, -0.5, 0.5),
            },
            Vertex {
                v_position: Point3::new(0.5, 0.5, 0.5),
            },
            Vertex {
                v_position: Point3::new(0.5, 0.5, 0.5),
            },
            Vertex {
                v_position: Point3::new(0.5, 0.5, -0.5),
            },
            Vertex {
                v_position: Point3::new(0.5, -0.5, -0.5),
            },
            // Back
            Vertex {
                v_position: Point3::new(-0.5, -0.5, 0.5),
            },
            Vertex {
                v_position: Point3::new(0.5, -0.5, 0.5),
            },
            Vertex {
                v_position: Point3::new(0.5, 0.5, 0.5),
            },
            Vertex {
                v_position: Point3::new(0.5, 0.5, 0.5),
            },
            Vertex {
                v_position: Point3::new(-0.5, 0.5, 0.5),
            },
            Vertex {
                v_position: Point3::new(-0.5, -0.5, 0.5),
            },
            // Left
            Vertex {
                v_position: Point3::new(-0.5, -0.5, -0.5),
            },
            Vertex {
                v_position: Point3::new(-0.5, -0.5, 0.5),
            },
            Vertex {
                v_position: Point3::new(-0.5, 0.5, 0.5),
            },
            Vertex {
                v_position: Point3::new(-0.5, 0.5, 0.5),
            },
            Vertex {
                v_position: Point3::new(-0.5, 0.5, -0.5),
            },
            Vertex {
                v_position: Point3::new(-0.5, -0.5, -0.5),
            },
        ];

        Self::new(gfx_queue, vertices, material_template_id)
    }
}
