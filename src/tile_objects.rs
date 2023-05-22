//! Functions and systems for manipulating and updating objects using tiled sprites from the
//! spritesheet
//!
//! In the future, setup spritesheet, tilestretch, etc and process spritesheet images

use std::collections::HashSet;

use bevy::prelude::*;

use crate::controllers;

#[derive(Component, Debug, Deref)]
pub struct ObjectName(pub String);

#[derive(Component, Debug)]
pub struct DynWallObject();

/// A resource storing the area of each sprite in the spritesheet. Nearly any conversion between
/// IVec<->Vec should be done trough TileStretch to ensure that sprites are being displayed within
/// the right grid.
#[derive(Resource)]
pub struct TileStretch(pub u8, pub u8);

#[derive(Resource, Deref)]
pub struct SpriteSheetHandle(pub Handle<TextureAtlas>);

/// Marks that an entity should be managed as a viewable/interactable tile object
#[derive(Component)]
pub struct TileObject();

impl TileStretch {
    pub fn into_ivec2(&self) -> IVec2 {
        self.into()
    }

    pub fn bevy_translation_to_tile(&self, t: &Vec3) -> IVec3 {
        IVec3::new(
            t.x as i32 / self.0 as i32,
            t.y as i32 / self.1 as i32,
            t.z as i32,
        )
    }
    pub fn tile_translation_to_bevy(&self, t: &IVec3) -> Vec3 {
        Vec3::new(
            t.x as f32 * self.0 as f32,
            t.y as f32 * self.1 as f32,
            t.z as f32,
        )
    }
}

impl Into<IVec2> for &TileStretch {
    fn into(self) -> IVec2 {
        IVec2::new(self.0 as i32, self.1 as i32)
    }
}

// 45 degreees to radians * 2
pub const ROTATE_TILE: f32 = 2. * 0.785398;

fn update_dyn_wall_sprites(
    walls: Query<(&mut TextureAtlasSprite, &Transform), With<DynWallObject>>,
) {
    const WALL_INDEX: usize = 202;
    const CONNECTED_WALL_INDEX: usize = 207;
    const PILLAR: usize = 9;

    todo!()
}

fn process_sprites(sprites: Query<(&mut TextureAtlasSprite, &Transform)>) {
    todo!()
}

pub fn setup_spritesheet(asset_server: Res<AssetServer>) {
    todo!()
}

pub fn cull_non_camera_layer_sprites(
    mut commands: Commands,
    cameras: Query<&Transform, With<Camera>>,
    mut renderables: Query<(&mut Visibility, &Transform), (With<TileObject>, Without<Camera>)>,
    player: Query<&controllers::player::Controller, Changed<Transform>>,
) {
    // TODO: depth affect by having culled sprites turn to colored dots or something similar

    /// only run if there is player movement
    if let Ok(_) = player.get_single() {
    } else {
        return;
    };

    trace!("Culling non-camera layer sprites because player location change");

    let camera_layers: HashSet<_> = cameras.iter().map(|c| c.translation.z as i64).collect();
    for mut r in renderables.iter_mut() {
        if !camera_layers.contains(&(r.1.translation.z as i64)) {
            // should not be renderable
            *r.0 = Visibility::Hidden;
        } else {
            *r.0 = Visibility::Visible;
        }
    }
}
