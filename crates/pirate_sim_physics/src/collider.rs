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

use std::fmt::Display;

use bevy::{prelude::*, utils::HashMap};
use pirate_sim_core::PhysicsSet;

use super::{
    movement::Ticker,
    tile_cast::tile_cast,
    velocity::{RelativeVelocity, TotalVelocity},
};
use pirate_sim_core::tile_grid::{GetTileLocation, TileStretch};

#[derive(Debug, Clone)]
pub struct CollisionEntity {
    pub constraints: Constraints,
    pub entity: Entity,
    pub violated: BVec3,
}

#[derive(Resource, Deref, Debug, Default)]
pub struct CollisionMap(HashMap<IVec3, (Entity, Constraints)>);

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
    pub fn was_in_conflict(&self) -> bool {
        self.conflict_along.any()
    }

    fn new(resolution: &ConflictInfo, colliders: &[Entity]) -> Self {
        EntityCollision {
            entity: resolution.entity,
            tile: resolution.position,
            conflict_along: resolution.to_block,
            colliding_with: colliders
                .iter()
                .filter(|e| **e != resolution.entity)
                .copied()
                .collect(),
        }
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
    pub fn new(constraints: Constraints) -> Self {
        Self { constraints }
    }
}

/// Predict the location of an entity  based on its current velocities. This will only be accurate
/// in between [`PhysicsSet::Velocity`] and [`PhysicsSet::Movement`]
///
/// TODO: switch this to predict_velocity, which is a more useful result, as it can just be added
/// to transform.translation() to get predicted location, which seems to be used less often than
/// predicted velocity, leading to more calculations of predicted - translation than there would be
/// for translation + predicted
fn predict_location(
    total_vel: Option<&TotalVelocity>,
    ticked_vel: Option<&Ticker>,
    current_location: Vec3,
    time_delta: f32,
    tile_stretch: TileStretch,
    name: &str,
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

    trace!("predicting {}", name);
    trace!(
        "total velocity {}, ticked velocity {}",
        total_velocity,
        ticked_velocity
    );
    trace!(
        "projected raw, rounded {}, {}",
        projected_movement_raw,
        projected_movement_rounded
    );

    // the projected movement is already in tilespace, so just convert the current location, then
    // add

    let current_tile = match tile_stretch.get_tile(current_location) {
        Ok(t) => t,
        Err(t) => {
            error!("transform not on grid: {}", t);
            t.to_closest()
        }
    };

    current_tile + projected_movement_rounded.as_ivec3()
}

#[derive(Debug, Clone)]
struct InhabitingTile {
    entity: Entity,
    constraints: Constraints,
    predicted_movement: IVec3,
}

// an amount to subtract from the entities velocity
struct ConflictInfo {
    entity: Entity,
    // if true subtract 1 * total_vel.signum() from total_vel
    to_block: BVec3,
    // for bookkeeping
    position: IVec3,
    constraints: Constraints,
}

fn log_collisions(mut events: EventReader<EntityCollision>, name_q: Query<&Name>) {
    for event in events.iter() {
        trace!(
            "Entity: {} collided at {}, with {} other entities, collision axes: {}",
            name_q
                .get(event.entity)
                .ok()
                .map_or_else(|| "Unnamed entity".to_string(), |v| v.to_string()),
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
fn tile_cast_collision(
    mut total_vel_q: Query<&mut TotalVelocity>,
    mut relative_vel_q: Query<&mut RelativeVelocity>,
    transform_q: Query<&GlobalTransform>,
    tile_stretch: Res<TileStretch>,
    predicted_map: Res<CollisionMap>,
) {
    let predicted_map = &**predicted_map;
    // we need a vec (Vec3,)

    for (&predicted_translation, &(entity, constraints)) in predicted_map {
        // send out event here
        if let Some((mut vel, mut r_vel)) = Option::zip(
            total_vel_q.get_mut(entity).ok(),
            relative_vel_q.get_mut(entity).ok(),
        ) {
            let translation = transform_q
                .get(entity)
                .expect("Entity with collider but no transform")
                .location(*tile_stretch);

            let hit_entities = tile_cast(
                translation,
                **vel,
                *tile_stretch,
                predicted_map.iter().map(|(l, (c, e))| ((c, e), l)),
                false,
            );

            // This fold should work because there's only one shortest distance so once we get the
            // vector of entities with that shortest distance it'll never get replaced
            let closest_entities = hit_entities.iter().fold(None, |acc, elem| {
                match acc {
                    None => Some(vec![elem]),
                    Some(mut acc) => {
                        let acc_d = acc[0].1.distance(vel.0);
                        let elem_d = elem.1.distance(vel.0);

                        // check against epilson just in case. Silences clippy lint
                        if (elem_d - acc_d).abs() <= f32::EPSILON {
                            acc.push(elem);
                            Some(acc)
                        } else if elem_d < acc_d {
                            Some(vec![elem])
                        } else {
                            Some(acc)
                        }
                    }
                }
            });

            // .0 is negative plane, .1 is positive
            let all_solid_axes = closest_entities.into_iter().flatten().fold(
                (BVec3::FALSE, BVec3::FALSE),
                |acc, elem| {
                    let constraints = elem.0 .1;

                    (
                        acc.0 | constraints.neg_solid_planes,
                        acc.0 | constraints.pos_solid_planes,
                    )
                },
            );

            let total_vel_signs = vel.0.signum().as_ivec3();

            let vel_change_x = if total_vel_signs.x == 1 && all_solid_axes.0.x {
                // remove velocity here
                todo!()
            } else if total_vel_signs.x == -1 && all_solid_axes.1.x {
                todo!()
            } else {
                0
            };

            // solve from all_solid_axes

            todo!("Find closest entity that it will hit, and only have a collision if that entity is going to be moved into next frame");

            // raycast out with predicted_map
        }
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

    trace!(
        "total velocity {}, ticked velocity {}",
        total_velocity,
        ticked_velocity
    );
    trace!(
        "projected raw, rounded {}, {}",
        projected_movement_raw,
        projected_movement_rounded
    );

    // the projected movement is already in tilespace & rounded, so just cast
    projected_movement_rounded.as_ivec3()
}

/// return a hashmap of (predicted location)->(Entity,Constraints)
/// TODO: return a Btreemap?
///
/// TODO: change to `build_collision_map` system
fn build_collision_map(
    collider_q: Query<(Entity, &Collider)>,
    total_vel_q: Query<&mut TotalVelocity>,
    ticker_q: Query<&Ticker>,
    transform_q: Query<&GlobalTransform>,
    time: Res<Time>,
    tile_stretch: Res<TileStretch>,
    mut collision_map: ResMut<CollisionMap>,
) {
    collision_map.0 = collider_q
        .into_iter()
        .map(|(entity, c)| {
            let predicted_location = calc_movement(
                total_vel_q.get(entity).ok(),
                ticker_q.get(entity).ok(),
                time.delta_seconds(),
            ) + transform_q
                .get(entity)
                .expect("Collider on entity with no transform")
                .location(*tile_stretch);

            (predicted_location, (entity, c.constraints))
        })
        .collect();
}
