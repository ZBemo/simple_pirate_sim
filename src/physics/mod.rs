//! A tile-based, real-time Physics Engine for this project
//!
//! See [`PhysicsPlugin`], and its build function to get started with the source code, or you can
//! likely read the file from top-down and understand it decently well.
//!
//! Currently, this file should only be for data definitions. Anything that requires a system
//! should be put into its own module.

use bevy::prelude::*;

pub mod collider;
pub mod movement;
pub mod velocity;

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
#[derive(Debug, Clone, Copy, Component, Deref, DerefMut)]
pub struct Weight(pub f32);

/// A way to request movement for a specific entity. Expects the entity to have a [`VelocityBundle`]
///
/// Each axis on the inner Vec3 represents the entities requested speed in that direction, similar
/// to a force diagram.
///
/// As valid movement is different for each entity, The physics engine does not check for "invalid" movement goals,
/// so it is the responsibility of  whoever is controlling an entity to make sure movement goals are valid before setting them.
#[derive(Debug, Component, Clone, Default, Deref, DerefMut)]
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
            // resolve collisions system here?
            .add_plugin(movement::Plugin());
    }
}
