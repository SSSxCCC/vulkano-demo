use image::{GenericImageView, ImageReader};
use std::io::Cursor;
use vulkano::{
    buffer::{Buffer, BufferContents, BufferCreateInfo, BufferUsage},
    command_buffer::{
        allocator::StandardCommandBufferAllocator, AutoCommandBufferBuilder, CommandBufferUsage,
        CopyBufferToImageInfo, PrimaryCommandBufferAbstract, RenderPassBeginInfo, SubpassBeginInfo,
        SubpassContents,
    },
    descriptor_set::{
        allocator::StandardDescriptorSetAllocator, PersistentDescriptorSet, WriteDescriptorSet,
    },
    format::Format,
    image::{
        sampler::{Sampler, SamplerCreateInfo},
        view::ImageView,
        Image, ImageCreateInfo, ImageUsage,
    },
    memory::allocator::{AllocationCreateInfo, MemoryTypeFilter},
    pipeline::{
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
        DynamicState, GraphicsPipeline, Pipeline, PipelineBindPoint, PipelineLayout,
        PipelineShaderStageCreateInfo,
    },
    render_pass::{Framebuffer, FramebufferCreateInfo, Subpass},
    sync::GpuFuture,
};
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

            layout(location = 0) out vec2 tex_coord;

            void main() {
                gl_Position = vec4(position, 0.0, 1.0);
                tex_coord = position + vec2(0.5);
            }
        ",
    }
}

mod fs {
    vulkano_shaders::shader! {
        ty: "fragment",
        src: r"
            #version 460

            layout(location = 0) in vec2 tex_coord;

            layout(location = 0) out vec4 f_color;

            layout(set = 0, binding = 0) uniform sampler s;
            layout(set = 0, binding = 1) uniform texture2D tex;

            void main() {
                f_color = texture(sampler2D(tex, s), tex_coord);
            }
        ",
    }
}

pub struct TriangleRenderer;

impl TriangleRenderer {
    pub fn draw(
        before_future: Box<dyn GpuFuture>,
        context: &VulkanoContext,
        renderer: &VulkanoWindowRenderer,
    ) -> Box<dyn GpuFuture> {
        let command_buffer_allocator =
            StandardCommandBufferAllocator::new(context.device().clone(), Default::default());

        let dynamic_image = ImageReader::new(Cursor::new(include_bytes!("../texture.jpg")))
            .with_guessed_format()
            .unwrap()
            .decode()
            .unwrap();
        let image_staging_buffer = Buffer::new_slice(
            context.memory_allocator().clone(),
            BufferCreateInfo {
                usage: BufferUsage::TRANSFER_SRC,
                ..Default::default()
            },
            AllocationCreateInfo {
                memory_type_filter: MemoryTypeFilter::HOST_SEQUENTIAL_WRITE
                    | MemoryTypeFilter::PREFER_HOST,
                ..Default::default()
            },
            (dynamic_image.dimensions().0 * dynamic_image.dimensions().1) as u64 * 4,
        )
        .unwrap();
        image_staging_buffer
            .write()
            .unwrap()
            .copy_from_slice(&dynamic_image.to_rgba8());
        let image = Image::new(
            context.memory_allocator().clone(),
            ImageCreateInfo {
                usage: ImageUsage::TRANSFER_DST | ImageUsage::SAMPLED,
                format: Format::R8G8B8A8_SRGB,
                extent: [
                    dynamic_image.dimensions().0,
                    dynamic_image.dimensions().1,
                    1,
                ],
                ..Default::default()
            },
            AllocationCreateInfo {
                memory_type_filter: MemoryTypeFilter::PREFER_DEVICE,
                ..Default::default()
            },
        )
        .unwrap();
        let image_view = ImageView::new_default(image.clone()).unwrap();
        let sampler = Sampler::new(
            context.device().clone(),
            SamplerCreateInfo::simple_repeat_linear_no_mipmap(),
        )
        .unwrap();
        let mut upload_image_commnad_buffer = AutoCommandBufferBuilder::primary(
            &command_buffer_allocator,
            context.graphics_queue().queue_family_index(),
            CommandBufferUsage::OneTimeSubmit,
        )
        .unwrap();
        upload_image_commnad_buffer
            .copy_buffer_to_image(CopyBufferToImageInfo::buffer_image(
                image_staging_buffer,
                image.clone(),
            ))
            .unwrap();
        upload_image_commnad_buffer
            .build()
            .unwrap()
            .execute(context.graphics_queue().clone())
            .unwrap()
            .then_signal_fence_and_flush()
            .unwrap()
            .wait(None)
            .unwrap();

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
            position: [-0.5, 0.5],
        };
        let vertex3 = MyVertex {
            position: [0.5, 0.5],
        };
        let vertex4 = MyVertex {
            position: [0.5, -0.5],
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
            vec![vertex1, vertex2, vertex3, vertex4].into_iter(),
        )
        .unwrap();

        let index_buffer = Buffer::from_iter(
            context.memory_allocator().clone(),
            BufferCreateInfo {
                usage: BufferUsage::INDEX_BUFFER,
                ..Default::default()
            },
            AllocationCreateInfo {
                memory_type_filter: MemoryTypeFilter::PREFER_DEVICE
                    | MemoryTypeFilter::HOST_SEQUENTIAL_WRITE,
                ..Default::default()
            },
            vec![0u16, 1, 2, 2, 3, 0].into_iter(),
        )
        .unwrap();

        let vs = vs::load(context.device().clone())
            .unwrap()
            .entry_point("main")
            .unwrap();
        let fs = fs::load(context.device().clone())
            .unwrap()
            .entry_point("main")
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

        let descriptor_set_allocator =
            StandardDescriptorSetAllocator::new(context.device().clone(), Default::default());
        let descriptor_set = PersistentDescriptorSet::new(
            &descriptor_set_allocator,
            pipeline.layout().set_layouts()[0].clone(),
            [
                WriteDescriptorSet::sampler(0, sampler),
                WriteDescriptorSet::image_view(1, image_view),
            ],
            [],
        )
        .unwrap();

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
                .bind_index_buffer(index_buffer.clone())
                .unwrap()
                .bind_descriptor_sets(
                    PipelineBindPoint::Graphics,
                    pipeline.layout().clone(),
                    0,
                    descriptor_set,
                )
                .unwrap()
                .draw_indexed(index_buffer.len() as u32, 1, 0, 0, 0)
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
