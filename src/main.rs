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

use bevy::{app::AppExit, prelude::*};
use pirate_sim_controllers::{player::PlayerControllerBundle, WalkSpeed};

use pirate_sim_core::tile_grid::TileStretch;

#[cfg(feature = "developer-tools")]
use bevy_inspector_egui::{bevy_egui::EguiPlugin, quick::WorldInspectorPlugin};

use pirate_sim_physics::{Collider, PhysicsPlugin, Weight};
use tile_objects::TileCamera;

#[cfg(feature = "fps-diagnostics")]
use bevy::diagnostic::FrameTimeDiagnosticsPlugin;

#[cfg(feature = "developer-tools")]
mod basic_commands;

mod tile_objects;

/// the bundle for spawning a player character
#[derive(Bundle)]
struct PlayerBundle {
    sprite: SpriteSheetBundle,
    physics_component: pirate_sim_physics::PhysicsComponentBase,
    weight: pirate_sim_physics::Weight,
    walkspeed: pirate_sim_controllers::WalkSpeed,
    collider: pirate_sim_physics::Collider,
    name: Name,
    player_controller_bundle: PlayerControllerBundle,
}

fn quit_on_eq(mut exit: EventWriter<AppExit>, keys: Res<Input<KeyCode>>) {
    if keys.pressed(KeyCode::Equals) {
        exit.send_default();
    }
}

fn main() {
    let mut app = App::new();

    // bevy plugins
    app.add_plugins((
        DefaultPlugins.set(ImagePlugin::default_nearest()),
        #[cfg(feature = "fps-diagnostics")]
        (FrameTimeDiagnosticsPlugin, fps_diagnostics::Plugin),
    ));

    // world inspector & egui plugins
    #[cfg(feature = "developer-tools")]
    app.add_plugins(EguiPlugin);

    #[cfg(feature = "developer-tools")]
    app.add_plugins(WorldInspectorPlugin::new());

    // our plugins
    app.add_plugins((
        PhysicsPlugin,
        tile_objects::Plugin,
        pirate_sim_controllers::Plugin,
        #[cfg(feature = "developer-tools")]
        pirate_sim_console::Plugin,
    ));

    trace!("setting up resources, adding startup systems");
    app.insert_resource(ClearColor(Color::BLACK))
        .insert_resource(Msaa::Off) // Pixel art doesn't need aa, so keep off for now
        .add_systems(
            Startup,
            (
                setup,
                pirate_sim_core::random::setup_generator,
                pirate_sim_core::tile_grid::register_types,
                #[cfg(feature = "developer-tools")]
                basic_commands::setup_basic_commands,
            ),
        );

    #[cfg(feature = "developer-tools")]
    app.add_systems(Update, quit_on_eq);

    trace!("Running app");
    app.run();
}

#[cfg(feature = "fps-diagnostics")]
mod fps_diagnostics {
    use bevy::diagnostic::{DiagnosticsStore, FrameTimeDiagnosticsPlugin};
    use bevy::prelude::*;

    #[derive(Component, Default)]
    struct FpsDisplay;

    fn setup_fps_display(mut commands: Commands, asset_server: Res<AssetServer>) {
        const DEFAULT_FONT: &str = "fonts/FiraCode/FiraCodeNerdFont-Regular.ttf";

        let font_handle = asset_server.load(DEFAULT_FONT);

        let text_section = |text: &str| {
            TextSection::new(
                text,
                TextStyle {
                    font: font_handle.clone(),
                    font_size: 16.,
                    color: Color::WHITE,
                },
            )
        };

        commands.spawn((
            TextBundle::from_sections([text_section("Avg FPS: "), text_section("")]).with_style(
                Style {
                    position_type: PositionType::Absolute,
                    top: Val::Percent(10.),
                    left: Val::Percent(10.),
                    ..default()
                },
            ),
            FpsDisplay,
        ));
    }
    fn update_fps_display(
        fps: Res<DiagnosticsStore>,
        mut text: Query<&mut Text, With<FpsDisplay>>,
    ) {
        let mut text = text
            .get_single_mut()
            .expect("Should only be a single entity marked FpsDisplay");

        let Some(fps) = fps.get(FrameTimeDiagnosticsPlugin::FPS) else {return;};
        let Some(avg) = fps.average() else {return};

        text.sections[1].value = format!("{avg}");
    }

    pub struct Plugin;
    impl bevy::app::Plugin for Plugin {
        fn build(&self, app: &mut App) {
            app.add_systems(Startup, setup_fps_display)
                .add_systems(Update, update_fps_display);
        }
    }
}

/// behemoth setup system needs to be chunked way out
///
/// basically just exists for prototyping
fn setup(
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
        Collider::new(pirate_sim_physics::collision::Constraints::WALL),
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
        Collider::new(pirate_sim_physics::collision::Constraints::WALL),
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
        physics_component: pirate_sim_physics::PhysicsComponentBase::default(),
        weight: Weight(0.),
        //TODO: figure out if 1. speed is really 1 grid per second
        walkspeed: WalkSpeed(5.),
        collider: Collider::new(pirate_sim_physics::collision::Constraints::ENTITY),
        name: Name::new("Player"),
    },));

    // continue this
}
