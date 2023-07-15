use bevy::prelude::*;

use pirate_sim_core::tile_grid::{GetTileLocation, TileStretch};

pub struct RaycastHit {
    /// The offset of the raycast from the original transform
    pub offset: IVec3,
}

impl RaycastHit {
    #[must_use]
    pub fn distance(&self, vel: Vec3) -> f32 {
        (self.offset.as_vec3() / vel).length()
    }
    #[must_use]
    pub fn location(&self, origin: IVec3) -> IVec3 {
        origin + self.offset
    }
}

/// Raycast from `start_translation` with velocity of `ray_vel`
///
/// Takes an iterator over any tuple (A, impl GetTileLocation), and returns any pair that
/// would be in the path of the ray.
///
/// If `include_origin` is true, then it will return any T in the
/// same grid as `start_translation`, and it is your responsibility to filter out unwanted entities,
/// ie if you're casting out from a specific entity.
///
/// TODO: Return more information on each entity
#[inline]
#[allow(clippy::cast_precision_loss)]
pub fn tile_cast<Data, Location>(
    start_translation: IVec3,
    ray_vel: Vec3,
    tile_stretch: TileStretch,
    entities_iter: impl Iterator<Item = (Data, Location)>,
    include_origin: bool,
) -> Vec<(Data, RaycastHit)>
where
    Location: GetTileLocation,
{
    entities_iter
        .filter_map(|(data, transform)| {
            // cast to grid
            let original_closest = transform.location(tile_stretch);
            // translate so that start_translation is origin
            let translated_closest = original_closest - start_translation;

            #[cfg(debug_assertions)]
            if translated_closest.x > 1 << f32::MANTISSA_DIGITS {
                error!("tile_space's X is too large in tile cast");
                return None;
            }
            #[cfg(debug_assertions)]
            if translated_closest.y > 1 << f32::MANTISSA_DIGITS {
                error!("tile_space's Y is too large in tile cast");
                return None;
            }
            #[cfg(debug_assertions)]
            if translated_closest.z > 1 << f32::MANTISSA_DIGITS {
                error!("tile_space's Z is too large in tile cast");
                return None;
            }

            // if ray doesn't move on {x,y,z} axis, and entity is on 0 of that axis, then ray will
            // hit on that axis. Otherwise, if it is in the same direction that the ray is moving
            // then it will hit
            let ray_will_hit_x = (translated_closest.x == 0 && ray_vel.x == 0.)
                || translated_closest.x as f32 % ray_vel.x == 0.;
            let ray_will_hit_y = (translated_closest.y == 0 && ray_vel.y == 0.)
                || translated_closest.y as f32 % ray_vel.y == 0.;
            let ray_will_hit_z = (translated_closest.z == 0 && ray_vel.z == 0.)
                || translated_closest.z as f32 % ray_vel.z == 0.;

            // if we do  include origin then if it's ivec3::zero it should be picked up
            (include_origin && (translated_closest == IVec3::ZERO)
                || (
                    // if we don't include origin we have to make sure that it's not on the origin
                    // and then check if it'll hit on x y and z
                    (translated_closest != IVec3::ZERO && !include_origin)
                        && ray_will_hit_x
                        && ray_will_hit_y
                        && ray_will_hit_z
                ))
                .then_some((
                    data,
                    RaycastHit {
                        offset: translated_closest,
                    },
                ))
        })
        .collect()
}

#[cfg(feature = "developer-tools")]
pub(super) mod console {
    use bevy::{ecs::system::Command, prelude::*};
    use pirate_sim_console::{self as console, Output, PrintStringCommand};
    use pirate_sim_core::tile_grid::TileStretch;
    use std::{collections::VecDeque, error::Error};

    #[allow(clippy::module_name_repetitions)]
    pub fn raycast_console(input: VecDeque<console::Token>, commands: &mut Commands) {
        // raycast start_x start_y start_z dir_x dir_y dir_z

        if input.len() == 6 {
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
                    "Invalid arguments: error `{e}`",
                ))),
            };
        } else {
            commands.add(PrintStringCommand(format!(
                "Incorrect length: expected 6 arguments but was given {}",
                input.len()
            )));
        }
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

            let entities = super::tile_cast(
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
}
