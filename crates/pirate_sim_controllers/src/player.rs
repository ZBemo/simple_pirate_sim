use bevy_ecs::prelude::*;
use bevy_input::prelude::*;
use bevy_log::prelude::*;
use bevy_math::prelude::*;
use bevy_render::prelude::*;
use bevy_transform::prelude::*;

use super::MovementGoals;
use crate::DIAG_SPEED;
#[cfg(feature = "developer-tools")]
use pirate_sim_console as console;
use pirate_sim_core::goals::MovementGoal;

/// A marker for an entity controlled as a player
#[derive(Component, Default)]
pub struct Controller();

#[derive(Bundle, Default)]
pub struct PlayerControllerBundle {
    movement_goals: MovementGoals,
    movement_goal: MovementGoal,
    controler: Controller,
}

/// a system to make the player the center of the screen
#[allow(unused)]
pub(super) fn camera_follow_player(
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
pub(super) fn update_movement_goals(
    char_input_events: Res<Input<KeyCode>>,
    mut player: Query<(&mut MovementGoals, &super::WalkSpeed), With<Controller>>,
    #[cfg(feature = "developer-tools")] console_open: Res<console::IsOpen>,
) {
    #[cfg(feature = "developer-tools")]
    if **console_open {
        return;
    }

    let (mut movement_goals, walk_speed) = player.get_single_mut().expect("Player not found");

    // should never have to grow
    let mut new_goals = Vec::with_capacity(7);

    for event in char_input_events.get_pressed() {
        match event {
            KeyCode::W => {
                new_goals.push((Vec3::Y * walk_speed.0, 1. / walk_speed.0, 0));
            }
            KeyCode::A => {
                new_goals.push((Vec3::X * walk_speed.0 * -1., 1. / walk_speed.0, 0));
            }
            KeyCode::X => {
                new_goals.push((Vec3::Y * walk_speed.0 * -1., 1. / walk_speed.0, 0));
            }
            KeyCode::D => {
                new_goals.push((Vec3::X * walk_speed.0, 1. / walk_speed.0, 0));
            }
            KeyCode::E => {
                new_goals.push((
                    (Vec3::Y + Vec3::X) * walk_speed.0 * DIAG_SPEED,
                    1. / (walk_speed.0 * DIAG_SPEED),
                    0,
                ));
            }
            KeyCode::Q => {
                new_goals.push((
                    (Vec3::Y + Vec3::X * -1.) * walk_speed.0 * DIAG_SPEED,
                    // should go one tile
                    1. / (walk_speed.0 * DIAG_SPEED),
                    0,
                ));
            }
            KeyCode::Z => {
                new_goals.push((
                    (Vec3::Y + Vec3::X) * -1. * walk_speed.0 * DIAG_SPEED,
                    1. / (walk_speed.0 * DIAG_SPEED),
                    0,
                ));
            }
            KeyCode::C => {
                new_goals.push((
                    (Vec3::Y * -1. + Vec3::X) * walk_speed.0 * DIAG_SPEED,
                    1. / (walk_speed.0 * DIAG_SPEED),
                    0,
                ));
            }
            c => {
                debug!("Ignoring unregistered char '{:?}'", c);
            }
        }
    }
    if !new_goals.is_empty() {
        trace!("adding movement goals {:?}", new_goals);

        // player's movement goal should only ever be updated by player input
        movement_goals.0 = new_goals;
    }
}
