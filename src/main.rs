// bevy requires complex types, most of which are quite reaable
#![allow(clippy::type_complexity)]
#![warn(clippy::unwrap_used)]

mod controllers;
mod gui;
mod physics;
mod random;
mod ships;
mod tile_objects;

use bevy::{prelude::*, reflect::GetTypeRegistration};
use bevy_inspector_egui::quick::WorldInspectorPlugin;
use controllers::{MovementGoalTimeout, WalkSpeed};
use physics::{
    collider::Collider, MovementGoal, PhysicsComponentBase, PhysicsPlugin, PhysicsSet, Weight,
};
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
    collider: Collider,
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
        .add_plugin(WorldInspectorPlugin::new())
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
    // window_q: Query<&Window, With<PrimaryWindow>>,
    asset_server: Res<AssetServer>,
    mut sprites: ResMut<Assets<TextureAtlas>>,
    type_registry: Res<AppTypeRegistry>, // mut tilestretch: ResMut<TileStretch>,
) {
    let mut type_registry_w = type_registry.write();

    type_registry_w.add_registration(physics::movement::Ticker::get_type_registration());
    type_registry_w.add_registration(physics::velocity::RelativeVelocity::get_type_registration());
    type_registry_w.add_registration(physics::velocity::TotalVelocity::get_type_registration());

    // dwarfs (0,2)
    // TODO: ACTUAL Sprite sheet code
    let tilestretch: TileStretch = TileStretch::new(IVec2::ONE * 32);
    commands.insert_resource(tilestretch.clone());

    let texture_handle = asset_server.load("tilesets/main.png");

    // TODO: image manipulation & get data for tilestretch

    let texture_atlas =
        TextureAtlas::from_grid(texture_handle, tilestretch.as_vec2(), 16, 16, None, None);

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
        Name::new("Random Wall"),
        Collider::new(physics::collider::Constraints::WALL),
    ));

    // player
    commands.spawn((
        PlayerBundle {
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
            collider: Collider::new(physics::collider::Constraints {
                solid_planes: BVec3::TRUE,
                move_along: BVec3::TRUE,
            }),
        },
        Name::new("Player"),
    ));

    // continue this
}
