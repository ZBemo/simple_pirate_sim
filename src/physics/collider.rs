//! Colliders and Collision systems
#![allow(unused)]

use std::{assert_eq, assert_ne, hint::unreachable_unchecked};

use bevy::{
    prelude::*,
    utils::{HashMap, HashSet},
};

use crate::tile_objects::{ObjectName, TileStretch};

use super::{TotalVelocity, VelocityTicker};

// an entity will be added to a collision event if it is on the same tile as another entity's
// collider, regardless of the event
#[derive(Debug, Clone)]
pub struct Collision {
    location: IVec3,
    entities: Vec<Entity>,
}

/// If a collider is sodid across the negative axis, positive, neither, or both
///
/// represent 1 or 2 2d planes along an axis, so that a Pos plane is essentially a line along the
/// border moving down from a tile onto the collider, and vice-versa. When combined with three
/// other AxisPlanes, this allows you to construct A Box with any number of sides cut out of it.
///
/// having Axis be a `repr(u8)` makes certain math easier.
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
    pub solid_axes: PVec3, //TODO
    /// which axes it can be pushed along in order to resolve collision
    pub move_along: PVec3,
}

impl Constraints {
    pub const BOX: Self = Self {
        solid_axes: PVec3::ALL,
        move_along: PVec3::NONE,
    };
    pub const FLOOR: Self = Self {
        solid_axes: PVec3 {
            x: AxisPlanes::None,
            y: AxisPlanes::None,
            z: AxisPlanes::Neg,
        },
        move_along: PVec3::NONE,
    };
    pub const ENTITY: Self = Self {
        solid_axes: PVec3 {
            x: AxisPlanes::None,
            y: AxisPlanes::None,
            z: AxisPlanes::Neg,
        },
        move_along: PVec3::ALL,
    };

    pub const SENSOR: Self = Self {
        solid_axes: PVec3::NONE,
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

/// clears all potential collisions
pub(super) fn update_collision_lists(mut collider_q: Query<&mut Collider>) {
    todo!()
}

fn predict_location(
    total_vel: Option<&TotalVelocity>,
    ticked_vel: Option<&VelocityTicker>,
    current_location: Vec3,
    time_delta: f32,
    tile_stretch: &TileStretch,
) -> IVec3 {
    // if either of these are not present assume they will not move it
    let total_velocity = total_vel.map_or_else(|| Vec3::ZERO, |c| **c);
    let ticked_velocity = ticked_vel.map_or_else(|| Vec3::ZERO, |c| **c);

    // its projected movement will just be however much the ticker is already filled, along
    // with its total velocity times the time delta to get how much it will move this frame
    //

    let projected_movement_raw = (total_velocity * time_delta + ticked_velocity);

    // make sure to round towards zero
    let projected_movement_rounded = (projected_movement_raw * projected_movement_raw.signum())
        .floor()
        * projected_movement_raw.signum();

    // todo!("LIKELY BUG HERE"); // we're treating either current_location or projected_movement
    // through the wrong tilespace

    // add projected_movement to absolute location to get projected absolute location. then
    // translate to tile space.

    (tile_stretch.bevy_translation_to_tile(&current_location)
        + projected_movement_rounded.as_ivec3())
}

/// This function performs collision checking on any entity with a TotalVelocity, GlobalTransform,
/// and collider, and then updates that onto the collider.
///
/// It works by predicting where the entity will be, and then finding any other entities that will
/// be in that same place.
///
/// All this system does is update the Colliders' list of who they'll collide with, which will then
/// be used by other systems to do things like avoid collision
pub(super) fn check_collisions(
    mut collider_query: Query<(Entity, &mut Collider)>,
    velocity_query: Query<(&TotalVelocity, Option<&VelocityTicker>)>,
    transform_query: Query<&GlobalTransform>,
    tile_stretch: Res<TileStretch>,
    mut collision_writer: EventWriter<Collision>,
    time: Res<Time>,
) {
    trace!("starting collision check");

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
    for (entity, collider) in collider_query.iter() {
        // start off by getting any velocities, the absolute transform of the entity, and its
        // collider and entity id

        let velocities = velocity_query.get(entity).ok();
        let transform = transform_query.get(entity).expect(
            "Entity with Collider has no transform. Any collider should also have a transform.",
        );

        // add projected_movement to absolute location to get projected absolute location. then
        // translate to tile space.
        let projected_tile_location = predict_location(
            velocities.map(|c| c.0),
            velocities.and_then(|c| c.1),
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

    // next, iterate through all tiles that will be inhabited and check if a collision will take
    // place on that tile
    for (location, entities) in inhabited_tiles.iter() {
        trace!("checking entities that will be in location {}", location);
        if entities.len() > 1 {
            trace!(
                "collision of {} entities on tile {}",
                entities.len(),
                location
            );

            let event = Collision {
                location: location.clone(),
                entities: entities.clone(),
            };

            collision_writer.send(event);
        }
    }
}

pub(super) fn resolve_collisions(
    mut collision_events: EventReader<Collision>,
    collider_q: Query<(Entity, &Collider)>,
    mut velocity_q: Query<(Option<&mut TotalVelocity>, Option<&VelocityTicker>)>,
    transform_query: Query<&GlobalTransform>,
    name_q: Query<&ObjectName>,
    tile_stretch: Res<TileStretch>,
) {
    enum ViolatedPlane {
        None,
        Neg,
        Pos,
    }

    let updated_entities: HashSet<Entity> = HashSet::new();

    for collision in collision_events.iter() {

        // query velocities, colliders, transform for each involved entity
    }
}

// TODO: collision plugin?
