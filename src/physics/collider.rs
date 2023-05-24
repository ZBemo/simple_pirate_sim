//! Colliders and Collision systems
#![allow(unused)]

use std::{assert_eq, assert_ne, hint::unreachable_unchecked};

use bevy::{prelude::*, utils::HashMap};

use crate::tile_objects::{ObjectName, TileStretch};

use super::{TotalVelocity, VelocityTicker};

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
    going_to_collide_with: Vec<Entity>,
    was_colliding_with: Vec<Entity>,
}

impl Collider {
    pub fn new(size: IVec3, constraints: Constraints) -> Self {
        Self {
            size,
            constraints,
            going_to_collide_with: Vec::new(),
            was_colliding_with: Vec::new(),
        }
    }

    /// Returns true if the collider was projected to collide with
    pub fn is_colliding(&self) -> bool {
        !self.going_to_collide_with.is_empty()
    }

    /// get all entities that the collider was projected to collide with
    pub fn get_colliding_with(&self) -> &[Entity] {
        &self.going_to_collide_with
    }

    pub fn get_was_colliding_with(&self) -> &[Entity] {
        &self.was_colliding_with
    }

    pub fn was_colliding_with(&self, e: &Entity) -> bool {
        self.was_colliding_with.contains(e)
    }

    /// returns true if the collider was projected to collide with e
    pub fn is_colliding_with(&self, e: &Entity) -> bool {
        self.going_to_collide_with.contains(e)
    }

    /// clear old collision list, then swap it with new collision list
    ///
    /// there should be some way to do this with [`std::mem::swap`] or something similar
    fn update_own_lists(&mut self) {
        self.going_to_collide_with = self.was_colliding_with.clone();
        self.was_colliding_with = Vec::new();
    }
}

/// clears all potential collisions
pub(super) fn update_collision_lists(mut collider_q: Query<&mut Collider>) {
    trace!("zeroing collisions");
    collider_q.par_iter_mut().for_each_mut(|mut c| {
        // clear was_colliding with. irrelevant
        c.update_own_lists();
    });
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

            // there is a collision
            // loop through entitites and update their colliding list

            for colliding_entity in entities.iter() {
                // for each colliding entity, update it with a list of collisions. Make sure it's
                // deduplicated, and doesn't include the actual entity.
                //
                // it might be prudent to do this lazily through an element implemented onto
                // Collider

                // safety: we know this search willl be succesful because this entity was
                // originally pulled from the collider query, then filtered down
                let mut collider =
                    unsafe { collider_query.get_mut(*colliding_entity).unwrap_unchecked() };

                // notify the entity that it is going to collide with all entities in entities

                let mut not_current_entity = entities.clone();

                // safety we know any searches for colliding_entity will be succesful because
                // colliding_entity exists in the vec that we just cloned.
                not_current_entity.remove(unsafe {
                    not_current_entity
                        .iter()
                        .position(|e| e == colliding_entity)
                        .unwrap_unchecked()
                });

                not_current_entity.dedup();

                collider.1.going_to_collide_with = not_current_entity;
            }
        }
    }
}

pub(super) fn resolve_collisions(
    collider_q: Query<(Entity, &Collider)>,
    mut velocity_q: Query<(Option<&mut TotalVelocity>, Option<&VelocityTicker>)>,
    transform_query: Query<&GlobalTransform>,
    name_q: Query<&ObjectName>,
    tile_stretch: Res<TileStretch>,
    time: Res<Time>,
) {
    struct Conflict {
        entities: (Entity, Entity),
        // can re-calculate passed axes
        violated_planes: (
            (ViolatedPlane, ViolatedPlane, ViolatedPlane),
            (ViolatedPlane, ViolatedPlane, ViolatedPlane),
        ),
    }

    enum ViolatedPlane {
        None,
        Neg,
        Pos,
    }

    let mut conflicts: Vec<Conflict> = Vec::new();

    // we want to build a map of actual conflicts, then iterate through in a deterministic
    // way and resolve them.

    // to parrelelise, collect as Vec<Option<Entity,Entity>> and filter that
    // find any conflicting collisons
    for (collider_entity, collider) in collider_q.iter() {
        // check c constraints

        // there is no chance of collision. do nothing
        if !collider.constraints.solid_axes.any_planes() {
            break;
        };

        for colliding_entity in collider.going_to_collide_with.iter() {
            assert_ne!(
                collider_entity, *colliding_entity,
                "Entity claims to be colliding with itself!"
            );

            // query all necessary variables to resolve.
            //
            // none of these should be incorrect. could unwrap_unchecked
            let colliding_entity_v = velocity_q
                .get(*colliding_entity)
                .expect("couldn't get an option? could probably unwrap_unchecked this");

            let colliding_collider = if let Ok(c) = collider_q.get(*colliding_entity) {
                c
            } else {
                warn!("collider dissapeared in between check_collisions and resolve_collisions!?");
                break;
            };

            let colliding_location = transform_query
                .get(*colliding_entity)
                .expect("Any collision enabled entity should have a transfoorm")
                .translation();

            let collider_entity_v = velocity_q
                .get(collider_entity)
                .expect("couldn't get an option? could probably unwrap_unchecked this");
            let collider_location = transform_query
                .get(collider_entity)
                .expect("Any collision enabled entity should have a transfoorm")
                .translation();

            let delta_time = time.delta_seconds();

            // we should be able to safely assume they will have the same location by theend of
            // this frame if they are colliding
            let colliding_predicted = predict_location(
                colliding_entity_v.0,
                colliding_entity_v.1,
                colliding_location,
                delta_time,
                &tile_stretch,
            );

            assert_eq!(
                colliding_predicted,
                predict_location(
                    collider_entity_v.0,
                    collider_entity_v.1,
                    collider_location,
                    delta_time,
                    &tile_stretch,
                ),
                "Predicted collision of colliders who will be at two seperate tiles!"
            );

            // this is calculated wrong somehow. is incorrect when moving in a positive direction,
            // but not in a negative direction
            //
            // also does not take diagonals into account?
            //
            // might need to get distance from {x,y,z} -> {x,y,z} instead of just subtracting?
            //
            // It looks like for some reason entities moving positively will not be collision
            // checked until after they move?
            let difference_collider = colliding_predicted
                - tile_stretch
                    .bevy_translation_to_tile(&(collider_location * collider_location.signum()));
            let difference_colliding = colliding_predicted
                - tile_stretch
                    .bevy_translation_to_tile(&(colliding_location * collider_location.signum()));

            trace!("checking for conflict at {}", colliding_predicted,);

            trace!(
                "collider: dfference {}, location {}, name {}",
                difference_collider,
                tile_stretch.bevy_translation_to_tile(&collider_location),
                name_q.get(collider_entity).map_or_else(|_| "", |n| &*n.0)
            );

            trace!(
                "colliding: difference {}, location {}, name {}",
                difference_colliding,
                tile_stretch.bevy_translation_to_tile(&colliding_location),
                name_q.get(*colliding_entity).map_or_else(|_| "", |n| &*n.0)
            );
            // make it last message
            // panic!();

            // check if passing through constraints
            //
            // convert movement -> violated planes

            // find which planes the colliders movement will violate
            let collider_violated_planes = {
                let mut planes = (
                    ViolatedPlane::None, // x
                    ViolatedPlane::None, // y
                    ViolatedPlane::None, // z
                );

                if difference_collider.x < 0
                    && colliding_collider
                        .1
                        .constraints
                        .solid_axes
                        .x
                        .across_positive()
                {
                    planes.0 = ViolatedPlane::Pos;
                } else if difference_collider.x > 0
                    && colliding_collider
                        .1
                        .constraints
                        .solid_axes
                        .x
                        .accross_negative()
                {
                    // violated negative
                    planes.0 = ViolatedPlane::Neg;
                }

                if difference_collider.y < 0
                    && colliding_collider
                        .1
                        .constraints
                        .solid_axes
                        .y
                        .across_positive()
                {
                    planes.1 = ViolatedPlane::Pos;
                } else if difference_collider.y > 0
                    && colliding_collider
                        .1
                        .constraints
                        .solid_axes
                        .y
                        .accross_negative()
                {
                    // violated negative
                    planes.1 = ViolatedPlane::Neg;
                }

                if difference_collider.z < 0
                    && colliding_collider
                        .1
                        .constraints
                        .solid_axes
                        .z
                        .across_positive()
                {
                    planes.0 = ViolatedPlane::Pos;
                } else if difference_collider.x > 0
                    && colliding_collider
                        .1
                        .constraints
                        .solid_axes
                        .z
                        .accross_negative()
                {
                    // violated negative
                    planes.2 = ViolatedPlane::Neg;
                }
            };

            todo!();
            // maybe we should just resolve conflict here, conflict won't be picked up when second
            // instance of conflict is iterated through
        }

        trace!("Found {} collision conflicts", conflicts.len());

        if conflicts.is_empty() {
            return;
        }

        // sort and deduplicate any collisions between the same 2 entities.
        // TODO: this probably doesn't sort correctly for deduplication
        conflicts.sort_by_key(|e| e.entities);
        conflicts.dedup_by(|e_r, e_l| {
            // imagine e_r and e_l are the tuple below
            //  e_r (entity_r0, entity_r1)
            //  e_l (entity_l0, entity_l1),
            //  we know it is the same collision if
            //  r0 == l0 && r1 == l1
            //  || r0 == l1 && r1 == l0
            (e_r.entities.0 == e_l.entities.0) && (e_r.entities.1 == e_l.entities.1)
                || (e_r.entities.0 == e_l.entities.1) && (e_r.entities.1 == e_l.entities.0)
        });

        trace!("Found {} unique conflicts, resolving...", conflicts.len());

        todo!("Resolve collisions");
    }

    // dedup conflicts then sort by something and iterate through
}

// TODO: collision plugin?
