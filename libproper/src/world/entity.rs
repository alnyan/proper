use std::{
    ops::DerefMut,
    sync::{Arc, Mutex},
};

use nalgebra::{Matrix4, Point3, Vector3};
use vulkano::device::Queue;

use crate::{
    error::Error,
    resource::{
        material::{MaterialInstanceCreateInfo, MaterialRegistry},
        model::Model,
    },
};

use super::scene::MeshObject;

pub struct Entity {
    position: Point3<f32>,
    mesh: Option<MeshObject>,
    mesh_parameters: Option<MeshParameters>,
}

pub struct MeshParameters {
    pub material_create_info: MaterialInstanceCreateInfo,
    pub model: Model,
}

impl Entity {
    pub fn new_with_mesh(position: Point3<f32>, mut mesh: MeshObject) -> Result<Self, Error> {
        let transform = Self::create_transform(Vector3::new(position.x, position.y, position.z));

        mesh.update_transform(&transform)?;

        Ok(Self {
            position,
            mesh: Some(mesh),
            mesh_parameters: None,
        })
    }

    pub fn new_dynamic(position: Point3<f32>, params: MeshParameters) -> Self {
        Self {
            position,
            mesh: None,
            mesh_parameters: Some(params),
        }
    }

    pub fn instantiate<I: DerefMut<Target = MaterialRegistry>>(
        &mut self,
        gfx_queue: Arc<Queue>,
        material_registry: &mut I,
    ) -> Result<(), Error> {
        let mut mesh_params = self.mesh_parameters.take().unwrap();

        mesh_params.model.load(gfx_queue.clone())?;

        let mut mesh = MeshObject::new(
            gfx_queue,
            Arc::new(mesh_params.model),
            material_registry,
            mesh_params.material_create_info,
        )?;

        let transform = Self::create_transform(Vector3::new(
            self.position.x,
            self.position.y,
            self.position.z,
        ));
        mesh.update_transform(&transform)?;

        self.mesh = Some(mesh);

        Ok(())
    }

    #[inline]
    pub const fn position(&self) -> &Point3<f32> {
        &self.position
    }

    #[inline]
    pub const fn mesh(&self) -> Option<&MeshObject> {
        self.mesh.as_ref()
    }

    fn create_transform(translation: Vector3<f32>) -> Matrix4<f32> {
        Matrix4::new_translation(&translation)
    }
}
