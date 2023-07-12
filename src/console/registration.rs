//! See [`RegisterConsoleCommand`]

use super::{ConsoleCommandObject, RegisteredConsoleCommands};
use bevy::{prelude::*, utils::HashMap};

/// a struct to easily register a [`super::ConsoleCommand`] for the console to use
pub struct RegisterConsoleCommand(Box<str>, ConsoleCommandObject);

impl bevy::ecs::system::Command for RegisterConsoleCommand {
    fn apply(self, world: &mut World) {
        world
            .get_resource_or_insert_with(|| RegisteredConsoleCommands(HashMap::new()))
            .insert(self.0, self.1);
    }
}

impl RegisterConsoleCommand {
    /// create a registration command that will register `to_register`
    pub fn new(name: Box<str>, command: ConsoleCommandObject) -> Self {
        Self(name, command)
    }
}
