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

// still under heavy development
#![allow(unused)]

use bevy::prelude::*;

use crate::{
    physics::{self, collider::Collider},
    random::Generator,
    tile_grid::TileStretch,
    tile_objects,
};

mod creation;
mod interaction;

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

pub struct ShipBundle;

/// this doesn't belong here. the sea level of the world
#[derive(Debug, Resource, Deref)]
pub struct SeaLevel(i32);

#[derive(Bundle)]
pub struct SteeringWheelBundle {
    main_component: SteeringWheel,
}

#[derive(Component)]
pub struct SteeringWheel {}
