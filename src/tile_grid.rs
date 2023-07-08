//! Materials for establishing and working with a tile "grid" or map for use with spritesheets
//!
//! If something is "on grid" then that means it is the dimensions of tile_stretch*tile_stretch and
//! is situated on a multiple of tile_stretch.{x,y} in the {x,y} direction.
//!
//! So, if the current tile stretch is 32*32, then for something to be on grid translate.{x,y} % 32
//! must = 0. Its Z must be a whole number.
//!
//! We assume Zs are always a whole number.

use std::{borrow::Borrow, marker::PhantomData};

use bevy::{prelude::*, reflect::GetTypeRegistration};
use thiserror::Error;

/// A resource storing the area of each sprite in the spritesheet. Nearly any conversion between
/// IVec<->Vec should be done trough TileStretch to ensure that sprites are being displayed within
/// the right grid.
///
/// This should be a UVec2 for proper typing, but IVec2 makes conversions easier?
#[derive(Resource, Clone, Copy, Reflect, Debug)]
pub struct TileStretch(pub u8, pub u8);

impl From<IVec2> for TileStretch {
    fn from(value: IVec2) -> Self {
        Self::new(value.x as u8, value.y as u8)
    }
}

impl From<UVec2> for TileStretch {
    fn from(value: UVec2) -> Self {
        Self::new(value.x as u8, value.y as u8)
    }
}

impl From<TileStretch> for IVec2 {
    fn from(value: TileStretch) -> Self {
        Self::new(value.0 as i32, value.1 as i32)
    }
}

impl From<TileStretch> for UVec2 {
    fn from(value: TileStretch) -> Self {
        Self::new(value.0 as u32, value.1 as u32)
    }
}

impl From<TileStretch> for Vec2 {
    fn from(value: TileStretch) -> Self {
        Self::new(value.0 as f32, value.1 as f32)
    }
}

/// An error in conversion from bevy types
///
/// Can only originate from [`TileStretch::get_tile`], and is bound to the lifetime of the two
/// arguments of that function.
#[derive(Error, Debug)]
#[error("Coordinates {to_translate} not divisible by stretch {:?}",tile_stretch.0)]
pub struct GetTileError<'a, 'b, V: Borrow<Vec3> + 'a> {
    to_translate: V,
    tile_stretch: &'b TileStretch,
    // ensures this doesn't outlive V which should live for 'a
    ensurance: PhantomData<&'a ()>,
}

impl<'a, 'b, V: Borrow<Vec3>> GetTileError<'a, 'b, V> {
    fn new(to_translate: V, tile_stretch: &'b TileStretch) -> Self {
        Self {
            to_translate,
            tile_stretch,
            // ensure this lives as long as V?
            ensurance: PhantomData::default(),
        }
    }

    /// Translates the original translation to its closest grid tile.
    ///
    /// This is useful for error recovery: for example; moving an entity to the closest tile
    /// location, or simply ignoring that it's off-grid and continuing as normal.
    pub fn to_closest(&self) -> IVec3 {
        self.tile_stretch.get_closest(self.to_translate.borrow())
    }
}

impl TileStretch {
    /// returns closest tile from a bevy translation
    pub fn get_closest(&self, t: impl Borrow<Vec3>) -> IVec3 {
        let t = t.borrow();
        IVec3::new(
            t.x as i32 / self.0 as i32,
            t.y as i32 / self.1 as i32,
            t.z as i32,
        )
    }

    /// Fallible translation from bevy-space to tilespace.
    ///
    ///  It will return an error if the provided translation does not lie on grid. For graceful
    ///  recovery, you will probably want to call [`GetTileError::to_closest`]
    ///
    /// This should be renamed try_into_tile or something similar. Then we should re-evaluate the
    /// name of [`Self::get_closest`]
    pub fn get_tile<'a, 'b, V: Borrow<Vec3> + 'a>(
        &'b self,
        t: V,
    ) -> Result<IVec3, GetTileError<'a, 'b, V>> {
        if t.borrow().round() != *t.borrow()
            || t.borrow().x as i32 % self.0 as i32 != 0
            || t.borrow().y as i32 % self.1 as i32 != 0
        {
            Err(GetTileError::new(t, self))
        } else {
            Ok(self.get_closest(t))
        }
    }

    /// Take a tile translation and translate it bevy space. This is infallible, as all tile space
    /// should translate into bevy-space, ignoring floating point errors which we are not concerned with.
    pub fn get_bevy(&self, t: impl Borrow<IVec3>) -> Vec3 {
        let t = t.borrow();
        Vec3::new(
            t.x as f32 * self.0 as f32,
            t.y as f32 * self.1 as f32,
            t.z as f32,
        )
    }

    pub fn new(x: u8, y: u8) -> Self {
        Self(x, y)
    }
}

pub fn register_types(type_registry: Res<AppTypeRegistry>) {
    let mut type_registry_w = type_registry.write();

    type_registry_w.add_registration(TileStretch::get_type_registration());
}

#[cfg(test)]
mod test {
    use bevy::prelude::{IVec3, Vec3};

    use super::TileStretch;

    #[test]
    fn round_trip() {
        let start = Vec3::new(32., 64., 3.);
        let tile_stretch = TileStretch(32, 32);

        let cast_to_grid = tile_stretch.get_tile(start).unwrap();

        assert_eq!(cast_to_grid, IVec3::new(1, 2, 3));

        let cast_to_bevy = tile_stretch.get_bevy(cast_to_grid);

        assert_eq!(start, cast_to_bevy);
    }

    #[test]
    fn fail_off_grid() {
        let start = Vec3::new(33., 64., 3.);
        let tile_stretch = TileStretch(32, 32);

        let cast_to_grid = tile_stretch.get_tile(start);

        assert!(cast_to_grid.is_err());

        let closest = cast_to_grid.unwrap_err().to_closest();

        assert_eq!(closest, IVec3::new(1, 2, 3));
        assert_eq!(tile_stretch.get_closest(&start), closest);
    }
}
