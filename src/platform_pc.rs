use std::sync::Arc;

use vulkano::{instance::Instance, swapchain::{Surface, SurfaceCreationError}};
use winit::window::Window;

pub struct Platform {
    window: Arc<Window>,
}

impl Platform {
    pub fn new(window: Arc<Window>) -> Platform {
        Platform { window }
    }

    pub fn create_surface(&self, instance: Arc<Instance>) -> Result<Arc<Surface>, SurfaceCreationError> {
        vulkano_win::create_surface_from_winit(self.window.clone(), instance)
    }

    pub fn get_surface_size(&self) -> [u32; 2] {
        [self.window.inner_size().width, self.window.inner_size().height]
    }
}
