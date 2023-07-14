use bevy::{diagnostic::FrameTimeDiagnosticsPlugin, prelude::*};
use pirate_sim_controllers::{player::PlayerControllerBundle, WalkSpeed};
use pirate_sim_core::{
    bevy_egui::EguiPlugin, bevy_inspector_egui::quick::WorldInspectorPlugin, tile_grid::TileStretch,
};
use pirate_sim_physics::{Collider, PhysicsPlugin, Weight};
use tile_objects::TileCamera;

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

pub fn run_game() {
    let mut app = App::new();

    // bevy plugins
    app.add_plugins((
        DefaultPlugins.set(ImagePlugin::default_nearest()),
        FrameTimeDiagnosticsPlugin,
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
            ),
        );

    trace!("Running app");
    app.run();
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
        Collider::new(pirate_sim_physics::collider::Constraints::WALL),
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
        Collider::new(pirate_sim_physics::collider::Constraints::WALL),
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
        collider: Collider::new(pirate_sim_physics::collider::Constraints::ENTITY),
        name: Name::new("Player"),
    },));

    // continue this
}
