use std::{any::Any, format};

use bevy::{ecs::query::WorldQuery, prelude::*};

pub const DEFAULT_FONT_PATH: &str = "fonts/FiraCode/FiraCodeNerdFont-Bold.ttf";

use crate::{controllers, tile_objects::TileStretch};

#[derive(Component)]
pub struct InfoInsert {
    order: u8, // up to 255 things
    to_render: String,
}

#[derive(Component)]
pub struct CoordsText();

#[derive(Component)]
pub struct InfoDisplay();

pub fn update_coords_display(
    player: Query<(&controllers::player::Controller, &Transform), Changed<Transform>>,
    mut query: Query<(&mut InfoInsert), With<CoordsText>>,
    tile_stretch: Res<TileStretch>,
) {
    if let Ok(player) = &player.get_single() {
        let pos = tile_stretch.bevy_translation_to_tile(&player.1.translation);

        let new_text = format!("{}, {}, {}", pos.x, pos.y, pos.z);

        for mut text in query.iter_mut() {
            text.to_render = new_text.clone();
        }
    }
}

/// puts together all inserts into one single display in the upper lefthand corner
///
/// could probably use caching in the future so we're not doing string operations every frame
/// or have each insert have its own text, with positioning managed by a central node entity
fn update_info_display(text_q: Query<&mut Text, With<InfoDisplay>>, inserts_q: Query<&InfoInsert>) {
    let mut inserts_sorted: Vec<&InfoInsert> = inserts_q.iter().collect();
    inserts_sorted.sort_by_key(|c| c.order);

    todo!()
}

pub fn setup_info_display(mut commands: Commands, asset_server: Res<AssetServer>) {
    commands.spawn((
        TextBundle::from_section(
            "",
            TextStyle {
                font: asset_server.load(DEFAULT_FONT_PATH),
                font_size: 20.0,
                color: Color::WHITE,
            },
        )
        .with_text_alignment(TextAlignment::Left)
        .with_style(Style {
            position_type: PositionType::Absolute,
            position: UiRect {
                top: Val::Percent(1.),
                left: Val::Percent(1.),
                ..default()
            },
            ..default()
        }),
        CoordsText(),
    ));
}

/// figure out some way to have the plugin be built, and then handle building of textboxes
/// procedurally.
///
/// perhaps have inserts register blank marker structs that they then query for.
struct InfoDisplayPlugin();

impl Plugin for InfoDisplayPlugin {
    fn build(&self, app: &mut App) {
        todo!()
    }
}
