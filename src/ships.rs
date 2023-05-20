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
use bevy::prelude::*;

use crate::{
    physics::{self, Collider, LinkVelocity, PhysicsComponentBase},
    random::{RandomGenerator, Seed},
    tile_objects::{DynWallSprite, ObjectName, TileStretch},
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

// setup ships system
fn setup_ships(
    mut commands: Commands,
    tile_stretch: Res<TileStretch>,
    generator: ResMut<RandomGenerator>,
    asset_server: Res<AssetServer>,
) {
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
        Collider(Vec3::ONE),
        PhysicsComponentBase::default(),
        LinkVelocity(*parent),
        DynWallSprite(),
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
