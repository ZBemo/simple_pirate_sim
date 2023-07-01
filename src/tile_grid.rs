//! Materials for establishing and working with a tile "grid" or map for use with spritesheets
//!
//! If something is "on grid" then that means it is the dimensions of tile_stretch*tile_stretch and
//! is situated on a multiple of tile_stretch.{x,y} in the {x,y} direction.
//!
//! So, if the current tile stretch is 32*32, then for something to be on grid translate.{x,y} % 32
//! must = 0. Its Z must be a whole number.
//!
//! We assume Zs are always a whole number.

use bevy::{prelude::*, reflect::GetTypeRegistration};
use thiserror::Error;

/// A resource storing the area of each sprite in the spritesheet. Nearly any conversion between
/// IVec<->Vec should be done trough TileStretch to ensure that sprites are being displayed within
/// the right grid.
///
/// This should be a UVec2 for proper typing, but IVec2 makes conversions easier?
#[derive(Resource, Clone, Deref, Reflect, Debug)]
pub struct TileStretch(IVec2);

/// An error in conversion from bevy types
///
/// Can only originate from [`TileStretch::get_tile`], and is bound to the lifetime of the two
/// arguments of that function.
#[derive(Error, Debug)]
#[error("Coordinates {to_translate} not divisible by stretch {:?}",tile_stretch.0)]
pub struct GetTileError<'a, 'b> {
    to_translate: &'a Vec3,
    tile_stretch: &'b TileStretch,
}

impl<'a, 'b> GetTileError<'a, 'b> {
    fn new(to_translate: &'a Vec3, tile_stretch: &'b TileStretch) -> Self {
        Self {
            to_translate,
            tile_stretch,
        }
    }

    /// Translates the original translation to its closest grid tile.
    ///
    /// This is useful for error recovery: for example; moving an entity to the closest tile
    /// location, or simply ignoring that it's off-grid and continuing as normal.
    pub fn to_closest(&self) -> IVec3 {
        self.tile_stretch.get_closest(self.to_translate)
    }
}

impl TileStretch {
    /// returns closest tile from a bevy translation
    pub fn get_closest(&self, t: &Vec3) -> IVec3 {
        IVec3::new(t.x as i32 / self.x, t.y as i32 / self.y, t.z as i32)
    }

    /// Fallible translation from bevy-space to tilespace.
    ///
    /// Currently it will only return a `Err([FromBevyError])` if the provided translation does not lie on grid.
    ///
    /// This should be renamed try_into_tile or something similar. Then we should re-evaluate the
    /// name of [`closest_tile`]
    pub fn get_tile<'a, 'b>(&'b self, t: &'a Vec3) -> Result<IVec3, GetTileError<'a, 'b>> {
        if t.round() != *t || t.x as i32 % self.x != 0 || t.y as i32 % self.y != 0 {
            Err(GetTileError::new(t, self))
        } else {
            Ok(self.get_closest(t))
        }
    }

    /// Take a tile translation and translate it bevy space. This is infallible, as all tile space
    /// should translate into bevy-space, ignoring floating point errors which we are not concerned with.
    pub fn get_bevy(&self, t: &IVec3) -> Vec3 {
        Vec3::new(
            t.x as f32 * self.x as f32,
            t.y as f32 * self.y as f32,
            t.z as f32,
        )
    }

    pub fn new(v: IVec2) -> Self {
        Self(v)
    }
}

pub fn register_types(type_registry: Res<AppTypeRegistry>) {
    let mut type_registry_w = type_registry.write();

    type_registry_w.add_registration(TileStretch::get_type_registration());
}
