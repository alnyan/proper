use nalgebra::{Matrix4, Point3, Vector3};

use super::scene::MeshObject;

pub struct Entity {
    position: Point3<f32>,
    mesh: Option<MeshObject>
}

impl Entity {
    pub fn new(position: Point3<f32>, mut mesh: Option<MeshObject>) -> Self {
        let transform = Self::create_transform(Vector3::new(position.x, position.y, position.z));

        if let Some(mesh) = mesh.as_mut() {
            mesh.update_transform(&transform);
        }

        Self {
            position,
            mesh
        }
    }

    pub const fn position(&self) -> &Point3<f32> {
        &self.position
    }

    pub const fn mesh(&self) -> Option<&MeshObject> {
        self.mesh.as_ref()
    }

    fn create_transform(translation: Vector3<f32>) -> Matrix4<f32> {
        Matrix4::new_translation(&translation)
    }
}