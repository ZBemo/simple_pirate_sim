//! Functions and systems for creating and updating ships
//!
//! 'w' = wall
//!
//! 'f' = floor
//!
//! '>' = up, '<' = down
//!
//! 's' = steering wheel
//!
//! 'c' = canon
//!
//! ' ' = open space
use std::ops::Add;

use bevy::prelude::*;

use crate::{
    physics::{self, Collider, LinkVelocity, PhysicsComponentBase},
    random::{RandomGenerator, Seed},
    tile_objects::{DynWallObject, ObjectName, TileStretch},
};

/// a basic template for a ship. not piratey at all because I suck at art
/// this ship is not yet leak proof
const BASIC_SHIP: [&str; 3] = [
    "
     www  
    wwfww  
   wwfffww 
  wwfffffww
  wwfffffww
  wwff>ffww
  wwfffffww
  wwwwwwwww",
    "
     www   
    wwfww  
   wwfffww 
  wwffsffww
  wwrfffrww
  wwff<ffww
  wwfffffww
  wwwwwwwww",
    "
     fff    
    ff ff  
   ff   ff 
  ff     ff
  cf     fc
  ff     ff
  ff     ff
  fffffffff",
];

#[derive(Component)]
pub struct Ship;

/// this doesn't belong here. the sea level of the world
#[derive(Debug, Resource, Deref)]
pub struct SeaLevel(i32);

// setup ships system
fn setup_ships(
    mut commands: Commands,
    tile_stretch: Res<TileStretch>,
    mut generator: ResMut<RandomGenerator>,
    asset_server: Res<AssetServer>,
    sea_level: Res<SeaLevel>,
) {
    // TODO: multiply these by the ship size or something
    const FIRST_SHIP_RANGE: i32 = 200;
    const SECOND_SHIP_OFFSET_MAX: i32 = 20;
    const SECOND_SHIP_OFFSET_MIN: i32 = 10;

    let mut g = generator;
    let first_ship_translate_tile_space = &IVec3::new(
        g.range(-FIRST_SHIP_RANGE, FIRST_SHIP_RANGE),
        g.range(-FIRST_SHIP_RANGE, FIRST_SHIP_RANGE),
        sea_level.0,
    );

    // if statement maps {0,1} => {-1,1} to get ship 2 below or to the left as well
    let x_offset = g.range(SECOND_SHIP_OFFSET_MIN, SECOND_SHIP_OFFSET_MAX)
        * if g.range(0, 1) == 1 { -1 } else { 1 };
    let y_offset = g.range(SECOND_SHIP_OFFSET_MIN, SECOND_SHIP_OFFSET_MAX)
        * if g.range(0, 1) == 1 { -1 } else { 1 };

    let second_ship_translate_tile_space = &IVec3::new(
        x_offset + first_ship_translate_tile_space.x,
        y_offset + first_ship_translate_tile_space.y,
        1,
    );

    todo!()
}

fn spawn_ship_from_blueprint(
    start_position: Vec3,
    dimensions: (u8, u8),
    blueprint: &[&str],
    commands: &mut Commands,
    tile_stretch: &TileStretch,
) -> () {
    let mut position = start_position;

    let ship = commands
        .spawn((
            Transform::from_translation(start_position),
            physics::PhysicsComponentBase::default(),
        ))
        .id();

    for z in 0..blueprint.len() {
        for x in 0..dimensions.0 as usize {
            for y in 0..dimensions.1 as usize {
                // index into blueprint
                let char = blueprint[z]
                    .chars()
                    .take(x * dimensions.0 as usize + y)
                    .last()
                    .unwrap();

                match char {
                    // 'w' => spawn_wall(commands, position.clone(), &ship, asset_server, sprites),
                    c => {
                        panic!("blueprint char {} not recognized", c)
                    }
                }

                position.y += 1.;
            }
            position.x += 1.;
        }
        position.z += 1.
    }
}

fn spawn_wall(
    commands: &mut Commands,
    location: Vec3,
    parent: &Entity,
    asset_server: Res<AssetServer>,
    sprites: Res<Assets<TextureAtlas>>,
) {
    // only one spritesheet lol
    let texture_atlas_handle = sprites.get_handle(sprites.iter().next().unwrap().0);

    commands.spawn((
        Collider(IVec3::ONE),
        physics::TotalVelocity::default(),
        LinkVelocity(*parent),
        DynWallObject(),
        ObjectName("Ship Wall".into()),
        SpriteSheetBundle {
            // TODO: dynamically update walls or something
            sprite: TextureAtlasSprite::new(202),
            texture_atlas: texture_atlas_handle,
            transform: Transform::from_translation(location),
            ..default()
        },
    ));
}
