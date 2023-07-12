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

use super::{
    movement::Ticker,
    tile_cast::tile_cast,
    velocity::{RelativeVelocity, TotalVelocity},
    PhysicsSet,
};
use crate::tile_grid::{TileStretch, GetTileLocation,};

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
                .cloned()
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

// function to check for collisions. updates collision map in place
fn check_collisions(
    collider_q: &Query<(Entity, &Collider)>,
    transform_q: &Query<&GlobalTransform>,
    velocity_q: &Query<&super::velocity::TotalVelocity>,
    ticker_q: &Query<&super::movement::Ticker>,
    name_q: &Query<&Name>,
    tile_stretch: TileStretch,
    delta_time: f32,
) -> HashMap<IVec3, Vec<InhabitingTile>> {
    // TODO: look into tracking average amount of units w/ colliders. Reserve capacity for that
    // much
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
            projected_tile_location - tile_stretch.get_closest(&transform.translation())
        );

        // TODO: this might need error handling
        let tile = InhabitingTile {
            entity,
            predicted_movement: projected_tile_location
                - tile_stretch.get_closest(&transform.translation()),
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
unsafe fn find_and_resolve_conflicts(
    collisions: &HashMap<IVec3, Vec<InhabitingTile>>,
    collider_q: &Query<(Entity, &Collider)>,
    // this should be separated into another function to
    // keep this one functionally pure
) -> Vec<(ConflictInfo, EntityCollision)> {
    trace!("Finding conflicts & resolutions");

    #[cfg(all())]
    return Vec::new();

    #[cfg(any())]
    {
        // start by mapping each possible movement violation to any entities that would have their collider
        // constraints violated
        #[derive(Debug, Default)]
        struct ViolatablePlanes {
            x: Vec<Entity>,
            y: Vec<Entity>,
            z: Vec<Entity>,
        }

        collisions
            .into_iter()
            .filter(|v| v.1.len() > 1)
            .flat_map(move |(position, inhabitants)| {
                // empty
                let mut planes = ViolatablePlanes::default();

                let collision_map = inhabitants;

                for entity in collision_map.iter() {
                    // SAFETY: any entity involved in a collision must have a collider
                    // thus, we've already guaranteed that this entity has a collider associated
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
                            unreachable!();
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
                            unreachable!();
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
                            unreachable!();
                        }
                    }

                    let info = ConflictInfo {
                        entity: entity.entity,
                        to_block: current_resolution,
                        position: *position,
                        constraints: entity.constraints.clone(),
                    };

                    let event = EntityCollision::new(
                        &info,
                        &inhabitants.iter().map(|t| t.entity).collect::<Box<_>>(),
                    );

                    (info, event)
                })
            })
            .collect()
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
        *tile_stretch,
        delta_time,
    );

    // SAFETY: inhabited_tiles is "filtered" from a list of (Entity,&Collider), so we know that it
    // will have a collider associated with itself
    let resolutions = unsafe { find_and_resolve_conflicts(&inhabited_tiles, &collider_q) };

    trace!("implementing resolutions");
    for (resolution, event) in resolutions {
        writer.send(event);

        if let Some(mut rel_vel) = rel_velocity_q.get_mut(resolution.entity).ok() {
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
        )
    }
}

pub(super) struct Plugin;

impl bevy::prelude::Plugin for Plugin {
    fn build(&self, app: &mut App) {
        app.add_system(
            check_and_resolve_collisions
                .in_set(PhysicsSet::Collision)
                .after(PhysicsSet::Velocity)
                .before(PhysicsSet::Movement),
        )
        .add_system(log_collisions.after(PhysicsSet::Collision))
        .add_system(check_and_resolve_collisions.after(PhysicsSet::Collision))
        .add_event::<EntityCollision>();
    }
}

#[cfg(test)]
mod test {
    use bevy::{
        prelude::{App, Events, GlobalTransform, Name, Vec3},
        time::Time,
        transform::TransformBundle,
    };

    use crate::{
        physics::{movement::MovementBundle, MovementGoal},
        test,
    };

    use super::Collider;

    #[test]
    /// collision should work under super basic conditions
    fn collision_works_basic() {
        let mut app = App::new();

        app.add_plugin(test::DefaultTestPlugin);

        app.add_plugin(crate::physics::PhysicsPlugin);

        let move_id = app
            .world
            .spawn((
                Name::new("Move"),
                MovementBundle::default(),
                Collider::new(super::Constraints::WALL),
                TransformBundle::from_transform(bevy::prelude::Transform::from_xyz(0., 0., 0.)),
                MovementGoal(Vec3::new(1., 1., 0.)),
            ))
            .id();

        let wall_id = app
            .world
            .spawn((
                Name::new("Wall"),
                Collider::new(super::Constraints::WALL),
                TransformBundle::from_transform(bevy::prelude::Transform::from_xyz(2., 2., 0.)),
            ))
            .id();

        app.setup();

        // run long enough for Move to move x + 2, y +2
        while app
            .world
            .resource::<Events<super::EntityCollision>>()
            .is_empty()
        {
            app.update();

            if app.world.resource::<Time>().elapsed_seconds() >= 3. {
                panic!("Three seconds elapsed but no collision detected");
            }
        }

        let collisions = app.world.resource::<Events<super::EntityCollision>>();
        let mut reader = collisions.get_reader();

        assert!(!reader.is_empty(collisions));

        let collisions = reader.iter(collisions).collect::<Vec<_>>();

        assert_eq!(collisions.len(), 2);

        let translation = |id| app.world.get::<GlobalTransform>(id).unwrap().translation();

        assert_ne!(translation(move_id), translation(wall_id));
    }

    #[test]
    #[ignore = "Re-enable once tilecast based collisin implemented."]
    /// collision should work under super basic conditions
    fn collision_works_skips() {
        let mut app = App::new();

        app.add_plugin(test::DefaultTestPlugin);

        app.add_plugin(crate::physics::PhysicsPlugin);

        let move_id = app
            .world
            .spawn((
                Name::new("Move"),
                MovementBundle::default(),
                Collider::new(super::Constraints::WALL),
                TransformBundle::from_transform(bevy::prelude::Transform::from_xyz(0., 0., 0.)),
                MovementGoal(Vec3::new(1., 1., 0.)),
            ))
            .id();

        let wall_id = app
            .world
            .spawn((
                Name::new("Wall"),
                Collider::new(super::Constraints::WALL),
                TransformBundle::from_transform(bevy::prelude::Transform::from_xyz(2., 2., 0.)),
            ))
            .id();

        app.setup();

        app.world
            .resource_mut::<Time>()
            .set_relative_speed_f64(1000.);

        // run long enough for Move to move x + 2, y +2
        while app
            .world
            .resource::<Events<super::EntityCollision>>()
            .is_empty()
        {
            app.update();

            if app.world.resource::<Time>().elapsed_seconds() >= 3. {
                panic!("Three seconds (Time::elapsed_seconds) elapsed but no collision detected");
            }
        }

        let collisions = app.world.resource::<Events<super::EntityCollision>>();
        let mut reader = collisions.get_reader();

        assert!(!reader.is_empty(collisions));

        let collisions = reader.iter(collisions).collect::<Vec<_>>();

        assert_eq!(collisions.len(), 2);

        let translation = |id| app.world.get::<GlobalTransform>(id).unwrap().translation();

        assert_ne!(translation(move_id), translation(wall_id));
    }
}

fn tile_cast_collision(
    collider_q: Query<(Entity, &Collider)>,
    mut total_vel_q: Query<&mut TotalVelocity>,
    mut relative_vel_q: Query<&mut RelativeVelocity>,
    ticker_q: Query<&Ticker>,
    name_q: Query<&Name>,
    transform_q: Query<&GlobalTransform>,
    time: Res<Time>,
    tile_stretch: Res<TileStretch>,
) {
    let delta_time = time.delta_seconds();

    let predicted_map = predict_locations(
        &collider_q,
        &total_vel_q,
        &ticker_q,
        &name_q,
        &transform_q,
        delta_time,
        &tile_stretch,
    );
    // we need a vec (Vec3,)

    predicted_map.iter().for_each(|&(entity, predicted_translation)| {
        // send out event here
        if let Some(mut vel) = total_vel_q.get_mut(entity).ok() {
            let translation =
            transform_q
                .get(entity)
                .expect("Entity with collider but no transform")
                .location(*tile_stretch);

            let hit_entities = tile_cast(
                translation,
                **vel,
                *tile_stretch,
                predicted_map.iter(),
                false,
            );

            // for sorting; we know that an entity will either be on the same tile, or not the same
            // tile. 

            let total_vel_signs =  (predicted_translation - translation).signum();
            let closest_entities = hit_entities.iter().fold(None, |acc,elem| {
                match acc {
                    None => Some(vec![elem]),
                    Some(mut acc) => {

                        // vec should always have len > 0
                        let acc_t = acc[0].1.as_vec3().distance(translation.as_vec3());
                        let elem_t = elem.1.as_vec3().distance(translation.as_vec3());

                        // does this skip elements?
                        if elem_t == acc_t {
                            acc.push(elem);
                            Some(acc)
                        } else if elem_t < acc_t {
                            Some(vec![elem])
                        } else {
                            Some(acc)
                        }
                    }
                }
            });

            

            // .0 is negative plane, .1 is positive
            let all_solid_axes = closest_entities.into_iter().flatten().fold((BVec3::FALSE,BVec3::FALSE),|acc,elem|{
                let constraints = unsafe { collider_q.get(elem.0).unwrap_unchecked() }.1.constraints;
                
                (acc.0 | constraints.neg_solid_planes, acc.0 | constraints.pos_solid_planes)
            });

            if total_vel_signs.x == 1 && all_solid_axes.0.x {
                // remove velocity here
                todo!()
            } else if total_vel_signs.x == -1 && all_solid_axes.1.x {
                todo!()
            }

            // solve from all_solid_axes

            todo!("Find closest entity that it will hit, and only have a collision if that entity is going to be moved into next frame");

            // raycast out with predicted_map
        }
    });
}

/// Predict the change in grid location of an entity based on its current velocities. This will only be accurate
/// in between [`PhysicsSet::Velocity`] and [`PhysicsSet::Movement`] \(ie. during
/// [`PhysicsSet::Collision`])
fn calc_movement(
    total_vel: Option<&TotalVelocity>,
    ticked_vel: Option<&Ticker>,
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

    // the projected movement is already in tilespace & rounded, so just cast
    projected_movement_rounded.as_ivec3()
}

/// return a hashmap of (predicted location)->(Entity,Constraints)
fn predict_locations(
    collider_q: &Query<(Entity, &Collider)>,
    total_vel_q: &Query<&mut TotalVelocity>,
    ticker_q: &Query<&Ticker>,
    name_q: &Query<&Name>,
    transform_q: &Query<&GlobalTransform>,
    delta_time: f32,
    tile_stretch: &TileStretch,
) -> Vec<(Entity, IVec3)> {
    collider_q
        .into_iter()
        .map(|(entity, _)| {
            let predicted_location = calc_movement(
                total_vel_q.get(entity).ok(),
                ticker_q.get(entity).ok(),
                delta_time,
                tile_stretch,
                name_q
                    .get(entity)
                    .map_or_else(|_| "Unnamed Entity", |s| s.as_str()),
            ) + tile_stretch.get_closest(
                &transform_q
                    .get(entity)
                    .expect("Collider on entity with no transform")
                    .translation(),
            );

            (entity, predicted_location)
        })
        .collect()
}
