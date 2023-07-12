use std::borrow::Borrow;

use bevy::prelude::*;

use crate::tile_grid::{GetTileLocation, TileStretch};

/// Raycast out from start_translation in the direction of ray_vel.
///
/// Takes an iterator over any tuple (impl Copy, impl GetTileLocation), and returns any pair that
/// would be in the path of the ray.
///
/// If include_origin is true, then it will return any T in the
/// same grid as start_translation, and it is your responsibility to filter out unwanted entities,
/// ie if you're casting out from a specific entity.
///
/// Usually T will be entity, but it will accept any T impl clone, in order to make the api more ergonomic
#[inline]
pub fn tile_cast<Data: Clone, Location: GetTileLocation, BT: Borrow<(Data, Location)>>(
    start_translation: IVec3,
    ray_vel: Vec3,
    tile_stretch: TileStretch,
    entities_iter: impl Iterator<Item = BT>,
    include_origin: bool,
) -> Vec<(Data, IVec3)> {
    entities_iter
        .filter_map(|bt| {
            let (entity, transform) = bt.borrow();

            // cast to grid
            let original_closest = transform.location(tile_stretch);
            // translate so that start_translation is origin
            let translated_closest = original_closest - start_translation;

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
                .then(|| (entity.clone(), original_closest))
        })
        .collect()
}

#[cfg(test)]
mod test {
    #[test]
    fn tile_cast_works() {
        todo!();
    }
}
