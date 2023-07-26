use bevy::{
    ecs::query::WorldQuery,
    prelude::{Entity, Query, Vec3},
};

#[must_use]
pub fn get_or_empty<V: core::ops::Deref<Target = Vec3> + bevy::prelude::Component>(
    vec_q: &Query<&V>,
    entity: Entity,
) -> Vec3 {
    vec_q.get(entity).map_or(Vec3::ZERO, |v| **v)
}
