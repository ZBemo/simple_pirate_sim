//! Velocity calculations

use bevy_app::{App, PostUpdate, Update};
use bevy_core::Name;
use bevy_derive::{Deref, DerefMut};
use bevy_ecs::prelude::*;
use bevy_hierarchy::{Children, Parent};
use bevy_log::trace;
use bevy_math::prelude::*;
use bevy_reflect::prelude::*;
use bevy_transform::prelude::*;

use crate::{tile_cast, Collider};

use pirate_sim_core::{
    system_sets::PhysicsSet,
    utils::{self, get_or_zero},
};

/// The Velocity that an entity moves at individually. For example, if an entities parent has a
/// [TotalVelocity] of (1,0,0) and the entity has a [RelativeVelocity] of (0,1,0) it will move (1,1,0)
/// grids per second in total
///
/// [RelativeVelocity] is multiplied by delta time before being applied, & acts on the tile grid. eg
/// a [TotalVelocity] of (1,1,0) should move up one grid and one grid to the right each second.
///
/// This might not be true due to implementation error/timescale weirdness
///
/// If you want an object to "have" velocity, but only move with its parent, give it a Velocity
/// Bundle but no ticker
#[derive(Debug, Component, Clone, Default, Deref, DerefMut, Reflect)]
pub(super) struct RelativeVelocity(pub Vec3);

/// [RelativeVelocity] + parent's [TotalVelocity]
///
/// [TotalVelocity] will = [RelativeVelocity] when an entity has no parents
///
/// [RelativeVelocity] is multiplied by delta time before being applied, & acts on the tile grid. eg
/// a [TotalVelocity] of (1,1,0) should move up one grid and one grid to the right each second.
///
/// This is currently only guaranteed to be accurate between [`PhysicsSet::Velocity`] and
/// [`PhysicsSet::Collision`]
#[derive(Debug, Component, Clone, Default, Deref, DerefMut, Reflect)]
pub(super) struct TotalVelocity(pub Vec3);

#[derive(Debug, Component, Clone, Default, Deref, Reflect)]
pub struct LastRelative(Vec3);
#[derive(Debug, Component, Clone, Default, Deref, Reflect)]
pub struct LastTotal(Vec3);

impl From<Vec3> for LastRelative {
    fn from(value: Vec3) -> Self {
        Self(value)
    }
}

impl From<Vec3> for LastTotal {
    fn from(value: Vec3) -> Self {
        Self(value)
    }
}
/// A maintained velocity over time. Will be decayed based on certain constants by the physics
/// engine
#[derive(Debug, Clone, Component, Default, Deref, DerefMut, Reflect)]
pub struct Mantained(pub Vec3);

#[derive(Clone, Component, Default, Reflect)]
pub struct FromGround(Vec3);

fn zero_total_vel(mut total_vel_q: Query<&mut TotalVelocity>) {
    total_vel_q.iter_mut().for_each(|mut t| {
        *t = TotalVelocity(Vec3::ZERO);
    });
}

/// replace T with the value of F
///
/// This can easily be made more generic, but as it's only used for Vec3, there's no need
/// currently
///
/// a more generic version would look like this, but with more bounds on `F::Target`
/// ```
/// fn update_from_into<F, I>(mut from_query: Query<Ref<F>,&mut I>)
///     where
///     F: std::ops::Deref + Component,
///     I: From<<F as std::ops::Deref>::Target> + Component
///     {
///         // same body
///     }
/// ```
fn update_last<Current, Last>(mut from_query: Query<(Ref<Current>, &mut Last)>)
where
    Current: std::ops::Deref<Target = Vec3> + Component,
    Last: From<Vec3> + Component,
{
    for (current, mut last) in from_query.iter_mut() {
        if current.is_changed() {
            *last = (**current).into();
        }
    }
}

// TODO: This runs in PostUpdate; make sure that makes sense
fn propagate_from_ground(
    mut from_ground_q: Query<(Entity, &mut FromGround)>,
    collider_q: Query<&Collider>,
    total_vel_q: Query<&TotalVelocity>,
) {
    from_ground_q.par_iter_mut().for_each_mut(|(e, mut f)| {
        let c = collider_q.get(e).expect("From ground with no collider");

        *f = FromGround(Vec3::ZERO);

        let Some(collision) = c.collision() else {return };

        let total_from_ground = collision
            .other_entities
            .iter()
            .filter(|&&oe| {
                let oc = collider_q
                    .get(oe.data)
                    .expect("Entity in collision should have collider");

                (oe.offset.z == -1 && oc.constraints.pos_solid_planes.z)
                    || (oe.offset.z == 0 && oc.constraints.neg_solid_planes.z)
            })
            .fold(Vec3::ZERO, |acc, hit| {
                acc + get_or_zero(&total_vel_q, hit.data)
            });

        // add total_from_ground

        *f = FromGround(total_from_ground);

        // .iter_mut()
        // .filter_map(|(e, c, f)| c.collision().map(|c| (e, c, f)))
        // .for_each(|(e, c, f)| {
        //     // check if any of them are below / on same tile with right z masks
        //     todo!()
        // });
    });
}

/// Takes all factors that could affect a physics component's velocity on each frame and then
/// calculates a "total velocity" as a function of all of these factors
///
/// This does not move any components, nor update their ticker
///
/// This should wait until movement finalization to multiply by delta time.
fn calculate_relative_velocity(
    mut phsyics_components: Query<(
        &mut RelativeVelocity,
        Option<&super::MovementGoal>,
        Option<&super::Weight>,
        Option<&Mantained>,
        Option<&FromGround>,
    )>,
) {
    for component in phsyics_components.iter_mut() {
        let mut new_relative_velocity = Vec3::splat(0.);

        let (mut relative_velocity, movement_goal, weight, mantained, from_ground) = component;

        // it is up to the controller to ensure that the movement goal is reasonable
        if let Some(movement_goal) = movement_goal {
            new_relative_velocity += movement_goal.0;
        }

        // maybe gravity should be part of maintained velocity
        if let Some(weight) = weight {
            if **weight != 0. {
                new_relative_velocity.z -= super::GRAVITY;
            }
        }

        if let Some(mantained) = mantained {
            new_relative_velocity += mantained.0;
        }

        if let Some(from_ground) = from_ground {
            new_relative_velocity += from_ground.0;
        }

        relative_velocity.0 = new_relative_velocity;
    }
}

/// This function decays any persistent velocities.
///
/// It needs a rework, and is currently not used
fn decay_persistent_velocity(mut velocity: Query<&mut Mantained>) {
    const DECAY_CONST: f32 = 0.1;
    // TODO: use signs

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

/// Propagate velocities down from an entities parents so that its Total and Relative Velocity remains accurate
///
/// needs parent total and child relative along with child total
///
/// This is lifted from the bevy source code, which is dual-licensed under the Apache 2.0, and MIT
/// license. See <https://github.com/bevyengine/bevy/LICENSE-APACHE> or <./../credits/> for more details.
/// or <https://github.com/bevyengine/bevy/LICENSE-MIT>
///
/// TODO: Reintroduce change detection checking
fn propagate_velocities(
    mut root_query: Query<
        (
            Entity,
            Option<&Children>,
            Ref<RelativeVelocity>,
            &mut TotalVelocity,
        ),
        Without<Parent>,
    >,
    velocity_query: Query<
        (Ref<RelativeVelocity>, &mut TotalVelocity, Option<&Children>),
        With<Parent>,
    >,
    parent_query: Query<(Entity, Ref<Parent>)>,
    name_query: Query<&Name>,
) {
    trace!("starting velocity propagataion");

    root_query
        .par_iter_mut()
        .for_each_mut(|(entity, children, relative, mut total)| {
            trace!(
                "propogating root {}",
                name_query
                    .get(entity)
                    .map_or_else(|_| "UnnamedEntity".into(), ToString::to_string)
            );

            total.0 += relative.0;

            let Some(children) = children else {return};

            for (child, actual_parent) in parent_query.iter_many(children) {
                assert_eq!(actual_parent.get(), entity, "Bad hierarchy");
                unsafe {
                    propagate_recursive(&total, &velocity_query, &parent_query, child);
                }
            }
        });
}

/// This is lifted from the bevy source code, which is dual-licensed under the Apache 2.0, and MIT
/// license. see <https://github.com/bevyengine/bevy/LICENSE-APACHE> or <./../credits/> for more details.
unsafe fn propagate_recursive(
    parent_total: &TotalVelocity,
    velocity_query: &Query<
        (Ref<RelativeVelocity>, &mut TotalVelocity, Option<&Children>),
        With<Parent>,
    >,
    parent_query: &Query<(Entity, Ref<Parent>)>,
    entity: Entity,
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

        total.0 += parent_total.0 + relative.0;
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
                // not messing with whatever bevy does
                global_matrix,
                velocity_query,
                parent_query,
                child,
            );
        }
    }
}

/// You probably want [`super::movement::MovementBundle`]
///
/// This bundle allows an entity to be acted on by all systems in the velocity module/plugin
#[derive(Bundle, Debug, Default)]
pub(crate) struct VelocityBundle {
    total: RelativeVelocity,
    relative_total: TotalVelocity,
    last_total: LastTotal,
    last_relative: LastRelative,
}

pub struct Plugin;

impl bevy_app::Plugin for Plugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, zero_total_vel.before(calculate_relative_velocity))
            .add_systems(
                Update,
                (calculate_relative_velocity, propagate_velocities)
                    .chain()
                    .in_set(PhysicsSet::Velocity),
            )
            .add_systems(
                PostUpdate,
                (
                    propagate_from_ground,
                    update_last::<TotalVelocity, LastTotal>,
                    update_last::<RelativeVelocity, LastRelative>,
                ),
            );
        // don't put in
        // Velocity as it can actually run during input
    }
}
