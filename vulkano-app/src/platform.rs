use std::sync::Arc;

use vulkano::{instance::{Instance, InstanceExtensions}, swapchain::{Surface, SurfaceCreationError}, VulkanLibrary};

pub trait Platform {
    fn required_extensions(&self, library: &Arc<VulkanLibrary>) -> InstanceExtensions;
    fn create_surface(&self, instance: Arc<Instance>) -> Result<Arc<Surface>, SurfaceCreationError>;
    fn get_surface_size(&self) -> [u32; 2];
}
