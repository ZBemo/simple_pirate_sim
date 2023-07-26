//! A tile-based, real-time Physics Engine for this project
//!
//! See [`PhysicsPlugin`], and its build function to get started with the source code, or you ca

#![warn(clippy::unwrap_used)]
#![warn(clippy::perf, clippy::disallowed_types)] // performance warns
#![warn(clippy::pedantic)]
// most bevy systems violate these. Nothing I can do about it at the moment.
#![allow(
    clippy::type_complexity,
    clippy::too_many_arguments,
    clippy::needless_pass_by_value // TODO: separate out system functions from non-system 
)]
#![allow(clippy::cast_possible_truncation)]

use bevy::{prelude::*, reflect::GetTypeRegistration};
pub use pirate_sim_core::PhysicsSet;

pub use collision::Collider;

pub mod collision;
pub mod movement;
pub mod tile_cast;
pub mod velocity;

pub use pirate_sim_core::goals::MovementGoal;

#[cfg(test)]
pub mod test;

/// The gravity constant used for weight velocity gain
pub const GRAVITY: f32 = 9.8;

/// Any component with a weight will have gravity applied to it on each physics update
///
/// Any entity with a Weight will have a velocity of [`GRAVITY`] * Weight added to its relative
/// velocity during calculation.
#[derive(Debug, Clone, Copy, Component, Deref, DerefMut, Reflect)]
pub struct Weight(pub f32);

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

fn startup(type_registry: Res<AppTypeRegistry>, mut commands: Commands) {
    // register raycast command
    #[cfg(feature = "developer-tools")]
    commands.add(
        pirate_sim_console::registration::RegisterConsoleCommand::new(
            "raycast".into(),
            tile_cast::console::raycast_console,
        ),
    );

    let mut type_registry_w = type_registry.write();

    type_registry_w.add_registration(movement::Ticker::get_type_registration());
    type_registry_w.add_registration(velocity::RelativeVelocity::get_type_registration());
    type_registry_w.add_registration(velocity::MantainedVelocity::get_type_registration());
    type_registry_w.add_registration(velocity::TotalVelocity::get_type_registration());
    type_registry_w.add_registration(collision::Constraints::get_type_registration());
    type_registry_w.add_registration(collision::Collider::get_type_registration());
    type_registry_w.add_registration(collision::CollisionMap::get_type_registration());
    type_registry_w.add_registration(MovementGoal::get_type_registration());
    type_registry_w.add_registration(Weight::get_type_registration());
}

/// A plugin to setup essential physics systems
///
/// Any system that wants to use the results of a physics engine update should not run until after
/// [`PhysicsSet::Movement`] has been completed
///
/// Any systems that want to affect the physics engine in a given frame must run before
/// [`PhysicsSet::Velocity`].
pub struct PhysicsPlugin;

impl Plugin for PhysicsPlugin {
    fn build(&self, app: &mut App) {
        app.configure_set(Update, PhysicsSet::Input.before(PhysicsSet::Velocity))
            .configure_set(Update, PhysicsSet::Velocity.before(PhysicsSet::Collision))
            .configure_set(Update, PhysicsSet::Collision.after(PhysicsSet::Velocity))
            .configure_set(Update, PhysicsSet::Movement.after(PhysicsSet::Collision))
            .configure_set(Update, PhysicsSet::Completed.after(PhysicsSet::Movement))
            .add_plugins((velocity::Plugin, collision::Plugin, movement::Plugin))
            .add_systems(Startup, startup);
    }
}
