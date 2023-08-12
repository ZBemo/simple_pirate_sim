#![allow(clippy::unwrap_used)]

use bevy_app::prelude::*;
use bevy_core::Name;
use bevy_ecs::system::Query;
use bevy_hierarchy::BuildWorldChildren;
use bevy_math::prelude::*;
use bevy_time::Time;
use bevy_transform::prelude::*;

use crate::MovementGoal;
use crate::{movement::MovementBundle, tile_cast::tile_cast};

#[cfg(test)]
use pirate_sim_core::test_utils::DefaultTestPlugin;

use pirate_sim_core::tile_grid::TileStretch;

use super::collision::{Collider, Constraints};
use super::velocity::{RelativeVelocity, TotalVelocity, VelocityBundle};

#[test]
fn complex_tile_cast_works() {
    let entities: Vec<(usize, IVec3)> = [
        IVec3::new(0, 0, 0),
        IVec3::new(0, 2, 5),
        IVec3::new(0, 3, 7),
        IVec3::new(0, 4, 10),
        IVec3::new(0, 4, 8),
    ]
    .into_iter()
    .enumerate()
    .collect();

    let casted_entities = tile_cast(
        crate::tile_cast::Origin {
            tile: IVec3::new(0, 0, 0),
            ..Default::default()
        },
        Vec3::new(0., 1., 2.5),
        TileStretch(32, 32), // this shouldn't matter, but put this in to test it
        entities.into_iter(),
    )
    .collect::<Vec<_>>();

    for e in &casted_entities {
        println!("{} : {}", e.data, e.translation,);
    }

    assert_eq!(casted_entities.len(), 4);
    // should only include 0,1,3
    assert!(casted_entities
        .iter()
        .all(|a| [0, 2, 1, 3].iter().any(|n| a.data == *n)));
}
#[test]
fn tile_cast_works() {
    let entities: Vec<(usize, IVec3)> = [
        IVec3::new(0, 1, 1),
        IVec3::new(0, 2, 2),
        IVec3::new(1, 1, 2),
        IVec3::new(0, 3, 5),
        IVec3::new(0, 3, 3),
    ]
    .into_iter()
    .enumerate()
    .collect();

    let casted_entities = tile_cast(
        crate::tile_cast::Origin {
            tile: IVec3::new(0, 1, 1),
            ..Default::default()
        },
        Vec3::new(0., 1., 1.),
        TileStretch(1, 1),
        entities.into_iter().filter(|a| a.1 != IVec3::new(0, 1, 1)),
    )
    .collect::<Vec<_>>();

    for e in &casted_entities {
        println!("{} : {}", e.data, e.translation,);
    }

    assert_eq!(casted_entities.len(), 2);
    assert!(casted_entities[0].data == 1);
    assert!(casted_entities[1].data == 4);
}

#[test]
/// collision should work under super basic conditions
fn collision_works_basic() {
    let mut app = App::new();

    app.add_plugins(DefaultTestPlugin);
    app.add_plugins(crate::PhysicsPlugin);

    let move_id = app
        .world
        .spawn((
            Name::new("Move"),
            MovementBundle::default(),
            Collider::new(Constraints::ENTITY),
            TransformBundle::from_transform(Transform::from_xyz(0., 0., 0.)),
            MovementGoal(Vec3::new(1., 1., 0.)),
        ))
        .id();

    let wall_id = app
        .world
        .spawn((
            Name::new("Wall"),
            Collider::new(Constraints::WALL),
            TransformBundle::from_transform(Transform::from_xyz(2., 2., 0.)),
        ))
        .id();

    app.add_systems(PostUpdate, move |transform_q: Query<&GlobalTransform>| {
        let wall_location = transform_q.get(wall_id).unwrap().translation();
        let move_location = transform_q.get(move_id).unwrap().translation();

        assert_ne!(wall_location, move_location);
        assert_ne!(
            wall_location.cmpge(wall_location),
            BVec3::new(true, true, false)
        );
    });

    // TODO: is this necessary?
    app.cleanup();

    // run long enough for Move to move x + 2, y +2
    while app.world.resource::<Time>().elapsed_seconds() <= 3.1 {
        app.update();
    }
}

#[test]
fn entity_collisions_are_updated_properly() {
    todo!()
}

#[test]
#[ignore]
fn collision_works_weird_normalize() {
    todo!(
        "test that collision works when normalizing vel results in one axis rounding down to zero"
    );
}

#[test]
fn entity_collision_works_with_floor() {
    let mut app = App::new();

    app.add_plugins(DefaultTestPlugin);
    app.add_plugins(crate::PhysicsPlugin);

    let move_id = app
        .world
        .spawn((
            Name::new("Move"),
            MovementBundle::default(),
            Collider::new(Constraints::ENTITY),
            TransformBundle::from_transform(Transform::from_xyz(0., 0., 0.)),
            MovementGoal(Vec3::new(1., 1., 0.)),
        ))
        .id();

    let wall_id = app
        .world
        .spawn((
            Name::new("Wall"),
            Collider::new(Constraints::WALL),
            TransformBundle::from_transform(Transform::from_xyz(2., 2., 0.)),
        ))
        .id();

    app.world.spawn((
        Name::new("Floor"),
        Collider::new(Constraints::FLOOR),
        TransformBundle::from_transform(Transform::from_xyz(2., 2., 0.)),
    ));
    app.world.spawn((
        Name::new("Floor"),
        Collider::new(Constraints::FLOOR),
        TransformBundle::from_transform(Transform::from_xyz(1., 1., 0.)),
    ));

    app.add_systems(PostUpdate, move |transform_q: Query<&GlobalTransform>| {
        let wall_location = transform_q.get(wall_id).unwrap().translation();
        let move_location = transform_q.get(move_id).unwrap().translation();

        assert_ne!(wall_location, move_location);
        assert_ne!(
            wall_location.cmpge(wall_location),
            BVec3::new(true, true, false)
        );
    });

    // TODO: is this necessary?
    app.cleanup();

    // run long enough for Move to move x + 2, y +2
    while app.world.resource::<Time>().elapsed_seconds() <= 3.1 {
        app.update();
    }
}

#[test]
/// collision should work under super basic conditions
fn collision_works_skips() {
    let mut app = App::new();

    app.add_plugins(DefaultTestPlugin);

    app.add_plugins(crate::PhysicsPlugin);

    let move_id = app
        .world
        .spawn((
            Name::new("Move"),
            MovementBundle::default(),
            Collider::new(Constraints::ENTITY),
            TransformBundle::from_transform(Transform::from_xyz(0., 0., 0.)),
            MovementGoal(Vec3::new(1., 1., 0.)),
        ))
        .id();

    let wall_id = app
        .world
        .spawn((
            Name::new("Wall"),
            Collider::new(Constraints::WALL),
            TransformBundle::from_transform(Transform::from_xyz(2., 2., 0.)),
        ))
        .id();

    app.add_systems(PostUpdate, move |transform_q: Query<&GlobalTransform>| {
        let wall_location = transform_q.get(wall_id).unwrap().translation();
        let move_location = transform_q.get(move_id).unwrap().translation();

        assert_ne!(wall_location, move_location);
        assert!((wall_location.cmpge(wall_location) ^ BVec3::new(true, true, false)).any());
    });

    // TODO: is this necessary?
    app.cleanup();

    app.world
        .resource_mut::<Time>()
        .set_relative_speed_f64(1000.);

    // run long enough for Move to move x + 2, y +2
    while app.world.resource::<Time>().elapsed_seconds() <= 3.5 {
        app.update();
    }
}

#[test]
fn total_velocity_is_propagated() {
    let mut app = App::new();

    app.add_plugins(DefaultTestPlugin);
    app.add_plugins(crate::PhysicsPlugin);

    // this should have RelVel == TotalVel with both being Vec3::X
    let no_parent = app
        .world
        .spawn((
            Name::new("No parent"),
            VelocityBundle::default(),
            MovementGoal(Vec3::X),
            TransformBundle::from_transform(Transform::from_xyz(0., 0., 0.)),
        ))
        .id();

    // this should have RelVel == TotalVel with both being Vec3::X
    let moving_parent = app
        .world
        .spawn((
            Name::new("Moving Parent"),
            VelocityBundle::default(),
            MovementGoal(Vec3::X),
            TransformBundle::from_transform(Transform::from_xyz(0., 0., 0.)),
        ))
        .id();
    // this should have RelVel == moving_parent's TotalVel with both being Vec3::X
    let still_child = app
        .world
        .spawn((
            Name::new("Still Child"),
            VelocityBundle::default(),
            TransformBundle::from_transform(Transform::from_xyz(0., 0., 0.)),
        ))
        .set_parent(moving_parent)
        .id();

    // this should have RelVel == TotalVel with both at 0
    let still_parent = app
        .world
        .spawn((
            Name::new("Still Parent"),
            VelocityBundle::default(),
            TransformBundle::from_transform(Transform::from_xyz(0., 0., 0.)),
        ))
        .id();
    // this should have RelVel == TotalVel with both at Vec3::X
    let moving_child = app
        .world
        .spawn((
            Name::new("Moving Child"),
            MovementGoal(Vec3::X),
            VelocityBundle::default(),
            TransformBundle::from_transform(Transform::from_xyz(0., 0., 0.)),
        ))
        .set_parent(still_parent)
        .id();

    // TODO: check this out
    app.cleanup();

    let mut frames = 0;
    // run updates for 5 seconds
    while frames <= 60 {
        app.update();

        frames += 1;
        // check velocities here

        let total_vel = |id| app.world.get::<TotalVelocity>(id).unwrap().0;
        let relative_vel = |id| app.world.get::<RelativeVelocity>(id).unwrap().0;

        assert_eq!(total_vel(no_parent), Vec3::X);
        assert_eq!(relative_vel(no_parent), Vec3::X);

        assert_eq!(total_vel(moving_parent), Vec3::X);
        assert_eq!(relative_vel(moving_parent), Vec3::X);

        assert_eq!(total_vel(still_child), Vec3::X);
        assert_eq!(relative_vel(still_child), Vec3::ZERO);

        assert_eq!(total_vel(still_parent), Vec3::ZERO);
        assert_eq!(relative_vel(still_parent), Vec3::ZERO);

        assert_eq!(total_vel(moving_child), Vec3::X);
        assert_eq!(relative_vel(moving_child), Vec3::X);
    }
}
