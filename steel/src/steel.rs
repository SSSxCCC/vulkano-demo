use std::sync::{Arc, RwLock};
use specs::prelude::*;
use rapier2d::prelude::*;

pub struct Engine {
    world: World, // ecs world, also contains resources and managers
}

impl Engine {
    pub fn new() -> Arc<RwLock<Self>> {
        let mut world = World::new();
        world.insert(Physics2DManager::new());
        world.insert(ObjectManager::new());
        Arc::new(RwLock::new(Engine { world }))
    }

    pub fn init(&mut self) {
        println!("Engine::init");

        let physics2d_manager = self.world.get_mut::<Physics2DManager>().unwrap();

        /* Create the ground. */
        let collider = ColliderBuilder::cuboid(100.0, 0.1).build();
        physics2d_manager.collider_set.insert(collider);

        /* Create the bouncing ball. */
        let rigid_body = RigidBodyBuilder::dynamic()
                .translation(vector![0.0, 10.0])
                .build();
        let collider = ColliderBuilder::ball(0.5).restitution(0.7).build();
        let ball_body_handle = physics2d_manager.rigid_body_set.insert(rigid_body);
        physics2d_manager.collider_set.insert_with_parent(collider, ball_body_handle, &mut physics2d_manager.rigid_body_set);

        /* Run the game loop, stepping the simulation once per frame. */
        for _ in 0..200 {
            physics2d_manager.update();

            let ball_body = &physics2d_manager.rigid_body_set[ball_body_handle];
            println!("Ball altitude: {}", ball_body.translation().y);
        }
    }

    pub fn update(&self) {

    }

    pub fn draw(&self) {

    }
}

// The game object manager
struct ObjectManager {
    objects: Vec<Arc<RwLock<Object>>>,
}

impl ObjectManager {
    fn new() -> Self {
        ObjectManager { objects: Vec::new() }
    }
}

struct Object {
    behaviours: Vec<Arc<RwLock<dyn Behaviour>>>,
    engine: Arc<RwLock<Engine>>,
}

impl Object {
    fn add_behaviour(&mut self, behaviour: Arc<RwLock<dyn Behaviour>>) {
        self.behaviours.push(behaviour);
    }
}

trait Behaviour: Send + Sync { // Send and Sync are required to insert into specs::World as resource
    fn new() -> Arc<RwLock<dyn Behaviour>> where Self: Sized;
    fn on_create(&mut self, _object: Arc<RwLock<Object>>) { }
    fn on_update(&mut self) { }
    fn on_draw(&mut self) { }
    fn on_destroy(&mut self) { }
}

struct RigidBody2D {
    object: Option<Arc<RwLock<Object>>>,
}

impl Behaviour for RigidBody2D {
    fn new() -> Arc<RwLock<dyn Behaviour>> {
        Arc::new(RwLock::new(RigidBody2D { object: None }))
    }

    fn on_create(&mut self, object: Arc<RwLock<Object>>) {
        self.object = Some(object);
        //object.scene.engine.get
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
