//! Colliders and Collision systems

use std::assert_ne;

use bevy::{
    prelude::*,
    utils::{HashMap, HashSet},
};

use crate::tile_objects::TileStretch;

use super::{TotalVelocity, VelocityTicker};

/// If a collider is sodid across the negative axis, positive, neither, or both
///
/// represent planes, with a plane going from across a whole side of
/// the axes, and preventing movement from +/-.
///
/// having Axis be a `repr(u8)` makes certain math easier.
#[repr(u8)]
#[derive(Debug, Clone, Copy, Default)]
pub enum Axis {
    #[default]
    None = 0,
    Pos = 0b01,
    Neg = 0b10,
    PosNeg = 0b11,
}

impl Axis {
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
pub struct AVec3 {
    pub x: Axis,
    pub y: Axis,
    pub z: Axis,
}

impl AVec3 {
    pub const ALL: Self = Self {
        x: Axis::PosNeg,
        y: Axis::PosNeg,
        z: Axis::PosNeg,
    };
    pub const NONE: Self = Self {
        x: Axis::None,
        y: Axis::None,
        z: Axis::None,
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
    pub solid_axes: AVec3, //TODO
    /// which axes it can be pushed along in order to resolve collision
    pub move_along: AVec3,
}

impl Constraints {
    pub const WALL: Self = Self {
        solid_axes: AVec3::ALL,
        move_along: AVec3::NONE,
    };
    pub const FLOOR: Self = Self {
        solid_axes: AVec3 {
            x: Axis::None,
            y: Axis::None,
            z: Axis::Neg,
        },
        move_along: AVec3::NONE,
    };
    pub const ENTITY: Self = Self {
        solid_axes: AVec3 {
            x: Axis::None,
            y: Axis::None,
            z: Axis::Neg,
        },
        move_along: AVec3::ALL,
    };

    pub const SENSOR: Self = Self {
        solid_axes: AVec3::NONE,
        move_along: AVec3::NONE,
    };
}

/// a tile collider, specified in tile space. Importantly, size essentially functions as another
/// corner of the Collider's "box", so a size of (0,0,0) should inhabit a single tile. See
/// constraints for more size granularity
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

        // if total_velocity or ticked_velocity are not found, assume the collider will not move,
        // so set it to 0
        let total_velocity;
        let ticked_velocity;

        if let Some(velocities) = velocities {
            total_velocity = velocities.0 .0;
            ticked_velocity = velocities.1.map_or_else(|| Vec3::ZERO, |v| v.0);
        } else {
            total_velocity = Vec3::ZERO;
            ticked_velocity = Vec3::ZERO;
        }

        // its projected movement will just be however much the ticker is already filled, along
        // with its total velocity times the time delta to get how much it will move this frame
        let projected_movement = (total_velocity * time.delta_seconds() + ticked_velocity).floor();

        // add projected_movement to absolute location to get projected absolute location. then
        // translate to tile space.
        let projected_tile_location =
            tile_stretch.bevy_translation_to_tile(&(transform.translation() + projected_movement));

        // if collider is more than 0x0x0, draw out from there.
        for x in range_to_n(collider.size.x) {
            for y in range_to_n(collider.size.x) {
                for z in range_to_n(collider.size.x) {
                    let inhabiting = projected_tile_location + IVec3::new(x, y, z);

                    trace!("pushing inhabiting of {}", inhabiting);

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
        trace!("checking location {}", location);
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
    mut velocity_q: Query<(&mut TotalVelocity, Option<&VelocityTicker>)>,
    transform_query: Query<&GlobalTransform>,
    tile_stretch: Res<TileStretch>,
    time: Res<Time>,
) {
    fn is_moving_diagonal(v: Vec3) -> BVec3 {
        let mut diagonals = BVec3::FALSE;

        if v.z == v.x {
            diagonals.z = true;
            diagonals.x = true;
        }
        if v.x == v.y {
            diagonals.y = true;
            diagonals.x = true
        }
        if v.y == v.z {
            diagonals.y = true;
            diagonals.z = true;
        }

        diagonals
    }

    let mut conflicts: Vec<(Entity, Entity)> = Vec::new();

    // we want to build a map of actual conflicts, then iterate through in a deterministic
    // way and resolve them.

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
            let mut colliding_entity_v = velocity_q.get_mut(*colliding_entity).unwrap();
            let colliding_collider = collider_q.get(*colliding_entity).unwrap();
            let mut collider_entity_v = velocity_q.get_mut(collider_entity).unwrap();

            // calculate velocity for both e
            //
            // figure out which planes each e needs to move through
            //
            // check if those planes are constrained
            // push to conflicts if so
        }
    }

    // dedup conflicts then sort by something and iterate through
}
