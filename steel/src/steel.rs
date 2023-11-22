use std::collections::HashMap;
use glam::{Vec2, Vec3, Vec4};
use rapier2d::prelude::*;
use shipyard::{World, Component, EntityId, View, IntoIter, IntoWithId, Unique, UniqueViewMut};

pub trait Engine {
    fn init(&mut self);
    fn update(&mut self);
    fn draw(&mut self);
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
        self.world.add_entity(Transform2D { position: Vec2 { x: 1.0, y: 2.0 }, rotation: 30.0 });



        self.world.run(|mut physics2d_manager: UniqueViewMut<Physics2DManager>| {
            let physics2d_manager = physics2d_manager.as_mut();

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
        });
    }

    fn update(&mut self) {
        log::info!("Engine::update");
        self.world.run(|mut physics2d_manager: UniqueViewMut<Physics2DManager>| {
            physics2d_manager.update();
        });

        let mut world_data = WorldData::new();
        world_data.add_component::<Transform2D>(&self.world);
        log::info!("world_data={:?}", world_data);
    }

    fn draw(&mut self) {

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
struct WorldData(HashMap<EntityId, EntityData>);

impl WorldData {
    fn new() -> Self {
        WorldData(HashMap::new())
    }

    fn add_component<T: Edit + Send + Sync>(&mut self, world: &World) {
        world.run(|c: View<T>| {
            for (e, c) in c.iter().with_id() {
                let entity_data = self.0.entry(e).or_default();
                entity_data.insert(T::name(), c.to_data());
            }
        })
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

#[derive(Component)]
struct RigidBody2D {
    handle: RigidBodyHandle,
}

impl Edit for RigidBody2D {
    fn name() -> &'static str { "RigidBody2D" }
}

#[derive(Component)]
struct CuboidCollider2D {
    handle: ColliderHandle,
    size: Vec2,
    restitution: f32,
}

impl Edit for CuboidCollider2D {
    fn name() -> &'static str { "CuboidCollider2D" }
}
