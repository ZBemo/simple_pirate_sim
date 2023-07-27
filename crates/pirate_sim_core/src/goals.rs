//! Goals for communicating between controllers and other engines
//!
//! For example, [`MovementGoal`] communicates to the physics system where a controller would like to
//! move

use bevy_derive::{Deref, DerefMut};
use bevy_ecs::prelude::Component;
use bevy_math::Vec3;
use bevy_reflect::Reflect;
/// A way to request movement for a specific entity. Expects the entity to have a [`velocity::VelocityBundle`]
///
/// Each axis on the inner Vec3 represents the entities requested speed in that direction, similar
/// to a force diagram.
///
/// As valid movement is different for each entity, The physics engine has no concept of movement
/// goal validity, as it assumes that whomever has checked the movementgoal has run necessary
/// validity checks.
#[derive(Debug, Component, Clone, Default, Deref, DerefMut, Reflect)]
pub struct MovementGoal(pub Vec3);
