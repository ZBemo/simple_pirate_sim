#![allow(unused)]

mod controllers;
mod gui;
mod physics;
mod random;
mod ships;
mod tile_objects;

use bevy::prelude::*;
use bevy::window::PrimaryWindow;
use controllers::{MovementGoalTimeout, WalkSpeed};
use physics::{Collider, MovementGoal, PhysicsComponentBase, PhysicsPlugin, PhysicsSet, Weight};
use tile_objects::{cull_non_camera_layer_sprites, TileObject, TileStretch};

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

#[derive(SystemSet, Hash, Eq, PartialEq, Debug, Clone)]
pub enum StartupSets {
    Random,
    Rendering,
    // WorldSetUp,
}

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugin(PhysicsPlugin)
        .add_state::<GameState>()
        .add_startup_system(setup)
        // .add_startup_system(gui::setup_coords_display)
        .add_startup_system(random::setup_generator)
        // .add_system(gui::update_coords_display)
        // .add_system(cull_non_camera_layer_sprites.after(PhysicsSet::FinalMovement))
        .add_system(controllers::update_goal_timeout.after(PhysicsSet::FinalizeVelocity))
        // .add_system(
        //     controllers::player::camera_follow_player
        //         .after(PhysicsSet::FinalMovement)
        // )
        .add_system(controllers::player::update_movement_goals.before(PhysicsSet::FinalizeVelocity))
        // add system here
        .run();
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
    // TODO: ACTUAL Sprite sheet code
    let tilestretch = TileStretch(32, 32);
    commands.insert_resource(TileStretch(32, 32));

    let texture_handle = asset_server.load("tilesets/main.png");

    // TODO: image manipulation & get data for tilestretch

    let texture_atlas = TextureAtlas::from_grid(
        texture_handle,
        tilestretch.into_ivec2().as_vec2(),
        16,
        16,
        None,
        None,
    );

    let texture_atlas_handle = sprites.add(texture_atlas);
    commands.insert_resource(tile_objects::SpriteSheetHandle(
        texture_atlas_handle.clone(),
    ));

    //TODO: Save atlas handle as a resource

    commands.spawn(Camera2dBundle {
        transform: Transform::from_xyz(0., 0., 2.),
        ..default()
    });

    // random wall one layer down
    commands.spawn((
        SpriteSheetBundle {
            texture_atlas: texture_atlas_handle.clone_weak(),
            sprite: TextureAtlasSprite::new(5),
            transform: Transform::from_translation(
                tilestretch.tile_translation_to_bevy(&IVec3::new(1, 0, 1)),
            ),
            ..default()
        },
        tile_objects::TileObject(),
        tile_objects::ObjectName("Random Wall".into()),
        Collider::new(IVec3::ONE, physics::CollisionType::Solid),
    ));

    // player
    commands.spawn(
        (PlayerBundle {
            sprite: SpriteSheetBundle {
                texture_atlas: texture_atlas_handle.clone_weak(),
                sprite: TextureAtlasSprite::new(2),
                transform: Transform::from_xyz(0., 0., 1.),
                ..default()
            },
            physics_component: PhysicsComponentBase::default(),
            controller: controllers::player::Controller(),
            movement_goal: MovementGoal(Vec3::ZERO),
            m_goal_timeout: MovementGoalTimeout(0.),
            weight: Weight(0.),
            //TODO: figure out if 1. speed is really 1 grid per second
            walkspeed: WalkSpeed(5.),
        }),
    );

    // continue this
}

fn check_wall_player(
    wall: Query<&Transform, With<TileObject>>,
    player: Query<&Transform, With<controllers::player::Controller>>,
) {
    warn!(
        "player: {}; wall: {}",
        player.single().translation,
        wall.single().translation
    )
}
