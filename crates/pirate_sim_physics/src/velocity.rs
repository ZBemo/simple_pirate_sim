//! Velocity calculations

use bevy::prelude::*;

use crate::tile_cast;

use pirate_sim_core::system_sets::PhysicsSet;

/// The Velocity that an entity moves at individually. For example, if an entities parent has a
/// TotalVelocity of (1,0,0) and the entity has a RelativeVelocity of (0,1,0) it will move (1,1,0)
/// grids per second in total
///
/// RelativeVelocity is multiplied by delta_time before being applied, & acts on the tile_grid. eg
/// a TotalVelocity of (1,1,0) should move up one grid and one grid to the right each second.
///
/// If you want an object to "have" velocity, but only move with its parent, give it a Velocity
/// Bundle but no ticker
#[derive(Debug, Component, Clone, Default, Deref, DerefMut, Reflect)]
pub(super) struct RelativeVelocity(pub Vec3);

/// RelativeVelocity + parent's TotalVelocity
///
/// TotalVelocity will = RelativeVelocity when an entity has no parents
///
/// RelativeVelocity is multiplied by delta_time before being applied, & acts on the tile_grid. eg
/// a TotalVelocity of (1,1,0) should move up one grid and one grid to the right each second.
///
/// This is currently only guaranteed to be accurate between [`PhysicsSet::Velocity`] and
/// [`PhysicsSet::Collision`]
#[derive(Debug, Component, Clone, Default, Deref, DerefMut, Reflect)]
pub(super) struct TotalVelocity(pub Vec3);

/// A maintained velocity over time. Will be decayed based on certain constants by the physics
/// engine
#[derive(Debug, Clone, Component, Default, Deref, DerefMut, Reflect)]
pub struct MantainedVelocity(pub Vec3);

#[derive(Debug, Clone, Component, Default)]
pub struct VelocityFromGround;

fn zero_total_vel(mut total_vel_q: Query<&mut TotalVelocity>) {
    total_vel_q.iter_mut().for_each(|mut t| {
        *t = TotalVelocity(Vec3::ZERO);
    });
}

// this uses an oddly high amount of time even when no entities have VelocityFromGround
fn propagate_from_ground(
    entity_q: Query<Entity, With<VelocityFromGround>>,
    global_transform_q: Query<(Entity, &GlobalTransform)>,
    mut total_vel_q: Query<&mut TotalVelocity>,
    mut relative_vel_q: Query<&mut RelativeVelocity>,
    tile_stretch: Res<pirate_sim_core::tile_grid::TileStretch>,
) {
    for e in entity_q.iter() {
        let translation = global_transform_q
            .get(e)
            .expect("Velocity From Ground tagged with no transformBundle")
            .1
            .translation();

        let below = tile_cast::tile_cast(
            tile_stretch.get_closest(translation),
            Vec3::NEG_Z,
            *tile_stretch,
            global_transform_q.iter(),
            false,
        );

        for (fe, l) in below {
            if (l.offset.z as f32 - translation.z).abs() < 1. + f32::EPSILON {
                let floor_total_v = total_vel_q.get(fe).map_or_else(|_| Vec3::ZERO, |t| t.0);

                trace!("Adding total v {floor_total_v} from floor to entity above it");

                let mut e_total_v = total_vel_q
                    .get_mut(e)
                    .expect("Tagged velocity from ground with no VelocityBundle");
                let mut e_rel_vel = relative_vel_q
                    .get_mut(e)
                    .expect("Tagged velocity from ground with no VelocityBundle");

                e_total_v.0 += floor_total_v;
                e_rel_vel.0 += floor_total_v;
            }
        }
    }
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
        Option<&MantainedVelocity>,
    )>,
) {
    for component in phsyics_components.iter_mut() {
        let mut new_relative_velocity = Vec3::splat(0.);

        let (mut relative_velocity, movement_goal, weight, mantained) = component;

        // it is up to the controller to ensure that the movement goal is reasonable
        if let Some(movement_goal) = movement_goal {
            new_relative_velocity += movement_goal.0;
        }

        // maybe gravity should be part of maintained velocity
        if let Some(weight) = weight {
            new_relative_velocity.z -= weight.0 * super::GRAVITY;
        }

        if let Some(mantained) = mantained {
            new_relative_velocity += mantained.0;
        }

        relative_velocity.0 = new_relative_velocity;
    }
}

/// This function decays any persistent velocities.
///
/// It needs a rework, and is currently not used
fn decay_persistent_velocity(mut velocity: Query<&mut MantainedVelocity>) {
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
/// TODO: do differential updates instead of zeroing every frame?
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
pub struct VelocityBundle {
    total: RelativeVelocity,
    relative_total: TotalVelocity,
}

pub struct Plugin;

impl bevy::prelude::Plugin for Plugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            (
                (zero_total_vel, calculate_relative_velocity),
                propagate_velocities,
                propagate_from_ground,
            )
                .chain()
                .in_set(PhysicsSet::Velocity),
        );
    }
}