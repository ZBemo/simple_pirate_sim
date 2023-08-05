use bevy_ecs::prelude::*;
use bevy_input::prelude::*;
use bevy_log::prelude::*;
use bevy_math::prelude::*;
use bevy_render::prelude::*;
use bevy_transform::prelude::*;

use crate::{MovementGoalTimer, DIAG_SPEED};
#[cfg(feature = "developer-tools")]
use pirate_sim_console as console;
use pirate_sim_core::goals::MovementGoal;

/// A marker for an entity controlled as a player
#[derive(Component, Default)]
pub struct Controller();

#[derive(Bundle, Default)]
pub struct PlayerControllerBundle {
    movement_goal: MovementGoal,
    timer: MovementGoalTimer,
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
pub(super) fn update_movement_goal(
    char_input_events: Res<Input<KeyCode>>,
    mut player: Query<
        (
            &mut MovementGoal,
            &mut super::MovementGoalTimer,
            &super::WalkSpeed,
        ),
        With<Controller>,
    >,
    #[cfg(feature = "developer-tools")] console_open: Res<console::IsOpen>,
) {
    #[cfg(feature = "developer-tools")]
    if **console_open {
        return;
    }

    let (mut movement_goal, mut movement_goal_timer, walk_speed) =
        player.get_single_mut().expect("Player not found");

    // return if no movement was requested
    let Some(wanted_dir) = char_input_events
        .get_pressed()
        .fold(None, |acc, event| {
                
            match event {
                KeyCode::W => Some(Vec3::Y),
                KeyCode::A => Some(Vec3::NEG_X),
                KeyCode::X => Some(Vec3::NEG_Y),
                KeyCode::D => Some(Vec3::X),
                KeyCode::E => Some(Vec3::X + Vec3::Y),
                KeyCode::Q => Some(Vec3::NEG_X + Vec3::Y),
                KeyCode::Z => Some(Vec3::NEG_Y + Vec3::NEG_X),
                KeyCode::C => Some(Vec3::NEG_Y + Vec3::X),
                _ => None,
            }.map(|dir| dir + acc.unwrap_or(Vec3::ZERO))
            
        }) else {return};

    let wanted_dir = wanted_dir.clamp(Vec3::NEG_ONE, Vec3::ONE);

    let amt_directions_requested =
        (wanted_dir.y as i32).signum().abs() + (wanted_dir.x as i32).signum().abs();

    let diagonal_loss = if amt_directions_requested == 2 {
        DIAG_SPEED
    } else {
        1.
    };

    *movement_goal = MovementGoal(walk_speed.0 * diagonal_loss * wanted_dir);
    *movement_goal_timer = MovementGoalTimer::new(1. / (walk_speed.0 * diagonal_loss));
}
