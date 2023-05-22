use std::format;

use bevy::prelude::*;

pub const DEFAULT_FONT_PATH: &str = "fonts/FiraCode/FiraCodeNerdFont-Bold.ttf";

use crate::{controllers, tile_objects::TileStretch};

#[derive(Component)]
pub struct CoordsText();

pub fn update_coords_display(
    player: Query<(&controllers::player::Controller, &Transform), Changed<Transform>>,
    mut query: Query<(&mut Text), With<CoordsText>>,
    tile_stretch: Res<TileStretch>,
) {
    if let Ok(player) = &player.get_single() {
        let pos = tile_stretch.bevy_translation_to_tile(&player.1.translation);

        let new_text = format!("{}, {}, {}", pos.x, pos.y, pos.z);

        for mut text in query.iter_mut() {
            text.sections[0].value = new_text.clone();
        }
    }
}

pub fn setup_coords_display(mut commands: Commands, asset_server: Res<AssetServer>) {
    commands.spawn((
        TextBundle::from_section(
            "-, -, -",
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
