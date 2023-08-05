use bevy_ecs::{
    prelude::{Entity, Query},
    query::WorldQuery,
};
use bevy_math::{BVec3, Vec3};

/// Lookup a newtype over [`Vec3`] or return [`Vec3::ZERO`]
#[must_use]
#[inline]
pub fn get_or_zero<'a, V, VPointer>(vec_q: &'a Query<'_, 'a, VPointer>, entity: Entity) -> Vec3
where
    V: core::ops::Deref<Target = Vec3>,
    VPointer: WorldQuery,
    <<VPointer as WorldQuery>::ReadOnly as WorldQuery>::Item<'a>: core::ops::Deref<Target = V>,
{
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
