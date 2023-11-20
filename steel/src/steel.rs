use std::collections::HashMap;
use glam::{Vec2, Vec3, Vec4};
use specs::{prelude::*, Component};
use rapier2d::prelude::*;

pub trait Engine {
    fn init(&mut self);
    fn update(&mut self);
    fn draw(&mut self);
}

pub fn create() -> Box<dyn Engine> {
    let mut world = World::new();
    let mut dispatcher = DispatcherBuilder::new()
        //.with(HelloWorld, "hello_world", &[])
        //.with(UpdatePos, "update_pos", &["hello_world"])
        //.with(HelloWorld, "hello_updated", &["update_pos"])
        .build();
    Box::new(EngineImpl { world, dispatcher })
}

struct EngineImpl<'a> {
    world: World, // ecs world, also contains resources and managers
    dispatcher: Dispatcher<'a, 'a>,
}

impl Engine for EngineImpl<'_> {
    fn init(&mut self) {
        log::info!("Engine::init");

        self.dispatcher.setup(&mut self.world);
        self.world.insert(Physics2DManager::new());
        self.world.register::<Transform2D>(); // TODO: remove this
        self.world.create_entity().with(Transform2D { position: Vec2 { x: 1.0, y: 2.0 }, rotation: 30.0 }).build();



        let physics2d_manager = self.world.get_mut::<Physics2DManager>().unwrap();

        /* Create the ground. */
        let collider = ColliderBuilder::cuboid(100.0, 0.1).build();
        physics2d_manager.collider_set.insert(collider);

        /* Create the bouncing ball. */
        let rigid_body = RigidBodyBuilder::dynamic()
                .translation(vector![0.0, 10.0])
                .build();
        let collider = ColliderBuilder::cuboid(0.5, 0.5).restitution(0.7).build();
        let ball_body_handle = physics2d_manager.rigid_body_set.insert(rigid_body);
        physics2d_manager.collider_set.insert_with_parent(collider, ball_body_handle, &mut physics2d_manager.rigid_body_set);

        /* Run the game loop, stepping the simulation once per frame. */
        for _ in 0..200 {
            physics2d_manager.update();

            let ball_body = &physics2d_manager.rigid_body_set[ball_body_handle];
            log::info!("Ball altitude: {}", ball_body.translation().y);
        }
    }

    fn update(&mut self) {
        log::info!("Engine::update");
        let physics2d_manager = self.world.get_mut::<Physics2DManager>().unwrap();
        physics2d_manager.update();

        self.dispatcher.dispatch(&mut self.world);
        self.world.maintain();

        let mut world_data = WorldData::new();
        world_data.add_component::<Transform2D>(&self.world);
        log::info!("world_data={:?}", world_data);
    }

    fn draw(&mut self) {

    }
}

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

trait Edit: Component {
    fn name() -> &'static str;

    fn to_data(&self) -> ComponentData {
        ComponentData::new()
    }

    fn from_data(&mut self, data: ComponentData) { }
}

#[derive(Debug)]
enum Variant {
    Int32(i32),
    Float32(f32),
    String(String),
    Vec2(Vec2),
    Vec3(Vec3),
    vec4(Vec4),
}

// ComponentData contains all variant in a component, key is variant name
type ComponentData = HashMap<&'static str, Variant>;

// EntityData contains all component data in a entity, key is component name
type EntityData = HashMap<&'static str, ComponentData>;

// WorldData contains all entity data in the world
#[derive(Debug)]
struct WorldData(HashMap<Entity, EntityData>);

impl WorldData {
    fn new() -> Self {
        WorldData(HashMap::new())
    }

    fn add_component<T: Edit>(&mut self, world: &World) {
        for (e, c) in (&world.entities(), &world.read_component::<T>()).join() {
            let entity_data = self.0.entry(e).or_default();
            entity_data.insert(T::name(), c.to_data());
        }
    }
}

#[derive(Component)]
struct Transform2D {
    position: Vec2,
    rotation: f32,
}

impl Edit for Transform2D {
    fn name() -> &'static str { "Transform2D" }

    fn to_data(&self) -> ComponentData {
        let mut data = ComponentData::new();
        data.insert("position", Variant::Vec2(self.position));
        data.insert("rotation", Variant::Float32(self.rotation));
        data
    }

    fn from_data(&mut self, data: ComponentData) {
        self.position = if let Some(Variant::Vec2(position)) = data.get("position") { *position } else { Default::default() };
        self.rotation = if let Some(Variant::Float32(rotation)) = data.get("rotation") { *rotation } else { Default::default() };
    }
}
