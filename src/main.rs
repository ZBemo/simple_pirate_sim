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

mod console;
mod controllers;
mod gui;
mod physics;
mod random;
mod ships;
#[cfg(test)]
mod test;
mod tile_grid;
mod tile_objects;

use bevy::prelude::*;
use bevy_egui::EguiPlugin;
use bevy_inspector_egui::quick::WorldInspectorPlugin;
use controllers::{player::PlayerControllerBundle, WalkSpeed};
use physics::{collider::Collider, PhysicsComponentBase, PhysicsPlugin, Weight};
use tile_grid::TileStretch;
use tile_objects::TileCamera;

pub use bevy_inspector_egui::bevy_egui;

/// an unused gamestate system
// #[derive(Debug, Clone, Copy, Default, Eq, PartialEq, Hash, States)]
// enum GameState {
//     MainMenu,
//     #[default]
//     RealTime,
//     PauseTime,
// }

/// the bundel for spawning a player character
#[derive(Bundle)]
struct PlayerBundle {
    sprite: SpriteSheetBundle,
    physics_component: physics::PhysicsComponentBase,
    weight: Weight,
    walkspeed: WalkSpeed,
    collider: Collider,
    name: Name,
    player_controller_bundle: PlayerControllerBundle,
}

fn main() {
    trace!("Adding plugins");
    let mut app = App::new();
    app.add_plugins(DefaultPlugins)
        .add_plugins(EguiPlugin)
        .add_plugins(WorldInspectorPlugin::new())
        .add_plugins((
            PhysicsPlugin,
            tile_objects::Plugin,
            controllers::Plugin,
            console::Plugin,
        ));
    trace!("setting up resources, adding startup systems");
    app.insert_resource(ClearColor(Color::BLACK))
        .insert_resource(Msaa::Off) // Pixel art doesn't need aa, so keep off for now
        .add_systems(
            Startup,
            (setup, random::setup_generator, tile_grid::register_types),
        );
    trace!("Running app");
    app.run();
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
    // TODO: base on spritesheet
    let tilestretch: TileStretch = TileStretch::new(32, 32);
    commands.insert_resource(tilestretch);

    let texture_handle = asset_server.load("tilesets/main.png");

    // TODO: image manipulation & get data for tilestretch

    let texture_atlas =
        TextureAtlas::from_grid(texture_handle, tilestretch.into(), 16, 16, None, None);

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

    commands.spawn((
        SpriteSheetBundle {
            texture_atlas: texture_atlas_handle.clone(),
            sprite: TextureAtlasSprite::new(5),
            transform: Transform::from_translation(tilestretch.get_bevy(IVec3::new(1, 0, 1))),
            ..default()
        },
        tile_objects::TileObject::new(5, 6, 7),
        Name::new("Random Wall"),
        Collider::new(physics::collider::Constraints::WALL),
    ));

    // moving wall
    commands.spawn((
        SpriteSheetBundle {
            texture_atlas: texture_atlas_handle.clone(),
            sprite: TextureAtlasSprite::new(5),
            transform: Transform::from_translation(tilestretch.get_bevy(IVec3::new(1, 0, 0))),
            ..default()
        },
        tile_objects::TileObject::new(5, 6, 7),
        Name::new("Random Wall"),
        Collider::new(physics::collider::Constraints::WALL),
    ));

    // player
    commands.spawn((PlayerBundle {
        player_controller_bundle: default(),
        sprite: SpriteSheetBundle {
            texture_atlas: texture_atlas_handle,
            sprite: TextureAtlasSprite::new(2),
            transform: Transform::from_xyz(0., 0., 1.),
            ..default()
        },
        physics_component: PhysicsComponentBase::default(),
        weight: Weight(0.),
        //TODO: figure out if 1. speed is really 1 grid per second
        walkspeed: WalkSpeed(5.),
        collider: Collider::new(physics::collider::Constraints::ENTITY),
        name: Name::new("Player"),
    },));

    // continue this
}
