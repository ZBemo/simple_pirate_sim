use bevy_math::prelude::*;
use bevy_log::trace;

use bevy_reflect::Reflect;
use pirate_sim_core::tile_grid::{GetTileLocation, TileStretch};

#[derive(Debug,Clone,Copy, Reflect)]
pub struct Hit<Data> {
    /// The position of the hit on the tilegrid
    pub translation: IVec3,
    /// the distance from Origin + Ticker
    pub distance: f32,
    /// The data passed in from the original iterator
    pub data: Data,
}

impl<Data> Hit<Data>
{
    /// Create a `Hit<U>` from `Fn(D) -> U`, only changing `U`
    pub fn map<U>(self, f: impl FnOnce(Data) -> U) -> Hit<U> {
        Hit {
            translation: self.translation,
            distance: self.distance,
            data: f(self.data)
        }
    }
}


/// The origin of a tile-cast
#[derive(Debug, Default)]
pub struct Origin {
    pub tile: IVec3,
    pub ticker: Vec3,
}

/// Raycast from `start_translation` with velocity of `ray_vel`
///
/// Takes an iterator over any tuple `(A, impl [GetTileLocation])` and returns a hit containing the
/// arbitrary data A, and other useful information
///
/// If `include_origin` is true, then it will return any T in the
/// same grid as `start_translation`, and it is your responsibility to filter out unwanted entities,
/// ie if you're casting out from a specific entity.
///
/// It currently rounds the ray onto the grid, which while being accurate in a tile-based physics
/// context, may lead to surprising results
///
/// TODO: get rid of include_origin, let caller deal with that if they care
#[inline]
#[must_use = "Tile casting is a relatively expensive operation that shouldn't change state. You should not use it if you don't need the result."]
pub fn tile_cast<Data, Location>(
    origin: Origin,
    ray_vel: Vec3,
    tile_stretch: TileStretch,
    entity_pool: impl Iterator<Item = (Data, Location)>,
    include_origin: bool,
) -> impl Iterator<Item = Hit<Data>>
where
    Location: GetTileLocation 
{
    

    let location_name = std::any::type_name::<Location>();
    let data_name = std::any::type_name::<Data>();

    let _span = bevy_log::info_span!("tile_cast", location_name = location_name, data_name = data_name).entered();
    

    trace!(
        "starting cast at origin {}:{} with vel {}",
        origin.tile,
        origin.ticker,
        ray_vel
    );


    let ray = bevy_math::Ray {
        origin: origin.tile.as_vec3(),
        direction: ray_vel.normalize(),
    };

    entity_pool.filter_map(move |(data, transform)| {
        // cast to grid
        let tile_translation = transform.location(tile_stretch);
        let tile_translation_vec3 = tile_translation.as_vec3();

        if tile_translation == origin.tile {
            return (include_origin).then_some(Hit {
                distance: 0.,
                translation: tile_translation,
                data,
            });
        };

        // TODO: see if there's some way to get better perf here. Could do like bevy_translation.x /
        // ray.vel.x after translating so ray.origin is [0,0,0], Then use that as distance/scale
        // factor?

        let expected_distance = Vec3::distance(origin.tile.as_vec3(), tile_translation_vec3);
        

        // ticker shouldn't influence the tile
        let casted_to_distance = origin.tile.as_vec3() + ray.direction * expected_distance;

        // account for epsilon to be safe
        // round distance because everything will be on grid
        let has_hit = Vec3::cmple((casted_to_distance.round() - tile_translation_vec3).abs(), Vec3::splat(f32::EPSILON)).all();

        trace!("checking {tile_translation_vec3}; expected_distance: {expected_distance}; casted: {casted_to_distance}");
        trace!("{has_hit}");

        has_hit.then_some(Hit {
            data,
            translation: tile_translation,
            distance: expected_distance,
        })
    })


}

#[cfg(feature = "developer-tools")]
pub(super) mod console {
    use bevy_ecs::{prelude::*,system::Command};
    use bevy_transform::prelude::*;
    use bevy_math::prelude::*;
    use bevy_core::Name;
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
