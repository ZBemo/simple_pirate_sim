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

#![allow(unused)]

use std::{assert_eq, assert_ne, hint::unreachable_unchecked};

use bevy::{
    prelude::*,
    utils::{HashMap, HashSet},
};

use crate::tile_objects::{ObjectName, TileStretch};

use super::{movement::Ticker, velocity::TotalVelocity, PhysicsSet};

#[derive(Debug, Clone)]
pub struct Collision {
    location: IVec3,
    entities: Vec<Entity>,
    // some way to ignore other colliders
}

/// If a collider has a solid plane parallel to the tile boundary on the negative axis, positive, neither, or both
#[repr(u8)]
#[derive(Debug, Clone, Copy, Default, Eq, PartialEq)]
pub enum AxisPlanes {
    #[default]
    None = 0,
    Pos = 0b01,
    Neg = 0b10,
    PosNeg = 0b11,
}

impl AxisPlanes {
    /// returns true if the axis has a plane across its negative
    pub fn accross_negative(&self) -> bool {
        (*self as u8 & 0b10) != 0
    }
    /// returns true if the axis has a plane across its positive
    pub fn across_positive(&self) -> bool {
        (*self as u8 & 0b01) != 0
    }

    pub fn has_plane(&self) -> bool {
        *self as u8 != 0
    }
}

/// A Vec3 of [`SolidAxis`] constraints
#[derive(Debug, Clone, Default)]
pub struct PVec3 {
    pub x: AxisPlanes,
    pub y: AxisPlanes,
    pub z: AxisPlanes,
}

impl PVec3 {
    pub const ALL: Self = Self {
        x: AxisPlanes::PosNeg,
        y: AxisPlanes::PosNeg,
        z: AxisPlanes::PosNeg,
    };
    pub const NONE: Self = Self {
        x: AxisPlanes::None,
        y: AxisPlanes::None,
        z: AxisPlanes::None,
    };

    /// returns true if any of its axes are not Axis::None
    pub fn any_planes(&self) -> bool {
        // None represented as 0
        (self.z as u8 | self.x as u8 | self.y as u8) != 0
    }
}

/// constraints put onto a collider and its collisions
#[derive(Debug, Clone)]
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
    pub solid_planes: PVec3,
    /// which axes it can be pushed along in order to resolve collision
    pub move_along: PVec3,
}

impl Constraints {
    pub const BOX: Self = Self {
        solid_planes: PVec3::ALL,
        move_along: PVec3::NONE,
    };
    pub const FLOOR: Self = Self {
        solid_planes: PVec3 {
            x: AxisPlanes::None,
            y: AxisPlanes::None,
            z: AxisPlanes::Neg,
        },
        move_along: PVec3::NONE,
    };
    pub const ENTITY: Self = Self {
        solid_planes: PVec3 {
            x: AxisPlanes::None,
            y: AxisPlanes::None,
            z: AxisPlanes::Neg,
        },
        move_along: PVec3::ALL,
    };

    pub const SENSOR: Self = Self {
        solid_planes: PVec3::NONE,
        move_along: PVec3::NONE,
    };
}

/// a tile collider, specified in tile space. Importantly, size essentially functions as another
/// corner of the Collider's "box", so a size of (0,0,0) should inhabit a single tile. See
/// constraints for more size granularity
///
/// Currently, transform scale is not taken into account when calculating collision
///
/// Any entity with a collider must also have a transform
#[derive(Component, Debug)]
pub struct Collider {
    pub size: IVec3,
    pub constraints: Constraints,
}

impl Collider {
    pub fn new(size: IVec3, constraints: Constraints) -> Self {
        Self { size, constraints }
    }
}

/// Predict the location of an entity after FinalizeMovement based on its current velocities
fn predict_location(
    total_vel: Option<&TotalVelocity>,
    ticked_vel: Option<&Ticker>,
    current_location: Vec3,
    time_delta: f32,
    tile_stretch: &TileStretch,
) -> IVec3 {
    // if either of these are not present assume they will contribute to moving the entity
    // If they are, just copy them
    let total_velocity = total_vel.map_or_else(|| Vec3::ZERO, |c| **c);
    let ticked_velocity = ticked_vel.map_or_else(|| Vec3::ZERO, |c| **c);

    // its projected movement will just be however much the ticker is already filled, along
    // with its total velocity times the time delta to get how much it will move this frame
    //

    let projected_movement_raw = (total_velocity * time_delta + ticked_velocity);

    // multiplying Signum before flooring makes sure it will floor towards zero, then we just
    // reverse it
    let projected_movement_rounded = (projected_movement_raw * projected_movement_raw.signum())
        .floor()
        * projected_movement_raw.signum();

    // the projected movement is already in tilespace, so just convert the current location, then
    // add
    (tile_stretch.bevy_translation_to_tile(&current_location)
        + projected_movement_rounded.as_ivec3())
}

/// Behemoth function for checking and then resolving collisions
///
/// as there's no good way to persist info between systems, it makes the most sense currently to
/// just check collision and then resolve them using information generated in the same system
///
/// This also allows us to write more detailed Collision events
fn check_and_resolve_collisions(
    mut collision_events: EventReader<Collision>,
    mut velocity_q: Query<&mut super::velocity::TotalVelocity>,
    ticker_q: Query<&super::movement::Ticker>,
    collider_q: Query<(Entity, &Collider)>,
    transform_query: Query<&GlobalTransform>,
    name_q: Query<&ObjectName>,
    tile_stretch: Res<TileStretch>,
    time: Res<Time>,
    collision_writer: EventWriter<Collision>,
) {
    trace!("Starting collision checking and resolution");

    /// gets a range to n..=0 that will start at n if n is negative, or start at zero otherwise
    /// so you will always get the same amount of steps regardless of n's sign
    fn range_to_n(n: i32) -> std::ops::RangeInclusive<i32> {
        if n.is_negative() {
            n..=0
        } else {
            0..=n
        }
    }
    // this will keep track of any tiles that will be inhabited, as well as which colliders will be
    // in that tile
    let mut inhabited_tiles: HashMap<IVec3, Vec<Entity>> = HashMap::new();

    // could loop concurrently to create a Vec of expected tiles, and then loop that in single
    // thread to populate inhabited hashmap?
    for (entity, collider) in collider_q.iter() {
        // start off by getting any velocities, the absolute transform of the entity, and its
        // collider and entity id

        let velocities = (velocity_q.get(entity).ok(), ticker_q.get(entity).ok());
        let transform = transform_query.get(entity).expect(
            "Entity with Collider has no transform. Any collider should also have a transform.",
        );

        // add projected_movement to absolute location to get projected absolute location. then
        // translate to tile space.
        let projected_tile_location = predict_location(
            velocities.0,
            velocities.1,
            transform.translation(),
            time.delta_seconds(),
            &tile_stretch,
        );

        // if collider is more than 0x0x0, draw out from there.
        for x in range_to_n(collider.size.x) {
            for y in range_to_n(collider.size.x) {
                for z in range_to_n(collider.size.x) {
                    let inhabiting = projected_tile_location + IVec3::new(x, y, z);

                    let tile_space_translation =
                        tile_stretch.bevy_translation_to_tile(&transform.translation());

                    trace!(
                        "pushing inhabiting with real location of {}, predicted movement of {}",
                        transform.translation(),
                        projected_tile_location - tile_space_translation
                    );

                    if let Some(inhabited_vec) = inhabited_tiles.get_mut(&inhabiting) {
                        inhabited_vec.push(entity);
                    } else {
                        inhabited_tiles.insert_unique_unchecked(inhabiting, vec![entity]);
                    }
                }
            }
        }
    }

    /// collision, based on the location of the collision
    struct CollisionOnLocation {
        location: IVec3,
        entities: Vec<Entity>,
    }

    // next, iterate through all tiles that will be inhabited and check if a collision will take
    // place on that tile.
    let collision_events = inhabited_tiles.iter().filter_map(|(location, entities)| {
        trace!("checking entities that will be in location {}", location);
        if entities.len() > 1 {
            trace!(
                "collision of {} entities on tile {}",
                entities.len(),
                location
            );

            Some(CollisionOnLocation {
                location: *location,
                entities: entities.clone(),
            })
        } else {
            None
        }
    });

    enum ViolatedPlane {
        None,
        Neg,
        Pos,
    }

    // all of the immutable data associated with an entity involved in a collision.
    struct CollidingEntity<'a, 'b> {
        entity: Entity,
        collider: &'a Collider,
        location: IVec3,
        ticker: Option<&'a Ticker>,
        name: Option<&'b str>,
    }

    let updated_entities: HashSet<Entity> = HashSet::new();

    for collision in collision_events {
        let colliding_entities: Vec<CollidingEntity> = collision
            .entities
            .iter()
            .map(|e| CollidingEntity {
                entity: *e,
                collider: collider_q.get(*e).expect("Collision with no collider? Make sure to update any colliders after PhysicsSet::FinalizeCollision.").1,
                location: tile_stretch.bevy_translation_to_tile(
                    &transform_query
                        .get(*e)
                        .expect("Collider with no transform. Add a transform to any entity with a collider")
                        .translation(),
                ),
                ticker: ticker_q.get(*e).ok(),
                name: name_q.get(*e).ok().map(|name| &*name.0),
            })
            .collect();

        struct Resolution {
            entity: Entity,
            requested_new_velocity: Vec3,
        };

        let mut queud_resolutions: Vec<Resolution> = Vec::new();

        // colliding_entities.sort_by_key(|e| e.entity);
        todo!();
    }
}

pub(super) struct Plugin();

impl bevy::prelude::Plugin for Plugin {
    fn build(&self, app: &mut App) {
        app.add_system(
            check_and_resolve_collisions
                .in_set(PhysicsSet::FinalizeCollision)
                .after(PhysicsSet::FinalizeVelocity)
                .before(PhysicsSet::FinalizeMovement),
        );
    }
}
