use std::collections::HashMap;
use glam::{Vec2, Vec3, Vec4};
use rapier2d::prelude::*;
use rayon::iter::ParallelIterator;
use shipyard::{World, Component, EntityId, View, IntoIter, IntoWithId, Unique, UniqueViewMut, ViewMut, AddComponent, Get};

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
        //self.world.add_entity(Transform2D { position: Vec2 { x: 1.0, y: 2.0 }, rotation: 30.0 });



        /*self.world.run(|mut physics2d_manager: UniqueViewMut<Physics2DManager>| {
            let physics2d_manager = physics2d_manager.as_mut();

            /* Create the ground. */
            let collider = ColliderBuilder::cuboid(100.0, 0.1).build();
            physics2d_manager.collider_set.insert(collider);

            /* Create the bouncing ball. */
            let rigid_body = RigidBodyBuilder::dynamic()
                    .translation(vector![0.0, 10.0])
                    .build();
            let collider = ColliderBuilder::new(SharedShape::cuboid(0.5, 0.5)).restitution(0.7).build();
            let ball_body_handle = physics2d_manager.rigid_body_set.insert(rigid_body);
            physics2d_manager.collider_set.insert_with_parent(collider, ball_body_handle, &mut physics2d_manager.rigid_body_set);

            /* Run the game loop, stepping the simulation once per frame. */
            for _ in 0..200 {
                physics2d_manager.update();

                let ball_body = &physics2d_manager.rigid_body_set[ball_body_handle];
                log::info!("Ball altitude: {}", ball_body.translation().y);
            }
        });*/

        self.world.add_entity((Transform2D { position: Vec2 { x: 0.0, y: 10.0 }, rotation: 0.0 },
                RigidBody2D::new(RigidBodyType::Dynamic),
                Collider2D::new(SharedShape::cuboid(0.5, 0.5), 0.7)));
        self.world.add_entity((Transform2D { position: Vec2 { x: 0.0, y: 0.0 }, rotation: 0.0 },
                Collider2D::new(SharedShape::cuboid(100.0, 0.1), 0.7)));
        self.world.run(physics2d_maintain_system);

        for _ in 0..200 {
            self.world.run(physics2d_update_system);
            let mut world_data = WorldData::new();
            world_data.add_component::<Transform2D>(&self.world);
            //world_data.add_component::<RigidBody2D>(&self.world);
            //world_data.add_component::<Collider2D>(&self.world);
            log::info!("world_data={:?}", world_data);
        }
    }

    fn update(&mut self) {
        log::info!("Engine::update");
        //self.world.run(|mut physics2d_manager: UniqueViewMut<Physics2DManager>| {
        //    physics2d_manager.update();
        //});
        //self.world.run(physics2d_update_system);

        //let mut world_data = WorldData::new();
        //world_data.add_component::<Transform2D>(&self.world);
        //world_data.add_component::<RigidBody2D>(&self.world);
        //world_data.add_component::<Collider2D>(&self.world);
        //log::info!("world_data={:?}", world_data);
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

        log::info!("update rb2d! col2d.get(e)={:?}", col2d.get(e));
        if let Ok(col2d) = col2d.get(e) {
            log::info!("physics2d_manager.collider_set.contains(col2d.handle)={:?}", physics2d_manager.collider_set.contains(col2d.handle));
            if physics2d_manager.collider_set.contains(col2d.handle) {
                log::info!("update parent!");
                physics2d_manager.collider_set.set_parent(col2d.handle, Some(rb2d.handle), &mut physics2d_manager.rigid_body_set)
            }
        }
    }

    log::info!("count={:?}", col2d.inserted_or_modified_mut().iter().count());
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

#[derive(Component, Debug, Default)]
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
