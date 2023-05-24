//! Controllers for the player and AI characters
//!
//! The final output of controllers will be "requests" to actually change physical world, like
//! how MovementGoals are a request to change entities location trough velocity.

use bevy::prelude::*;

use crate::physics::MovementGoal;

// make diagonals a little slower so they're less desireable
const DIAG_SPEED: f32 = 1. / 1.5;

/// This should probably be a f32 as it is exponentially more granular than necessary
#[derive(Component, Default)]
pub struct MovementGoalTimeout(pub f64);
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
                goal.0 = Vec3::ZERO;
            }
        }
    }
}

pub mod npc {
    #[allow(unused)]
    use bevy::prelude::*;
    //todo;
}

pub mod player {
    use crate::{controllers::DIAG_SPEED, physics::MovementGoal};
    use bevy::prelude::*;

    /// A marker for an entity controlled as a player
    #[derive(Component, Default)]
    pub struct Controller();

    /// a system to make the player the center of the screen
    #[allow(unused)]
    pub fn camera_follow_player(
        player: Query<(&Controller, &Transform), (Without<Camera>, Changed<Transform>)>,
        mut cameras: Query<(&mut Transform, &Camera), Without<Controller>>,
    ) {
        if let Ok(player) = player.get_single() {
            // in the future with multi camera system this will need to iterate
            let mut camera = cameras.get_single_mut().expect("Camera not found");

            camera.0.translation = player.1.translation;
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
            player.get_single_mut().expect("Player not found");

        for event in char_input_events.iter() {
            match event.char {
                'w' => {
                    **movement_goal = Vec3::Y * walk_speed.0;
                    // should go one tile
                    movement_timeout.0 = 1. / walk_speed.0 as f64;
                }
                'a' => {
                    **movement_goal = Vec3::X * walk_speed.0 * -1.;
                    // should go one tile
                    movement_timeout.0 = 1. / walk_speed.0 as f64;
                }
                'x' => {
                    **movement_goal = Vec3::Y * walk_speed.0 * -1.;
                    // should go one tile
                    movement_timeout.0 = 1. / walk_speed.0 as f64;
                }
                'd' => {
                    **movement_goal = Vec3::X * walk_speed.0;
                    // should go one tile
                    movement_timeout.0 = 1. / walk_speed.0 as f64;
                }
                'e' => {
                    **movement_goal = (Vec3::Y + Vec3::X) * walk_speed.0 * DIAG_SPEED;
                    // should go one tile
                    movement_timeout.0 = 1. / (walk_speed.0 as f64 * DIAG_SPEED as f64);
                }
                'q' => {
                    **movement_goal = (Vec3::Y + Vec3::X * -1.) * walk_speed.0 * DIAG_SPEED;
                    // should go one tile
                    movement_timeout.0 = 1. / (walk_speed.0 as f64 * DIAG_SPEED as f64);
                }
                'z' => {
                    **movement_goal = (Vec3::Y + Vec3::X) * -1. * walk_speed.0 * DIAG_SPEED;
                    // should go one tile
                    movement_timeout.0 = 1. / (walk_speed.0 as f64 * DIAG_SPEED as f64);
                }
                'c' => {
                    **movement_goal = (Vec3::Y * -1. + Vec3::X) * walk_speed.0 * DIAG_SPEED;
                    // should go one tile
                    movement_timeout.0 = 1. / (walk_speed.0 as f64 * DIAG_SPEED as f64);
                }
                c => {
                    debug!("Ignoring unregistered char '{}'", c)
                }
            }
        }
        trace!(
            "current movement goal: {}. current movement timeout: {}",
            **movement_goal,
            movement_timeout.0
        );
    }
}
