//! A tile-based Physics Engine for this project
//!
//! See [`PhysicsPlugin`], and its build function to get started with the source code, or you can
//! likely read the file from top-down and understand it decently well.

use std::{assert_eq, todo};

use bevy::prelude::*;

use crate::{controllers, tile_objects::TileStretch};

/// a tile collider, specified in tile space. use a z of 0 to have it at the bottom of its x,y tile
#[derive(Component, Debug, Deref, DerefMut)]
pub struct Collider(pub IVec3);

#[derive(SystemSet, Hash, Debug, Clone, Eq, PartialEq)]
/// physics system sets
/// register all velocity wants for the current frame before FinalMovement
/// if wanting to use previously updated locations, run after FinalMovement
///
/// Currently, only FinalizeMovement is used by the Physics engine, and setting anything relative
/// to FinalizeVelocity or CollisionCheck will break things.
pub enum PhysicsSet {
    FinalizeVelocity,
    CollisionCheck,
    FinalizeMovement,
}

/// Velocity for current frame relative to its parents velocity
///
/// this is rarely ever acurate except during FinalMovement set, and potentially afterwards.
///
/// If you want an object to "have" velocity, but only move with its parent, give it a Velocity
/// Bundle but no ticker
///
///  Relative Velocity should likely not be chnaged outside of the physics engine
#[derive(Debug, Component, Clone, Default, Deref)]
pub struct RelativeVelocity(Vec3);

/// RelativeVelocity + parent's TotalVelocity
///
/// TotalVelocity will = RelativeVelocity when an entity has no parents
///
/// All of an entity's parents must have a Velocity bundle in order for the entity to have one
#[derive(Debug, Component, Clone, Default, Deref)]
pub struct TotalVelocity(Vec3);

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
/// buffering velocity into its ticker until at least a whole tile has been moved.
/// This makes it so that speeds of less than 1 tile per second can be used without a complex
/// movement system or something, and in real time, as it will essentially buffer all movements
/// until they're enough to move at least a full tile.
///
/// Currently if a component has 0 velocity, its ticker will be reset to 0,0,0. In the future this
/// might be changed so that you can reset your ticker trough a request like RequestResetTicker.
///
/// As this Ticker is meant to be wholely managed by the physics engine, it is not public, and must
/// be instantiated trough a Bundle like [`PhysicsComponentBase`]
#[derive(Debug, Component, Clone, Copy, Default, Deref, DerefMut)]
struct VelocityTicker(Vec3);

/// "Raycast out" based on current velocity ticker and then "trim" the ticker down if an object is
/// likely to be collided with
///
/// in the future consider having collision events?
///
/// entities will be at something like TotalVel - RelVol + Ticker
fn perform_collision_checks_and_cancelling() {
    todo!()
}

/// Takes all factors that could affect a physics component's velocity on each frame and then
/// calculates a "total velocity" as a function of all of these factors
///
/// This does not move any components, nor update their ticker
fn calculate_relative_velocity(
    mut commands: Commands,
    mut phsyics_components: Query<(
        &mut RelativeVelocity,
        Option<&MovementGoal>,
        Option<&Weight>,
        Option<&MantainedVelocity>,
        Option<&Collider>,
    )>,
    time: Res<Time>,
) {
    let delta_time = time.delta().as_secs_f32();

    for component in phsyics_components.iter_mut() {
        let mut new_total_velocity = Vec3::splat(0.);

        let (mut total_velocity, movement_goal, weight, mantained, collider) = component;

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

        total_velocity.0 = new_total_velocity;
    }
}

/// Finaly, applies any tickers that have moved at least one tile. This is essentially flushing the
/// VelocityTicker buffer.
///
/// This will reset any tickers with a totalVelocity of 0 to 0,0,0. This may lead to bugs in the
/// future
fn apply_total_movement(
    mut phsyics_components: Query<(
        &mut Transform,
        &mut VelocityTicker,
        &RelativeVelocity,
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

/// Propogate velocities down from an entities parents so that its Total and Relative Velocity remains accurate
///
/// needs parent total and child relative along with child total
///
/// This is lifted from the bevy source code, which is dual-licensed under the Apache 2.0, and MIT
/// license. see (https://github.com/bevyengine/bevy/LICENSE-APACHE) for more details.
///
/// Also see (./credits/bevy)
fn propogate_velocities(
    mut root_query: Query<
        (Entity, &Children, Ref<RelativeVelocity>, &mut TotalVelocity),
        Without<Parent>,
    >,
    velocity_query: Query<
        (Ref<RelativeVelocity>, &mut TotalVelocity, Option<&Children>),
        With<Parent>,
    >,
    parent_query: Query<(Entity, Ref<Parent>)>,
) {
    trace!("starting velocity propagataion");

    // TODO: par iter
    root_query
        .par_iter_mut()
        .for_each_mut(|(entity, children, relative, mut total)| {
            let changed = relative.is_changed();
            if changed {
                total.0 = relative.0;
            }

            for (child, actual_parent) in parent_query.iter_many(children) {
                assert_eq!(actual_parent.get(), entity, "Bad hierarchy");
                propagate_recursive(
                    &total,
                    &velocity_query,
                    &parent_query,
                    child,
                    changed || actual_parent.is_changed(),
                );
            }
        });
}

/// This is lifted from the bevy source code, which is dual-licensed under the Apache 2.0, and MIT
/// license. see (https://github.com/bevyengine/bevy/LICENSE-APACHE) for more details.
///
/// Also see (./credits/bevy)
fn propagate_recursive(
    parent_total: &TotalVelocity,
    velocity_query: &Query<
        (Ref<RelativeVelocity>, &mut TotalVelocity, Option<&Children>),
        With<Parent>,
    >,
    parent_query: &Query<(Entity, Ref<Parent>)>,
    entity: Entity,
    mut changed: bool,
) {
    let (global_matrix, children) = {
        let Ok((relative, mut total, children)) =
            // SAFETY: This call cannot create aliased mutable references.
            //   - The top level iteration parallelizes on the roots of the hierarchy.
            //   - The caller ensures that each child has one and only one unique parent throughout the entire
            //     hierarchy.
            //
            // For example, consider the following malformed hierarchy:
            //
            //     A
            //   /   \
            //  B     C
            //   \   /
            //     D
            //
            // D has two parents, B and C. If the propagation passes through C, but the Parent component on D points to B,
            // the above check will panic as the origin parent does match the recorded parent.
            //
            // Also consider the following case, where A and B are roots:
            //
            //  A       B
            //   \     /
            //    C   D
            //     \ /
            //      E
            //
            // Even if these A and B start two separate tasks running in parallel, one of them will panic before attempting
            // to mutably access E.
            (unsafe { velocity_query.get_unchecked(entity) }) else {
                return;
            };

        changed |= relative.is_changed();
        if changed {
            total.0 = parent_total.0 + relative.0;
        }
        (parent_total, children)
    };

    let Some(children) = children else { return };
    for (child, actual_parent) in parent_query.iter_many(children) {
        assert_eq!(
            actual_parent.get(), entity,
            "Malformed hierarchy. This probably means that your hierarchy has been improperly maintained, or contains a cycle"
        );
        // SAFETY: The caller guarantees that `transform_query` will not be fetched
        // for any descendants of `entity`, so it is safe to call `propagate_recursive` for each child.
        //
        // The above assertion ensures that each child has one and only one unique parent throughout the
        // entire hierarchy.
        unsafe {
            propagate_recursive(
                &global_matrix,
                velocity_query,
                parent_query,
                child,
                changed || actual_parent.is_changed(),
            );
        }
    }
}

/// The components necessary for movement by the physics engine to take place on an entity's
/// transform.
///
/// You must provide a transform yourself in order to get movement, in order to stay compatible
/// with other bundles.
///
/// TODO: consider clearing ticker even if not attached to a Transform
#[derive(Bundle, Debug, Default)]
pub struct PhysicsComponentBase {
    ticker: VelocityTicker,
    total_velocity: VelocityBundle,
}

#[derive(Debug, Bundle)]
pub struct PhysicsComponentFull {
    pub base: PhysicsComponentBase,
    pub weight: Weight,
    pub mantained_velocity: MantainedVelocity,
    pub collider: Collider,
}

/// Allows an entity to exist in the physics system with a velocity
/// Total velocity is the total velocity that the object should be moved, while relative velocity
/// is how it is moving relative to its parent in the object hierarchy
#[derive(Bundle, Debug, Default)]
pub struct VelocityBundle {
    total: RelativeVelocity,
    relative_total: TotalVelocity,
}

/// A plugin to setup essential physics systems
///
/// Any system that wants to use the results of a physics engine update should not run until after
/// [`PhysicsSet::FinalMovement`] has been completed
///
/// Any systems that want to affect the physics engine in a given frame must run before
/// [`PhysicsSet::FinalizeVelocity].
///
/// See [`PhysicsPlugin::build`] for the flow of the physics update
pub struct PhysicsPlugin;

impl Plugin for PhysicsPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            (
                calculate_relative_velocity,
                propogate_velocities.after(calculate_relative_velocity),
                decay_persistent_velocity.after(calculate_relative_velocity),
            )
                .in_set(PhysicsSet::FinalizeVelocity),
        )
        // collision here
        .add_system(
            // other movement stuff here
            apply_total_movement
                .in_set(PhysicsSet::FinalizeMovement)
                .after(PhysicsSet::FinalizeVelocity),
        );
    }
}
