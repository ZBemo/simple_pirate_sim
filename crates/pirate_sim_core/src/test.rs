//! Tests for core and a testing plugin for bevy

use bevy::prelude::*;

use crate::tile_grid::TileStretch;

/// A plugin that sets up things that nearly every system expects to exist, for quick test setup
pub struct DefaultTestPlugin;

impl Plugin for DefaultTestPlugin {
    fn build(&self, app: &mut App) {
        // 1<->1 conversion for simplicity
        app.insert_resource(TileStretch::new(1, 1));

        // system to log location of every named entity when it moves
        app.add_systems(
            Last,
            |q: Query<
                (&GlobalTransform, &Name),
                Or<(Changed<GlobalTransform>, Added<GlobalTransform>)>,
            >| {
                q.iter()
                    .for_each(|e| debug!("`{}` moved to {}", e.1.to_string(), e.0.translation()));
            },
        );

        // almost every system assumes these plugins are present
        app.add_plugins(bevy::log::LogPlugin::default())
            .add_plugins(bevy::time::TimePlugin)
            .add_plugins(bevy::transform::TransformPlugin);
    }
}

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
