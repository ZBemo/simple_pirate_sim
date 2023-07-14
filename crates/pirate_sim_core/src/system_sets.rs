//! System sets use externally between crates

use bevy::prelude::SystemSet;

#[derive(SystemSet, Hash, Debug, Clone, Eq, PartialEq)]
/// We recommend running any system that plans to input into the Physics system before
/// [`PhysicsSet::Velocity`], or it may not be considered at all or until the next frame.
///
/// If wanting to use previously newly update locations, run after [`PhysicsSet::Movement`]
///
/// systems making use of collision checking should run after [`PhysicsSet::Collision`], or
/// collision data may be wildly inaccurate
pub enum PhysicsSet {
    Input,
    Velocity,
    Collision,
    Movement,
    Completed,
}
