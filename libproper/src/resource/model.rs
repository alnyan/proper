use std::{sync::Arc, io::BufReader, fs::File, path::Path};

use nalgebra::Point2;
use obj::{Obj, TexturedVertex};
use vulkano::{
    buffer::{BufferUsage, ImmutableBuffer},
    device::Queue,
    sync::GpuFuture
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

    pub fn load_to_device(gfx_queue: Arc<Queue>, path: &str, material_template_id: MaterialTemplateId) -> Result<Self, Error> {
        let data = Self::load_obj(gfx_queue.clone(), path)?;
        Ok(Self {
            data,
            gfx_queue,
            material_template_id
        })
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

    pub fn load(&mut self) -> Result<(), Error> {
        if let Storage::Host(path) = &self.data {
            self.data = Self::load_obj(self.gfx_queue.clone(), path)?;
            Ok(())
        } else {
            Err(Error::AlreadyLoaded)
        }
    }

    fn load_obj<P: AsRef<Path>>(gfx_queue: Arc<Queue>, path: P) -> Result<Storage, Error> {
        let input = BufReader::new(File::open(path).unwrap());
        let obj: Obj<TexturedVertex> = obj::load_obj(input).unwrap();

        let vertices = obj.indices.iter().map(|&i| {
            let v = obj.vertices[i as usize];
            Vertex {
                v_position: v.position.into(),
                v_normal: v.normal.into(),
                v_tex_coord: Point2::new(v.texture[0], v.texture[1]),
            }
        });

        let (buffer, init) =
            ImmutableBuffer::from_iter(vertices, BufferUsage::vertex_buffer(), gfx_queue)?;

        init.then_signal_fence_and_flush()?.wait(None).unwrap();

        Ok(Storage::Device(buffer))
    }

    // // Helper constructors for debugging
    // pub fn triangle(
    //     gfx_queue: Arc<Queue>,
    //     material_template_id: MaterialTemplateId,
    // ) -> Result<Self, Error> {
    //     let vertices = vec![
    //         Vertex {
    //             v_position: Point3::new(-0.5, -0.5, 0.0),
    //         },
    //         Vertex {
    //             v_position: Point3::new(0.0, 0.5, 0.0),
    //         },
    //         Vertex {
    //             v_position: Point3::new(0.5, -0.5, 0.0),
    //         },
    //     ];

    //     Self::new(gfx_queue, vertices, material_template_id)
    // }

    // pub fn cube(
    //     gfx_queue: Arc<Queue>,
    //     material_template_id: MaterialTemplateId,
    // ) -> Result<Self, Error> {
    //     let vertices = vec![
    //         // Front
    //         Vertex {
    //             v_position: Point3::new(-0.5, -0.5, -0.5),
    //         },
    //         Vertex {
    //             v_position: Point3::new(0.5, -0.5, -0.5),
    //         },
    //         Vertex {
    //             v_position: Point3::new(0.5, 0.5, -0.5),
    //         },
    //         Vertex {
    //             v_position: Point3::new(0.5, 0.5, -0.5),
    //         },
    //         Vertex {
    //             v_position: Point3::new(-0.5, 0.5, -0.5),
    //         },
    //         Vertex {
    //             v_position: Point3::new(-0.5, -0.5, -0.5),
    //         },
    //         // Right
    //         Vertex {
    //             v_position: Point3::new(0.5, -0.5, -0.5),
    //         },
    //         Vertex {
    //             v_position: Point3::new(0.5, -0.5, 0.5),
    //         },
    //         Vertex {
    //             v_position: Point3::new(0.5, 0.5, 0.5),
    //         },
    //         Vertex {
    //             v_position: Point3::new(0.5, 0.5, 0.5),
    //         },
    //         Vertex {
    //             v_position: Point3::new(0.5, 0.5, -0.5),
    //         },
    //         Vertex {
    //             v_position: Point3::new(0.5, -0.5, -0.5),
    //         },
    //         // Back
    //         Vertex {
    //             v_position: Point3::new(-0.5, -0.5, 0.5),
    //         },
    //         Vertex {
    //             v_position: Point3::new(0.5, -0.5, 0.5),
    //         },
    //         Vertex {
    //             v_position: Point3::new(0.5, 0.5, 0.5),
    //         },
    //         Vertex {
    //             v_position: Point3::new(0.5, 0.5, 0.5),
    //         },
    //         Vertex {
    //             v_position: Point3::new(-0.5, 0.5, 0.5),
    //         },
    //         Vertex {
    //             v_position: Point3::new(-0.5, -0.5, 0.5),
    //         },
    //         // Left
    //         Vertex {
    //             v_position: Point3::new(-0.5, -0.5, -0.5),
    //         },
    //         Vertex {
    //             v_position: Point3::new(-0.5, -0.5, 0.5),
    //         },
    //         Vertex {
    //             v_position: Point3::new(-0.5, 0.5, 0.5),
    //         },
    //         Vertex {
    //             v_position: Point3::new(-0.5, 0.5, 0.5),
    //         },
    //         Vertex {
    //             v_position: Point3::new(-0.5, 0.5, -0.5),
    //         },
    //         Vertex {
    //             v_position: Point3::new(-0.5, -0.5, -0.5),
    //         },
    //     ];

    //     Self::new(gfx_queue, vertices, material_template_id)
    // }
}
