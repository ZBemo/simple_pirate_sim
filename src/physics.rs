//! Physics updating and simulation for tile objects
//!
//!

use std::todo;

use bevy::prelude::*;

use crate::{controllers, tile_objects::TileStretch};

/// allows you to take on the velocity of another entity, useful for ships, etc
///
/// this should probably be re-architected in the future
#[derive(Component, Debug, Deref, DerefMut)]
pub struct LinkVelocity(pub Entity);

/// a tile collider
/// specified in tile_space
#[derive(Component, Debug, Deref, DerefMut)]
pub struct Collider(pub Vec3);

#[derive(SystemSet, Hash, Debug, Clone, Eq, PartialEq)]
/// physics system sets
/// register all velocity wants for the current frame before FinalMovement
/// if wanting to use previously updated locations, run after FinalMovement
pub enum PhysicsSet {
    FinalMovement,
}

/// a total velocity per frame, used for updating movement
/// this is rarely ever acurate except during FinalMovement set, and potentially afterwards
///
/// Is not currently public, to avoid the impression that it is accurate outside of physics
/// systems.
#[derive(Debug, Component, Clone, Default, Deref, DerefMut)]
struct TotalVelocity(pub Vec3);

/// Any component with a weight will have gravity applied to it on each physics update
#[derive(Debug, Clone, Copy, Component, Deref, DerefMut)]
pub struct Weight(pub f32);

/// A mantained velocity over time. Will be decayed based on certain constants by the physics
/// engine
#[derive(Debug, Clone, Component, Default, Deref, DerefMut)]
pub struct MantainedVelocity(pub Vec3);

/// a tile velocity that is wiped after every update, for willfully moving characters, usually
/// trough controllers
///
/// As valid movement is different for each entity, The physics engine does not check for "invalid" movement goals,
/// and it is the responsibility of  whoever is controlling an entity to make sure movement goals are valid.
#[derive(Debug, Component, Clone, Default, Deref, DerefMut)]
pub struct MovementGoal(pub Vec3);

/// A Velocity Ticker, used to keep track of when to actually move a physics component, by
/// buffering velocity into its ticker until at least a whole tile has been moved
///
/// Currently if a component has 0 velocity, its ticker will be reset to 0,0,0
#[derive(Debug, Component, Clone, Copy, Default, Deref, DerefMut)]
struct VelocityTicker(Vec3);

/// Takes all factors that could affect a physics component's velocity on each frame and then
/// calculates a "total velocity" as a function of all of these factors
///
/// This does not move any components, nor update their ticker
fn calculate_total_velocity(
    mut commands: Commands,
    mut phsyics_components: Query<(
        &mut TotalVelocity,
        Option<&MovementGoal>,
        Option<&Weight>,
        Option<&MantainedVelocity>,
        Option<&Collider>,
        Option<&LinkVelocity>,
    )>,
    time: Res<Time>,
) {
    let delta_time = time.delta().as_secs_f32();

    let mut to_calc_linked = Vec::new();

    for component in phsyics_components.iter_mut() {
        let mut new_total_velocity = Vec3::splat(0.);

        let (mut total_velocity, movement_goal, weight, mantained, collider, link) = component;

        // it is up to the controller to ensure that the movement goal is reasonable
        if let Some(movement_goal) = movement_goal {
            new_total_velocity += movement_goal.0 * delta_time;
        }

        // maybe gravity should be part of mantained velocity
        if let Some(weight) = weight {
            new_total_velocity.z -= weight.0 * 4.9 * delta_time;
        }

        if let Some(mantained) = mantained {
            new_total_velocity += mantained.0;
        }

        if let Some(collider) = collider {
            todo!("Collision not yet implemented")
        }

        if let Some(linked) = link {
            to_calc_linked.push((linked.0, commands.get_entity(linked.0).unwrap().id()));
        }

        total_velocity.0 = new_total_velocity;
    }

    // post pass any linked velocities to ensure they use updated velocities
    // doesn't support "nested" linked velocities, which could be a problem later
    // might need to recurse through and force linked velocities to update based on dependencie
    // order
    for (linked, linked_to) in to_calc_linked.iter() {
        // there's got to be a way to not have to clone this..
        // cheap clone I guess
        let linked_to_vel = phsyics_components
            .get_component::<TotalVelocity>(*linked_to)
            .unwrap()
            .clone();

        let mut linked_vel = phsyics_components
            .get_component_mut::<TotalVelocity>(*linked)
            .unwrap();

        linked_vel.0 += linked_to_vel.0;
    }
}

/// Finaly, applies any tickers that have moved at least one tile. This is essentially flushing the
/// VelocityTicker buffer.
///
/// This will reset any tickers with a totalVelocity of 0 to 0,0,0. This may lead to bugs in the
/// future
fn apply_total_velocity(
    mut phsyics_components: Query<(
        &mut Transform,
        &mut VelocityTicker,
        &TotalVelocity,
        Option<&controllers::player::Controller>,
    )>,
    tile_stretch: Res<TileStretch>,
) {
    // this will make it so entities only move a tile once an entire tiles worth of movement
    // has been "made", keeping it in a grid based system
    //
    // also converts to 32x32

    for (mut transform, mut ticker, total_velocity, player) in phsyics_components.iter_mut() {
        // update ticker
        ticker.0 += total_velocity.0;

        debug!("updating with ticker {}", ticker.0);

        let mut ticker_ticked = false;

        while ticker.0.z.abs() >= 1. {
            transform.translation.z += ticker.0.z.signum();
            ticker.0.z -= 1. * ticker.0.z.signum();
            ticker_ticked = true;
        }
        while ticker.0.y.abs() >= 1. {
            transform.translation.y += tile_stretch.1 as f32 * ticker.0.y.signum();
            ticker.0.y -= 1. * ticker.0.y.signum();
            ticker_ticked = true;
        }
        while ticker.0.x.abs() >= 1. {
            transform.translation.x += tile_stretch.0 as f32 * ticker.0.x.signum();
            ticker.0.x -= 1. * ticker.0.x.signum();
            ticker_ticked = true;
        }

        // this might break things in the future!
        // if total_veolicity is 0 reset ticker to 0
        // this probably does not belong in this system. maybe in its own system?
        if total_velocity.0 == Vec3::ZERO {
            ticker.0 = Vec3::ZERO;
        }
    }
}

/// This function decays any persistent velocities.
///
/// It needs a rework.
fn decay_persistent_velocity(mut velocity: Query<&mut MantainedVelocity>) {
    const DECAY_CONST: f32 = 0.1;
    // todo: use signs

    for mut vel in velocity.iter_mut() {
        if vel.0.z > 0. {
            vel.0.z -= DECAY_CONST;
            vel.0.z = vel.0.z.clamp(0., f32::INFINITY);
        } else if vel.0.z != 0. {
            vel.0.z += DECAY_CONST;
            vel.0.z = vel.0.z.clamp(f32::NEG_INFINITY, 0.);
        }
        if vel.0.y > 0. {
            vel.0.y -= DECAY_CONST;
            vel.0.y = vel.0.y.clamp(0., f32::INFINITY);
        } else if vel.0.y != 0. {
            vel.0.y += DECAY_CONST;
            vel.0.y = vel.0.y.clamp(f32::NEG_INFINITY, 0.);
        }
        if vel.0.y > 0. {
            vel.0.y -= DECAY_CONST;
            vel.0.y = vel.0.y.clamp(0., f32::INFINITY);
        } else if vel.0.y != 0. {
            vel.0.y += DECAY_CONST;
            vel.0.y = vel.0.y.clamp(f32::NEG_INFINITY, 0.);
        }
    }
}

/// The components necessary for internal physics engine systems to take affect on an entity.
#[derive(Bundle, Debug, Default)]
pub struct PhysicsComponentBase {
    ticker: VelocityTicker,
    total_velocity: TotalVelocity,
}

#[derive(Debug, Bundle)]
pub struct PhysicsComponentFull {
    pub base: PhysicsComponentBase,
    pub weight: Weight,
    pub mantained_velocity: MantainedVelocity,
    pub collider: Collider,
}

/// A plugin to setup essential physics systems
///
/// Any system that wants to use the results of a physics engine update should not run until after
/// [`PhysicsSet::FinalMovement`] has been completed
///
/// Any systems that want to affect the physics engine in a given frame must run before `FinalMovement`.
pub struct PhysicsPlugin;

impl Plugin for PhysicsPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            (
                calculate_total_velocity,
                apply_total_velocity.after(calculate_total_velocity),
                decay_persistent_velocity.after(calculate_total_velocity),
            )
                .in_set(PhysicsSet::FinalMovement),
        );
    }
}
