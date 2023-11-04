use vulkano::{render_pass::{FramebufferCreateInfo, Framebuffer, Subpass}, buffer::{BufferContents, Buffer, BufferCreateInfo, BufferUsage}, pipeline::{graphics::{vertex_input::Vertex, viewport::{Viewport, ViewportState}, input_assembly::InputAssemblyState}, GraphicsPipeline}, memory::allocator::{AllocationCreateInfo, MemoryUsage}, command_buffer::{allocator::StandardCommandBufferAllocator, AutoCommandBufferBuilder, CommandBufferUsage, RenderPassBeginInfo, SubpassContents, PrimaryCommandBufferAbstract}, sync::GpuFuture};
use vulkano_util::{context::VulkanoContext, renderer::VulkanoWindowRenderer};

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

pub struct TriangleRenderer {

}

impl TriangleRenderer {
    pub fn draw(before_future: Box<dyn GpuFuture>, context: &VulkanoContext, renderer: &VulkanoWindowRenderer) -> Box<dyn GpuFuture> {
        let render_pass = vulkano::single_pass_renderpass!(
            context.device().clone(),
            attachments: {
                color: {
                    load: Clear,
                    store: Store,
                    format: renderer.swapchain_format(), // set the format the same as the swapchain
                    samples: 1,
                },
            },
            pass: {
                color: [color],
                depth_stencil: {},
            },
        )
        .unwrap();

        let framebuffer = Framebuffer::new(
            render_pass.clone(),
            FramebufferCreateInfo {
                attachments: vec![renderer.swapchain_image_view()],
                ..Default::default()
            },
        )
        .unwrap();

        let vertex1 = MyVertex {
            position: [-0.5, -0.5],
        };
        let vertex2 = MyVertex {
            position: [-0.5, 0.5],
        };
        let vertex3 = MyVertex {
            position: [0.5, 0.5],
        };
        let vertex4 = MyVertex {
            position: [0.5, -0.5],
        };
        let vertex_buffer = Buffer::from_iter(
            context.memory_allocator().as_ref(),
            BufferCreateInfo {
                usage: BufferUsage::VERTEX_BUFFER,
                ..Default::default()
            },
            AllocationCreateInfo {
                usage: MemoryUsage::Upload,
                ..Default::default()
            },
            vec![vertex1, vertex2, vertex3, vertex4].into_iter(),
        )
        .unwrap();

        let index_buffer = Buffer::from_iter(
            context.memory_allocator(),
            BufferCreateInfo { usage: BufferUsage::INDEX_BUFFER, ..Default::default() },
            AllocationCreateInfo { usage: MemoryUsage::Upload, ..Default::default() },
            vec![0u16, 1, 2, 2, 3, 0].into_iter()).unwrap();
    
        let vs = vs::load(context.device().clone()).expect("failed to create shader module");
        let fs = fs::load(context.device().clone()).expect("failed to create shader module");
    
        let viewport = Viewport {
            origin: [0.0, 0.0],
            dimensions: renderer.window().inner_size().into(),
            depth_range: 0.0..1.0,
        };
    
        let pipeline = GraphicsPipeline::start()
        .vertex_input_state(MyVertex::per_vertex())
        .vertex_shader(vs.entry_point("main").unwrap(), ())
        .input_assembly_state(InputAssemblyState::new())
        .viewport_state(ViewportState::viewport_fixed_scissor_irrelevant([viewport]))
        .fragment_shader(fs.entry_point("main").unwrap(), ())
        .render_pass(Subpass::from(render_pass, 0).unwrap())
        .build(context.device().clone())
        .unwrap();

        let command_buffer_allocator = StandardCommandBufferAllocator::new(context.device().clone(), Default::default());
    
        let command_buffer = {
            let mut builder = AutoCommandBufferBuilder::primary(
                &command_buffer_allocator,
                renderer.graphics_queue().queue_family_index(),
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
                .bind_index_buffer(index_buffer.clone())
                .draw_indexed(index_buffer.len() as u32, 1, 0, 0, 0)
                .unwrap()
                .end_render_pass()
                .unwrap();

            builder.build().unwrap()
        };

        return command_buffer.execute_after(before_future, renderer.graphics_queue()).unwrap().boxed();
    }
}