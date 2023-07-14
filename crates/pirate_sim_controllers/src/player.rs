use super::MovementGoals;
#[cfg(feature = "developer-tools")]
use crate::console;
use crate::DIAG_SPEED;
use bevy::prelude::*;
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
    mut char_input_events: EventReader<ReceivedCharacter>,
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

    for event in char_input_events.iter() {
        match event.char {
            'w' => {
                new_goals.push((Vec3::Y * walk_speed.0, 1. / walk_speed.0, 0));
            }
            'a' => {
                new_goals.push((Vec3::X * walk_speed.0 * -1., 1. / walk_speed.0, 0));
            }
            'x' => {
                new_goals.push((Vec3::Y * walk_speed.0 * -1., 1. / walk_speed.0, 0));
            }
            'd' => {
                new_goals.push((Vec3::X * walk_speed.0, 1. / walk_speed.0, 0));
            }
            'e' => {
                new_goals.push((
                    (Vec3::Y + Vec3::X) * walk_speed.0 * DIAG_SPEED,
                    1. / (walk_speed.0 * DIAG_SPEED),
                    0,
                ));
            }
            'q' => {
                new_goals.push((
                    (Vec3::Y + Vec3::X * -1.) * walk_speed.0 * DIAG_SPEED,
                    // should go one tile
                    1. / (walk_speed.0 * DIAG_SPEED),
                    0,
                ));
            }
            'z' => {
                new_goals.push((
                    (Vec3::Y + Vec3::X) * -1. * walk_speed.0 * DIAG_SPEED,
                    1. / (walk_speed.0 * DIAG_SPEED),
                    0,
                ));
            }
            'c' => {
                new_goals.push((
                    (Vec3::Y * -1. + Vec3::X) * walk_speed.0 * DIAG_SPEED,
                    1. / (walk_speed.0 * DIAG_SPEED),
                    0,
                ));
            }
            c => {
                debug!("Ignoring unregistered char '{}'", c);
            }
        }
    }
    if !new_goals.is_empty() {
        trace!("adding movement goals {:?}", new_goals);

        // player's movement goal should only ever be updated by player input
        movement_goals.0 = new_goals;
    }
}
