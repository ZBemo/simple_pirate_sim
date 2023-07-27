//! Tests for core and a testing plugin for bevy

use bevy_app::prelude::*;
use bevy_core::Name;
use bevy_ecs::prelude::*;
use bevy_log::prelude::*;
use bevy_math::{IVec3, Vec3};
use bevy_transform::prelude::*;

use crate::tile_grid::TileStretch;

#[test]
fn tile_stretch_round_trip() {
    let start = Vec3::new(32., 64., 3.);
    let tile_stretch = TileStretch(32, 32);

    let cast_to_grid = tile_stretch
        .get_tile(start)
        .expect("we know this is divisible");

    assert_eq!(cast_to_grid, IVec3::new(1, 2, 3));

    let cast_to_bevy = tile_stretch.get_bevy(cast_to_grid);

    assert_eq!(start, cast_to_bevy);
}

#[test]
fn tile_stretch_fail_off_grid() {
    let start = Vec3::new(33., 64., 3.);
    let tile_stretch = TileStretch(32, 32);

    let cast_to_grid = tile_stretch.get_tile(start);

    assert!(cast_to_grid.is_err());

    let closest = cast_to_grid
        .expect_err("just asserted that self.is_err()")
        .to_closest();

    assert_eq!(closest, IVec3::new(1, 2, 3));
    assert_eq!(tile_stretch.get_closest(start), closest);
}
