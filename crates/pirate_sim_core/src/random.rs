//! simple wrappers around `bracket_random`
//
//! this probably shouldn't be a single file, but it doesn't particularly fit anywhere else yet

use bevy_derive::{Deref, DerefMut};
use bevy_ecs::prelude::*;
use std::time::UNIX_EPOCH;

use bracket_random::prelude::*;

/// A Seed for random number generation.
#[derive(Debug, Resource, Deref)]
pub struct Seed(u64);

/// A random number generator, but as a resource
#[derive(Resource, Deref, DerefMut)]
pub struct Generator(RandomNumberGenerator);

pub fn setup_generator(mut commands: Commands) {
    let seed = std::time::SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("unable to get time since unix_epoch.. make sure you have an OS?")
        .as_secs();

    commands.insert_resource(Seed(seed));
    commands.insert_resource(Generator(RandomNumberGenerator::seeded(seed)));
}
