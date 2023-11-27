use std::collections::HashMap;
use glam::{Vec2, Vec3, Vec4};
use rapier2d::prelude::*;
use rayon::iter::ParallelIterator;
use shipyard::{World, Component, EntityId, View, IntoIter, IntoWithId, Unique, UniqueViewMut, ViewMut, AddComponent, Get};
use vulkano::{render_pass::{FramebufferCreateInfo, Framebuffer, Subpass}, buffer::{BufferContents, Buffer, BufferCreateInfo, BufferUsage}, pipeline::{graphics::{vertex_input::Vertex, viewport::{Viewport, ViewportState}, input_assembly::InputAssemblyState}, GraphicsPipeline}, memory::allocator::{AllocationCreateInfo, MemoryUsage}, command_buffer::{allocator::StandardCommandBufferAllocator, AutoCommandBufferBuilder, CommandBufferUsage, RenderPassBeginInfo, SubpassContents, PrimaryCommandBufferAbstract}, sync::GpuFuture};
use vulkano_util::{context::VulkanoContext, renderer::VulkanoWindowRenderer};

pub trait Engine {
    fn init(&mut self);
    fn update(&mut self);
    fn draw(&mut self, before_future: Box<dyn GpuFuture>, context: &VulkanoContext, renderer: &VulkanoWindowRenderer) -> Box<dyn GpuFuture>;
}

pub fn create() -> Box<dyn Engine> {
    let world = World::new();
    Box::new(EngineImpl { world })
}

struct EngineImpl {
    world: World, // ecs world, also contains resources and managers
}

impl Engine for EngineImpl {
    fn init(&mut self) {
        log::info!("Engine::init");

        self.world.add_unique(Physics2DManager::new());

        self.world.add_entity((Transform2D { position: Vec2 { x: 0.0, y: 10.0 }, rotation: 0.0 },
                RigidBody2D::new(RigidBodyType::Dynamic),
                Collider2D::new(SharedShape::cuboid(0.5, 0.5), 0.7)));
        self.world.add_entity((Transform2D { position: Vec2 { x: 0.0, y: 0.0 }, rotation: 0.0 },
                Collider2D::new(SharedShape::cuboid(100.0, 0.1), 0.7)));
    }

    fn update(&mut self) {
        log::info!("Engine::update");

        self.world.run(physics2d_maintain_system);
        self.world.run(physics2d_update_system);

        let mut world_data = WorldData::new();
        world_data.add_component::<Transform2D>(&self.world);
        world_data.add_component::<RigidBody2D>(&self.world);
        world_data.add_component::<Collider2D>(&self.world);
        log::info!("world_data={:?}", world_data);
    }

    fn draw(&mut self, before_future: Box<dyn GpuFuture>, context: &VulkanoContext, renderer: &VulkanoWindowRenderer) -> Box<dyn GpuFuture> {
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

#[derive(Unique)]
struct Physics2DManager {
    rigid_body_set: RigidBodySet,
    collider_set: ColliderSet,
    gravity: Vector<Real>,
    integration_parameters: IntegrationParameters,
    physics_pipeline: PhysicsPipeline,
    island_manager: IslandManager,
    broad_phase: BroadPhase,
    narrow_phase: NarrowPhase,
    impulse_joint_set: ImpulseJointSet,
    multibody_joint_set: MultibodyJointSet,
    ccd_solver: CCDSolver,
    physics_hooks: Box<dyn PhysicsHooks>,
    event_handler: Box<dyn EventHandler>,
}

impl Physics2DManager {
    fn new() -> Self {
        Physics2DManager { rigid_body_set: RigidBodySet::new(), collider_set: ColliderSet::new(), gravity: vector![0.0, -9.81],
            integration_parameters: IntegrationParameters::default(), physics_pipeline: PhysicsPipeline::new(),
            island_manager: IslandManager::new(), broad_phase: BroadPhase::new(), narrow_phase: NarrowPhase::new(),
            impulse_joint_set: ImpulseJointSet::new(), multibody_joint_set: MultibodyJointSet::new(),
            ccd_solver: CCDSolver::new(), physics_hooks: Box::new(()), event_handler: Box::new(()) }
    }

    fn update(&mut self) {
        self.physics_pipeline.step(
            &self.gravity,
            &self.integration_parameters,
            &mut self.island_manager,
            &mut self.broad_phase,
            &mut self.narrow_phase,
            &mut self.rigid_body_set,
            &mut self.collider_set,
            &mut self.impulse_joint_set,
            &mut self.multibody_joint_set,
            &mut self.ccd_solver,
            None,
            self.physics_hooks.as_ref(),
            self.event_handler.as_ref(),
        );
    }
}

fn physics2d_maintain_system(mut physics2d_manager: UniqueViewMut<Physics2DManager>,
        mut rb2d: ViewMut<RigidBody2D>, mut col2d: ViewMut<Collider2D>,
        mut transform2d: ViewMut<Transform2D>) {
    let physics2d_manager = physics2d_manager.as_mut();
    for (e, mut rb2d) in rb2d.inserted_or_modified_mut().iter().with_id() {
        if let Some(rigid_body) = physics2d_manager.rigid_body_set.get_mut(rb2d.handle) {
            rigid_body.set_body_type(rb2d.body_type, true);
        } else {
            if !transform2d.contains(e) {
                transform2d.add_component_unchecked(e, Transform2D::default());
            }
            let transform2d = transform2d.get(e).unwrap();
            let rigid_body = RigidBodyBuilder::new(rb2d.body_type)
                    .translation(vector![transform2d.position.x, transform2d.position.y])
                    .rotation(transform2d.rotation).build();
            rb2d.handle = physics2d_manager.rigid_body_set.insert(rigid_body);
        }

        if let Ok(col2d) = col2d.get(e) {
            if physics2d_manager.collider_set.contains(col2d.handle) {
                physics2d_manager.collider_set.set_parent(col2d.handle, Some(rb2d.handle), &mut physics2d_manager.rigid_body_set)
            }
        }
    }

    for (e, mut col2d) in col2d.inserted_or_modified_mut().iter().with_id() {
        if let Some(collider) = physics2d_manager.collider_set.get_mut(col2d.handle) {
            collider.set_shape(col2d.shape.clone());
            collider.set_restitution(col2d.restitution);
        } else {
            if !transform2d.contains(e) {
                transform2d.add_component_unchecked(e, Transform2D::default());
            }
            let transform2d = transform2d.get(e).unwrap();
            let mut collider = ColliderBuilder::new(col2d.shape.clone()).restitution(col2d.restitution).build();
            if let Ok(rb2d) = &rb2d.get(e) {
                // TODO: add position and rotation relative to parent
                col2d.handle = physics2d_manager.collider_set.insert_with_parent(collider, rb2d.handle, &mut physics2d_manager.rigid_body_set);
            } else {
                collider.set_translation(vector![transform2d.position.x, transform2d.position.y]);
                //collider.set_rotation(transform2d.rotation); TODO: how to set_rotation?
                col2d.handle = physics2d_manager.collider_set.insert(collider);
            }
        }
    }

    rb2d.clear_all_inserted_and_modified();
    col2d.clear_all_inserted_and_modified();
}

fn physics2d_update_system(mut physics2d_manager: UniqueViewMut<Physics2DManager>,
        rb2d: View<RigidBody2D>, mut transform2d: ViewMut<Transform2D>) {
    physics2d_manager.update();
    (&rb2d, &mut transform2d).par_iter().for_each(|(rb2d, mut transform2d)| {
        let rigid_body = &physics2d_manager.rigid_body_set[rb2d.handle];
        transform2d.position.x = rigid_body.translation().x;
        transform2d.position.y = rigid_body.translation().y;
        transform2d.rotation = rigid_body.rotation().angle();
    });
}

trait Edit: Component {
    fn name() -> &'static str;

    fn to_data(&self) -> ComponentData {
        ComponentData::new(Self::name())
    }

    fn from_data(&mut self, data: ComponentData) { }
}

#[derive(Debug)]
enum Value {
    Int32(i32),
    Float32(f32),
    String(String),
    Vec2(Vec2),
    Vec3(Vec3),
    Vec4(Vec4),
}

#[derive(Debug)]
struct Variant {
    name: &'static str,
    value: Value,
}

// ComponentData contains all variant in a component
#[derive(Debug)]
struct ComponentData {
    name: &'static str,
    variants: Vec<Variant>,
}

impl ComponentData {
    fn new(name: &'static str) -> Self {
        ComponentData { name, variants: Vec::new() }
    }
}

// EntityData contains all component data in a entity, key is component name
#[derive(Debug)]
struct EntityData {
    id: EntityId,
    components: Vec<ComponentData>,
}

// WorldData contains all entity data in the world
#[derive(Debug)]
struct WorldData{
    entities: Vec<EntityData>,
    id_index_map: HashMap<EntityId, usize>,
}

impl WorldData {
    fn new() -> Self {
        WorldData{entities: Vec::new(), id_index_map: HashMap::new()}
    }

    fn add_component<T: Edit + Send + Sync>(&mut self, world: &World) {
        world.run(|c: View<T>| {
            for (e, c) in c.iter().with_id() {
                let index = *self.id_index_map.entry(e).or_insert(self.entities.len());
                if index == self.entities.len() {
                    self.entities.push(EntityData { id: e, components: Vec::new() });
                }
                self.entities[index].components.push(c.to_data());
            }
        })
    }
}

#[derive(Component, Debug, Default)]
struct Transform2D {
    position: Vec2,
    rotation: f32,
}

impl Edit for Transform2D {
    fn name() -> &'static str { "Transform2D" }

    fn to_data(&self) -> ComponentData {
        let mut data = ComponentData::new(Self::name());
        data.variants.push(Variant { name: "position", value: Value::Vec2(self.position) });
        data.variants.push(Variant { name: "rotation", value: Value::Float32(self.rotation) });
        data
    }

    fn from_data(&mut self, data: ComponentData) {
        for v in data.variants {
            match v.name {
                "position" => self.position = if let Value::Vec2(position) = v.value { position } else { Default::default() },
                "rotation" => self.rotation = if let Value::Float32(rotation) = v.value { rotation } else { Default::default() },
                _ => (),
            }
        }
    }
}

#[derive(Component, Debug)]
#[track(All)]
struct RigidBody2D {
    handle: RigidBodyHandle,
    body_type: RigidBodyType,
}

impl RigidBody2D {
    fn new(body_type: RigidBodyType) -> Self {
        RigidBody2D { handle: RigidBodyHandle::invalid(), body_type }
    }
}

impl Edit for RigidBody2D {
    fn name() -> &'static str { "RigidBody2D" }
}

struct ShapeWrapper(SharedShape);

impl std::ops::Deref for ShapeWrapper {
    type Target = SharedShape;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl std::fmt::Debug for ShapeWrapper {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("ShapeWrapper").field(&self.shape_type()).finish() // TODO: print all members
    }
}

#[derive(Component, Debug)]
#[track(All)]
struct Collider2D {
    handle: ColliderHandle,
    shape: ShapeWrapper,
    restitution: f32,
}

impl Collider2D {
    fn new(shape: SharedShape, restitution: f32) -> Self {
        Collider2D { handle: ColliderHandle::invalid(), shape: ShapeWrapper(shape), restitution }
    }
}

impl Edit for Collider2D {
    fn name() -> &'static str { "Collider2D" }
}
