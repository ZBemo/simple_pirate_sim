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
pub mod utils;

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
