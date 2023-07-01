use std::str::FromStr;

use crate::tile_grid::TileStretch;

use super::{
    registration::RegisterConsoleCommand, CommandOutput, ConsoleCommand, PrintStringCommand, Token,
};
use bevy::{app::AppExit, prelude::*};

#[derive(Reflect)]
struct EchoConsole;

impl ConsoleCommand for EchoConsole {
    fn start_command(&self, input: Vec<Token>, commands: &mut Commands) {
        commands.add(PrintStringCommand(
            input
                .iter()
                .map(|i| &i.string)
                .cloned()
                .collect::<Vec<_>>()
                .join(" "),
        ))
    }
}

#[derive(Reflect)]
struct ExitConsole;

impl ConsoleCommand for ExitConsole {
    fn start_command(&self, _input: Vec<Token>, commands: &mut Commands) {
        commands.add(|world: &mut World| world.send_event(AppExit));
    }
}

#[derive(Reflect)]
struct MoveConsole;

impl ConsoleCommand for MoveConsole {
    fn start_command(&self, input: Vec<Token>, commands: &mut Commands) {
        if input.len() != 4 {
            commands.add(PrintStringCommand(format!(
                "Wrong amount of inputs. Expected 4, got {}",
                input.len()
            )));
            return;
        }

        let name = input[0].string.clone();

        let parsed = || -> Result<IVec3, <i32 as FromStr>::Err> {
            let x = input[1].string.parse::<i32>()?;
            let y = input[2].string.parse::<i32>()?;
            let z = input[3].string.parse::<i32>()?;

            Ok(IVec3::new(x, y, z))
        }();

        match parsed {
            Ok(new_translation) => commands.add(move |world: &mut World| {
                let mut name_query = world.query::<(Entity, &Name)>();
                let mut location_query = world.query::<&mut Transform>();
                let output: String;

                let to_move = name_query.iter(world).find_map(|e| {
                    if e.1.as_str() == name {
                        Some(e.0)
                    } else {
                        None
                    }
                });

                match to_move {
                    Some(new_entity) => {
                        let tile_stretch = world
                            .get_resource::<TileStretch>()
                            .expect("No TileStretch resource")
                            .clone();
                        let transform = location_query.get_mut(world, new_entity);

                        if let Ok(mut transform) = transform {
                            *transform =
                                transform.with_translation(tile_stretch.get_bevy(&new_translation));
                        }

                        output = "Moved an entity".into();
                    }
                    None => output = "Could not find entity".into(),
                }

                if let Some(mut to_output) = world.get_resource_mut::<CommandOutput>() {
                    to_output.0 = Some(output)
                }
            }),
            Err(e) => commands.add(PrintStringCommand(format!("Parsing error `{}`", e))),
        }
    }
}

pub(super) fn setup_basic_commands(mut commands: Commands) {
    // run each command in this array
    [
        RegisterConsoleCommand::new("echo".into(), Box::new(EchoConsole)),
        RegisterConsoleCommand::new("exit".into(), Box::new(ExitConsole)),
        RegisterConsoleCommand::new("move".into(), Box::new(MoveConsole)),
    ]
    .into_iter()
    .for_each(|to_register| {
        commands.add(to_register);
    });
}
