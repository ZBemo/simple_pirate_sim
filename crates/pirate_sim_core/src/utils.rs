use bevy_ecs::prelude::{Entity, Query};
use bevy_math::Vec3;

#[must_use]
pub fn get_or_empty<V: core::ops::Deref<Target = Vec3> + bevy_ecs::prelude::Component>(
    vec_q: &Query<&V>,
    entity: Entity,
) -> Vec3 {
    vec_q.get(entity).map_or(Vec3::ZERO, |v| **v)
}
#[must_use]
pub fn get_or_empty_mut<V: core::ops::Deref<Target = Vec3> + bevy_ecs::prelude::Component>(
    vec_q: &Query<&mut V>,
    entity: Entity,
) -> Vec3 {
    vec_q.get(entity).map_or(Vec3::ZERO, |v| **v)
}
