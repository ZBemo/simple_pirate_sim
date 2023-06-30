// bevy requires complex types, most of which are quite reaable
#![allow(clippy::type_complexity)]
#![warn(clippy::unwrap_used)]

mod console;
mod controllers;
mod gui;
mod physics;
mod random;
mod ships;
mod tile_grid;
mod tile_objects;

use bevy::prelude::*;
use bevy_egui::EguiPlugin;
use bevy_inspector_egui::quick::WorldInspectorPlugin;
use controllers::{MovementGoalTimeout, WalkSpeed};
use physics::{
    collider::Collider, MovementGoal, PhysicsComponentBase, PhysicsPlugin, PhysicsSet, Weight,
};
use tile_grid::TileStretch;
use tile_objects::TileCamera;

pub use bevy_inspector_egui::bevy_egui;

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
    collider: Collider,
    name: Name,
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
        .add_plugin(EguiPlugin)
        .add_plugin(PhysicsPlugin)
        .add_plugin(tile_objects::Plugin)
        .add_plugin(WorldInspectorPlugin::new())
        .add_plugin(console::Plugin)
        .insert_resource(ClearColor(Color::BLACK))
        .insert_resource(Msaa::Off) // Pixel art doesn't need aa, so keep off for now
        .add_state::<GameState>()
        .add_startup_system(setup)
        .add_startup_system(random::setup_generator)
        .add_startup_system(tile_grid::register_types)
        // .add_system(
        //     controllers::player::camera_follow_player
        //         .after(PhysicsSet::FinalMovement)
        // )
        // add system here
        .run();
}

/// behemoth setup system needs to be chunked way out
///
/// basically just exists for prototyping
pub fn setup(
    mut commands: Commands,
    // window_q: Query<&Window, With<PrimaryWindow>>,
    asset_server: Res<AssetServer>,
    mut sprites: ResMut<Assets<TextureAtlas>>,
) {
    // dwarfs (0,2)
    // TODO: ACTUAL Sprite sheet code
    let tilestretch: TileStretch = TileStretch::new(IVec2::ONE * 32);
    commands.insert_resource(tilestretch.clone());

    let texture_handle = asset_server.load("tilesets/main.png");

    // TODO: image manipulation & get data for tilestretch

    let texture_atlas =
        TextureAtlas::from_grid(texture_handle, tilestretch.as_vec2(), 16, 16, None, None);

    let texture_atlas_handle = sprites.add(texture_atlas);

    // consider if this should be a weak clone. Probably not as we want the texture atlas to be
    // loaded for the duration of the program
    commands.insert_resource(tile_objects::SpriteSheetHandle(
        texture_atlas_handle.clone(),
    ));

    commands.spawn((
        Camera2dBundle {
            transform: Transform::from_xyz(0., 0., 2.),
            ..default()
        },
        TileCamera(),
    ));

    // random wall one layer down
    commands.spawn((
        SpriteSheetBundle {
            texture_atlas: texture_atlas_handle.clone(),
            sprite: TextureAtlasSprite::new(5),
            transform: Transform::from_translation(tilestretch.tile_to_bevy(&IVec3::new(1, 0, 1))),
            ..default()
        },
        tile_objects::TileObject::new(5, 6, 7),
        Name::new("Random Wall"),
        Collider::new(physics::collider::Constraints::WALL),
    ));

    // player
    commands.spawn((PlayerBundle {
        sprite: SpriteSheetBundle {
            texture_atlas: texture_atlas_handle.clone(),
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
        collider: Collider::new(physics::collider::Constraints {
            solid_planes: BVec3::TRUE,
            move_along: BVec3::TRUE,
        }),
        name: Name::new("Player"),
    },));

    // continue this
}
