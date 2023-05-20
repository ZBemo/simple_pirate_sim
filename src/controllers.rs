//! Controllers for the player and AI characters
//!
//! Final output of controllers will be "requests" to actually change physical world, like
//! MovementGoal is a request to change entities location trough velocity.

use bevy::prelude::*;

use crate::physics::MovementGoal;

// make diagonals a little slower so they're less desireable
const DIAG_SPEED: f32 = 1. / 1.5;

pub mod npc {
    use bevy::prelude::*;
}

#[derive(Component, Default)]
pub struct WalkSpeed(pub f32);

/// A system to timeout movement goals based on their timeout component.
///
/// Should run after physics updates
pub fn update_goal_timeout(
    mut components: Query<(&mut MovementGoalTimeout, &mut MovementGoal)>,
    timer: Res<Time>,
) {
    let delta_time = timer.delta_seconds_f64();

    for (mut timeout, mut goal) in components.iter_mut() {
        if timeout.0 > 0. {
            timeout.0 -= delta_time;

            if timeout.0 < 0. {
                goal.goal = Vec3::ZERO;
            }
        }
    }
}
// might need to be vec3?
#[derive(Component, Default)]
pub struct MovementGoalTimeout(pub f64);

pub mod player {
    use crate::{controllers::DIAG_SPEED, physics::MovementGoal};
    use bevy::prelude::*;
    use std::todo;

    /// an empty event to notify other systems that the player has moved.
    /// is sent out in physics loop
    ///
    /// Could be extrapolated into a generic "subscriber" for when an entity is moved, but that would
    /// be less efecient.
    pub struct PlayerMoved();

    /// A marker for an entity controlled as a player
    #[derive(Component, Default)]
    pub struct Controller();

    /// a system to make the player the center of the screen
    pub fn camera_follow_player(
        player: Query<(&Controller, &Transform), Without<Camera>>,
        mut cameras: Query<(&mut Transform, &Camera), Without<Controller>>,
    ) {
        let player = player.get_single().unwrap();
        let mut camera = cameras.get_single_mut().unwrap();

        if camera.0.translation != player.1.translation {
            camera.0.translation = player.1.translation.clone();
        }
    }

    /// Handle player inputs to do with movement goals.
    pub fn update_movement_goals(
        mut char_input_events: EventReader<ReceivedCharacter>,
        mut player: Query<(
            &Controller,
            &mut super::MovementGoalTimeout,
            &mut MovementGoal,
            &super::WalkSpeed,
        )>,
    ) {
        let (_, mut movement_timeout, mut movement_goal, walk_speed) =
            player.get_single_mut().unwrap();

        for event in char_input_events.iter() {
            match event.char {
                'w' => {
                    movement_goal.goal = Vec3::ZERO;
                    movement_goal.goal += Vec3::Y * walk_speed.0;
                    // should go one tile
                    movement_timeout.0 = 1. / walk_speed.0 as f64;
                }
                'a' => {
                    movement_goal.goal = Vec3::ZERO;
                    movement_goal.goal += Vec3::X * walk_speed.0 * -1.;
                    // should go one tile
                    movement_timeout.0 = 1. / walk_speed.0 as f64;
                }
                'x' => {
                    movement_goal.goal = Vec3::ZERO;
                    movement_goal.goal += Vec3::Y * walk_speed.0 * -1.;
                    // should go one tile
                    movement_timeout.0 = 1. / walk_speed.0 as f64;
                }
                'd' => {
                    movement_goal.goal = Vec3::ZERO;
                    movement_goal.goal += Vec3::X * walk_speed.0;
                    // should go one tile
                    movement_timeout.0 = 1. / walk_speed.0 as f64;
                }
                'e' => {
                    movement_goal.goal = Vec3::ZERO;
                    movement_goal.goal += (Vec3::Y + Vec3::X) * walk_speed.0 * DIAG_SPEED;
                    // should go one tile
                    movement_timeout.0 = 1. / (walk_speed.0 as f64 * DIAG_SPEED as f64);
                }
                'q' => {
                    movement_goal.goal = Vec3::ZERO;
                    movement_goal.goal += (Vec3::Y + Vec3::X * -1.) * walk_speed.0 * DIAG_SPEED;
                    // should go one tile
                    movement_timeout.0 = 1. / (walk_speed.0 as f64 * DIAG_SPEED as f64);
                }
                'z' => {
                    movement_goal.goal = Vec3::ZERO;
                    movement_goal.goal += (Vec3::Y + Vec3::X) * -1. * walk_speed.0 * DIAG_SPEED;
                    // should go one tile
                    movement_timeout.0 = 1. / (walk_speed.0 as f64 * DIAG_SPEED as f64);
                }
                'c' => {
                    movement_goal.goal = Vec3::ZERO;
                    movement_goal.goal += (Vec3::Y * -1. + Vec3::X) * walk_speed.0 * DIAG_SPEED;
                    // should go one tile
                    movement_timeout.0 = 1. / (walk_speed.0 as f64 * DIAG_SPEED as f64);
                }
                c => {
                    trace!("Ignoring unregistered char '{}'", c)
                }
            }
        }
        warn!(
            "current movement goal: {}. current movement timeout: {}",
            movement_goal.goal, movement_timeout.0
        );
    }
}
