use std::{collections::VecDeque, str::FromStr};

use crate::tile_grid::TileStretch;

use super::{registration::RegisterConsoleCommand, ConsoleOutput, PrintStringCommand, Token};
use bevy::{app::AppExit, prelude::*};

fn echo_command(input: VecDeque<Token>, commands: &mut Commands) {
    commands.add(PrintStringCommand(
        input
            .iter()
            .map(|i| &i.string)
            .fold("".to_owned(), |acc, str| format!("{acc}{str} ")),
    ))
}

fn exit_command(_input: VecDeque<Token>, commands: &mut Commands) {
    commands.add(|world: &mut World| world.send_event(AppExit));
}

fn move_command(mut input: VecDeque<Token>, commands: &mut Commands) {
    if input.len() != 4 {
        commands.add(PrintStringCommand(format!(
            "Wrong amount of inputs. Expected 4, got {}",
            input.len()
        )));
        return;
    }

    let name = input.pop_front().unwrap().string;

    let parsed = || -> Result<IVec3, <i32 as FromStr>::Err> {
        let x = input.pop_front().unwrap().string.parse::<i32>()?;
        let y = input.pop_front().unwrap().string.parse::<i32>()?;
        let z = input.pop_front().unwrap().string.parse::<i32>()?;

        Ok(IVec3::new(x, y, z))
    }();

    match parsed {
        Ok(new_translation) => commands.add(move |world: &mut World| {
            let mut name_query = world.query::<(Entity, &Name)>();
            let mut location_query = world.query::<&mut Transform>();
            let output: String;

            let to_move = name_query
                .iter(world)
                .find_map(|e| (e.1.as_str() == name).then(|| e.0));

            match to_move {
                Some(new_entity) => {
                    let tile_stretch = world
                        .get_resource::<TileStretch>()
                        .expect("No TileStretch resource")
                        .clone();
                    let transform = location_query.get_mut(world, new_entity);

                    if let Ok(mut transform) = transform {
                        *transform =
                            transform.with_translation(tile_stretch.get_bevy(new_translation));
                    }

                    output = "Moved an entity".into();
                }
                None => output = "Could not find entity".into(),
            }

            world.send_event(ConsoleOutput::String(output));
            world.send_event(ConsoleOutput::End);
        }),
        Err(e) => commands.add(PrintStringCommand(format!("Parsing error `{}`", e))),
    }
}

pub(super) fn setup_basic_commands(mut commands: Commands) {
    // run each command in this array
    [
        RegisterConsoleCommand::new("echo".into(), Box::new(echo_command)),
        RegisterConsoleCommand::new("exit".into(), Box::new(exit_command)),
        RegisterConsoleCommand::new("move".into(), Box::new(move_command)),
    ]
    .into_iter()
    .for_each(|to_register| {
        commands.add(to_register);
    });
}
