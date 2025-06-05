use std::sync::Arc;

use vulkano::buffer::{Buffer, BufferContents, BufferCreateInfo, BufferUsage, Subbuffer};
use vulkano::command_buffer::allocator::StandardCommandBufferAllocator;
use vulkano::command_buffer::{
    AutoCommandBufferBuilder, CommandBufferExecFuture, CommandBufferUsage,
    PrimaryAutoCommandBuffer, RenderPassBeginInfo, SubpassBeginInfo, SubpassContents,
};
use vulkano::device::physical::{PhysicalDevice, PhysicalDeviceType};
use vulkano::device::{
    Device, DeviceCreateInfo, DeviceExtensions, Queue, QueueCreateInfo, QueueFlags,
};
use vulkano::image::view::ImageView;
use vulkano::image::{Image, ImageUsage};
use vulkano::instance::{Instance, InstanceCreateInfo};
use vulkano::memory::allocator::{AllocationCreateInfo, MemoryTypeFilter, StandardMemoryAllocator};
use vulkano::pipeline::graphics::color_blend::{ColorBlendAttachmentState, ColorBlendState};
use vulkano::pipeline::graphics::input_assembly::InputAssemblyState;
use vulkano::pipeline::graphics::multisample::MultisampleState;
use vulkano::pipeline::graphics::rasterization::RasterizationState;
use vulkano::pipeline::graphics::vertex_input::{Vertex, VertexDefinition};
use vulkano::pipeline::graphics::viewport::{Scissor, Viewport, ViewportState};
use vulkano::pipeline::graphics::GraphicsPipelineCreateInfo;
use vulkano::pipeline::layout::PipelineDescriptorSetLayoutCreateInfo;
use vulkano::pipeline::{GraphicsPipeline, PipelineLayout, PipelineShaderStageCreateInfo};
use vulkano::render_pass::{Framebuffer, FramebufferCreateInfo, RenderPass, Subpass};
use vulkano::shader::ShaderModule;
use vulkano::swapchain::{
    self, PresentFuture, Surface, Swapchain, SwapchainAcquireFuture, SwapchainCreateInfo,
    SwapchainPresentInfo,
};
use vulkano::sync::future::{FenceSignalFuture, JoinFuture};
use vulkano::sync::{self, GpuFuture};
use vulkano::{Validated, VulkanError, VulkanLibrary};
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
    swapchain_images: Vec<Arc<Image>>,
    render_pass: Arc<RenderPass>,
    framebuffers: Vec<Arc<Framebuffer>>,
    memory_allocator: Arc<StandardMemoryAllocator>,

    vertex_buffer: Subbuffer<[MyVertex]>,
    vs: Arc<ShaderModule>,
    fs: Arc<ShaderModule>,
    viewport: Viewport,
    pipeline: Arc<GraphicsPipeline>,
    command_buffer_allocator: Arc<StandardCommandBufferAllocator>,
    command_buffers: Vec<Arc<PrimaryAutoCommandBuffer>>,

    window_resized: bool,
    recreate_swapchain: bool,
    frames_in_flight: usize,
    fences: Vec<
        Option<
            Arc<
                FenceSignalFuture<
                    PresentFuture<
                        CommandBufferExecFuture<
                            JoinFuture<Box<dyn GpuFuture>, SwapchainAcquireFuture>,
                        >,
                    >,
                >,
            >,
        >,
    >, // TODO: simplify
    previous_fence_i: u32,
}

impl VulkanApp {
    pub fn new(window: Arc<Window>) -> VulkanApp {
        let library = vulkano::VulkanLibrary::new().expect("no local Vulkan library/DLL");
        let required_extensions = Surface::required_extensions(window.as_ref()).unwrap();
        let instance = Instance::new(
            library.clone(),
            InstanceCreateInfo {
                enabled_extensions: required_extensions,
                ..Default::default()
            },
        )
        .expect("failed to create instance");

        let surface = Surface::from_window(instance.clone(), window.clone())
            .expect("failed to create surface");

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
            let image_format = physical_device
                .surface_formats(&surface, Default::default())
                .unwrap()[0]
                .0;

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

        let memory_allocator = Arc::new(StandardMemoryAllocator::new_default(device.clone()));

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
            memory_allocator.clone(),
            BufferCreateInfo {
                usage: BufferUsage::VERTEX_BUFFER,
                ..Default::default()
            },
            AllocationCreateInfo {
                memory_type_filter: MemoryTypeFilter::PREFER_DEVICE
                    | MemoryTypeFilter::HOST_SEQUENTIAL_WRITE,
                ..Default::default()
            },
            vec![vertex1, vertex2, vertex3].into_iter(),
        )
        .unwrap();

        let vs = vs::load(device.clone()).expect("failed to create shader module");
        let fs = fs::load(device.clone()).expect("failed to create shader module");

        let viewport = Viewport {
            offset: [0.0, 0.0],
            extent: window.inner_size().into(),
            depth_range: 0.0..=1.0,
        };

        let pipeline = Self::get_pipeline(
            device.clone(),
            vs.clone(),
            fs.clone(),
            render_pass.clone(),
            viewport.clone(),
        );

        let command_buffer_allocator = Arc::new(StandardCommandBufferAllocator::new(
            device.clone(),
            Default::default(),
        ));

        let command_buffers = Self::get_command_buffers(
            command_buffer_allocator.clone(),
            &queue,
            &pipeline,
            &framebuffers,
            &vertex_buffer,
        );

        let num_swapchain_images = swapchain_images.len();
        VulkanApp {
            window,
            library,
            instance,
            surface,
            physical_device,
            device,
            queue,
            swapchain,
            swapchain_images,
            render_pass,
            framebuffers,
            memory_allocator,
            vertex_buffer,
            vs,
            fs,
            viewport,
            pipeline,
            command_buffer_allocator,
            command_buffers,
            window_resized: false,
            recreate_swapchain: false,
            frames_in_flight: num_swapchain_images,
            fences: vec![None; num_swapchain_images],
            previous_fence_i: 0,
        }
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
                    format: swapchain.image_format(), // set the format the same as the swapchain
                    samples: 1,
                    load_op: Clear,
                    store_op: Store,
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
        images: &[Arc<Image>],
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
        let vs = vs.entry_point("main").unwrap();
        let fs = fs.entry_point("main").unwrap();
        let vertex_input_state = MyVertex::per_vertex().definition(&vs).unwrap();
        let stages = [
            PipelineShaderStageCreateInfo::new(vs),
            PipelineShaderStageCreateInfo::new(fs),
        ];
        let layout = PipelineLayout::new(
            device.clone(),
            PipelineDescriptorSetLayoutCreateInfo::from_stages(&stages)
                .into_pipeline_layout_create_info(device.clone())
                .unwrap(),
        )
        .unwrap();
        let subpass = Subpass::from(render_pass, 0).unwrap();
        GraphicsPipeline::new(
            device.clone(),
            None,
            GraphicsPipelineCreateInfo {
                stages: stages.into_iter().collect(),
                vertex_input_state: Some(vertex_input_state),
                input_assembly_state: Some(InputAssemblyState::default()),
                rasterization_state: Some(RasterizationState::default()),
                multisample_state: Some(MultisampleState::default()),
                color_blend_state: Some(ColorBlendState::with_attachment_states(
                    subpass.num_color_attachments(),
                    ColorBlendAttachmentState::default(),
                )),
                viewport_state: Some(ViewportState {
                    viewports: [viewport].into_iter().collect(),
                    scissors: [Scissor::default()].into_iter().collect(),
                    ..Default::default()
                }),
                subpass: Some(subpass.into()),
                ..GraphicsPipelineCreateInfo::layout(layout)
            },
        )
        .unwrap()
    }

    fn get_command_buffers(
        command_buffer_allocator: Arc<StandardCommandBufferAllocator>,
        queue: &Arc<Queue>,
        pipeline: &Arc<GraphicsPipeline>,
        framebuffers: &[Arc<Framebuffer>],
        vertex_buffer: &Subbuffer<[MyVertex]>,
    ) -> Vec<Arc<PrimaryAutoCommandBuffer>> {
        framebuffers
            .iter()
            .map(|framebuffer| {
                let mut builder = AutoCommandBufferBuilder::primary(
                    command_buffer_allocator.clone(),
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
                        SubpassBeginInfo {
                            contents: SubpassContents::Inline,
                            ..Default::default()
                        },
                    )
                    .unwrap()
                    .bind_pipeline_graphics(pipeline.clone())
                    .unwrap()
                    .bind_vertex_buffers(0, vertex_buffer.clone())
                    .unwrap();

                unsafe { builder.draw(vertex_buffer.len() as u32, 1, 0, 0) }.unwrap();
                builder.end_render_pass(Default::default()).unwrap();

                builder.build().unwrap()
            })
            .collect()
    }

    pub fn window(&self) -> &Arc<Window> {
        &self.window
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
                Err(e) => panic!("failed to recreate swapchain: {e}"),
            };
            self.swapchain = new_swapchain;
            let new_framebuffers = Self::get_framebuffers(&new_images, self.render_pass.clone());

            if self.window_resized {
                self.window_resized = false;

                self.viewport.extent = new_dimensions.into();
                let new_pipeline = Self::get_pipeline(
                    self.device.clone(),
                    self.vs.clone(),
                    self.fs.clone(),
                    self.render_pass.clone(),
                    self.viewport.clone(),
                );
                self.command_buffers = Self::get_command_buffers(
                    self.command_buffer_allocator.clone(),
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
                Err(Validated::Error(VulkanError::OutOfDate)) => {
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
            .then_execute(
                self.queue.clone(),
                self.command_buffers[image_i as usize].clone(),
            )
            .unwrap()
            .then_swapchain_present(
                self.queue.clone(),
                SwapchainPresentInfo::swapchain_image_index(self.swapchain.clone(), image_i),
            )
            .then_signal_fence_and_flush();

        self.fences[image_i as usize] = match future {
            Ok(value) => Some(Arc::new(value)),
            Err(Validated::Error(VulkanError::OutOfDate)) => {
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
