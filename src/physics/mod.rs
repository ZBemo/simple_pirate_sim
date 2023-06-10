//! A tile-based, real-time Physics Engine for this project
//!
//! See [`PhysicsPlugin`], and its build function to get started with the source code, or you can
//! likely read the file from top-down and understand it decently well.
//!
//! Currently, this file should only be for data definitions. Anything that requires a system
//! should be put into its own module.

use bevy::{prelude::*, reflect::GetTypeRegistration};

pub mod collider;
pub mod movement;
pub mod velocity;

/// A resource storing the area of each sprite in the spritesheet. Nearly any conversion between
/// IVec<->Vec should be done trough TileStretch to ensure that sprites are being displayed within
/// the right grid.
#[derive(Resource, Clone, Deref, Reflect)]
pub struct TileStretch(IVec2);

impl TileStretch {
    pub fn bevy_translation_to_tile(&self, t: &Vec3) -> IVec3 {
        // common sense check that t contains only whole numbers before casting
        debug_assert!(
            t.round() == *t,
            "attempted translation of vector with non-whole numbers into tilespace"
        );

        IVec3::new(t.x as i32 / self.x, t.y as i32 / self.y, t.z as i32)
    }
    pub fn tile_translation_to_bevy(&self, t: &IVec3) -> Vec3 {
        Vec3::new(
            t.x as f32 * self.x as f32,
            t.y as f32 * self.y as f32,
            t.z as f32,
        )
    }

    pub fn new(v: IVec2) -> Self {
        v.into()
    }
}

impl From<IVec2> for TileStretch {
    fn from(value: IVec2) -> Self {
        Self(value)
    }
}

/// The gravity constant used for weight velocity gain
pub const GRAVITY: f32 = 9.8;

#[derive(SystemSet, Hash, Debug, Clone, Eq, PartialEq)]
/// We recommend running any system that plans to input into the Physics system before
/// [`PhysicsSet::FinalizeVelocity`], although some may be able to run before
/// [`PhysicsSet::CollisionCheck`] and be fine.
///
/// If wanting to use previously newly update locations, run after [`PhysicsSet::FinalizeMovement`]
///
/// systems making use of collision checking should run after [`PhysicsSet::CollisionCheck`], or
/// collision data may be wildly inaccurate
pub enum PhysicsSet {
    // PhysicsInput,
    FinalizeVelocity,
    FinalizeCollision,
    FinalizeMovement,
}

/// Any component with a weight will have gravity applied to it on each physics update
///
/// Any entity with a Weight will have a velocity of [`GRAVITY`] * weight added to its relative
/// velocity during calculation.
#[derive(Debug, Clone, Copy, Component, Deref, DerefMut, Reflect)]
pub struct Weight(pub f32);

/// A way to request movement for a specific entity. Expects the entity to have a [`VelocityBundle`]
///
/// Each axis on the inner Vec3 represents the entities requested speed in that direction, similar
/// to a force diagram.
///
/// As valid movement is different for each entity, The physics engine does not check for "invalid" movement goals,
/// so it is the responsibility of  whoever is controlling an entity to make sure movement goals are valid before setting them.
#[derive(Debug, Component, Clone, Default, Deref, DerefMut, Reflect)]
pub struct MovementGoal(pub Vec3);

/// The components necessary for movement by the physics engine to take place on an entity's
/// transform.
///
/// You must provide a transform yourself in order to get movement, in order to stay compatible
/// with other bundles.
///
/// TODO: consider clearing ticker even if not attached to a Transform
#[derive(Bundle, Debug, Default)]
pub struct PhysicsComponentBase {
    ticker: movement::Ticker,
    total_velocity: velocity::VelocityBundle,
}

fn register_types_startup(type_registry: Res<AppTypeRegistry>) {
    let mut type_registry_w = type_registry.write();

    type_registry_w.add_registration(movement::Ticker::get_type_registration());
    type_registry_w.add_registration(velocity::RelativeVelocity::get_type_registration());
    type_registry_w.add_registration(velocity::MantainedVelocity::get_type_registration());
    type_registry_w.add_registration(velocity::TotalVelocity::get_type_registration());
    type_registry_w.add_registration(collider::Constraints::get_type_registration());
    type_registry_w.add_registration(collider::Collider::get_type_registration());
    type_registry_w.add_registration(MovementGoal::get_type_registration());
    type_registry_w.add_registration(Weight::get_type_registration());
    type_registry_w.add_registration(TileStretch::get_type_registration());
}

/// A plugin to setup essential physics systems
///
/// Any system that wants to use the results of a physics engine update should not run until after
/// [`PhysicsSet::FinalizeMovement`] has been completed
///
/// Any systems that want to affect the physics engine in a given frame must run before
/// [`PhysicsSet::FinalizeVelocity`].
///
/// See the source of [`PhysicsPlugin::build`] for how systems are ordered.
pub struct PhysicsPlugin;

impl Plugin for PhysicsPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugin(velocity::Plugin())
            .add_plugin(collider::Plugin())
            .add_plugin(movement::Plugin())
            .add_system(register_types_startup.on_startup());
    }
}
