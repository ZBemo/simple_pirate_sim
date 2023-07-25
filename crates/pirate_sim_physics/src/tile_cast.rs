use bevy::prelude::*;

use pirate_sim_core::tile_grid::{GetTileLocation, TileStretch};

#[derive(Debug)]
pub struct Hit<Data> {
    /// The data passed in from the original iterator
    pub data: Data,
    // convert to Tile grid?
    pub translation: Vec3,
    pub distance: f32,
}

/// The origin of a tile-cast
#[derive(Debug, Default)]
pub struct Origin {
    pub tile: IVec3,
    pub ticker: Vec3,
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
#[must_use = "Tile casting is a relatively expensive operation that shouldn't change state. You should not use it if you don't need the result."]
pub fn tile_cast<Data, Location>(
    start_translation: Origin,
    ray_vel: Vec3,
    tile_stretch: TileStretch,
    entity_pool: impl Iterator<Item = (Data, Location)>,
    include_origin: bool,
) -> impl Iterator<Item = Hit<Data>>
where
    Location: GetTileLocation,
{
    let ticker_offset_bevy = tile_stretch * start_translation.ticker;

    let origin_bevy = tile_stretch.get_bevy(start_translation.tile) + ticker_offset_bevy;

    let ray = bevy::math::Ray {
        origin: origin_bevy,
        direction: (tile_stretch * ray_vel).normalize(),
    };

    entity_pool.filter_map(move |(data, transform)| {
        // cast to grid
        let tile_translation = transform.location(tile_stretch);

        let bevy_translation = tile_stretch.get_bevy(tile_translation);

        if tile_translation == start_translation.tile {
            return (include_origin).then_some(Hit {
                distance: 0.,
                translation: bevy_translation,
                data,
            });
        };

        let expected_distance = bevy::math::Vec3::distance(ray.origin, bevy_translation);

        let casted_to_distance = ray.origin + ray.direction * expected_distance;

        let has_hit = casted_to_distance == bevy_translation;

        has_hit.then_some(Hit {
            data,
            translation: bevy_translation,
            distance: expected_distance,
        })
    })
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
            let mut ticker_query = world.query::<&crate::movement::Ticker>();
            let mut name_query = world.query::<&Name>();
            let tile_stretch = world
                .get_resource::<TileStretch>()
                .expect("No tile stretch initialized??");
            let mut output = String::new();

            let entities = super::tile_cast(
                super::Origin {
                    tile: self.start,
                    ticker: Vec3::ZERO,
                },
                self.direction,
                *tile_stretch,
                entity_query.iter(world),
                true,
            );

            for entity in entities.collect::<Box<[_]>>().iter() {
                // log name or whatever
                let name = name_query
                    .get(world, entity.data)
                    .map_or_else(|_| "UnNamed Entity", |n| n.as_str());

                let translation = entity_query
                    .get(world, entity.data)
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
