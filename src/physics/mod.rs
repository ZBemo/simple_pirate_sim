//! A tile-based, real-time Physics Engine for this project
//!
//! See [`PhysicsPlugin`], and its build function to get started with the source code, or you can
//! likely read the file from top-down and understand it decently well.
//!
//! Currently, this file should only be for data definitions. Anything that requires a system
//! should be put into its own module.

use std::collections::VecDeque;
use std::error::Error;

use bevy::{ecs::system::Command, prelude::*, reflect::GetTypeRegistration};

use crate::console::{self, Output, PrintStringCommand};
use crate::tile_grid::TileStretch;

pub mod collider;
pub mod movement;
mod tile_cast;
pub mod velocity;

/// The gravity constant used for weight velocity gain
pub const GRAVITY: f32 = 9.8;

#[derive(SystemSet, Hash, Debug, Clone, Eq, PartialEq)]
/// We recommend running any system that plans to input into the Physics system before
/// [`PhysicsSet::Velocity`], or it may not be considered at all or until the next frame.
///
/// If wanting to use previously newly update locations, run after [`PhysicsSet::Movement`]
///
/// systems making use of collision checking should run after [`PhysicsSet::Collision`], or
/// collision data may be wildly inaccurate
pub enum PhysicsSet {
    Input,
    Velocity,
    Collision,
    Movement,
    Completed,
}

/// Any component with a weight will have gravity applied to it on each physics update
///
/// Any entity with a Weight will have a velocity of [`GRAVITY`] * Weight added to its relative
/// velocity during calculation.
#[derive(Debug, Clone, Copy, Component, Deref, DerefMut, Reflect)]
pub struct Weight(pub f32);

/// A way to request movement for a specific entity. Expects the entity to have a [`velocity::VelocityBundle`]
///
/// Each axis on the inner Vec3 represents the entities requested speed in that direction, similar
/// to a force diagram.
///
/// As valid movement is different for each entity, The physics engine does not check for "invalid" movement goals,
/// so it is the responsibility of  whoever is controlling an entity to make sure movement goals are valid before setting them.
#[derive(Debug, Component, Clone, Default, Deref, DerefMut, Reflect)]
pub struct MovementGoal(pub Vec3);

/// The components necessary for movement by the physics engine to take place on an entity's
/// transform.
///
/// You must provide a transform yourself in order to get movement, in order to stay compatible
/// with other bundles.
///
/// TODO: consider clearing ticker even if not attached to a Transform
#[derive(Bundle, Debug, Default)]
pub struct PhysicsComponentBase {
    ticker: movement::Ticker,
    total_velocity: velocity::VelocityBundle,
}

// TODO: move to self::tile_cast
fn raycast_console(input: VecDeque<crate::console::Token>, commands: &mut Commands) {
    // raycast start_x start_y start_z dir_x dir_y dir_z

    if input.len() != 6 {
        commands.add(PrintStringCommand(format!(
            "Incorrect length: expected 6 arguments but was given {}",
            input.len()
        )));
    } else {
        // TODO: switch this to using try blocks once out of nightly
        let vectors_result = || -> Result<_, Box<dyn Error>> {
            let start_x: i32 = input[0].string.parse()?;
            let start_y: i32 = input[1].string.parse()?;
            let start_z: i32 = input[2].string.parse()?;
            let dir_x: f32 = input[3].string.parse()?;
            let dir_y: f32 = input[4].string.parse()?;
            let dir_z: f32 = input[5].string.parse()?;

            Ok((
                IVec3::new(start_x, start_y, start_z),
                Vec3::new(dir_x, dir_y, dir_z),
            ))
        }();

        match vectors_result {
            Ok(vectors) => commands.add(RaycastCommand {
                start: vectors.0,
                direction: vectors.1,
            }),
            Err(e) => commands.add(PrintStringCommand(format!(
                "Invalid arguments: error `{}`",
                e
            ))),
        };
    };
}

struct RaycastCommand {
    start: IVec3,
    direction: Vec3,
}

impl Command for RaycastCommand {
    fn apply(self, world: &mut World) {
        let mut entity_query = world.query::<(Entity, &GlobalTransform)>();
        let mut name_query = world.query::<&Name>();
        let tile_stretch = world
            .get_resource::<TileStretch>()
            .expect("No tile stretch initialized??");
        let mut output = String::new();

        let entities = tile_cast::tile_cast(
            self.start,
            self.direction,
            *tile_stretch,
            entity_query.iter(world),
            true,
        );

        for entity in entities {
            // log name or whatever
            let name = name_query
                .get(world, entity.0)
                .map_or_else(|_| "UnNamed Entity", |n| n.as_str());

            let translation = entity_query
                .get(world, entity.0)
                .expect("Entity found in raycast but has no translation. This is not possible")
                .1
                .translation();

            output.push_str("Entity found in raycast:");
            output.push_str(name);
            output.push(':');
            output.push_str(&translation.to_string());
            output.push('\n');
        }

        if output.is_empty() {
            output = "No entities on ray".into();
        }

        world.send_event(Output::String(output));
        world.send_event(Output::End);
    }
}

fn startup(type_registry: Res<AppTypeRegistry>, mut commands: Commands) {
    // register raycast command
    commands.add(console::registration::RegisterConsoleCommand::new(
        "raycast".into(),
        Box::new(raycast_console),
    ));

    let mut type_registry_w = type_registry.write();

    type_registry_w.add_registration(movement::Ticker::get_type_registration());
    type_registry_w.add_registration(velocity::RelativeVelocity::get_type_registration());
    type_registry_w.add_registration(velocity::MantainedVelocity::get_type_registration());
    type_registry_w.add_registration(velocity::TotalVelocity::get_type_registration());
    type_registry_w.add_registration(collider::Constraints::get_type_registration());
    type_registry_w.add_registration(collider::Collider::get_type_registration());
    type_registry_w.add_registration(MovementGoal::get_type_registration());
    type_registry_w.add_registration(Weight::get_type_registration());
}

/// A plugin to setup essential physics systems
///
/// Any system that wants to use the results of a physics engine update should not run until after
/// [`PhysicsSet::Movement`] has been completed
///
/// Any systems that want to affect the physics engine in a given frame must run before
/// [`PhysicsSet::Velocity`].
pub struct PhysicsPlugin;

impl Plugin for PhysicsPlugin {
    fn build(&self, app: &mut App) {
        app.configure_set(Update, PhysicsSet::Input.before(PhysicsSet::Velocity))
            .configure_set(Update, PhysicsSet::Velocity.before(PhysicsSet::Collision))
            .configure_set(Update, PhysicsSet::Collision.after(PhysicsSet::Velocity))
            .configure_set(Update, PhysicsSet::Movement.after(PhysicsSet::Collision))
            .configure_set(Update, PhysicsSet::Completed.after(PhysicsSet::Movement))
            .add_plugins((velocity::Plugin, collider::Plugin, movement::Plugin))
            .add_systems(Startup, startup);
    }
}
