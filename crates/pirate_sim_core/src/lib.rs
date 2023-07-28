//! Core functionality for the tile-based engine
//!
//! This includes things like randomness, bevy, etc
//!
//! Almost every other crate will depend on this crate.

#![warn(clippy::unwrap_used)]
#![warn(clippy::perf, clippy::disallowed_types)] // performance warns
#![warn(clippy::pedantic)]
// most bevy systems violate these. Nothing I can do about it at the moment.
#![allow(
    clippy::type_complexity,
    clippy::too_many_arguments,
    clippy::needless_pass_by_value // TODO: separate out system functions from non-system 
)]
#![allow(clippy::cast_possible_truncation)]

pub use thiserror;

pub mod goals;
pub mod random;
pub mod system_sets;

pub mod tile_grid;
pub mod utils;

pub use system_sets::PhysicsSet;

#[cfg(test)]
mod test;

/// set up core resources, systems, `system_sets`, etc
pub struct CorePlugin;

impl bevy_app::Plugin for CorePlugin {
    fn build(&self, app: &mut bevy_app::App) {
        #[allow(clippy::enum_glob_use)]
        use self::system_sets::PhysicsSet::*;
        use bevy_app::prelude::*;
        use bevy_ecs::schedule::IntoSystemSetConfigs;
        app.configure_sets(
            Update,
            (Input, Velocity, Collision, Movement, Completed).chain(),
        );
    }
}

/// A plugin that sets up things that nearly every system expects to exist, for quick test setup
pub mod test_utils {
    pub struct DefaultTestPlugin;

    use crate::tile_grid::TileStretch;
    use bevy_app::prelude::*;
    use bevy_core::Name;
    use bevy_ecs::prelude::*;
    use bevy_log::prelude::*;
    use bevy_transform::prelude::*;

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
                    q.iter().for_each(|e| {
                        debug!("`{}` moved to {}", e.1.to_string(), e.0.translation());
                    });
                },
            );

            // almost every system assumes these plugins are present
            app.add_plugins(bevy_log::LogPlugin::default())
                .add_plugins(bevy_time::TimePlugin)
                .add_plugins(bevy_transform::TransformPlugin);
        }
    }
}
