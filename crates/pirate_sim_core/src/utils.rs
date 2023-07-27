use bevy_ecs::prelude::{Entity, Query};
use bevy_math::{BVec3, Vec3};

#[must_use]
#[inline]
pub fn get_or_empty<V: core::ops::Deref<Target = Vec3> + bevy_ecs::prelude::Component>(
    vec_q: &Query<&V>,
    entity: Entity,
) -> Vec3 {
    vec_q.get(entity).map_or(Vec3::ZERO, |v| **v)
}

#[must_use]
#[inline]
pub fn get_or_empty_mut<V: core::ops::Deref<Target = Vec3> + bevy_ecs::prelude::Component>(
    vec_q: &Query<&mut V>,
    entity: Entity,
) -> Vec3 {
    vec_q.get(entity).map_or(Vec3::ZERO, |v| **v)
}

/// Turn a bvec into a vec where rhs * `bvec_to_vec(vec)` will "mask" away any falses
#[must_use]
#[inline]
pub fn bvec_to_mask(vec: BVec3) -> Vec3 {
    let x = if vec.x { 1. } else { 0. };
    let y = if vec.y { 1. } else { 0. };
    let z = if vec.z { 1. } else { 0. };

    Vec3::new(x, y, z)
}
