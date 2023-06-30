//! Controllers for the player and AI characters
//!
//! The final output of controllers will be "requests" to actually change physical world, like
//! how MovementGoals are a request to change entities location trough velocity.

use bevy::{prelude::*, reflect::GetTypeRegistration};

use crate::physics::{MovementGoal, PhysicsSet};

pub mod player;

// make diagonals a little slower so they're less desireable
const DIAG_SPEED: f32 = 1. / 1.5;

/// This should probably be a f32 as it is exponentially more granular than necessary
#[derive(Component, Default, Reflect)]
pub struct MovementGoalTimeout(pub f64);
#[derive(Component, Default, Reflect, Deref, DerefMut)]
pub struct WalkSpeed(pub f32);

#[derive(Component, Reflect, FromReflect, Debug, Clone, Default, Deref, DerefMut)]
/// (MovementGoal, Timeout)
struct MovementGoals(Vec<(Vec3, f64, u8)>);

/// A system to timeout movement goals based on their timeout component.
///
/// Should run after physics updates
fn count_down_goals_timeout(mut components: Query<&mut MovementGoals>, timer: Res<Time>) {
    let delta_time = timer.delta_seconds_f64();

    components
        .par_iter_mut()
        .for_each_mut(|mut goal_vec| goal_vec.iter_mut().for_each(|goal| goal.1 -= delta_time));
}

/// A system to timeout movement goals based on their timeout component.
///
/// Should run after count_down_goals_timeout
fn remove_timedout_goals(mut components: Query<&mut MovementGoals>) {
    components
        .par_iter_mut()
        .for_each_mut(|mut goal_vec| goal_vec.retain(|&(_, timeout, _)| timeout > 0.));
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
    type_registry_w.add_registration(self::MovementGoalTimeout::get_type_registration());
    type_registry_w.add_registration(self::WalkSpeed::get_type_registration());
}

pub struct Plugin;
impl bevy::prelude::Plugin for Plugin {
    fn build(&self, app: &mut App) {
        app.add_startup_system(register_types)
            .add_system(count_down_goals_timeout.after(PhysicsSet::FinalizeVelocity))
            .add_system(remove_timedout_goals.after(count_down_goals_timeout))
            .add_systems(
                (player::update_movement_goals, goals_to_goal)
                    .chain()
                    .before(PhysicsSet::FinalizeVelocity),
            );
    }
}
