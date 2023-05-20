//! simple wrappers around bracket_random
//!
//! this probably shouldn't be a single file, but it doesn't particularly fit anywhere else yet

use bevy::prelude::Resource;
use bracket_random::prelude::*;

/// A Seed for random number generation.
#[derive(Debug, Resource)]
pub struct Seed(u64);

/// A random number generator, but as a resource
#[derive(Resource)]
pub struct RandomGenerator(RandomNumberGenerator);
