use std::{cell::RefCell, rc::Rc};

use crate::{
    physics,
    random::RandomGenerator,
    ships::BASIC_SHIP,
    tile_grid::TileStretch,
    tile_objects::{self, SpriteSheetHandle},
};
use bevy::{
    ecs::system::{CommandQueue, SystemState},
    prelude::*,
    transform::commands,
};

use super::SeaLevel;

// setup ships through an exclusive system/Command
fn setup_ships(world: &mut World) {
    let mut system = SystemState::<(
        ResMut<RandomGenerator>,
        Res<TileStretch>,
        Res<SpriteSheetHandle>,
        Res<SeaLevel>,
        Commands,
    )>::new(world);

    let (generator, tile_stretch, spritesheet_handle, sea_level, mut commands) =
        system.get_mut(world);

    // TODO: multiply these by the ship size or something
    const FIRST_SHIP_RANGE: i32 = 200;
    const SECOND_SHIP_OFFSET_MAX: i32 = 20;
    const SECOND_SHIP_OFFSET_MIN: i32 = 10;

    let mut g = generator;
    let first_ship_translate_tile_space = IVec3::new(
        g.range(-FIRST_SHIP_RANGE, FIRST_SHIP_RANGE),
        g.range(-FIRST_SHIP_RANGE, FIRST_SHIP_RANGE),
        sea_level.0,
    );

    // if statement maps {true,false} => {-1,1} to get ship 2 below or to the left as well
    let x_offset = g.range(SECOND_SHIP_OFFSET_MIN, SECOND_SHIP_OFFSET_MAX)
        * if g.rand::<bool>() { -1 } else { 1 };
    let y_offset = g.range(SECOND_SHIP_OFFSET_MIN, SECOND_SHIP_OFFSET_MAX)
        * if g.rand::<bool>() { -1 } else { 1 };

    let second_ship_translate_tile_space = IVec3::new(
        x_offset + first_ship_translate_tile_space.x,
        y_offset + first_ship_translate_tile_space.y,
        1,
    );

    // spawn first ship
    spawn_ship_from_blueprint(
        &first_ship_translate_tile_space,
        (0, 0),
        &BASIC_SHIP,
        &mut commands,
        &tile_stretch,
        &spritesheet_handle,
    );

    todo!();

    system.apply(world); // make it so our changes actually take effect
}

fn spawn_ship_from_blueprint(
    start_translation: &IVec3,
    dimensions: (u8, u8),
    blueprint: &[&str],
    commands: &mut Commands,
    tile_stretch: &TileStretch,
    spritesheet_handle: &Handle<TextureAtlas>,
) {
    let ship = commands
        .spawn((
            Transform::from_translation(tile_stretch.get_bevy(start_translation)),
            physics::PhysicsComponentBase::default(),
        ))
        .id();

    // todo convert this to Iter::enumerate

    for z in 0..blueprint.len() {
        for x in 0..dimensions.0 as usize {
            for y in 0..dimensions.1 as usize {
                // index into blueprint
                let char = blueprint[z]
                    .chars()
                    .nth(x * dimensions.0 as usize + y)
                    .expect("Malformed blueprint or incorrect dimensions - attempted to index char that does not exist");

                let current_translation = IVec3::new(x as i32, y as i32, z as i32);

                match char {
                    ' ' => {} // ignore spaces
                    'w' => spawn_wall(
                        commands,
                        tile_stretch.get_bevy(current_translation),
                        ship,
                        spritesheet_handle,
                    ),
                    c => {
                        panic!("blueprint char {} not recognized", c)
                    }
                }
            }
        }
    }
}

fn spawn_wall(
    commands: &mut Commands,
    location: Vec3,
    parent: Entity,
    spritesheet_handle: &Handle<TextureAtlas>,
) {
    commands
        .spawn((
            physics::collider::Collider::new(physics::collider::Constraints::WALL),
            physics::velocity::VelocityBundle::default(),
            tile_objects::TileObject::new(202, 203, 204),
            Name::new("Ship Wall"),
            SpriteSheetBundle {
                // TODO: dynamically update walls or something
                sprite: TextureAtlasSprite::new(202),
                texture_atlas: spritesheet_handle.clone(),
                transform: Transform::from_translation(location),
                ..default()
            },
        ))
        .set_parent(parent);
}
