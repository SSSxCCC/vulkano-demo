use std::sync::Arc;

use vulkano::{instance::{Instance, InstanceExtensions}, swapchain::{SurfaceCreationError, Surface}, VulkanLibrary};
use vulkano_app::platform::Platform;
use winit::window::Window;

pub struct WindowsPlatform {
    window: Arc<Window>,
}

impl WindowsPlatform {
    pub fn new(window: Arc<Window>) -> WindowsPlatform {
        WindowsPlatform { window }
    }
}

impl Platform for WindowsPlatform {
    fn required_extensions(&self, library: &Arc<VulkanLibrary>) -> InstanceExtensions {
        vulkano_win::required_extensions(library)
    }

    fn create_surface(&self, instance: Arc<Instance>) -> Result<Arc<Surface>, SurfaceCreationError> {
        vulkano_win::create_surface_from_winit(self.window.clone(), instance)
    }

    fn get_surface_size(&self) -> [u32; 2] {
        [self.window.inner_size().width, self.window.inner_size().height]
    }
}