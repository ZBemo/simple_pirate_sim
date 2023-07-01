//! Colliders and Collision systems
//!
//! Functions and documents in this module will often refer to `collisions` and `conflicts`, which
//! are two different things. A conflict is when two or more colliders are going to move through a
//! collider in a way that conflicts on both colliders' axis-planes, or vice-versa. It is the
//! physics systems job to prevent any conflicts from actually happening. Usually through
//! cancelling the velocity of one or more object.
//!
//! Collisions however, are solely when two colliders will overlap, which does not always
//! necessitate interference from the physics systems
//!
//! This module is probably rife with opportunities for performance improvements.

// #![allow(unused)]

use std::unreachable;

use bevy::{prelude::*, utils::HashMap};

use super::{movement::Ticker, velocity::TotalVelocity, PhysicsSet};
use crate::tile_grid::TileStretch;

#[derive(Debug, Clone, Deref, Reflect)]
pub struct Impulse(IVec3);

#[derive(Debug, Clone)]
pub struct CollisionEntity {
    pub constraints: Constraints,
    pub entity: Entity,
    pub violated: BVec3,
}

/// A collision Event. If an entity is in the collision on a specific location,  
/// it will be in the hashmap, mapping to any impulse applied for conflict resolution.
#[derive(Debug, Clone)]
pub struct TileCollision {
    /// which tile
    pub tile: IVec3,
    /// which entities were involved
    pub entities: Vec<CollisionEntity>,
}

/// An event where there was an entity collision
///
/// TODO: replace with TileCollision
#[derive(Debug, Clone)]
pub struct EntityCollision {
    /// which entity was involved in the collision
    pub entity: Entity,
    pub tile: IVec3,
    pub conflict_along: BVec3,
    pub colliding_with: Vec<Entity>,
}

impl EntityCollision {
    pub fn was_in_conflict(&self) -> bool {
        self.conflict_along.any()
    }
}

/// constraints put onto a collider and its collisions
#[derive(Debug, Clone, Reflect)]
pub struct Constraints {
    /// which axes it is "solid" in a plane along, and thus will cause a collision conflict
    ///
    /// for example, a hallway oriented -x->+x might only allowing moving through it from +-x->+-x,
    /// and so would have y as PosNeg, to prevent moving into it from the y axes
    ///
    /// diagonal movement will interpret solid axes in the most generous way possible, so in the
    /// hallway example above as long as the entity moving into it is not at hallway.translate +/-
    /// TileVec::Y, it will be able to move through it
    ///
    /// Essentially, if an object is moving along an axis with a sign the same as its Axis
    /// selection, it will trigger a conflict
    pub solid_planes: BVec3,
    /// Which axes it can be pushed along in order to resolve collision
    ///
    /// This is currently ignored
    pub move_along: BVec3,
}

impl Constraints {
    pub const WALL: Self = Self {
        solid_planes: BVec3::TRUE,
        move_along: BVec3::FALSE,
    };
    pub const FLOOR: Self = Self {
        solid_planes: BVec3 {
            x: false,
            y: false,
            z: true,
        },
        move_along: BVec3::FALSE,
    };
    pub const ENTITY: Self = Self {
        solid_planes: BVec3 {
            x: true,
            y: true,
            z: true,
        },
        move_along: BVec3::TRUE,
    };

    pub const SENSOR: Self = Self {
        solid_planes: BVec3::FALSE,
        move_along: BVec3::FALSE,
    };
}

/// a tile collider, specified in tile space. Importantly, size essentially functions as another
/// corner of the Collider's "box", so a size of (0,0,0) should inhabit a single tile. See
/// constraints for more size granularity
///
/// Currently, transform scale is not taken into account when calculating collision
///
/// Any entity with a collider must also have a transform
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
/// in between FinalizeVelocity and FinalizeMovement
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
    tile_stretch: &TileStretch,
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

    let current_tile = match tile_stretch.bevy_to_tile(&current_location) {
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

// function to check for collisions. updates collision map in place
fn check_collisions(
    collider_q: &Query<(Entity, &Collider)>,
    transform_q: &Query<&GlobalTransform>,
    velocity_q: &Query<&super::velocity::TotalVelocity>,
    ticker_q: &Query<&super::movement::Ticker>,
    name_q: &Query<&Name>,
    tile_stretch: &TileStretch,
    delta_time: f32,
) -> HashMap<IVec3, Vec<InhabitingTile>> {
    let mut collision_map: HashMap<IVec3, Vec<InhabitingTile>> = HashMap::new();

    // could loop concurrently to create a Vec of expected tiles, and then loop that in single
    // thread to populate inhabited hashmap?
    for (entity, collider) in collider_q.iter() {
        // start off by getting any velocities, the absolute transform of the entity, and its
        // collider and entity id

        let velocities = (velocity_q.get(entity).ok(), ticker_q.get(entity).ok());
        let name = name_q
            .get(entity)
            .map_or_else(|_| "UnnamedEntity".into(), |n| n.to_string());
        let transform = transform_q.get(entity).expect(
            "Entity with Collider has no transform. Any collider should also have a transform.",
        );

        // add projected_movement to absolute location to get projected absolute location. then
        // translate to tile space.
        let projected_tile_location = predict_location(
            velocities.0,
            velocities.1,
            transform.translation(),
            delta_time,
            tile_stretch,
            &name,
        );

        trace!(
            "pushing inhabiting with real location of {}, predicted movement of {}",
            transform.translation(),
            projected_tile_location
                - tile_stretch
                    .bevy_to_tile(&transform.translation())
                    .map_or_else(|m| m.to_closest(), |m| m)
        );

        // TODO: this might need error handling
        let tile = InhabitingTile {
            entity,
            predicted_movement: projected_tile_location
                - tile_stretch
                    .bevy_to_tile(&transform.translation())
                    .map_or_else(|m| m.to_closest(), |m| m),
            constraints: collider.constraints.clone(),
        };

        if let Some(inhabited_vec) = collision_map.get_mut(&projected_tile_location) {
            inhabited_vec.push(tile);
        } else {
            collision_map.insert_unique_unchecked(projected_tile_location, vec![tile]);
        }
    }

    collision_map
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

/// check if any projected movement onto a single tile will result in a conflicting collision
///
/// SAFETY: this function assumes that the calling function has already ensured every entity passed
/// to it has an associated collider.
///
/// Sometime in the future, we should be able to find a way to return a Iterator<Item = ConflictResolution>, but currently the lifetimes are out of my understanding.
/// could just inline this?
///
/// we need to clarify that Entity, &Collider live for 'b, while both references are 'a. 'a  must
/// be valid for >= 'b, and our return will be valid for 'a. It looks like hidden lifetimes from
/// the collider query  are what mess things up
fn find_and_resolve_conflicts(
    collisions: &HashMap<IVec3, Vec<InhabitingTile>>,
    collider_q: &Query<(Entity, &Collider)>,
    writer: &mut EventWriter<EntityCollision>, // this should be separated into another function to
                                               // keep this one functionally pure
) -> Vec<ConflictInfo> {
    // start by mapping each possible movement violation to any entities that would have their collider
    // constraints violated
    #[derive(Debug, Default)]
    struct ViolatablePlanes {
        x: Vec<Entity>,
        y: Vec<Entity>,
        z: Vec<Entity>,
    }

    collisions
        .iter()
        .filter(|v| v.1.len() > 1)
        .flat_map(|(position, inhabitants)| {
            // empty
            let mut planes = ViolatablePlanes::default();

            let collision_map = inhabitants;

            for entity in collision_map.iter() {
                // safety: any entity involved in a collision must have a collider
                let collider = unsafe { collider_q.get(entity.entity).unwrap_unchecked().1 };

                // add entity to violatableplanes if it is violatable
                if collider.constraints.solid_planes.z {
                    planes.z.push(entity.entity)
                }
                if collider.constraints.solid_planes.y {
                    planes.y.push(entity.entity)
                }
                if collider.constraints.solid_planes.x {
                    planes.x.push(entity.entity)
                }
            }

            // now, check for collisions
            collision_map.iter().map(move |entity| {
                let movement_signs = entity.predicted_movement.signum();
                let mut current_resolution: BVec3 = BVec3::FALSE;
                debug!("{}->{}", movement_signs, entity.predicted_movement);

                match movement_signs.z {
                    1 | -1 => {
                        if Iterator::zip(1.., planes.z.iter().filter(|e| **e != entity.entity))
                            .map(|e| e.0)
                            .last()
                            .unwrap_or(0)
                            >= 1
                        {
                            current_resolution.z = entity.constraints.solid_planes.z;
                        }
                    }
                    0 => {
                        // do nothing
                    }
                    _ => {
                        #[cfg(debug_assertions)]
                        unreachable!();
                        #[cfg(not(debug_assertions))]
                        unreachable_unchecked()
                    }
                }
                match movement_signs.x {
                    1 | -1 => {
                        if Iterator::zip(1.., planes.x.iter().filter(|e| **e != entity.entity))
                            .map(|e| e.0)
                            .last()
                            .unwrap_or(0)
                            >= 1
                        {
                            current_resolution.x = entity.constraints.solid_planes.x;
                        }
                    }
                    0 => {
                        // do nothing
                    }
                    _ => {
                        #[cfg(debug_assertions)]
                        unreachable!();
                        #[cfg(not(debug_assertions))]
                        unreachable_unchecked()
                    }
                }
                match movement_signs.y {
                    1 | -1 => {
                        if Iterator::zip(1.., planes.y.iter().filter(|e| **e != entity.entity))
                            .map(|e| e.0)
                            .last()
                            .unwrap_or(0)
                            >= 1
                        {
                            current_resolution.y = entity.constraints.solid_planes.y;
                        }
                    }
                    0 => {
                        // do nothing
                    }
                    _ => {
                        #[cfg(debug_assertions)]
                        unreachable!();
                        #[cfg(not(debug_assertions))]
                        unreachable_unchecked()
                    }
                }

                let info = ConflictInfo {
                    entity: entity.entity,
                    to_block: current_resolution,
                    position: *position,
                    constraints: entity.constraints.clone(),
                };

                let event = gen_collision_event(
                    &info,
                    &inhabitants.iter().map(|t| t.entity).collect::<Box<_>>(),
                );

                (info, event)
            })
        })
        .map(|(ret, event)| {
            // send event and discard as it's now irrelevant
            writer.send(event);
            ret
        })
        .collect()
}

// TODO: move this to [`EntityCollision::new`]
fn gen_collision_event(resolution: &ConflictInfo, colliders: &[Entity]) -> EntityCollision {
    EntityCollision {
        entity: resolution.entity,
        tile: resolution.position,
        conflict_along: resolution.to_block,
        colliding_with: colliders
            .iter()
            .filter(|e| **e != resolution.entity)
            .cloned()
            .collect(),
    }
}

/// Behemoth system for checking and then resolving collisions
///
/// For now this only does one "layer" of collision checking, which means it assumes that any
/// moving entity is starting from a position that does not violate any constraints.
///
/// This should be fixed in the future. Can probably just slap it all in a loop, with
/// change tracking for performance
///
/// We should also consider having this simply update an Asset with wanted resolutions or something
/// of the sort, and then have other systems act on that to do things like actually clamp velocity,
/// send out events, etc. This might be less performant but would lead to far cleaner code.
///
/// Perhaps this should have its own component for adding a reactive velocity? This can be easily
/// done if we see cases where it is beneficial in the future.
fn check_and_resolve_collisions(
    // mut collision_events: EventReader<Collision>,
    total_velocity_q: Query<&super::velocity::TotalVelocity>,
    mut rel_velocity_q: Query<&mut super::velocity::RelativeVelocity>,
    ticker_q: Query<&super::movement::Ticker>,
    collider_q: Query<(Entity, &Collider)>,
    transform_q: Query<&GlobalTransform>,
    tile_stretch: Res<TileStretch>,
    name_q: Query<&Name>,
    time: Res<Time>,
    mut writer: EventWriter<EntityCollision>,
) {
    trace!("Starting collision checking and resolution");

    let delta_time = time.delta_seconds();

    let inhabited_tiles = check_collisions(
        &collider_q,
        &transform_q,
        &total_velocity_q,
        &ticker_q,
        &name_q,
        &tile_stretch,
        delta_time,
    );

    let resolutions = find_and_resolve_conflicts(&inhabited_tiles, &collider_q, &mut writer);

    for resolution in resolutions {
        // TODO: Consider other colliders collision.

        let mut rel_vel = unsafe { rel_velocity_q.get_mut(resolution.entity).unwrap_unchecked() };
        if resolution.to_block.z {
            rel_vel.0.z = 0.;
        }
        if resolution.to_block.x {
            rel_vel.0.x = 0.;
        }
        if resolution.to_block.y {
            rel_vel.0.y = 0.;
        }
    }
}

#[cfg(debug_assertions)]
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
        )
    }
}

pub(super) struct Plugin;

impl bevy::prelude::Plugin for Plugin {
    fn build(&self, app: &mut App) {
        app.add_system(
            check_and_resolve_collisions
                .in_set(PhysicsSet::FinalizeCollision)
                .after(PhysicsSet::FinalizeVelocity)
                .before(PhysicsSet::FinalizeMovement),
        )
        .add_system(log_collisions.after(PhysicsSet::FinalizeCollision))
        .add_system(check_and_resolve_collisions.after(PhysicsSet::FinalizeCollision))
        .add_event::<EntityCollision>();
    }
}
