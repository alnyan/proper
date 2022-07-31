use nalgebra::{Point3, Vector3};

#[derive(Default)]
pub struct Camera {
    position: Point3<f32>,
    pitch: f32,
    yaw: f32
}

impl Camera {
    #[inline]
    pub const fn position(&self) -> &Point3<f32> {
        &self.position
    }

    pub fn forward(&self) -> Vector3<f32> {
        let xzlen = self.pitch.cos();
        Vector3::new(self.yaw.cos() * xzlen, self.pitch.sin(), self.yaw.sin() * xzlen)
    }

    pub fn sideward(&self) -> Vector3<f32> {
        let xzlen = self.pitch.cos();
        Vector3::new(-self.yaw.sin() * xzlen, self.pitch.sin(), self.yaw.cos() * xzlen)
    }

    pub fn translate(&mut self, delta: Vector3<f32>) {
        self.position += delta;
    }

    pub fn reset_rotation(&mut self) {
        self.pitch = 0.0;
        self.yaw = 0.0;
    }

    pub fn rotate_angles(&mut self, pitch: f32, yaw: f32) {
        self.pitch += pitch;
        self.yaw += yaw;
    }
}
