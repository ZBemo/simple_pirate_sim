//! simple wrappers around bracket_random
//!
//! this probably shouldn't be a single file, but it doesn't particularly fit anywhere else yet

use std::time::UNIX_EPOCH;

use bevy::prelude::{Commands, Deref, DerefMut, Resource};
use bracket_random::prelude::*;

/// A Seed for random number generation.
#[derive(Debug, Resource, Deref)]
pub struct Seed(u64);

/// A random number generator, but as a resource
#[derive(Resource, Deref, DerefMut)]
pub struct RandomGenerator(RandomNumberGenerator);

pub fn setup_generator(mut commands: Commands) {
    let seed = std::time::SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs_f64()
        * std::time::SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .subsec_nanos() as f64;
    let seed = seed as u64;

    commands.insert_resource(Seed(seed));
    commands.insert_resource(RandomGenerator(RandomNumberGenerator::seeded(seed)));
}
