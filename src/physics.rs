//! A tile-based, realtime Physics Engine for this project
//!
//! See [`PhysicsPlugin`], and its build function to get started with the source code, or you can
//! likely read the file from top-down and understand it decently well.

use std::{assert_eq, todo};

use bevy::{prelude::*, utils::HashMap};

use crate::{controllers, tile_objects::TileStretch};

/// The gravity constant used for weight velocity gain
pub const GRAVITY: f32 = 9.8;

/// a tile collider, specified in tile space. Importantly, size essentially functions as another
/// corner of the Collider's "box", so a size of (0,0,0) should inhabit a single tile.
#[derive(Component, Debug)]
pub struct Collider {
    pub size: IVec3,
    collision_type: ColliderType,
    going_to_collide_with: Vec<Entity>,
}

impl Collider {
    pub fn new(size: IVec3, collision_type: ColliderType) -> Self {
        Self {
            size,
            collision_type,
            going_to_collide_with: Vec::new(),
        }
    }
}

#[derive(SystemSet, Hash, Debug, Clone, Eq, PartialEq)]
/// physics system sets
/// register all velocity wants for the current frame before [`PhysicsSet::FinalizeVelocity`]
/// if wanting to use previously updated locations, run after [`PhysicsSet::FinalizeMovement`]
///
/// Currently, only FinalizeMovement is used by the Physics engine, and setting anything relative
/// to FinalizeVelocity or CollisionCheck will break things.
pub enum PhysicsSet {
    FinalizeVelocity,
    CollisionCheck,
    FinalizeMovement,
}

/// Both types of colliders will keep track of any collisions, but a Solid collider will attempt to
/// avoid moving into or having another collision enabled entity move into it, while a sensor will
/// simply keep track of any collisions.
#[derive(Debug, Clone, Default)]
pub enum ColliderType {
    #[default]
    Solid,
    Sensor,
}

/// Velocity for current frame relative to its parents velocity
///
/// If you want an object to "have" velocity, but only move with its parent, give it a Velocity
/// Bundle but no ticker
///
///  Relative Velocity should likely not be chnaged outside of the physics engine
#[derive(Debug, Component, Clone, Default, Deref)]
pub struct RelativeVelocity(Vec3);

/// RelativeVelocity + parent's TotalVelocity
///
/// TotalVelocity will = RelativeVelocity when an entity has no parents
///
/// All of an entity's parents must have a Velocity bundle in order for the entity to have one
#[derive(Debug, Component, Clone, Default, Deref)]
pub struct TotalVelocity(Vec3);

/// Any component with a weight will have gravity applied to it on each physics update
///
/// Any entity with a Weight will have a velocity of [`GRAVITY`] * weight added to its relative
/// velocity during calculation.
#[derive(Debug, Clone, Copy, Component, Deref, DerefMut)]
pub struct Weight(pub f32);

/// A mantained velocity over time. Will be decayed based on certain constants by the physics
/// engine
#[derive(Debug, Clone, Component, Default, Deref, DerefMut)]
pub struct MantainedVelocity(pub Vec3);

/// a tile velocity that is wiped after every update, for willfully moving characters, usually
/// through controllers
///
/// Each axis on the inner Vec3 represents the entities requested speed in that direction, similar
/// to a force diagram.
///
/// As valid movement is different for each entity, The physics engine does not check for "invalid" movement goals,
/// so it is the responsibility of  whoever is controlling an entity to make sure movement goals are valid before setting them.
#[derive(Debug, Component, Clone, Default, Deref, DerefMut)]
pub struct MovementGoal(pub Vec3);

/// A Velocity Ticker, used to keep track of when to actually move a physics component by
/// buffering velocity into its ticker until at least a whole tile has been moved.
///
/// This makes it so that velocities of less than 1 tile per second can be represented in the
/// engine in real time.
///
/// Currently if a component has 0 velocity, its ticker will be reset to 0,0,0. In the future this
/// should be changed so that you can reset your ticker trough a request like RequestResetTicker.
///
/// As this Ticker is meant to be wholely managed by the physics engine, it is not public, and must
/// be instantiated trough a Bundle like [`PhysicsComponentBase`]
#[derive(Debug, Component, Clone, Copy, Default, Deref, DerefMut)]
struct VelocityTicker(Vec3);

/// clears all potential collisions
fn clear_collisions(mut collider_q: Query<&mut Collider>) {
    collider_q
        .par_iter_mut()
        .for_each_mut(|mut c| c.going_to_collide_with = Vec::new());
}

/// This function performs collision checking on any entity with a TotalVelocity, GlobalTransform,
/// and collider, and then updates that onto the collider.
///
/// It works by predicting where the entity will be, and then finding any other entities that will
/// be in that same place.
///
/// All this system does is update the Colliders' list of who they'll collide with, which will then
/// be used by other systems to do things like avoid collision
fn perform_collision_checks(
    mut collider_query: Query<(Entity, &mut Collider)>,
    velocity_query: Query<(&TotalVelocity, Option<&VelocityTicker>)>,
    transform_query: Query<&GlobalTransform>,
    tile_stretch: Res<TileStretch>,
    time: Res<Time>,
) {
    /// gets a range to n..=0 that will start at n if n is negative, or start at zero otherwise
    /// so you will always get the same amount of steps regardless of n's sign
    fn range_to_n(n: i32) -> std::ops::Range<i32> {
        if n.is_negative() {
            n..0
        } else {
            0..n
        }
    }

    // this will keep track of any tiles that will be inhabited, as well as which colliders will be
    // in that tile
    let mut inhabited_tiles: HashMap<IVec3, Vec<Entity>> = HashMap::new();

    // could loop concurrently to create a Vec of expected tiles, and then loop that in single
    // thread to populate inhabited hashmap?
    let collected_entities = collider_query.iter().for_each(|(entity, collider)| {
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

        /// assert actual translation is only whole numbers, so that projection to tilespace will
        /// be correct
        assert!(transform.translation() == transform.translation().floor());

        // add projected_movement to absolute location to get projected absolute location. then
        // translate to tile space.
        let projected_tile_location =
            tile_stretch.bevy_translation_to_tile(&(transform.translation() + projected_movement));

        /// if collider is more than 0x0x0, draw out from there.
        for x in range_to_n(collider.size.x) {
            for y in range_to_n(collider.size.x) {
                for z in range_to_n(collider.size.x) {
                    let inhabiting = projected_tile_location + IVec3::new(x, y, z);
                    if let Some(mut inhabited_vec) = inhabited_tiles.get_mut(&inhabiting) {
                        inhabited_vec.push(entity);
                    } else {
                        inhabited_tiles.insert_unique_unchecked(inhabiting, vec![entity]);
                    }
                }
            }
        }
    });

    // next, iterate through all tiles that will be inhabited and check if a collision will take
    // place on that tile
    for (location, entities) in inhabited_tiles.iter() {
        if entities.len() > 1 {
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

/// Takes all factors that could affect a physics component's velocity on each frame and then
/// calculates a "total velocity" as a function of all of these factors
///
/// This does not move any components, nor update their ticker
///
/// This should wait until movement finalization to multiply by delta time.
fn calculate_relative_velocity(
    mut commands: Commands,
    mut phsyics_components: Query<(
        &mut RelativeVelocity,
        Option<&MovementGoal>,
        Option<&Weight>,
        Option<&MantainedVelocity>,
    )>,
    // time: Res<Time>,
) {
    // let delta_time = time.delta().as_secs_f32();

    for component in phsyics_components.iter_mut() {
        let mut new_total_velocity = Vec3::splat(0.);

        let (mut total_velocity, movement_goal, weight, mantained) = component;

        // it is up to the controller to ensure that the movement goal is reasonable
        if let Some(movement_goal) = movement_goal {
            new_total_velocity += movement_goal.0;
        }

        // maybe gravity should be part of mantained velocity
        if let Some(weight) = weight {
            new_total_velocity.z -= weight.0 * GRAVITY;
        }

        if let Some(mantained) = mantained {
            new_total_velocity += mantained.0;
        }

        total_velocity.0 = new_total_velocity;
    }
}

/// Finaly, applies any tickers that have moved at least one tile. This is essentially flushing the
/// VelocityTicker buffer.
///
/// This will reset any tickers with a totalVelocity of 0 to 0,0,0. This may lead to bugs in the
/// future
///
/// delta time should be applied here?
fn apply_total_movement(
    mut phsyics_components: Query<(&mut Transform, &mut VelocityTicker, &RelativeVelocity)>,
    tile_stretch: Res<TileStretch>,
    time: Res<Time>,
) {
    // this will make it so entities only move a tile once an entire tiles worth of movement
    // has been "made", keeping it in a grid based system
    //
    // also converts to 32x32

    for (mut transform, mut ticker, total_velocity) in phsyics_components.iter_mut() {
        // update ticker, only apply velocity * delta to keep time consistent
        ticker.0 += total_velocity.0 * time.delta_seconds();

        debug!("updating with ticker {}", ticker.0);

        while ticker.0.z.abs() >= 1. {
            transform.translation.z += ticker.0.z.signum();
            ticker.0.z -= 1. * ticker.0.z.signum();
        }
        while ticker.0.y.abs() >= 1. {
            transform.translation.y += tile_stretch.1 as f32 * ticker.0.y.signum();
            ticker.0.y -= 1. * ticker.0.y.signum();
        }
        while ticker.0.x.abs() >= 1. {
            transform.translation.x += tile_stretch.0 as f32 * ticker.0.x.signum();
            ticker.0.x -= 1. * ticker.0.x.signum();
        }

        // this might break things in the future!
        // if total_veolicity is 0 reset ticker to 0
        // this probably does not belong in this system. maybe in its own system?
        if total_velocity.0 == Vec3::ZERO {
            ticker.0 = Vec3::ZERO;
        }
    }
}

/// This function decays any persistent velocities.
///
/// It needs a rework.
fn decay_persistent_velocity(mut velocity: Query<&mut MantainedVelocity>) {
    const DECAY_CONST: f32 = 0.1;
    // todo: use signs

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

/// Propogate velocities down from an entities parents so that its Total and Relative Velocity remains accurate
///
/// needs parent total and child relative along with child total
///
/// This is lifted from the bevy source code, which is dual-licensed under the Apache 2.0, and MIT
/// license. see <https://github.com/bevyengine/bevy/LICENSE-APACHE> or <./../credits/> for more details.
fn propogate_velocities(
    mut root_query: Query<
        (Entity, &Children, Ref<RelativeVelocity>, &mut TotalVelocity),
        Without<Parent>,
    >,
    velocity_query: Query<
        (Ref<RelativeVelocity>, &mut TotalVelocity, Option<&Children>),
        With<Parent>,
    >,
    parent_query: Query<(Entity, Ref<Parent>)>,
) {
    trace!("starting velocity propagataion");

    // TODO: par iter
    root_query
        .par_iter_mut()
        .for_each_mut(|(entity, children, relative, mut total)| {
            let changed = relative.is_changed();
            if changed {
                total.0 = relative.0;
            }

            for (child, actual_parent) in parent_query.iter_many(children) {
                assert_eq!(actual_parent.get(), entity, "Bad hierarchy");
                propagate_recursive(
                    &total,
                    &velocity_query,
                    &parent_query,
                    child,
                    changed || actual_parent.is_changed(),
                );
            }
        });
}

/// This is lifted from the bevy source code, which is dual-licensed under the Apache 2.0, and MIT
/// license. see <https://github.com/bevyengine/bevy/LICENSE-APACHE> or <./../credits/> for more details.
fn propagate_recursive(
    parent_total: &TotalVelocity,
    velocity_query: &Query<
        (Ref<RelativeVelocity>, &mut TotalVelocity, Option<&Children>),
        With<Parent>,
    >,
    parent_query: &Query<(Entity, Ref<Parent>)>,
    entity: Entity,
    mut changed: bool,
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

        changed |= relative.is_changed();
        if changed {
            total.0 = parent_total.0 + relative.0;
        }
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
                &global_matrix,
                velocity_query,
                parent_query,
                child,
                changed || actual_parent.is_changed(),
            );
        }
    }
}

/// The components necessary for movement by the physics engine to take place on an entity's
/// transform.
///
/// You must provide a transform yourself in order to get movement, in order to stay compatible
/// with other bundles.
///
/// TODO: consider clearing ticker even if not attached to a Transform
#[derive(Bundle, Debug, Default)]
pub struct PhysicsComponentBase {
    ticker: VelocityTicker,
    total_velocity: VelocityBundle,
}

#[derive(Debug, Bundle)]
pub struct PhysicsComponentFull {
    pub base: PhysicsComponentBase,
    pub weight: Weight,
    pub mantained_velocity: MantainedVelocity,
    pub collider: Collider,
}

/// Allows an entity to exist in the physics system with a velocity
/// Total velocity is the total velocity that the object should be moved, while relative velocity
/// is how it is moving relative to its parent in the object hierarchy
#[derive(Bundle, Debug, Default)]
pub struct VelocityBundle {
    total: RelativeVelocity,
    relative_total: TotalVelocity,
}

/// A plugin to setup essential physics systems
///
/// Any system that wants to use the results of a physics engine update should not run until after
/// [`PhysicsSet::FinalizeMovement`] has been completed
///
/// Any systems that want to affect the physics engine in a given frame must run before
/// [`PhysicsSet::FinalizeVelocity`].
///
/// See the source of [`PhysicsPlugin::build`] for how systems are ordered.
pub struct PhysicsPlugin;

impl Plugin for PhysicsPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            (
                calculate_relative_velocity,
                propogate_velocities.after(calculate_relative_velocity),
                decay_persistent_velocity.after(calculate_relative_velocity),
            )
                .in_set(PhysicsSet::FinalizeVelocity),
        )
        .add_system(
            clear_collisions
                .before(perform_collision_checks)
                .in_set(PhysicsSet::CollisionCheck),
        )
        .add_system(
            perform_collision_checks
                .after(propogate_velocities)
                .in_set(PhysicsSet::CollisionCheck),
        )
        .add_system(
            apply_total_movement
                .in_set(PhysicsSet::FinalizeMovement)
                .after(PhysicsSet::CollisionCheck),
        );
    }
}
