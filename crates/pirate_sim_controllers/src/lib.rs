//! Controllers for the player and AI characters
//!
//! The final output of controllers will be "requests" to actually change physical world

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

use std::time::Duration;

use bevy_app::{App, PostUpdate, Startup, Update};
use bevy_derive::{Deref, DerefMut};
use bevy_ecs::prelude::*;
use bevy_log::warn;
use bevy_math::prelude::*;
use bevy_reflect::{prelude::*, GetTypeRegistration};
use bevy_time::{Time, Timer};

use pirate_sim_core::goals::MovementGoal;
use pirate_sim_core::PhysicsSet;

pub mod player;

// make diagonals a little slower so they're less desireable
const DIAG_SPEED: f32 = 1. / 1.5;

#[derive(Component, Default, Reflect, Deref, DerefMut)]
pub struct WalkSpeed(pub f32);

#[derive(Component, Debug, Default, Reflect, Deref)]
pub(self) struct MovementGoalTimer(Timer);

impl MovementGoalTimer {
    fn new(start: f32) -> Self {
        Self(Timer::from_seconds(start, bevy_time::TimerMode::Once))
    }
}

/// A system to timeout movement goals based on their timeout component.
///
/// Should run after physics updates
fn count_down_goals(
    mut components: Query<(Option<&mut MovementGoal>, &mut MovementGoalTimer)>,
    time: Res<Time>,
) {
    let delta = time.delta();

    components.for_each_mut(|(movement_goal, mut timer)| {
        if timer.0.finished() {
            if let Some(mut movement_goal) = movement_goal {
                movement_goal.0 = Vec3::ZERO;
            }
        } else {
            timer.0.tick(delta);
        }
    });
}

fn register_types(type_registry: Res<AppTypeRegistry>) {
    let mut type_registry_w = type_registry.write();

    type_registry_w.add_registration(self::MovementGoalTimer::get_type_registration());
    type_registry_w.add_registration(self::WalkSpeed::get_type_registration());
}

pub struct Plugin;
impl bevy_app::Plugin for Plugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, register_types)
            .add_systems(PostUpdate, count_down_goals)
            .add_systems(
                Update,
                player::update_movement_goal.in_set(PhysicsSet::Input),
            );
    }
}
