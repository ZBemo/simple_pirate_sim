#![allow(unused)]

mod controllers;
mod gui;
mod physics;
mod random;
mod ships;
mod tile_objects;

use bevy::prelude::*;
use bevy::window::PrimaryWindow;
use controllers::{player::PlayerMoved, MovementGoalTimeout, WalkSpeed};
use physics::{MovementGoal, PhysicsComponentBase, PhysicsPlugin, PhysicsSet, Weight};
use tile_objects::TileStretch;

/// an unused gamestate system
#[derive(Debug, Clone, Copy, Default, Eq, PartialEq, Hash, States)]
enum GameState {
    MainMenu,
    #[default]
    RealTime,
    PauseTime,
}

/// the bundel for spawning a player character
#[derive(Bundle)]
struct PlayerBundle {
    sprite: SpriteSheetBundle,
    physics_component: physics::PhysicsComponentBase,
    controller: controllers::player::Controller,
    movement_goal: MovementGoal,
    m_goal_timeout: MovementGoalTimeout,
    weight: Weight,
    walkspeed: WalkSpeed,
}

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugin(PhysicsPlugin)
        .add_state::<GameState>()
        .add_event::<PlayerMoved>()
        .add_startup_system(setup)
        .add_startup_system(gui::setup_coords_display)
        .add_system(gui::update_coords_display.run_if(on_event::<PlayerMoved>()))
        .add_system(controllers::update_goal_timeout.after(PhysicsSet::FinalMovement))
        // .add_system(
        //     controllers::player::camera_follow_player
        //         .after(PhysicsSet::FinalMovement)
        //         .run_if(on_event::<PlayerMoved>()),
        // )
        .add_system(controllers::player::update_movement_goals.before(PhysicsSet::FinalMovement))
        // add systems here
        .run();
}

fn setup_menu(mut commands: Commands) -> () {
    // start button

    // take seed for ship generation ?
    // crew size slider?
}

/// behemoth setup system needs to be chunked way out
///
/// basically just exists for prototyping
pub fn setup(
    mut commands: Commands,
    window_q: Query<&Window, With<PrimaryWindow>>,
    asset_server: Res<AssetServer>,
    mut sprites: ResMut<Assets<TextureAtlas>>,
    // mut tilestretch: ResMut<TileStretch>,
) -> () {
    let window = window_q.get_single().unwrap();

    // dwarfs (0,2)
    // TODO: ACTUAL Sprite sheet
    //
    //
    //
    let tilestretch = TileStretch(32, 32);
    commands.insert_resource(TileStretch(32, 32));

    let texture_handle = asset_server.load("tilesets/main.png");

    // TODO: image manipulation & get data for tilestretch

    let texture_atlas =
        TextureAtlas::from_grid(texture_handle, tilestretch.into_vec2(), 16, 16, None, None);

    let texture_atlas_handle = sprites.add(texture_atlas);

    //TODO: Save atlas handle as a resource

    commands.spawn(Camera2dBundle {
        transform: Transform::from_xyz(0., 0., 0.),
        ..default()
    });

    commands.spawn(
        (PlayerBundle {
            sprite: SpriteSheetBundle {
                texture_atlas: texture_atlas_handle,
                sprite: TextureAtlasSprite::new(2),
                transform: Transform::from_xyz(0., 0., 0.),
                ..default()
            },
            physics_component: PhysicsComponentBase::default(),
            controller: controllers::player::Controller(),
            movement_goal: MovementGoal { goal: Vec3::ZERO },
            m_goal_timeout: MovementGoalTimeout(0.),
            weight: Weight(0.),
            // TODO: figure out why 1. isn't 1 grid per second
            walkspeed: WalkSpeed(5.),
        }),
    );
    // continue this
}
