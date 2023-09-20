// Copyright (c) 2017 The vulkano developers
// Licensed under the Apache License, Version 2.0
// <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT
// license <LICENSE-MIT or http://opensource.org/licenses/MIT>,
// at your option. All files in the project carrying such
// notice may not be copied, modified, or distributed except
// according to those terms.

//! This is the source code of the "Windowing" chapter at http://vulkano.rs.
//!
//! It is not commented, as the explanations can be found in the guide itself.

use std::sync::Arc;

use vulkano::VulkanLibrary;
use vulkano::buffer::{Buffer, BufferContents, BufferCreateInfo, BufferUsage, Subbuffer};
use vulkano::command_buffer::allocator::StandardCommandBufferAllocator;
use vulkano::command_buffer::{
    AutoCommandBufferBuilder, CommandBufferExecFuture, CommandBufferUsage,
    PrimaryAutoCommandBuffer, RenderPassBeginInfo, SubpassContents,
};
use vulkano::device::physical::{PhysicalDevice, PhysicalDeviceType};
use vulkano::device::{
    Device, DeviceCreateInfo, DeviceExtensions, Queue, QueueCreateInfo, QueueFlags,
};
use vulkano::image::view::ImageView;
use vulkano::image::{ImageUsage, SwapchainImage};
use vulkano::instance::{Instance, InstanceCreateInfo};
use vulkano::memory::allocator::{AllocationCreateInfo, MemoryUsage, StandardMemoryAllocator};
use vulkano::pipeline::graphics::input_assembly::InputAssemblyState;
use vulkano::pipeline::graphics::vertex_input::Vertex;
use vulkano::pipeline::graphics::viewport::{Viewport, ViewportState};
use vulkano::pipeline::GraphicsPipeline;
use vulkano::render_pass::{Framebuffer, FramebufferCreateInfo, RenderPass, Subpass};
use vulkano::shader::ShaderModule;
use vulkano::swapchain::{
    self, AcquireError, PresentFuture, Surface, Swapchain, SwapchainAcquireFuture,
    SwapchainCreateInfo, SwapchainCreationError, SwapchainPresentInfo,
};
use vulkano::sync::future::{FenceSignalFuture, JoinFuture};
use vulkano::sync::{self, FlushError, GpuFuture};
use winit::window::Window;

#[derive(BufferContents, Vertex)]
#[repr(C)]
struct MyVertex {
    #[format(R32G32_SFLOAT)]
    position: [f32; 2],
}

mod vs {
    vulkano_shaders::shader! {
        ty: "vertex",
        src: r"
            #version 460

            layout(location = 0) in vec2 position;

            void main() {
                gl_Position = vec4(position, 0.0, 1.0);
            }
        ",
    }
}

mod fs {
    vulkano_shaders::shader! {
        ty: "fragment",
        src: r"
            #version 460

            layout(location = 0) out vec4 f_color;

            void main() {
                f_color = vec4(1.0, 0.0, 0.0, 1.0);
            }
        ",
    }
}

pub struct VulkanApp {
    window: Arc<Window>,

    library: Arc<VulkanLibrary>,
    instance: Arc<Instance>,
    surface: Arc<Surface>,
    physical_device: Arc<PhysicalDevice>,
    device: Arc<Device>,
    queue: Arc<Queue>,
    swapchain: Arc<Swapchain>,
    swapchain_images: Vec<Arc<SwapchainImage>>,
    render_pass: Arc<RenderPass>,
    framebuffers: Vec<Arc<Framebuffer>>,
    memory_allocator: StandardMemoryAllocator,

    vertex_buffer: Subbuffer<[MyVertex]>,
    vs: Arc<ShaderModule>,
    fs: Arc<ShaderModule>,
    viewport: Viewport,
    pipeline: Arc<GraphicsPipeline>,
    command_buffer_allocator: StandardCommandBufferAllocator,
    command_buffers: Vec<Arc<PrimaryAutoCommandBuffer>>,

    window_resized: bool,
    recreate_swapchain: bool,
    frames_in_flight: usize,
    fences: Vec<Option<Arc<FenceSignalFuture<PresentFuture<CommandBufferExecFuture<JoinFuture<Box<dyn GpuFuture>, SwapchainAcquireFuture>>>>>>>, // TODO: simplify
    previous_fence_i: u32,
}

impl VulkanApp {
    pub fn new(window: Arc<Window>) -> VulkanApp {
        let library = vulkano::VulkanLibrary::new().expect("no local Vulkan library/DLL");
        let required_extensions = vulkano_win::required_extensions(&library);
        let instance = Instance::new(
            library.clone(),
            InstanceCreateInfo {
                enabled_extensions: required_extensions,
                ..Default::default()
            },
        )
        .expect("failed to create instance");

        let surface = vulkano_win::create_surface_from_winit(window.clone(), instance.clone()).expect("failed to create surface");

        let device_extensions = DeviceExtensions {
            khr_swapchain: true,
            ..DeviceExtensions::empty()
        };
    
        let (physical_device, queue_family_index) =
            Self::select_physical_device(&instance, &surface, &device_extensions);
    
        let (device, mut queues) = Device::new(
            physical_device.clone(),
            DeviceCreateInfo {
                queue_create_infos: vec![QueueCreateInfo {
                    queue_family_index,
                    ..Default::default()
                }],
                enabled_extensions: device_extensions, // new
                ..Default::default()
            },
        )
        .expect("failed to create device");
    
        let queue = queues.next().unwrap();
    
        let (swapchain, swapchain_images) = {
            let caps = physical_device
                .surface_capabilities(&surface, Default::default())
                .expect("failed to get surface capabilities");
    
            let dimensions = window.inner_size();
            let composite_alpha = caps.supported_composite_alpha.into_iter().next().unwrap();
            let image_format = Some(
                physical_device
                    .surface_formats(&surface, Default::default())
                    .unwrap()[0]
                    .0,
            );
    
            Swapchain::new(
                device.clone(),
                surface.clone(),
                SwapchainCreateInfo {
                    min_image_count: caps.min_image_count,
                    image_format,
                    image_extent: dimensions.into(),
                    image_usage: ImageUsage::COLOR_ATTACHMENT,
                    composite_alpha,
                    ..Default::default()
                },
            )
            .unwrap()
        };
    
        let render_pass = Self::get_render_pass(device.clone(), swapchain.clone());
        let framebuffers = Self::get_framebuffers(&swapchain_images, render_pass.clone());
    
        let memory_allocator = StandardMemoryAllocator::new_default(device.clone());
    
        let vertex1 = MyVertex {
            position: [-0.5, -0.5],
        };
        let vertex2 = MyVertex {
            position: [0.0, 0.5],
        };
        let vertex3 = MyVertex {
            position: [0.5, -0.25],
        };
        let vertex_buffer = Buffer::from_iter(
            &memory_allocator,
            BufferCreateInfo {
                usage: BufferUsage::VERTEX_BUFFER,
                ..Default::default()
            },
            AllocationCreateInfo {
                usage: MemoryUsage::Upload,
                ..Default::default()
            },
            vec![vertex1, vertex2, vertex3].into_iter(),
        )
        .unwrap();
    
        let vs = vs::load(device.clone()).expect("failed to create shader module");
        let fs = fs::load(device.clone()).expect("failed to create shader module");
    
        let viewport = Viewport {
            origin: [0.0, 0.0],
            dimensions: window.inner_size().into(),
            depth_range: 0.0..1.0,
        };
    
        let pipeline = Self::get_pipeline(
            device.clone(),
            vs.clone(),
            fs.clone(),
            render_pass.clone(),
            viewport.clone(),
        );
    
        let command_buffer_allocator =
            StandardCommandBufferAllocator::new(device.clone(), Default::default());
    
        let command_buffers = Self::get_command_buffers(
            &command_buffer_allocator,
            &queue,
            &pipeline,
            &framebuffers,
            &vertex_buffer,
        );

        let num_swapchain_images = swapchain_images.len();
        VulkanApp { window, library, instance, surface, physical_device, device, queue, swapchain, swapchain_images, render_pass, framebuffers, memory_allocator,
            vertex_buffer, vs, fs, viewport, pipeline, command_buffer_allocator, command_buffers,
            window_resized: false, recreate_swapchain: false, frames_in_flight: num_swapchain_images, fences: vec![None; num_swapchain_images], previous_fence_i: 0 }
    }

    fn select_physical_device(
        instance: &Arc<Instance>,
        surface: &Arc<Surface>,
        device_extensions: &DeviceExtensions,
    ) -> (Arc<PhysicalDevice>, u32) {
        instance
            .enumerate_physical_devices()
            .expect("failed to enumerate physical devices")
            .filter(|p| p.supported_extensions().contains(device_extensions))
            .filter_map(|p| {
                p.queue_family_properties()
                    .iter()
                    .enumerate()
                    .position(|(i, q)| {
                        q.queue_flags.contains(QueueFlags::GRAPHICS)
                            && p.surface_support(i as u32, surface).unwrap_or(false)
                    })
                    .map(|q| (p, q as u32))
            })
            .min_by_key(|(p, _)| match p.properties().device_type {
                PhysicalDeviceType::DiscreteGpu => 0,
                PhysicalDeviceType::IntegratedGpu => 1,
                PhysicalDeviceType::VirtualGpu => 2,
                PhysicalDeviceType::Cpu => 3,
                _ => 4,
            })
            .expect("no device available")
    }
    
    fn get_render_pass(device: Arc<Device>, swapchain: Arc<Swapchain>) -> Arc<RenderPass> {
        vulkano::single_pass_renderpass!(
            device,
            attachments: {
                color: {
                    load: Clear,
                    store: Store,
                    format: swapchain.image_format(), // set the format the same as the swapchain
                    samples: 1,
                },
            },
            pass: {
                color: [color],
                depth_stencil: {},
            },
        )
        .unwrap()
    }
    
    fn get_framebuffers(
        images: &[Arc<SwapchainImage>],
        render_pass: Arc<RenderPass>,
    ) -> Vec<Arc<Framebuffer>> {
        images
            .iter()
            .map(|image| {
                let view = ImageView::new_default(image.clone()).unwrap();
                Framebuffer::new(
                    render_pass.clone(),
                    FramebufferCreateInfo {
                        attachments: vec![view],
                        ..Default::default()
                    },
                )
                .unwrap()
            })
            .collect::<Vec<_>>()
    }
    
    fn get_pipeline(
        device: Arc<Device>,
        vs: Arc<ShaderModule>,
        fs: Arc<ShaderModule>,
        render_pass: Arc<RenderPass>,
        viewport: Viewport,
    ) -> Arc<GraphicsPipeline> {
        GraphicsPipeline::start()
            .vertex_input_state(MyVertex::per_vertex())
            .vertex_shader(vs.entry_point("main").unwrap(), ())
            .input_assembly_state(InputAssemblyState::new())
            .viewport_state(ViewportState::viewport_fixed_scissor_irrelevant([viewport]))
            .fragment_shader(fs.entry_point("main").unwrap(), ())
            .render_pass(Subpass::from(render_pass, 0).unwrap())
            .build(device)
            .unwrap()
    }
    
    fn get_command_buffers(
        command_buffer_allocator: &StandardCommandBufferAllocator,
        queue: &Arc<Queue>,
        pipeline: &Arc<GraphicsPipeline>,
        framebuffers: &[Arc<Framebuffer>],
        vertex_buffer: &Subbuffer<[MyVertex]>,
    ) -> Vec<Arc<PrimaryAutoCommandBuffer>> {
        framebuffers
            .iter()
            .map(|framebuffer| {
                let mut builder = AutoCommandBufferBuilder::primary(
                    command_buffer_allocator,
                    queue.queue_family_index(),
                    CommandBufferUsage::MultipleSubmit,
                )
                .unwrap();
    
                builder
                    .begin_render_pass(
                        RenderPassBeginInfo {
                            clear_values: vec![Some([0.0, 0.0, 1.0, 1.0].into())],
                            ..RenderPassBeginInfo::framebuffer(framebuffer.clone())
                        },
                        SubpassContents::Inline,
                    )
                    .unwrap()
                    .bind_pipeline_graphics(pipeline.clone())
                    .bind_vertex_buffers(0, vertex_buffer.clone())
                    .draw(vertex_buffer.len() as u32, 1, 0, 0)
                    .unwrap()
                    .end_render_pass()
                    .unwrap();
    
                Arc::new(builder.build().unwrap())
            })
            .collect()
    }

    pub fn notify_window_resized(&mut self) {
        self.window_resized = true;
    }

    pub fn draw_frame(&mut self) {
        if self.window_resized || self.recreate_swapchain {
            self.recreate_swapchain = false;

            let new_dimensions = self.window.inner_size();

            let (new_swapchain, new_images) = match self.swapchain.recreate(SwapchainCreateInfo {
                image_extent: new_dimensions.into(),
                ..self.swapchain.create_info()
            }) {
                Ok(r) => r,
                Err(SwapchainCreationError::ImageExtentNotSupported { .. }) => return,
                Err(e) => panic!("failed to recreate swapchain: {e}"),
            };
            self.swapchain = new_swapchain;
            let new_framebuffers = Self::get_framebuffers(&new_images, self.render_pass.clone());

            if self.window_resized {
                self.window_resized = false;

                self.viewport.dimensions = new_dimensions.into();
                let new_pipeline = Self::get_pipeline(
                    self.device.clone(),
                    self.vs.clone(),
                    self.fs.clone(),
                    self.render_pass.clone(),
                    self.viewport.clone(),
                );
                self.command_buffers = Self::get_command_buffers(
                    &self.command_buffer_allocator,
                    &self.queue,
                    &new_pipeline,
                    &new_framebuffers,
                    &self.vertex_buffer,
                );
            }
        }

        let (image_i, suboptimal, acquire_future) =
            match swapchain::acquire_next_image(self.swapchain.clone(), None) {
                Ok(r) => r,
                Err(AcquireError::OutOfDate) => {
                    self.recreate_swapchain = true;
                    return;
                }
                Err(e) => panic!("failed to acquire next image: {e}"),
            };

        if suboptimal {
            self.recreate_swapchain = true;
        }

        // wait for the fence related to this image to finish (normally this would be the oldest fence)
        if let Some(image_fence) = &self.fences[image_i as usize] {
            image_fence.wait(None).unwrap();
        }

        let previous_future = match self.fences[self.previous_fence_i as usize].clone() {
            // Create a NowFuture
            None => {
                let mut now = sync::now(self.device.clone());
                now.cleanup_finished();

                now.boxed()
            }
            // Use the existing FenceSignalFuture
            Some(fence) => fence.boxed(),
        };

        let future = previous_future
            .join(acquire_future)
            .then_execute(self.queue.clone(), self.command_buffers[image_i as usize].clone())
            .unwrap()
            .then_swapchain_present(
                self.queue.clone(),
                SwapchainPresentInfo::swapchain_image_index(self.swapchain.clone(), image_i),
            )
            .then_signal_fence_and_flush();

        self.fences[image_i as usize] = match future {
            Ok(value) => Some(Arc::new(value)),
            Err(FlushError::OutOfDate) => {
                self.recreate_swapchain = true;
                None
            }
            Err(e) => {
                println!("failed to flush future: {e}");
                None
            }
        };

        self.previous_fence_i = image_i;
    }
}
