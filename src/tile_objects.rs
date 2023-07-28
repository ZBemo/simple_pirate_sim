//! Functions and systems for manipulating and updating objects using tiled sprites from the
//! spritesheet
//!
//! In the future, setup spritesheet, tilestretch, etc and process spritesheet images

// still in heavy development

use bevy::{prelude::*, reflect::GetTypeRegistration};

use pirate_sim_core::tile_grid::TileStretch;
use pirate_sim_physics::PhysicsSet;

#[derive(Resource, Deref, DerefMut, Reflect)]
pub struct SpriteSheetHandle(pub Handle<TextureAtlas>);

#[derive(Clone, Copy, Component, Reflect, Debug)]
pub struct TileCamera();

#[derive(Clone, Copy, Component, Reflect)]
pub struct ConnectingWall();

/// Marks that an entity should be managed as a rendered object, as well as
/// providing information about how it should be rendered.
///
/// `main_layer`,`one_up_index`, and `two_up_index` will decide which sprite to use when the sprite is on camera,
/// based on how far up the camera is from them.
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

pub fn register_types(type_registry: Res<AppTypeRegistry>) {
    let mut type_registry_w = type_registry.write();

    type_registry_w.add_registration(ConnectingWall::get_type_registration());
    type_registry_w.add_registration(SpriteSheetHandle::get_type_registration());
    type_registry_w.add_registration(TileCamera::get_type_registration());
    type_registry_w.add_registration(TileObject::get_type_registration());
}

/// a 2d bounding box used to represent a cameras viewport
struct BB2 {
    top_left: Vec2,
    bottom_right: Vec2,
}
impl BB2 {
    ///  inclusively check if point is inside self
    pub fn inside(&self, point: Vec3) -> bool {
        // self.min.z <= point.z
        //     && point.z <= self.max.z
        self.top_left.x <= point.x
            && point.x <= self.bottom_right.x
            && self.bottom_right.y <= point.y
            && point.y <= self.top_left.y
    }
    pub fn new(bottom_right: Vec2, top_left: Vec2) -> Self {
        Self {
            top_left,
            bottom_right,
        }
    }
}

pub fn update_tile_sprites(
    tile_camera_q: Query<Entity, (With<TileCamera>, With<Camera>)>,
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
            // SAFETY: we filter camera_q on With<Camera>, then get e from that q
            let camera = unsafe { camera_q.get(camera_entity).unwrap_unchecked() };
            let camera_transform = transform_q
                .get(camera_entity)
                .expect("Camera with no transform??");

            let viewport = camera.logical_viewport_rect()?;

            // top left corner->bottom right corner of camera in world space.
            let bottom_right = camera.viewport_to_world_2d(camera_transform, viewport.min)?;
            let top_left = camera.viewport_to_world_2d(camera_transform, viewport.max)?;

            debug!("{} -> {}", bottom_right, top_left);

            // this should put the point into the middle of the tilespace grid based on
            // tilestretch?
            //
            // the tilespace grid functions such that each grid centers on a multiple of
            // tilestretch.{x,y} on the {x,y} axis, and is the same size.
            let round_to_tile_space = |to_round: Vec2| -> Vec2 {
                let x = to_round.x + f32::from(tile_stretch.0) * to_round.x.signum();
                let y = to_round.y + f32::from(tile_stretch.1) * to_round.y.signum();

                Vec2::new(x, y)
            };

            // align start and end to a grid, so that it will align with entity origins
            let bottom_right = round_to_tile_space(bottom_right);
            let top_left = round_to_tile_space(top_left);

            Some((
                BB2::new(top_left, bottom_right),
                camera_transform.translation().z,
            ))
        })
        .collect();

    apply_entity_from_bounds(&bounds, &mut tile_object_q);
}

// because we're parallel iterating over everything as essentially its own entity, and that's all we
// do in this function, have all queries in one parameter
//
// this function is split out in case bevy system piping ever becomes beneficially performant, and
// useful
fn apply_entity_from_bounds(
    all_bounds: &[(BB2, f32)],
    tile_object_q: &mut Query<(
        Option<&mut TextureAtlasSprite>,
        Option<&mut Visibility>,
        Ref<GlobalTransform>,
        Ref<TileObject>,
    )>,
) {
    // check each tile object
    tile_object_q.par_iter_mut().for_each_mut(
        |(option_sprite, option_visibility, transform, tile_object)| {
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
                        bound.top_left,
                        bound.bottom_right
                    );

                    if bound.inside(translation) {
                        Some(*z as isize)
                    } else {
                        None
                    }
                })
                .fold(None, |acc, e| -> Option<isize> {
                    let acc = acc.or(e);

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
        app.add_systems(Startup, register_types)
            .add_systems(Update, update_tile_sprites.in_set(PhysicsSet::Completed));
    }
}
