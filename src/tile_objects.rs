//! Functions and systems for manipulating and updating objects using tiled sprites from the
//! spritesheet
//!
//! In the future, setup spritesheet, tilestretch, etc and process spritesheet images

// still in heavy development
#![allow(unused)]

use bevy::{prelude::*, reflect::GetTypeRegistration};
use std::collections::HashSet;

use crate::controllers;
use crate::physics::TileStretch;

// #[derive(Component, Debug, Deref)]
// pub struct ObjectName(pub String);

#[derive(Component, Debug)]
pub struct DynWallObject();

#[derive(Resource, Deref)]
pub struct SpriteSheetHandle(pub Handle<TextureAtlas>);

/// Marks that an entity should be managed as a viewable/interactable tile object
#[derive(Component)]
pub struct TileObject();

// 45 degreees to radians * 2
pub const ROTATE_TILE: f32 = std::f32::consts::FRAC_1_PI;

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

pub fn register_types(type_registry: Res<AppTypeRegistry>) {
    let mut type_registry_w = type_registry.write();

    type_registry_w.add_registration(TileStretch::get_type_registration());
}

pub fn cull_non_camera_layer_sprites(
    cameras: Query<&Transform, With<Camera>>,
    mut renderables: Query<(&mut Visibility, &Transform), (With<TileObject>, Without<Camera>)>,
    player: Query<&controllers::player::Controller, Changed<Transform>>,
) {
    // TODO: depth affect by having culled sprites turn to colored dots or something similar

    // only run if there is player movement
    if player.get_single().is_err() {
        return;
    };

    trace!("Culling non-camera layer sprites because player location change");

    let camera_layers: Vec<_> = cameras.iter().map(|c| c.translation.z).collect();

    for mut r in renderables.iter_mut() {
        if camera_layers.contains(&r.1.translation.z) {
            *r.0 = Visibility::Visible;
        } else {
            // should not be renderable
            *r.0 = Visibility::Hidden;
        }
    }
}
