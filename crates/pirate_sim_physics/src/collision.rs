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

use std::{borrow::BorrowMut, fmt::Display};

use bevy::{prelude::*, utils::HashMap};
use pirate_sim_core::{utils::get_or_empty, PhysicsSet};

use crate::tile_cast;

use super::{
    movement::Ticker,
    tile_cast::tile_cast,
    velocity::{RelativeVelocity, TotalVelocity},
};

use pirate_sim_core::tile_grid::{GetTileLocation, TileStretch};
use pirate_sim_core::utils;

#[derive(Debug, Clone)]
pub struct CollisionEntity {
    pub constraints: Constraints,
    pub entity: Entity,
    pub violated: BVec3,
}

#[derive(Resource, Deref, Debug, Default, Reflect)]
pub struct CollisionMap(HashMap<IVec3, Vec<(Entity, Constraints)>>);

/// A collision Event. If an entity is in the collision on a specific location,  
/// it will be in the hashmap, mapping to any impulse applied for conflict resolution.
#[derive(Debug, Clone, Event)]
pub struct TileCollision {
    /// which tile
    pub tile: IVec3,
    /// which entities were involved
    pub entities: Vec<CollisionEntity>,
}

/// An event where there was an entity collision
///
/// TODO: replace with [`TileCollision`] or do both
#[derive(Debug, Clone, Event)]
pub struct EntityCollision {
    pub entity: Entity,
    pub tile: IVec3,
    pub conflict_along: BVec3,
    pub colliding_with: Vec<Entity>,
}

impl Display for EntityCollision {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Entity: {:?}, Tile: {}, conflict_along: {}, Colliding: {:?}",
            self.entity, self.tile, self.conflict_along, self.colliding_with
        )
    }
}

impl EntityCollision {
    #[must_use]
    pub fn was_in_conflict(&self) -> bool {
        self.conflict_along.any()
    }
}

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
        pos_solid_planes: BVec3::TRUE,
        neg_solid_planes: BVec3::TRUE,

        move_along: BVec3::TRUE,
    };

    pub const SENSOR: Self = Self {
        pos_solid_planes: BVec3::FALSE,
        neg_solid_planes: BVec3::FALSE,
        move_along: BVec3::FALSE,
    };
}

/// Currently, transform scale is not taken into account when calculating collision
///
/// Any entity with a collider must also have a transform
///
/// See constraints for choices on how to handle collision
#[derive(Component, Debug, Reflect)]
pub struct Collider {
    pub constraints: Constraints,
}

impl Collider {
    #[must_use]
    pub fn new(constraints: Constraints) -> Self {
        Self { constraints }
    }
}

fn log_collisions(mut events: EventReader<EntityCollision>, name_q: Query<&Name>) {
    for event in events.iter() {
        trace!(
            "Entity: {} collided at {}, with {} other entities, collision axes: {}",
            name_q.get(event.entity).ok().map_or_else(
                || "Unnamed entity".to_string(),
                std::string::ToString::to_string
            ),
            event.tile,
            event.colliding_with.len(),
            event.conflict_along
        );
    }
}

pub(super) struct Plugin;

impl bevy::prelude::Plugin for Plugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            (build_collision_map, tile_cast_collision)
                .chain()
                .in_set(PhysicsSet::Collision),
        )
        .add_event::<EntityCollision>()
        .init_resource::<CollisionMap>();
    }
}

// TODO: We don't account for ticker when tile casting
#[allow(clippy::too_many_lines)]
fn tile_cast_collision(
    mut total_vel_q: Query<&mut TotalVelocity>,
    mut relative_vel_q: Query<&mut RelativeVelocity>,
    transform_q: Query<&GlobalTransform>,
    ticker_q: Query<&Ticker>,
    name_q: Query<&Name>,
    tile_stretch: Res<TileStretch>,
    predicted_map: Res<CollisionMap>,
    time: Res<Time>,
) {
    /// Turn a bvec into a vec where rhs * bvec_to_vec(vec) will "mask" away any falses
    fn bvec_to_vec(vec: BVec3) -> Vec3 {
        let x = if vec.x { 1. } else { 0. };
        let y = if vec.y { 1. } else { 0. };
        let z = if vec.z { 1. } else { 0. };

        Vec3::new(x, y, z)
    }

    let mut flattened_predicted_map = Vec::new();

    for (tile, datas) in &**predicted_map {
        for data in datas {
            flattened_predicted_map.push((*tile, *data));
        }
    }

    // we need a vec (Vec3,)

    for &(_, (entity, _constraints)) in &flattened_predicted_map {
        // send out event here

        let name = name_q
            .get(entity)
            .map_or("Unnamed".to_owned(), std::convert::Into::into);

        trace!("checking collision of {name}");

        let Some((vel, _)) = Option::zip(
            total_vel_q.get(entity).ok(),
            relative_vel_q.get(entity).ok(),
        ) else {
            trace!("entity has no velocity bundle; skipping");
            continue;};

        if vel.0 == Vec3::ZERO {
            trace!("Entity not moving; skipping");
            continue;
        }

        trace!("Checking for conflicts for entity {}", name);
        trace!("Entity not skipped");

        let translation = transform_q
            .get(entity)
            .expect("Entity with collider but no transform")
            .location(*tile_stretch);

        let hit_entities = tile_cast(
            tile_cast::Origin {
                tile: translation,
                ticker: utils::get_or_empty(&ticker_q, entity),
            },
            **vel,
            *tile_stretch,
            flattened_predicted_map
                .iter()
                .filter(|(_, (e, _))| *e != entity)
                .map(|(l, (a, b))| ((a, b), l)),
            true,
        );

        // This fold should work because there's only one shortest distance so once we get the
        // vector of entities with that shortest distance it'll never get replaced
        let closest_entities = hit_entities.fold(vec![], |mut acc, elem| {
            if acc.is_empty() {
                vec![elem]
            } else {
                let acc_d = acc[0].distance;
                let elem_d = elem.distance;

                // check against epilson just in case. Silences clippy lint
                if (elem_d - acc_d).abs() <= f32::EPSILON {
                    acc.push(elem);
                    acc
                } else if elem_d < acc_d {
                    vec![elem]
                } else {
                    acc
                }
            }
        });

        if closest_entities.is_empty() {
            trace!("No entities along way; continuing");
            continue;
        }

        // this could probably be quicker. Check if it will move far enough in this frame to hit
        // the entity
        //
        // TODO: perhaps more performant to do distance^2 > (vel).dot().abs() as it avoids a sqrt,
        // instead using a square
        if (closest_entities[0].distance - get_or_empty(&ticker_q, entity).length()).abs()
            > (vel.0 * time.delta_seconds()).length().abs()
        {
            trace!("No possible conflict close enough");
            trace!(
                "found that {} < {}",
                closest_entities[0].distance.abs(),
                ((vel.0 * time.delta_seconds()).length()
                    + get_or_empty(&ticker_q, entity).length())
            );
            continue;
        };

        // .0 is negative plane, .1 is positive
        let all_solid_axes =
            closest_entities
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

        let total_change = (closest_entities[0].translation - translation).as_vec3()
            * bvec_to_vec(BVec3::new(needs_change_x, needs_change_y, needs_change_z))
            * vel.0
            * vel.0.signum()
            * -1.;

        trace!("applying vel change {total_change}");

        // SAFETY: we should have already returned if these queries are invalid
        let mut vel = unsafe { total_vel_q.get_mut(entity).unwrap_unchecked() };
        let mut r_vel = unsafe { relative_vel_q.get_mut(entity).unwrap_unchecked() };

        vel.0 += total_change;
        r_vel.0 += total_change;

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
/// FIXME: This breaks due to change detection??
fn build_collision_map(
    collider_q: Query<(Entity, &Collider)>,
    total_vel_q: Query<&TotalVelocity>,
    ticker_q: Query<&Ticker>,
    transform_q: Query<&GlobalTransform>,
    time: Res<Time>,
    tile_stretch: Res<TileStretch>,
    mut collision_map: ResMut<CollisionMap>,
) {
    let inner_cm = &mut collision_map.0;
    *inner_cm = HashMap::new();

    collider_q.for_each(|(entity, c)| {
        let predicted_location = calc_movement(
            total_vel_q.get(entity).ok(),
            ticker_q.get(entity).ok(),
            time.delta_seconds(),
        ) + transform_q
            .get(entity)
            .expect("Collider on entity with no transform")
            .location(*tile_stretch);

        inner_cm
            .entry(predicted_location)
            .or_default()
            .push((entity, c.constraints));
    });
}