//! Colliders and Collision systems
//!
//! Functions and documents in this module will often refer to `collisions` and `conflicts`, which
//! are two different things. A conflict is when two or more colliders are going to move through a
//! collider in a way that conflicts on both colliders' axis-planes, or vice-versa. It is the
//! physics system's job to prevent any conflicts from actually happening. Usually through
//! cancelling the velocity of one or more object.
//!
//! Collisions however, are any time when two colliders will overlap, which does not always
//! necessitate interference from the physics systems
//!
//! This module is probably rife with opportunities for performance improvements.

use bevy_app::prelude::*;
use bevy_core::Name;
use bevy_derive::Deref;
use bevy_ecs::prelude::*;
use bevy_log::prelude::*;
use bevy_math::prelude::*;
use bevy_reflect::prelude::*;
use bevy_time::Time;
use bevy_transform::prelude::GlobalTransform;

use pirate_sim_core::{utils::bvec_to_mask, PhysicsSet};

use crate::tile_cast;

use super::{
    movement::Ticker,
    tile_cast::tile_cast,
    velocity::{RelativeVelocity, TotalVelocity},
};

use pirate_sim_core::tile_grid::{GetTileLocation, TileStretch};
use pirate_sim_core::utils;

#[derive(Resource, Deref, Debug, Default, Reflect)]
pub struct CollisionMap(Vec<(IVec3, Entity, Constraints)>);

/// constraints put onto a collider and its collisions
#[derive(Debug, Clone, Copy, Reflect)]
pub struct Constraints {
    /// which axes it is "solid"  along, and thus will cause a collision conflict
    ///
    /// See the constants for [`Self`] for some examples
    pub pos_solid_planes: BVec3,
    pub neg_solid_planes: BVec3,
    /// Which axes it can be pushed along in order to resolve collision
    ///
    /// This is currently ignored
    pub move_along: BVec3,
}

impl Constraints {
    pub const WALL: Self = Self {
        pos_solid_planes: BVec3::TRUE,
        neg_solid_planes: BVec3::TRUE,
        move_along: BVec3::FALSE,
    };
    pub const FLOOR: Self = Self {
        pos_solid_planes: BVec3 {
            x: false,
            y: false,
            z: true,
        },
        neg_solid_planes: BVec3::FALSE,
        move_along: BVec3::FALSE,
    };
    pub const ENTITY: Self = Self {
        pos_solid_planes: BVec3 {
            x: true,
            y: true,
            z: false,
        },
        neg_solid_planes: BVec3 {
            x: true,
            y: true,
            z: false,
        },

        move_along: BVec3::TRUE,
    };

    pub const SENSOR: Self = Self {
        pos_solid_planes: BVec3::FALSE,
        neg_solid_planes: BVec3::FALSE,
        move_along: BVec3::FALSE,
    };

    fn wont_violate(&self, vel: Vec3) -> bool {
        let signs = vel.as_ivec3().signum();

        let check_neg = bvec_to_mask(self.pos_solid_planes).as_ivec3() * IVec3::NEG_ONE;
        let check_pos = bvec_to_mask(self.neg_solid_planes).as_ivec3() * IVec3::ONE;
        let signs_mask = signs.cmpeq(IVec3::ZERO);

        (signs_mask | (check_neg.cmpne(signs) & check_pos.cmpne(signs))).all()
    }
}

#[derive(Reflect, Debug, Clone)]
pub struct EntityCollision {
    pub other_entities: Vec<tile_cast::Hit<Entity>>,
    pub on_tile: IVec3,
    pub impulse: Vec3,
}

/// Currently, transform scale is not taken into account when calculating collision
///
/// Any entity with a collider must also have a transform
///
/// See constraints for choices on how to handle collision
#[derive(Component, Debug, Reflect)]
pub struct Collider {
    pub constraints: Constraints,
    collision: Option<EntityCollision>,
}

impl Collider {
    #[must_use]
    #[inline] // inline(always)?
    pub fn collision(&self) -> Option<&EntityCollision> {
        self.collision.as_ref()
    }

    #[must_use]
    #[inline]
    pub fn new(constraints: Constraints) -> Self {
        Self {
            constraints,
            collision: None,
        }
    }
}

#[allow(clippy::too_many_lines)]
/// Use tile casting to implement smooth collision impulses
fn tile_cast_collision(
    mut total_vel_q: Query<&mut TotalVelocity>,
    mut relative_vel_q: Query<&mut RelativeVelocity>,
    mut collider_q: Query<&mut Collider>,
    transform_q: Query<&GlobalTransform>,
    ticker_q: Query<&Ticker>,
    name_q: Query<&Name>,
    tile_stretch: Res<TileStretch>,
    predicted_map: Res<CollisionMap>,
) {
    use pirate_sim_core::utils::bvec_to_mask;

    // see build_collision_map
    for &(predicted_location, entity, constraints) in &**predicted_map {
        // SAFETY: entity was originally taken from a query over <(Entity, &Collider)> in the
        // current frame
        let mut collider = unsafe { collider_q.get_mut(entity).unwrap_unchecked() };

        // clear collider.collisions. This isn't really the right place to do this but it's fine
        collider.collision = None;

        let name = name_q
            .get(entity)
            .map_or("Unnamed".to_owned(), std::convert::Into::into);
        let translation = transform_q
            .get(entity)
            .expect("Entity with collider but no transform")
            .location(*tile_stretch);

        trace!("checking collision of {name} at predicted_location {predicted_location}, real location {translation}");

        let Some((vel, _)) = Option::zip(
            total_vel_q.get(entity).ok(),
            relative_vel_q.get(entity).ok(),
        ) else {
            trace!("entity has no velocity bundle; skipping");
            continue;
        };

        // This should never happen. Leave it in as common-sense assert
        debug_assert!(
            (translation.as_vec3() * vel.signum())
                .cmple(predicted_location.as_vec3() * vel.signum())
                .all(),
            "Predicted to move backwards from velocity"
        );

        if vel.0 == Vec3::ZERO {
            trace!("Entity not moving; skipping");
            continue;
        }

        trace!("Entity {} not skipped, checking for conflicts", name);

        // once this is correct, instead of folding to closest entity and checking that, go through
        // every possibly hit entity and bitor its constraints together
        let possibly_hit_entities = predicted_map.iter().filter(|(opl, oe, oc)| {
            // don't collide with ourselves
            *oe != entity
            // this entity is actually close enough to be hit
                && IVec3::cmple(
                   *opl * vel.0.signum().as_ivec3(),
                    predicted_location * vel.0.signum().as_ivec3(),
                )
                .all()
            //  add check against vel.0.signum()
            && !oc.wont_violate(**vel)
        });

        let hit_entities: Vec<_> = tile_cast(
            tile_cast::Origin {
                tile: translation,
                ticker: utils::get_or_zero(&ticker_q, entity),
            },
            **vel,
            *tile_stretch,
            possibly_hit_entities.map(|(l, a, b)| ((a, b), l)), // put it so that constraint & entity id are in data field
            true,
        )
        .collect();

        // This fold should work because there's only one shortest distance so once we get the
        // vector of entities with that shortest distance it'll never get replaced
        //
        // This fold finds only the entities we will collide with, assuming we do collide. This is
        // used for future checks of things like
        let Some(closest_distance) = hit_entities.iter().fold(None, |acc, elem| {
            Some(match acc {
                Some(acc) if acc > elem.distance => acc,
                _ => elem.distance,
            })
        }) else {
            trace!("No possible hit");
            continue;
        };

        // .0 is negative plane, .1 is positive
        let all_solid_axes = hit_entities
            .iter()
            .fold((BVec3::FALSE, BVec3::FALSE), |acc, elem| {
                let constraints = elem.data.1;

                (
                    acc.0 | constraints.neg_solid_planes,
                    acc.0 | constraints.pos_solid_planes,
                )
            });

        let total_vel_signs = vel.0.signum().as_ivec3();

        let needs_change_x = total_vel_signs.x == 1 && all_solid_axes.0.x
            || total_vel_signs.x == -1 && all_solid_axes.1.x;
        let needs_change_y = total_vel_signs.y == 1 && all_solid_axes.0.y
            || total_vel_signs.y == -1 && all_solid_axes.1.y;
        let needs_change_z = total_vel_signs.z == 1 && all_solid_axes.0.z
            || total_vel_signs.z == -1 && all_solid_axes.1.z;

        // to make this function continuous and avoid divide by zero bugs, multiply by 1 if
        // distance is 0. There might be a better thing to multiply but I'm not sure. Maybe 0?
        let stopping_factor = if closest_distance == 0. {
            1.
        } else {
            1. / closest_distance.round()
        };

        // FIXME: If expected to collide with entities at two locations, stopping_factor will be
        // incorrect
        let impulse = bvec_to_mask(BVec3::new(needs_change_x, needs_change_y, needs_change_z))
            * bvec_to_mask(constraints.move_along)
            * vel.0
            * stopping_factor;

        trace!("subtracting impulse {impulse}");

        // update collision info
        // FIXME: make it so on_tile is per entity
        collider.collision = Some(EntityCollision {
            other_entities: hit_entities.iter().map(|h| h.map(|(e, _)| *e)).collect(),
            on_tile: predicted_location,
            impulse,
        });

        // SAFETY: we should have already returned if these queries are invalid
        let mut vel = unsafe { total_vel_q.get_mut(entity).unwrap_unchecked() };
        let mut r_vel = unsafe { relative_vel_q.get_mut(entity).unwrap_unchecked() };

        vel.0 -= impulse;
        r_vel.0 -= impulse;

        // update collision info

        trace!("new vel r: {} t: {}", r_vel.0, vel.0);
    }
}

/// Predict the change in grid location of an entity based on its current velocities. This will only be accurate
/// in between [`PhysicsSet::Velocity`] and [`PhysicsSet::Movement`] \(ie. during
/// [`PhysicsSet::Collision`])
fn calc_movement(
    total_vel: Option<&TotalVelocity>,
    ticked_vel: Option<&Ticker>,
    time_delta: f32,
) -> IVec3 {
    // if either of these are not present assume they will contribute to moving the entity
    // If they are, just copy them
    let total_velocity = total_vel.map_or_else(|| Vec3::ZERO, |c| **c);
    let ticked_velocity = ticked_vel.map_or_else(|| Vec3::ZERO, |c| **c);

    // its projected movement will just be however much the ticker is already filled, along
    // with its total velocity times the time delta to get how much it will move this frame
    //

    let projected_movement_raw = total_velocity * time_delta + ticked_velocity;

    // multiplying Signum before flooring makes sure it will floor towards zero, then we just
    // reverse it
    let projected_movement_rounded = (projected_movement_raw * projected_movement_raw.signum())
        .floor()
        * projected_movement_raw.signum();

    // the projected movement is already in tilespace & rounded, so just cast
    projected_movement_rounded.as_ivec3()
}

/// PERF: we could consider updating in-place
///
/// TODO: I think it probably makes more sense to flatten it out to Vec<(IVec3,...)> for perf, etc
fn build_collision_map(
    collider_q: Query<(
        Entity,
        &Collider,
        Option<&TotalVelocity>,
        Option<&Ticker>,
        &GlobalTransform,
    )>,
    time: Res<Time>,
    tile_stretch: Res<TileStretch>,
    mut collision_map: ResMut<CollisionMap>,
) {
    collision_map.0 = collider_q
        .iter()
        .map(|(entity, c, total_v, ticker, transform)| {
            (
                calc_movement(total_v, ticker, time.delta_seconds())
                    + transform.location(*tile_stretch),
                entity,
                c.constraints,
            )
        })
        .collect();
}

pub(super) struct Plugin;

impl bevy_app::Plugin for Plugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            (build_collision_map, tile_cast_collision)
                .chain()
                .in_set(PhysicsSet::Collision),
        )
        .init_resource::<CollisionMap>();
    }
}

#[cfg(test)]
#[test]
fn constraints_properly_report() {
    let floor = Constraints::FLOOR;
    let wall = Constraints::WALL;

    let can_move_through_wall = wall.wont_violate(Vec3::new(1., 2., 3.));
    let can_move_over_floor = floor.wont_violate(Vec3::new(1., 2., 0.));
    let can_fall_through_floor = floor.wont_violate(Vec3::new(1., 2., -1.));

    assert!(!can_move_through_wall);
    assert!(!can_fall_through_floor);
    assert!(can_move_over_floor);
}
