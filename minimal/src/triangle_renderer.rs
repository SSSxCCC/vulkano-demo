use vulkano::{
    buffer::{Buffer, BufferContents, BufferCreateInfo, BufferUsage}, command_buffer::{
        allocator::StandardCommandBufferAllocator, AutoCommandBufferBuilder, CommandBufferUsage,
        PrimaryCommandBufferAbstract, RenderPassBeginInfo, SubpassBeginInfo, SubpassContents,
    }, memory::allocator::{AllocationCreateInfo, MemoryTypeFilter}, pipeline::{
        graphics::{
            color_blend::{ColorBlendAttachmentState, ColorBlendState},
            input_assembly::InputAssemblyState,
            multisample::MultisampleState,
            rasterization::RasterizationState,
            vertex_input::{Vertex, VertexDefinition},
            viewport::{Viewport, ViewportState},
            GraphicsPipelineCreateInfo,
        },
        layout::PipelineDescriptorSetLayoutCreateInfo,
        DynamicState, GraphicsPipeline, PipelineLayout, PipelineShaderStageCreateInfo,
    }, render_pass::{Framebuffer, FramebufferCreateInfo, Subpass}, shader::{ShaderModule, ShaderModuleCreateInfo}, sync::GpuFuture
};
use vulkano_util::{context::VulkanoContext, renderer::VulkanoWindowRenderer};

#[derive(BufferContents, Vertex)]
#[repr(C)]
struct MyVertex {
    #[format(R32G32_SFLOAT)]
    position: [f32; 2],
}

/*mod vs {
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
}*/

pub struct TriangleRenderer;

impl TriangleRenderer {
    pub fn draw(
        before_future: Box<dyn GpuFuture>,
        context: &VulkanoContext,
        renderer: &VulkanoWindowRenderer,
    ) -> Box<dyn GpuFuture> {
        let render_pass = vulkano::single_pass_renderpass!(
            context.device().clone(),
            attachments: {
                color: {
                    format: renderer.swapchain_format(), // set the format the same as the swapchain
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
            position: [0.0, 0.5],
        };
        let vertex3 = MyVertex {
            position: [0.5, -0.25],
        };
        let vertex_buffer = Buffer::from_iter(
            context.memory_allocator().clone(),
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

        const SHADER: &[u8] = include_bytes!(env!("minimal_shader.spv"));
        let mut shader_code = Vec::new();
        let mut i = 0;
        while i < SHADER.len() {
            let a = SHADER[i] as u32;
            let b = if i + 1 < SHADER.len() { (SHADER[i + 1] as u32) << 8 } else { 0 };
            let c = if i + 2 < SHADER.len() { (SHADER[i + 2] as u32) << 16 } else { 0 };
            let d = if i + 3 < SHADER.len() { (SHADER[i + 3] as u32) << 24 } else { 0 };
            shader_code.push(a | b | c | d);
            i += 4;
        }

        //let shader_code = SHADER.iter().map(|x| *x as u32).collect::<Vec<_>>();
        let shader_module = unsafe { ShaderModule::new(context.device().clone(), ShaderModuleCreateInfo::new(&shader_code)) }.unwrap();
        let vs = shader_module
            .entry_point("main_vs")
            .unwrap();
        let fs = shader_module
            .entry_point("main_fs")
            .unwrap();

        let vertex_input_state = MyVertex::per_vertex()
            .definition(&vs.info().input_interface)
            .unwrap();

        let stages = [
            PipelineShaderStageCreateInfo::new(vs),
            PipelineShaderStageCreateInfo::new(fs),
        ];

        let layout = PipelineLayout::new(
            context.device().clone(),
            PipelineDescriptorSetLayoutCreateInfo::from_stages(&stages)
                .into_pipeline_layout_create_info(context.device().clone())
                .unwrap(),
        )
        .unwrap();

        let viewport = Viewport {
            offset: [0.0, 0.0],
            extent: renderer.window().inner_size().into(),
            depth_range: 0.0..=1.0,
        };

        let subpass = Subpass::from(render_pass.clone(), 0).unwrap();

        let pipeline = GraphicsPipeline::new(
            context.device().clone(),
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
        .unwrap();

        let command_buffer_allocator =
            StandardCommandBufferAllocator::new(context.device().clone(), Default::default());

        let command_buffer = {
            let mut builder = AutoCommandBufferBuilder::primary(
                &command_buffer_allocator,
                renderer.graphics_queue().queue_family_index(),
                CommandBufferUsage::OneTimeSubmit,
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
                .set_viewport(0, [viewport].into_iter().collect())
                .unwrap()
                .bind_pipeline_graphics(pipeline.clone())
                .unwrap()
                .bind_vertex_buffers(0, vertex_buffer.clone())
                .unwrap()
                .draw(vertex_buffer.len() as u32, 1, 0, 0)
                .unwrap()
                .end_render_pass(Default::default())
                .unwrap();

            builder.build().unwrap()
        };

        return command_buffer
            .execute_after(before_future, renderer.graphics_queue())
            .unwrap()
            .boxed();
    }
}
