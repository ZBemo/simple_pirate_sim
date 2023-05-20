//! Functions and systems for manipulating and updating objects using tiled sprites from the
//! spritesheet
//!
//! In the future, setup spritesheet, tilestretch, etc and process spritesheet images

use bevy::prelude::*;

#[derive(Component, Debug)]
pub struct ObjectName(pub String);

#[derive(Component, Debug)]
pub struct DynWallSprite();

#[derive(Resource)]
pub struct TileStretch(pub u8, pub u8);

impl TileStretch {
    pub fn into_vec2(&self) -> Vec2 {
        self.into()
    }

    pub fn bevy_translation_to_tile(&self, t: &Vec3) -> Vec3 {
        Vec3::new(t.x / self.0 as f32, t.y / self.1 as f32, t.z)
    }
    pub fn tile_translation_to_bevy(&self, t: &Vec3) -> Vec3 {
        Vec3::new(t.x * self.0 as f32, t.y * self.1 as f32, t.z)
    }
}

impl Into<Vec2> for &TileStretch {
    fn into(self) -> Vec2 {
        Vec2::new(self.0 as f32, self.1 as f32)
    }
}

// 45 degreees to radians * 2
pub const ROTATE_TILE: f32 = 2. * 0.785398;

fn update_dyn_wall_sprites(walls: Query<(&mut TextureAtlasSprite, &Transform)>) {
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
    renderable: Query<(&mut Visibility, &Transform), Without<Camera>>,
) {
}
