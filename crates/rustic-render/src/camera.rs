//! Camera registry. See `PLAN.md` Section 7.
//!
//! Cameras own a 2D affine view (position, zoom, rotation) over the
//! 1280x720 logical baseline. The renderer turns each camera into a
//! view-projection matrix at draw time.

use glam::{Mat4, Vec2};
use rustic_core::ids::CameraId;

const FNF_WIDTH: f32 = 1280.0;
const FNF_HEIGHT: f32 = 720.0;

#[derive(Debug, Clone)]
#[non_exhaustive]
pub struct Camera {
    pub id: CameraId,
    pub name: &'static str,
    pub position: Vec2,
    pub zoom: f32,
    pub rotation: f32,
    /// Order key. Lower draws first; ties broken by registration order.
    pub order: i32,
}

impl Camera {
    pub const fn new(id: CameraId, name: &'static str, order: i32) -> Self {
        Self {
            id,
            name,
            position: Vec2::ZERO,
            zoom: 1.0,
            rotation: 0.0,
            order,
        }
    }

    /// View-projection matrix for the 1280x720 logical baseline.
    /// Top-left origin so FNF coordinates port directly: x grows right, y grows down.
    pub fn view_proj(&self, baseline_w: f32, baseline_h: f32) -> Mat4 {
        let proj = Mat4::orthographic_rh(0.0, baseline_w, baseline_h, 0.0, -1.0, 1.0);
        let center = Vec2::new(baseline_w * 0.5, baseline_h * 0.5);
        let translate = Mat4::from_translation((-self.position).extend(0.0));
        let scale = Mat4::from_scale(glam::Vec3::new(self.zoom, self.zoom, 1.0));
        let rot = Mat4::from_rotation_z(self.rotation);
        let origin_to_pivot = Mat4::from_translation(center.extend(0.0));
        proj * origin_to_pivot * rot * scale * translate
    }
}

#[derive(Debug, Default)]
pub struct CameraRegistry {
    cameras: Vec<Camera>,
}

impl CameraRegistry {
    pub fn new() -> Self {
        Self {
            cameras: Vec::new(),
        }
    }

    /// Initialize the base-FNF camera set: `camGame`, `camHUD`, `camOther`.
    pub fn with_default_fnf() -> Self {
        let mut r = Self::new();
        r.add(default_fnf_camera(CameraId(0), "camGame", 0));
        r.add(default_fnf_camera(CameraId(1), "camHUD", 1));
        r.add(default_fnf_camera(CameraId(2), "camOther", 2));
        r
    }

    pub fn add(&mut self, camera: Camera) -> CameraId {
        let id = camera.id;
        self.cameras.push(camera);
        id
    }

    pub fn get(&self, id: CameraId) -> Option<&Camera> {
        self.cameras.iter().find(|c| c.id == id)
    }

    pub fn get_mut(&mut self, id: CameraId) -> Option<&mut Camera> {
        self.cameras.iter_mut().find(|c| c.id == id)
    }

    pub fn iter(&self) -> impl Iterator<Item = &Camera> {
        self.cameras.iter()
    }

    pub fn order_key(&self, id: CameraId) -> i32 {
        self.get(id).map(|c| c.order).unwrap_or(i32::MAX)
    }

    pub fn len(&self) -> usize {
        self.cameras.len()
    }
    pub fn is_empty(&self) -> bool {
        self.cameras.is_empty()
    }
}

fn default_fnf_camera(id: CameraId, name: &'static str, order: i32) -> Camera {
    let mut camera = Camera::new(id, name, order);
    camera.position = Vec2::new(FNF_WIDTH * 0.5, FNF_HEIGHT * 0.5);
    camera
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use super::*;

    #[test]
    fn default_fnf_cameras_are_in_order() {
        let r = CameraRegistry::with_default_fnf();
        let names: Vec<_> = r.iter().map(|c| c.name).collect();
        assert_eq!(names, vec!["camGame", "camHUD", "camOther"]);
    }

    #[test]
    fn default_fnf_cameras_start_unscrolled() {
        let r = CameraRegistry::with_default_fnf();
        let cam = r.get(CameraId(0)).unwrap();
        let m = cam.view_proj(1280.0, 720.0);

        let top_left = m * glam::Vec4::new(0.0, 0.0, 0.0, 1.0);
        assert!((top_left.x + 1.0).abs() < 1e-4);
        assert!((top_left.y - 1.0).abs() < 1e-4);

        let center = m * glam::Vec4::new(640.0, 360.0, 0.0, 1.0);
        assert!(center.x.abs() < 1e-4);
        assert!(center.y.abs() < 1e-4);
    }

    #[test]
    fn view_proj_centers_camera_position_on_screen() {
        let mut cam = Camera::new(CameraId(0), "test", 0);
        cam.position = Vec2::new(740.0, 320.0);
        cam.zoom = 1.4;
        let m = cam.view_proj(1280.0, 720.0);
        // World point at the camera position should project to NDC origin (0,0).
        let v = m * glam::Vec4::new(740.0, 320.0, 0.0, 1.0);
        assert!(v.x.abs() < 1e-4);
        assert!(v.y.abs() < 1e-4);
    }

    #[test]
    fn view_proj_zooms_around_camera_position() {
        let mut cam = Camera::new(CameraId(0), "test", 0);
        cam.position = Vec2::new(740.0, 320.0);
        cam.zoom = 2.0;
        let m = cam.view_proj(1280.0, 720.0);

        let v = m * glam::Vec4::new(750.0, 320.0, 0.0, 1.0);
        assert!((v.x - (20.0 / 640.0)).abs() < 1e-4);
        assert!(v.y.abs() < 1e-4);
    }
}
