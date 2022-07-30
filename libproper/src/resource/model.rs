use std::{
    collections::BTreeMap,
    fs::File,
    io::BufReader,
    path::{Path, PathBuf},
    sync::Arc,
};

use nalgebra::Point2;
use obj::{Obj, TexturedVertex};
use vulkano::{
    buffer::{BufferUsage, ImmutableBuffer},
    device::Queue,
    sync::GpuFuture,
};

use crate::{error::Error, render::Vertex, world::scene::MeshObject};

use super::material::{MaterialInstanceCreateInfo, MaterialTemplate};

pub struct Model {
    data: Arc<ImmutableBuffer<[Vertex]>>,
    material_template: Arc<dyn MaterialTemplate>,
}

pub struct ModelRegistry {
    gfx_queue: Arc<Queue>,
    data: BTreeMap<String, Arc<Model>>,
}

impl Model {
    pub fn new<I>(
        gfx_queue: Arc<Queue>,
        vertices: I,
        material_template: Arc<dyn MaterialTemplate>,
    ) -> Result<Self, Error>
    where
        I: IntoIterator<Item = Vertex>,
        I::IntoIter: ExactSizeIterator,
    {
        let (buffer, init) =
            ImmutableBuffer::from_iter(vertices, BufferUsage::vertex_buffer(), gfx_queue)?;

        init.then_signal_fence_and_flush()?.wait(None).unwrap();

        Ok(Self {
            data: buffer,
            material_template,
        })
    }

    pub fn load_to_device<P: AsRef<Path>>(
        gfx_queue: Arc<Queue>,
        path: P,
        material_template: Arc<dyn MaterialTemplate>,
    ) -> Result<Self, Error> {
        let data = Self::load_obj(gfx_queue, path)?;
        Ok(Self {
            data,
            material_template,
        })
    }

    #[inline]
    pub const fn data(&self) -> &Arc<ImmutableBuffer<[Vertex]>> {
        &self.data
    }

    #[inline]
    pub const fn material_template(&self) -> &Arc<dyn MaterialTemplate> {
        &self.material_template
    }

    fn load_obj<P: AsRef<Path>>(
        gfx_queue: Arc<Queue>,
        path: P,
    ) -> Result<Arc<ImmutableBuffer<[Vertex]>>, Error> {
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

        Ok(buffer)
    }
}

impl ModelRegistry {
    pub fn new(gfx_queue: Arc<Queue>) -> Self {
        Self {
            gfx_queue,
            data: BTreeMap::new(),
        }
    }

    pub fn create_mesh_object(
        &mut self,
        name: &str,
        material_template: Arc<dyn MaterialTemplate>,
        material_create_info: MaterialInstanceCreateInfo,
    ) -> Result<MeshObject, Error> {
        let model = self.get_or_load(name, material_template.clone())?;
        let mesh = MeshObject::new(
            self.gfx_queue.clone(),
            model,
            material_template,
            material_create_info,
        )?;

        Ok(mesh)
    }

    pub fn get_or_load(
        &mut self,
        name: &str,
        material_template: Arc<dyn MaterialTemplate>,
    ) -> Result<Arc<Model>, Error> {
        if let Some(model) = self.data.get(name) {
            // TODO check material ID
            Ok(model.clone())
        } else {
            log::info!("Loading model {:?}", name);

            let filename = name.to_owned() + ".obj";
            let mut path = PathBuf::from("res/models/");
            path.push(filename);

            let data = Arc::new(Model::load_to_device(
                self.gfx_queue.clone(),
                path,
                material_template,
            )?);

            self.data.insert(name.to_owned(), data.clone());
            Ok(data)
        }
    }
}
