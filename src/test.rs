use bevy::prelude::*;

use crate::tile_grid::TileStretch;

/// A plugin that sets up things that nearly every system expects to exist, for quick test setup
pub struct DefaultTestPlugin;

impl Plugin for DefaultTestPlugin {
    fn build(&self, app: &mut App) {
        // 1<->1 conversion for simplicity
        app.insert_resource(TileStretch::new(IVec2::new(1, 1)));

        // system to log location of every named entity
        app.add_system(|q: Query<(&GlobalTransform, &Name)>| {
            q.iter()
                .for_each(|e| debug!("{}: {}", e.1.to_string(), e.0.translation()))
        });

        // almost every system assumes these plugins are present
        app.add_plugin(bevy::log::LogPlugin::default())
            .add_plugin(bevy::time::TimePlugin)
            .add_plugin(bevy::transform::TransformPlugin);
    }
}