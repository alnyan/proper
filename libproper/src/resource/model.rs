use std::sync::Arc;

use nalgebra::Point3;
use vulkano::{
    buffer::{BufferUsage, ImmutableBuffer},
    device::Queue,
    sync::GpuFuture,
};

use crate::render::Vertex;

pub enum Storage {
    Host(String),
    Device(Arc<ImmutableBuffer<[Vertex]>>),
}

pub struct Model {
    #[allow(dead_code)]
    gfx_queue: Arc<Queue>,
    data: Storage,
}

impl Model {
    pub fn new<I>(gfx_queue: Arc<Queue>, vertices: I) -> Self
    where
        I: IntoIterator<Item = Vertex>,
        I::IntoIter: ExactSizeIterator,
    {
        let (buffer, init) =
            ImmutableBuffer::from_iter(vertices, BufferUsage::vertex_buffer(), gfx_queue.clone())
                .unwrap();

        init.then_signal_fence_and_flush()
            .unwrap()
            .wait(None)
            .unwrap();

        Self {
            data: Storage::Device(buffer),
            gfx_queue,
        }
    }

    pub fn host(gfx_queue: Arc<Queue>, path: &str) -> Self {
        Self {
            data: Storage::Host(path.to_owned()),
            gfx_queue,
        }
    }

    pub const fn is_loaded(&self) -> bool {
        matches!(self.data, Storage::Device(_))
    }

    pub const fn data(&self) -> Option<&Arc<ImmutableBuffer<[Vertex]>>> {
        if let Storage::Device(data) = &self.data {
            Some(data)
        } else {
            None
        }
    }

    pub fn load(&mut self) {
        if let Storage::Host(_path) = &self.data {
            todo!()
        }
    }

    // Helper constructors for debugging
    pub fn triangle(gfx_queue: Arc<Queue>) -> Self {
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

        Self::new(gfx_queue, vertices)
    }

    pub fn cube(gfx_queue: Arc<Queue>) -> Self {
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
        ];

        Self::new(gfx_queue, vertices)
    }
}
