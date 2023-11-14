use std::{any::{TypeId, Any}, collections::HashMap};
use specs::prelude::*;
use rapier2d::prelude::*;

pub trait TEngine {
    fn init(&mut self);
    fn update(&mut self);
    fn draw(&mut self);
}

pub fn create() -> Box<dyn TEngine> {
    let mut world = World::new();
    world.insert(Physics2DManager::new());
    world.insert(ObjectManager::new());
    Box::new(Engine { world })
}

struct Engine {
    world: World, // ecs world, also contains resources and managers
}

impl TEngine for Engine {
    fn init(&mut self) {
        println!("Engine::init");

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
            println!("Ball altitude: {}", ball_body.translation().y);
        }
    }

    fn update(&mut self) {

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

// The game object manager
struct ObjectManager {
    objects: HashMap<ObjectHandle, Object>,
}

impl ObjectManager {
    fn new() -> Self {
        ObjectManager { objects: HashMap::new() }
    }

    fn create(&mut self, engine: &mut Engine) {
        for (handle, object) in &mut self.objects {
            for behaviour in &mut object.behaviours {
                behaviour.on_create(*handle, engine);
            }
        }
    }
}

#[derive(Eq, PartialEq, Hash, Clone, Copy)]
struct ObjectHandle(u32);

impl ObjectHandle {
    const INVALID: ObjectHandle = ObjectHandle(0);
}

struct Object {
    behaviours: Vec<Box<dyn Behaviour>>,
}

impl Object {
    fn new() -> Self {
        Object { behaviours: Vec::new() }
    }

    fn add_behaviour(&mut self, behaviour: Box<dyn Behaviour>) {
        self.behaviours.push(behaviour);
    }

    fn get_behaviour<T: 'static>(&self) -> Option<&Box<dyn Behaviour>> {
        for behaviour in &self.behaviours {
            if TypeId::of::<T>() == behaviour.as_any().type_id() { // TODO: can we get TypeId without as_any?
                return Some(behaviour);
            }
        }
        None
    }
}

trait Behaviour: Send + Sync { // Send and Sync are required to insert into specs::World as resource
    fn new() -> Box<dyn Behaviour> where Self: Sized;
    fn on_create(&mut self, object_handle: ObjectHandle, engine: &mut Engine) { }
    fn on_update(&mut self) { }
    fn on_draw(&mut self) { }
    fn on_destroy(&mut self) { }
    fn as_any(&self) -> &dyn Any;
}

struct RigidBody2D {
    handle: RigidBodyHandle,
}

impl Behaviour for RigidBody2D {
    fn new() -> Box<dyn Behaviour> {
        Box::new(RigidBody2D { handle: RigidBodyHandle::invalid() })
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn on_create(&mut self, object_handle: ObjectHandle, engine: &mut Engine) {
        let physics2d_manager = engine.world.get_mut::<Physics2DManager>().unwrap();
        let rigid_body = RigidBodyBuilder::dynamic()
                .translation(vector![0.0, 10.0])
                .build();
        self.handle = physics2d_manager.rigid_body_set.insert(rigid_body);
    }
}

struct CuboidCollider2D {
    handle: ColliderHandle,
}

impl Behaviour for CuboidCollider2D {
    fn new() -> Box<dyn Behaviour> {
        Box::new(CuboidCollider2D { handle: ColliderHandle::invalid() })
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn on_create(&mut self, object_handle: ObjectHandle, engine: &mut Engine) {
        let collider = ColliderBuilder::cuboid(0.5, 0.5).restitution(0.7).build();

        let rb2d_handle = {
            let object_manager = engine.world.read_resource::<ObjectManager>();
            let rb2d = object_manager.objects.get(&object_handle).unwrap().get_behaviour::<RigidBody2D>();
            rb2d.map(|rb2d| { rb2d.as_any().downcast_ref::<RigidBody2D>().unwrap().handle })
        };

        let physics2d_manager = engine.world.get_mut::<Physics2DManager>().unwrap();
        self.handle = if let Some(rb2d_handle) = rb2d_handle {
            physics2d_manager.collider_set.insert_with_parent(collider, rb2d_handle, &mut physics2d_manager.rigid_body_set)
        } else {
            physics2d_manager.collider_set.insert(collider)
        }
    }
}
