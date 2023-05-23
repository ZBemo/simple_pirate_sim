//! Colliders and Collision systems

use bevy::{prelude::*, utils::HashMap};

use crate::tile_objects::TileStretch;

use super::{TotalVelocity, VelocityTicker};

#[derive(Debug, Clone)]
pub struct Constraints {
    //TODO
}

/// a tile collider, specified in tile space. Importantly, size essentially functions as another
/// corner of the Collider's "box", so a size of (0,0,0) should inhabit a single tile.
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
        return !self.going_to_collide_with.is_empty();
    }

    /// get all entities that the collider was projected to collide with
    pub fn get_colliding_with(&self) -> &[Entity] {
        &self.going_to_collide_with
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
        // with its total velocity
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
