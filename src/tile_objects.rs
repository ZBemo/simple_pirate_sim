//! Functions and systems for manipulating and updating objects using tiled sprites from the
//! spritesheet
//!
//! In the future, setup spritesheet, tilestretch, etc and process spritesheet images

// still in heavy development
#![allow(unused)]

use bevy::utils::HashMap;
use bevy::{prelude::*, reflect::GetTypeRegistration};
use std::collections::HashSet;
use std::ops::{Add, Div};

use crate::physics::PhysicsSet;
use crate::{controllers, tile_grid::TileStretch};

// 90 degrees in radians?
pub const ROTATE_TILE: f32 = std::f32::consts::FRAC_1_PI;

// #[derive(Component, Debug, Deref)]
// pub struct ObjectName(pub String);

#[derive(Resource, Deref, DerefMut, Reflect)]
pub struct SpriteSheetHandle(pub Handle<TextureAtlas>);

#[derive(Clone, Copy, Component, Reflect, Debug)]
pub struct TileCamera();

#[derive(Clone, Copy, Component, Reflect)]
pub struct ConnectingWall();

/// Marks that an entity should be managed as a rendered object, as well as
/// providing information about how it should be rendered.
///
/// {main_layer,one_up,two_up} will decide which sprite to use when the sprite is on camera.
/// Otherwise it will be culled.
#[derive(Component, Clone, Copy, Reflect, Debug)]
pub struct TileObject {
    pub main_layer_index: usize,
    pub one_up_index: usize,
    pub two_up_index: usize,
}

// TODO: encapsulate so must be instantiated through TileObjectBundle
impl TileObject {
    pub fn new(main: usize, one_up: usize, two_up: usize) -> Self {
        Self {
            main_layer_index: main,
            one_up_index: one_up,
            two_up_index: two_up,
        }
    }
}

fn process_sprites(sprites: Query<(&mut TextureAtlasSprite, &Transform)>) {
    todo!()
}

pub fn setup_spritesheet(asset_server: Res<AssetServer>) {
    todo!()
}

pub fn register_types(type_registry: Res<AppTypeRegistry>) {
    let mut type_registry_w = type_registry.write();

    type_registry_w.add_registration(ConnectingWall::get_type_registration());
    type_registry_w.add_registration(SpriteSheetHandle::get_type_registration());
    type_registry_w.add_registration(TileCamera::get_type_registration());
    type_registry_w.add_registration(TileObject::get_type_registration());
}

/// a 2d bounding box used to represent a cameras viewport
struct BB2 {
    min: Vec2,
    max: Vec2,
}
impl BB2 {
    ///  inclusively check if point is inside self
    pub fn inside(&self, point: Vec3) -> bool {
        // self.min.z <= point.z
        //     && point.z <= self.max.z
        self.min.x <= point.x
            && point.x <= self.max.x
            && self.min.y <= point.y
            && point.y <= self.max.y
    }
    pub fn new(min: Vec2, max: Vec2) -> Self {
        Self { min, max }
    }
}

pub fn update_tile_sprites(
    tile_camera_q: Query<Entity, With<TileCamera>>,
    camera_q: Query<Ref<Camera>>,
    transform_q: Query<&GlobalTransform>,
    mut tile_object_q: Query<(
        Option<&mut TextureAtlasSprite>,
        Option<&mut Visibility>,
        Ref<GlobalTransform>,
        Ref<TileObject>,
    )>,
    tile_stretch: Res<TileStretch>,
) {
    if !(tile_object_q
        .iter()
        .any(|t| t.2.is_changed() || t.3.is_changed())
        || camera_q.iter().any(|c| c.is_changed()))
    {
        trace!("No tile sprite changes/camera changes to update");
        return;
    }

    trace!("Updating tile sprites");

    // although this may look like a dictionary, as we're going to be iterating through it for each
    // tile entity, we want quick iteration and not quick indexing.
    let bounds: Vec<(BB2, f32)> = tile_camera_q
        .iter()
        .filter_map(|camera_entity| {
            let camera = unsafe { camera_q.get(camera_entity).unwrap_unchecked() };
            let camera_transform = transform_q
                .get(camera_entity)
                .expect("Camera with no transform??");

            let viewport = camera.logical_viewport_rect()?;

            // top left corner->bottom right corner of camera in world space.
            let start = camera.viewport_to_world_2d(camera_transform, viewport.0)?;
            let end = camera.viewport_to_world_2d(camera_transform, viewport.1)?;

            // this should put the point into the middle of the tilespace grid based on
            // tilestretch?
            //
            // the tilespace grid functions such that each grid centers on a multiple of
            // tilestretch.{x,y} on the {x,y} axis, and is the same size.
            let round_to_tile_space = |to_round: Vec2| -> Vec2 {
                let rounded = to_round.round();
                let x = (to_round.x + tile_stretch.x as f32 - (to_round.x % tile_stretch.x as f32));
                let y = (to_round.y + tile_stretch.y as f32 - (to_round.y % tile_stretch.y as f32));

                Vec2::new(x, y)
            };

            // align start and end to a grid, so that it will align with entity origins
            let start_gridded = round_to_tile_space(start);
            let end_gridded = round_to_tile_space(end);

            Some((
                BB2::new(start_gridded, end_gridded),
                camera_transform.translation().z,
            ))
        })
        .collect();

    apply_entity_from_bounds(bounds, &mut tile_object_q);
}

// because we're parallel iterating over everything as essentially its own entity, and that's all we
// do in this function, have all queries in one parameter
//
// this function is split out in case bevy system piping ever becomes beneficially performant, and
// useful
fn apply_entity_from_bounds(
    all_bounds: Vec<(BB2, f32)>,
    tile_object_q: &mut Query<(
        Option<&mut TextureAtlasSprite>,
        Option<&mut Visibility>,
        Ref<GlobalTransform>,
        Ref<TileObject>,
    )>,
) {
    // check each tile object
    tile_object_q.par_iter_mut().for_each_mut(
        |(mut option_sprite, mut option_visibility, transform, tile_object)| {
            // ensure it has a sprite and a visibility associated
            let Some(mut sprite) = option_sprite else {
                warn!("TileObject with no sprite!");
                return;
            };

            let Some(mut visibility) = option_visibility  else {
                warn!("TileObject with no visibility");
                return;
            };

            let translation = transform.translation();
            let current_z = translation.z as isize;

            // check if it has a camera that will render it
            if let Some(lowest_z) = all_bounds
                .iter()
                .map(|(bound, z)| {
                    trace!(
                        "checking if {} is inside {}-{}",
                        translation,
                        bound.min,
                        bound.max
                    );

                    if bound.inside(translation) {
                        Some(*z as isize)
                    } else {
                        None
                    }
                })
                .fold(None, |acc, e| -> Option<isize> {
                    let acc = acc.or_else(|| e);

                    Option::zip(acc, e).map(|(acc, e)| {
                        if e < acc && (e - current_z <= 2) {
                            e
                        } else {
                            acc
                        }
                    })
                })
            {
                *visibility = Visibility::Inherited;
                let distance_from_camera = lowest_z - current_z;

                match distance_from_camera {
                    0 => {
                        sprite.index = tile_object.main_layer_index;
                    }
                    1 => {
                        sprite.index = tile_object.one_up_index;
                    }
                    2 => {
                        sprite.index = tile_object.two_up_index;
                    }
                    _ => *visibility = Visibility::Hidden, // too far down, or above camera
                }
            } else {
                *visibility = Visibility::Hidden; // not in view of a camera
            }
        },
    );
}

pub struct Plugin;
impl bevy::prelude::Plugin for Plugin {
    fn build(&self, app: &mut App) {
        app.add_startup_system(register_types)
            .add_system(update_tile_sprites.after(PhysicsSet::FinalizeCollision));
    }
}
