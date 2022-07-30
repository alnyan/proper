use nalgebra::{Matrix4, Point3, Vector3};

use crate::error::Error;

use super::scene::MeshObject;

pub struct Entity {
    position: Point3<f32>,
    mesh: MeshObject,
}

unsafe impl Send for Entity {}
unsafe impl Sync for Entity {}

impl Entity {
    pub fn new_with_mesh(position: Point3<f32>, mut mesh: MeshObject) -> Result<Self, Error> {
        let transform = Self::create_transform(Vector3::new(position.x, position.y, position.z));

        mesh.update_transform(&transform)?;

        Ok(Self { position, mesh })
    }

    #[inline]
    pub const fn position(&self) -> &Point3<f32> {
        &self.position
    }

    #[inline]
    pub const fn mesh(&self) -> &MeshObject {
        &self.mesh
    }

    fn create_transform(translation: Vector3<f32>) -> Matrix4<f32> {
        Matrix4::new_translation(&translation)
    }
}
