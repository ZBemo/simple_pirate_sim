use std::str::FromStr;

use crate::tile_grid::TileStretch;

use super::{
    registration::RegisterConsoleCommand, CommandOutput, ConsoleCommand, PrintStringCommand, Token,
};
use bevy::{app::AppExit, ecs::system::Command, prelude::*};

#[derive(Reflect)]
struct EchoConsole;

impl ConsoleCommand for EchoConsole {
    fn start_command(&self, input: Vec<Token>, commands: &mut Commands) {
        commands.add(PrintStringCommand(input.iter().fold(
            String::new(),
            |acc, e| {
                let mut new_string = String::new();
                new_string.push_str(&acc);
                new_string.push(' ');
                new_string.push_str(&e.string);

                trace!("{}", new_string);

                new_string
            },
        )))
    }
}

/// update an entity's name
///
/// TODO: get rid of changecommand and use closures instead
#[derive(Reflect)]
struct ChangeNameConsole;
enum ChangeNameCommand {
    WrongInputs(usize),
    ChangeName(String, String),
}

impl ConsoleCommand for ChangeNameConsole {
    fn start_command(&self, input: Vec<Token>, commands: &mut Commands) {
        let command = if input.len() != 2 {
            ChangeNameCommand::WrongInputs(input.len())
        } else {
            // we just checked length
            unsafe {
                ChangeNameCommand::ChangeName(
                    input.get_unchecked(0).string.clone(),
                    input.get_unchecked(1).string.clone(),
                )
            }
        };

        commands.add(command)
    }
}

impl Command for ChangeNameCommand {
    fn write(self, world: &mut World) {
        match self {
            ChangeNameCommand::ChangeName(from, to) => {
                let mut query = world.query::<&mut Name>();
                let mut output = format!("renaming entities with name {} to {}", from, to);

                for mut name in query.iter_mut(world) {
                    if name.as_str() == from {
                        name.set(to.clone());
                        output.push_str("renamed an entity\n");
                    }
                }

                world
                    .get_resource_mut::<super::io::CommandOutput>()
                    .expect("Console Command called with no CommandOutput resource")
                    .0 = Some(output);
            }
            ChangeNameCommand::WrongInputs(len) => {
                world
                    .get_resource_mut::<super::io::CommandOutput>()
                    .expect("Console Command called with no CommandOutput resource")
                    .0 = Some(format!(
                    "Wrong amount of inputs. Expected 2 inputs but instead was given {}",
                    len
                ));
            }
        }
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
    commands.add(super::registration::RegisterConsoleCommand::new(
        "echo".into(),
        Box::new(EchoConsole),
    ));
    commands.add(super::registration::RegisterConsoleCommand::new(
        "rename".into(),
        Box::new(ChangeNameConsole),
    ));

    commands.add(super::registration::RegisterConsoleCommand::new(
        "exit".into(),
        Box::new(ExitConsole),
    ));
    commands.add(RegisterConsoleCommand::new(
        "move".into(),
        Box::new(MoveConsole),
    ));
}
