#![allow(clippy::unwrap_used)]

use bevy::{
    prelude::{App, BuildWorldChildren, Events, GlobalTransform, Name, Transform, Vec3},
    time::Time,
    transform::TransformBundle,
};

use crate::{
    physics::{movement::MovementBundle, MovementGoal},
    test,
};

use super::collider::{Collider, Constraints};
use super::velocity::{RelativeVelocity, TotalVelocity, VelocityBundle};

#[test]
fn tile_cast_works() {
    todo!();
}

#[test]
/// collision should work under super basic conditions
fn collision_works_basic() {
    let mut app = App::new();

    app.add_plugins(test::DefaultTestPlugin);
    app.add_plugins(crate::physics::PhysicsPlugin);

    let move_id = app
        .world
        .spawn((
            Name::new("Move"),
            MovementBundle::default(),
            Collider::new(Constraints::WALL),
            TransformBundle::from_transform(bevy::prelude::Transform::from_xyz(0., 0., 0.)),
            MovementGoal(Vec3::new(1., 1., 0.)),
        ))
        .id();

    let wall_id = app
        .world
        .spawn((
            Name::new("Wall"),
            Collider::new(Constraints::WALL),
            TransformBundle::from_transform(bevy::prelude::Transform::from_xyz(2., 2., 0.)),
        ))
        .id();

    // TODO: is this necessary?
    app.cleanup();

    // run long enough for Move to move x + 2, y +2
    while app
        .world
        .resource::<Events<super::collider::EntityCollision>>()
        .is_empty()
    {
        app.update();

        assert!(
            app.world.resource::<Time>().elapsed_seconds() <= 3.,
            "Three seconds elapsed but no collision detected"
        );
    }

    let collisions = app
        .world
        .resource::<Events<super::collider::EntityCollision>>();
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

    app.add_plugins(test::DefaultTestPlugin);

    app.add_plugins(crate::physics::PhysicsPlugin);

    let move_id = app
        .world
        .spawn((
            Name::new("Move"),
            MovementBundle::default(),
            Collider::new(Constraints::WALL),
            TransformBundle::from_transform(bevy::prelude::Transform::from_xyz(0., 0., 0.)),
            MovementGoal(Vec3::new(1., 1., 0.)),
        ))
        .id();

    let wall_id = app
        .world
        .spawn((
            Name::new("Wall"),
            Collider::new(Constraints::WALL),
            TransformBundle::from_transform(bevy::prelude::Transform::from_xyz(2., 2., 0.)),
        ))
        .id();

    // TODO: is this necessary?
    app.cleanup();

    app.world
        .resource_mut::<Time>()
        .set_relative_speed_f64(1000.);

    // run long enough for Move to move x + 2, y +2
    while app
        .world
        .resource::<Events<super::collider::EntityCollision>>()
        .is_empty()
    {
        app.update();

        assert!(
            app.world.resource::<Time>().elapsed_seconds() <= 3.,
            "Three seconds (Time::elapsed_seconds) elapsed but no collision detected"
        );
    }

    let collisions = app
        .world
        .resource::<Events<super::collider::EntityCollision>>();
    let mut reader = collisions.get_reader();

    assert!(!reader.is_empty(collisions));

    let collisions = reader.iter(collisions).collect::<Vec<_>>();

    assert_eq!(collisions.len(), 2);

    let translation = |id| app.world.get::<GlobalTransform>(id).unwrap().translation();

    assert_ne!(translation(move_id), translation(wall_id));
}

#[test]
fn total_velocity_is_propagated() {
    let mut app = App::new();

    app.add_plugins(test::DefaultTestPlugin);
    app.add_plugins(crate::physics::PhysicsPlugin);

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

#[test]
fn movement_works() {
    todo!()
}
