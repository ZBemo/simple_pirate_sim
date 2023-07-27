//! Materials for establishing and working with a tile "grid" or map for use with spritesheets
//!
//! A [`TileStretch`] defines the canonical dimensions of a single grid on the world's tile grid, which
//! nearly all entities should sit within.
//!
//! [`TileStretch`] exists to hopefully easily deal with the use of different sized spritesheets, and
//! to allow any system that wishes to to work solely at the tilespace level.
//!
//! If something is "on grid" then that means its global transform's x is a multiple of
//! [`TileStretch`].0 and its y is a multiple of [`TileStretch`].1. Its Z should be a whole number.
//! There should only be one [`TileStretch`] per world, as there is only one spritesheet loaded.

use bevy_ecs::prelude::*;
use bevy_math::prelude::*;
use bevy_reflect::{GetTypeRegistration, Reflect};
use bevy_transform::prelude::GlobalTransform;
use thiserror::Error;

/// A resource storing the area of each sprite in the spritesheet. Nearly any conversion between
/// [`IVec3`]<->[`Vec3`] should be done trough [`TileStretch`] to ensure that sprites are being displayed within
/// the right grid.
///
/// `Self::0` is x, `Self::1` is y
///
/// TODO: change from tuple fields to named x,y
#[derive(Resource, Clone, Copy, Reflect, Debug)]
pub struct TileStretch(pub u8, pub u8);

impl From<IVec2> for TileStretch {
    #[inline]
    fn from(value: IVec2) -> Self {
        debug_assert!(value.signum().cmpge(IVec2::ZERO) == BVec2::TRUE);
        #[allow(clippy::cast_sign_loss)]
        Self::new(value.x as u8, value.y as u8)
    }
}

impl From<UVec2> for TileStretch {
    #[inline]
    fn from(value: UVec2) -> Self {
        Self::new(value.x as u8, value.y as u8)
    }
}

impl From<TileStretch> for IVec2 {
    #[inline]
    fn from(value: TileStretch) -> Self {
        Self::new(i32::from(value.0), i32::from(value.1))
    }
}

impl From<TileStretch> for UVec2 {
    #[inline]
    fn from(value: TileStretch) -> Self {
        Self::new(u32::from(value.0), u32::from(value.1))
    }
}

impl From<TileStretch> for Vec2 {
    #[inline]
    fn from(value: TileStretch) -> Self {
        Self::new(f32::from(value.0), f32::from(value.1))
    }
}

/// An error in conversion from bevy types
///
/// Can only originate from [`TileStretch::get_tile`], and is bound to the lifetime of the two
/// arguments of that function.
#[derive(Error, Debug, Clone, Copy)]
#[error("Coordinates {to_translate} not divisible by stretch {:?}",tile_stretch.0)]
pub struct GetTileError {
    to_translate: Vec3,
    tile_stretch: TileStretch,
}

impl GetTileError {
    #[inline]
    fn new(to_translate: Vec3, tile_stretch: TileStretch) -> Self {
        Self {
            to_translate,
            tile_stretch,
        }
    }

    /// Translates the original translation to its closest grid tile.
    ///
    /// This is useful for error recovery: for example; moving an entity to the closest tile
    /// location, or simply ignoring that it's off-grid and continuing as normal.
    #[must_use]
    #[inline]
    pub fn to_closest(&self) -> IVec3 {
        self.tile_stretch.get_closest(self.to_translate)
    }
}

impl TileStretch {
    /// returns closest tile from a bevy translation
    #[must_use]
    #[inline]
    pub fn get_closest(self, t: Vec3) -> IVec3 {
        IVec3::new(
            t.x as i32 / i32::from(self.0),
            t.y as i32 / i32::from(self.1),
            t.z as i32,
        )
    }

    /// Fallible translation from bevy-space to tilespace.
    ///
    ///  It will return an error if the provided translation does not lie on grid. For graceful
    ///  recovery, you will probably want to call [`GetTileError::to_closest`]
    ///
    /// # Errors
    /// This function fails if `t` is not on-grid. If you don't care about t being on grid, use
    /// [`get_closest`]
    #[inline]
    pub fn get_tile(self, t: Vec3) -> Result<IVec3, GetTileError> {
        if t.round() != t
            || t.x as i32 % i32::from(self.0) != 0
            || t.y as i32 % i32::from(self.1) != 0
        {
            Err(GetTileError::new(t, self))
        } else {
            Ok(self.get_closest(t))
        }
    }

    /// Take a tile translation and translate it bevy space. This is infallible, as all tile space
    /// should translate into bevy-space, ignoring floating point errors which we are not concerned with.
    ///
    /// # Panics
    /// Panics if f32 conversion to i32 might fail. This shouldn't happen to any location
    /// originally converted from bevy worldspace.
    #[must_use]
    #[inline]
    pub fn get_bevy(self, t: IVec3) -> Vec3 {
        //
        assert!(
            t.x < (1 << f32::MANTISSA_DIGITS),
            "Trying to translate with precision loss on x"
        );
        assert!(
            t.y < (1 << f32::MANTISSA_DIGITS),
            "Trying to translate with precision loss on y"
        );
        assert!(
            t.z < (1 << f32::MANTISSA_DIGITS),
            "Trying to translate with precision loss on z"
        );

        #[allow(clippy::cast_precision_loss)]
        // TODO: do like unity and check for if it's above 1 << 23
        Vec3::new(
            t.x as f32 * f32::from(self.0),
            t.y as f32 * f32::from(self.1),
            t.z as f32,
        )
    }

    #[must_use]
    #[inline]
    pub fn new(x: u8, y: u8) -> Self {
        Self(x, y)
    }
}

pub fn register_types(type_registry: Res<AppTypeRegistry>) {
    let mut type_registry_w = type_registry.write();

    type_registry_w.add_registration(TileStretch::get_type_registration());
}

/// A trait for getting a tile location from a struct.
pub trait GetTileLocation {
    fn location(&self, tile_stretch: TileStretch) -> IVec3;
}

impl GetTileLocation for GlobalTransform {
    #[inline]
    fn location(&self, tile_stretch: TileStretch) -> IVec3 {
        tile_stretch.get_closest(self.translation())
    }
}

impl GetTileLocation for &GlobalTransform {
    #[inline]
    fn location(&self, tile_stretch: TileStretch) -> IVec3 {
        tile_stretch.get_closest(self.translation())
    }
}

impl GetTileLocation for &Vec3 {
    #[inline]
    fn location(&self, tile_stretch: TileStretch) -> IVec3 {
        tile_stretch.get_closest(**self)
    }
}

impl GetTileLocation for Vec3 {
    #[inline]
    fn location(&self, tile_stretch: TileStretch) -> IVec3 {
        tile_stretch.get_closest(*self)
    }
}

// hacky. assume IVec3 is already in tile space
impl GetTileLocation for &IVec3 {
    #[inline]
    fn location(&self, _: TileStretch) -> IVec3 {
        **self
    }
}
// hacky. assume IVec3 is already in tile space
impl GetTileLocation for IVec3 {
    #[inline]
    fn location(&self, _: TileStretch) -> IVec3 {
        *self
    }
}

impl std::ops::Mul<Vec3> for TileStretch {
    type Output = Vec3;

    #[inline]
    fn mul(self, rhs: Vec3) -> Self::Output {
        Vec3::new(rhs.x * self.0 as f32, rhs.y * self.1 as f32, rhs.z)
    }
}
