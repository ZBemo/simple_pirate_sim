//! Controllers for the player and AI characters
//!
//! The final output of controllers will be "requests" to actually change physical world, like
//! how MovementGoals are a request to change entities location trough velocity.

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

use bevy_app::{App, Startup, Update};
use bevy_derive::{Deref, DerefMut};
use bevy_ecs::prelude::*;
use bevy_math::prelude::*;
use bevy_reflect::{prelude::*, GetTypeRegistration};
use bevy_time::Time;

use pirate_sim_core::goals::MovementGoal;
use pirate_sim_core::PhysicsSet;

pub mod player;

// make diagonals a little slower so they're less desireable
const DIAG_SPEED: f32 = 1. / 1.5;

#[derive(Component, Default, Reflect, Deref, DerefMut)]
pub struct WalkSpeed(pub f32);

#[derive(Component, Reflect, Debug, Clone, Default, Deref, DerefMut)]
/// (MovementGoal, Timeout)
struct MovementGoals(Vec<(Vec3, f32, u8)>);

/// A system to timeout movement goals based on their timeout component.
///
/// Should run after physics updates
fn count_down_goals_timeout(mut components: Query<&mut MovementGoals>, timer: Res<Time>) {
    let delta_time = timer.delta_seconds();

    components.par_iter_mut().for_each_mut(|mut goal_vec| {
        goal_vec.iter_mut().for_each(|goal| goal.1 -= delta_time);
        goal_vec.retain(|(_, timeout, _)| *timeout >= 0.);
    });
}

/// take a list of movement goals and coalesce them into a single movement goal for the physics
/// system
fn goals_to_goal(mut goals_goal_q: Query<(&MovementGoals, &mut MovementGoal)>) {
    for (input, mut output) in goals_goal_q.iter_mut() {
        output.0 = input.iter().fold(Vec3::ZERO, |acc, &(e, _, _)| acc + e);
    }
}

fn register_types(type_registry: Res<AppTypeRegistry>) {
    let mut type_registry_w = type_registry.write();

    type_registry_w.add_registration(self::MovementGoals::get_type_registration());
    type_registry_w.add_registration(self::WalkSpeed::get_type_registration());
}

pub struct Plugin;
impl bevy_app::Plugin for Plugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, register_types)
            .add_systems(Update, count_down_goals_timeout.after(PhysicsSet::Velocity))
            .add_systems(
                Update,
                (player::update_movement_goals, goals_to_goal)
                    .chain()
                    .in_set(PhysicsSet::Input),
            );
    }
}
