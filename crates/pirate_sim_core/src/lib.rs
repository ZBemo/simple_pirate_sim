//! Core functionality for the tile-based engine
//!
//! This includes things like randomness, bevy, etc
//!
//! Almost every other crate will depend on this crate.

pub use bevy;
#[cfg(feature = "developer-tools")]
pub use bevy_inspector_egui;
#[cfg(feature = "developer-tools")]
pub use bevy_inspector_egui::bevy_egui;

pub use thiserror;

pub mod goals;
pub mod random;
pub mod system_sets;
pub mod test;
pub mod tile_grid;

pub use system_sets::PhysicsSet;

/// set up core resources, systems, system_sets, etc
pub struct CorePlugin;

impl bevy::prelude::Plugin for CorePlugin {
    fn build(&self, app: &mut bevy::prelude::App) {
        use bevy::prelude::*;
        {
            use self::system_sets::PhysicsSet::*;
            app.configure_sets(
                Update,
                (Input, Velocity, Collision, Movement, Completed).chain(),
            );
        }
    }
}
