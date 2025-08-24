use std::slice;
use std::sync::Arc;
use vulkano::buffer::{Buffer, BufferContents, BufferCreateInfo, BufferUsage};
use vulkano::device::physical::{PhysicalDevice, PhysicalDeviceType};
use vulkano::device::{
    Device, DeviceCreateInfo, DeviceExtensions, Queue, QueueCreateInfo, QueueFlags,
};
use vulkano::image::ImageUsage;
use vulkano::instance::{Instance, InstanceCreateInfo};
use vulkano::memory::allocator::{AllocationCreateInfo, DeviceLayout, MemoryTypeFilter};
use vulkano::pipeline::graphics::color_blend::{ColorBlendAttachmentState, ColorBlendState};
use vulkano::pipeline::graphics::input_assembly::InputAssemblyState;
use vulkano::pipeline::graphics::multisample::MultisampleState;
use vulkano::pipeline::graphics::rasterization::RasterizationState;
use vulkano::pipeline::graphics::vertex_input::{Vertex, VertexDefinition};
use vulkano::pipeline::graphics::viewport::{Viewport, ViewportState};
use vulkano::pipeline::graphics::GraphicsPipelineCreateInfo;
use vulkano::pipeline::layout::PipelineDescriptorSetLayoutCreateInfo;
use vulkano::pipeline::{
    DynamicState, GraphicsPipeline, PipelineLayout, PipelineShaderStageCreateInfo,
};
use vulkano::render_pass::{Framebuffer, Subpass};
use vulkano::shader::ShaderModule;
use vulkano::swapchain::{Surface, Swapchain, SwapchainCreateInfo};
use vulkano::{Validated, VulkanError, VulkanLibrary};
use vulkano_taskgraph::command_buffer::RecordingCommandBuffer;
use vulkano_taskgraph::graph::{
    AttachmentInfo, CompileInfo, ExecutableTaskGraph, ExecuteError, TaskGraph,
};
use vulkano_taskgraph::resource::{
    AccessTypes, Flight, HostAccessType, ImageLayoutType, Resources,
};
use vulkano_taskgraph::{resource_map, Id, QueueFamilyType, Task, TaskContext, TaskResult};
use winit::window::Window;

const MAX_FRAMES_IN_FLIGHT: u32 = 2;
const VALIDATEION_LAYER: &'static str = "VK_LAYER_KHRONOS_validation";

#[derive(Clone, Copy, BufferContents, Vertex)]
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

    vs: Arc<ShaderModule>,
    fs: Arc<ShaderModule>,
    viewport: Viewport,

    recreate_swapchain: bool,

    resources: Arc<Resources>,
    flight_id: Id<Flight>,
    vertex_buffer_id: Id<Buffer>,
    swapchain_id: Id<Swapchain>,

    task_graph: ExecutableTaskGraph<Self>,
    virtual_swapchain_id: Id<Swapchain>,
    virtual_framebuffer_id: Id<Framebuffer>,
}

impl VulkanApp {
    pub fn new(window: Arc<Window>) -> VulkanApp {
        let library = vulkano::VulkanLibrary::new().expect("no local Vulkan library/DLL");
        let required_extensions = Surface::required_extensions(window.as_ref()).unwrap();
        let mut enabled_layers = Vec::new();
        if library
            .layer_properties()
            .unwrap()
            .any(|layer| layer.name() == VALIDATEION_LAYER)
        {
            enabled_layers.push(VALIDATEION_LAYER.into());
        }
        let instance = Instance::new(
            library.clone(),
            InstanceCreateInfo {
                enabled_extensions: required_extensions,
                enabled_layers,
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
        log::info!(
            "Using device: {} (type: {:?})",
            physical_device.properties().device_name,
            physical_device.properties().device_type,
        );

        let (device, mut queues) = Device::new(
            physical_device.clone(),
            DeviceCreateInfo {
                queue_create_infos: vec![QueueCreateInfo {
                    queue_family_index,
                    ..Default::default()
                }],
                enabled_extensions: device_extensions,
                ..Default::default()
            },
        )
        .expect("failed to create device");

        let queue = queues.next().unwrap();

        let vs = vs::load(device.clone()).expect("failed to create shader module");
        let fs = fs::load(device.clone()).expect("failed to create shader module");

        let viewport = Viewport {
            offset: [0.0, 0.0],
            extent: window.inner_size().into(),
            depth_range: 0.0..=1.0,
        };

        let resources = Resources::new(&device, &Default::default());

        let flight_id = resources.create_flight(MAX_FRAMES_IN_FLIGHT).unwrap();

        let vertices = [
            MyVertex {
                position: [-0.5, -0.5],
            },
            MyVertex {
                position: [0.0, 0.5],
            },
            MyVertex {
                position: [0.5, -0.25],
            },
        ];
        let vertex_buffer_id: Id<Buffer> = resources
            .create_buffer(
                BufferCreateInfo {
                    usage: BufferUsage::VERTEX_BUFFER,
                    ..Default::default()
                },
                AllocationCreateInfo {
                    memory_type_filter: MemoryTypeFilter::PREFER_DEVICE
                        | MemoryTypeFilter::HOST_SEQUENTIAL_WRITE,
                    ..Default::default()
                },
                DeviceLayout::for_value(vertices.as_slice()).unwrap(),
            )
            .unwrap();
        unsafe {
            vulkano_taskgraph::execute(
                &queue,
                &resources,
                flight_id,
                |_command_buffer, task_context| {
                    task_context
                        .write_buffer::<[MyVertex]>(vertex_buffer_id, ..)?
                        .copy_from_slice(&vertices);
                    Ok(())
                },
                [(vertex_buffer_id, HostAccessType::Write)],
                [],
                [],
            )
        }
        .unwrap();

        let (swapchain_id, swapchain_format) = {
            let caps = physical_device
                .surface_capabilities(&surface, Default::default())
                .expect("failed to get surface capabilities");

            let composite_alpha = caps.supported_composite_alpha.into_iter().next().unwrap();
            let image_format = physical_device
                .surface_formats(&surface, Default::default())
                .unwrap()[0]
                .0;

            (
                resources
                    .create_swapchain(
                        flight_id,
                        surface.clone(),
                        SwapchainCreateInfo {
                            min_image_count: caps.min_image_count,
                            image_format,
                            image_extent: window.inner_size().into(),
                            image_usage: ImageUsage::COLOR_ATTACHMENT,
                            composite_alpha,
                            ..Default::default()
                        },
                    )
                    .unwrap(),
                image_format,
            )
        };

        let mut task_graph = TaskGraph::new(&resources, 1, 2);

        let virtual_swapchain_id = task_graph.add_swapchain(&SwapchainCreateInfo {
            image_format: swapchain_format,
            ..Default::default()
        });
        let virtual_framebuffer_id = task_graph.add_framebuffer();

        let render_node_id = task_graph
            .create_task_node(
                "Render",
                QueueFamilyType::Graphics,
                RenderTask {
                    swapchain_id: virtual_swapchain_id,
                    pipeline: None,
                    vertex_buffer_id,
                    vertex_count: vertices.len() as _,
                },
            )
            .framebuffer(virtual_framebuffer_id)
            .color_attachment(
                virtual_swapchain_id.current_image_id(),
                AccessTypes::COLOR_ATTACHMENT_WRITE,
                ImageLayoutType::Optimal,
                &AttachmentInfo {
                    clear: true,
                    ..Default::default()
                },
            )
            .buffer_access(vertex_buffer_id, AccessTypes::VERTEX_ATTRIBUTE_READ)
            .build();

        let mut task_graph = unsafe {
            task_graph.compile(&CompileInfo {
                queues: &[&queue],
                present_queue: Some(&queue),
                flight_id,
                ..Default::default()
            })
        }
        .unwrap();

        let node = task_graph.task_node_mut(render_node_id).unwrap();

        let pipeline = Self::create_pipeline(
            device.clone(),
            vs.clone(),
            fs.clone(),
            node.subpass().unwrap().clone(),
        );

        node.task_mut()
            .downcast_mut::<RenderTask>()
            .unwrap()
            .pipeline = Some(pipeline);

        VulkanApp {
            window,
            library,
            instance,
            surface,
            physical_device,
            device,
            queue,
            vs,
            fs,
            viewport,
            recreate_swapchain: false,

            resources,
            flight_id,
            vertex_buffer_id,
            swapchain_id,

            task_graph,
            virtual_swapchain_id,
            virtual_framebuffer_id,
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

    fn create_pipeline(
        device: Arc<Device>,
        vs: Arc<ShaderModule>,
        fs: Arc<ShaderModule>,
        subpass: Subpass,
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
                viewport_state: Some(ViewportState::default()),
                dynamic_state: [DynamicState::Viewport].into_iter().collect(),
                subpass: Some(subpass.into()),
                ..GraphicsPipelineCreateInfo::layout(layout)
            },
        )
        .unwrap()
    }

    pub fn window(&self) -> &Arc<Window> {
        &self.window
    }

    pub fn notify_window_resized(&mut self) {
        self.recreate_swapchain = true;
    }

    pub fn draw_frame(&mut self) {
        let window_size = self.window.inner_size();
        if window_size.width == 0 || window_size.height == 0 {
            log::trace!("window size is 0, skip draw");
            return;
        }

        if self.recreate_swapchain {
            self.swapchain_id = self
                .resources
                .recreate_swapchain(self.swapchain_id, |create_info| SwapchainCreateInfo {
                    image_extent: window_size.into(),
                    ..create_info
                })
                .expect("failed to recreate swapchain");

            self.recreate_swapchain = false;
            self.viewport.extent = window_size.into();
        }

        let resource_map = resource_map!(
            &self.task_graph,
            self.virtual_swapchain_id => self.swapchain_id,
        )
        .unwrap();

        let flight = self.resources.flight(self.flight_id).unwrap();
        flight.wait(None).unwrap();

        match unsafe {
            self.task_graph
                .execute(resource_map, &self, || self.window.pre_present_notify())
        } {
            Ok(_) => {}
            Err(ExecuteError::Swapchain {
                error: Validated::Error(VulkanError::OutOfDate),
                ..
            }) => {
                self.recreate_swapchain = true;
            }
            Err(e) => {
                panic!("failed to execute next frame: {e:?}");
            }
        }
    }
}

struct RenderTask {
    swapchain_id: Id<Swapchain>,
    pipeline: Option<Arc<GraphicsPipeline>>,
    vertex_buffer_id: Id<Buffer>,
    vertex_count: u32,
}

impl Task for RenderTask {
    type World = VulkanApp;

    fn clear_values(&self, clear_values: &mut vulkano_taskgraph::ClearValues<'_>) {
        clear_values.set(self.swapchain_id.current_image_id(), [0.0, 0.0, 1.0, 1.0]);
    }

    unsafe fn execute(
        &self,
        command_buffer: &mut RecordingCommandBuffer<'_>,
        _task_context: &mut TaskContext<'_>,
        world: &Self::World,
    ) -> TaskResult {
        command_buffer.set_viewport(0, slice::from_ref(&world.viewport))?;
        command_buffer.bind_pipeline_graphics(self.pipeline.as_ref().unwrap())?;
        command_buffer.bind_vertex_buffers(0, &[self.vertex_buffer_id], &[0], &[], &[])?;
        unsafe { command_buffer.draw(self.vertex_count, 1, 0, 0) }?;
        Ok(())
    }
}
