//! A tile-based, real-time Physics Engine for this project
//!
//! See [`PhysicsPlugin`], and its build function to get started with the source code, or you can
//! likely read the file from top-down and understand it decently well.

use bevy::prelude::*;

use crate::tile_objects::TileStretch;

use self::velocity::RelativeVelocity;

pub mod collider;
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

/// A Velocity Ticker, used to keep track of when to actually move a physics component by
/// buffering velocity into its ticker until at least a whole tile has been moved.
///
/// This makes it so that velocities of less than 1 tile per second can be represented in the
/// engine in real time.
///
/// Currently if a component has 0 velocity, its ticker will be reset to 0,0,0. In the future this
/// should be changed so that you can reset your ticker trough a request like RequestResetTicker.
///
/// As this Ticker is meant to be wholely managed by the physics engine, it is not public, and must
/// be instantiated trough a Bundle like [`PhysicsComponentBase`]
#[derive(Debug, Component, Clone, Copy, Default, Deref, DerefMut)]
pub struct MovementTicker(Vec3);

/// Finally, applies any tickers that have moved at least one tile. This is essentially flushing the
/// MovementTicker buffer.
///
/// This will reset any tickers with a TotalVelocity of 0 to 0,0,0. This may lead to bugs in the
/// future
///
/// This will also avoid moving two [`ColliderType::Solid`] into each other by lessening their
/// velocity.
fn finalize_movement(
    mut phsyics_components: Query<(
        &mut Transform,
        &mut MovementTicker,
        &RelativeVelocity,
        Option<&collider::Collider>,
    )>,
    tile_stretch: Res<TileStretch>,
    time: Res<Time>,
) {
    // this will make it so entities only move a tile once an entire tiles worth of movement
    // has been "made", keeping it in a grid based system
    //
    // also converts to 32x32

    for (mut transform, mut ticker, total_velocity, _collider) in phsyics_components.iter_mut() {
        // update ticker, only apply velocity * delta to keep time consistent
        ticker.0 += total_velocity.0 * time.delta_seconds();

        debug!("updating with ticker {}", ticker.0);

        while ticker.0.z.abs() >= 1. {
            transform.translation.z += ticker.0.z.signum();
            ticker.0.z -= 1. * ticker.0.z.signum();
        }
        while ticker.0.y.abs() >= 1. {
            transform.translation.y += tile_stretch.y as f32 * ticker.0.y.signum();
            ticker.0.y -= 1. * ticker.0.y.signum();
        }
        while ticker.0.x.abs() >= 1. {
            transform.translation.x += tile_stretch.x as f32 * ticker.0.x.signum();
            ticker.0.x -= 1. * ticker.0.x.signum();
        }

        // this might break things in the future!
        // if total_velocity is 0 reset ticker to 0
        // this probably does not belong in this system. maybe in its own system?
        if **total_velocity == Vec3::ZERO {
            ticker.0 = Vec3::ZERO;
        }
    }
}

/// The components necessary for movement by the physics engine to take place on an entity's
/// transform.
///
/// You must provide a transform yourself in order to get movement, in order to stay compatible
/// with other bundles.
///
/// TODO: consider clearing ticker even if not attached to a Transform
#[derive(Bundle, Debug, Default)]
pub struct PhysicsComponentBase {
    ticker: MovementTicker,
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
            .add_system(
                finalize_movement
                    .in_set(PhysicsSet::FinalizeMovement)
                    .after(PhysicsSet::FinalizeCollision),
            );
    }
}
