use super::{ConsoleCommandRegistration, RegisteredConsoleCommands};
use bevy::{prelude::*, utils::HashMap};

/// a struct to easily register a [`ConsoleCommand`] for the console to use
pub struct RegisterConsoleCommand(Box<str>, ConsoleCommandRegistration);

impl bevy::ecs::system::Command for RegisterConsoleCommand {
    fn write(self, world: &mut World) {
        let mut registered_commands =
            world.get_resource_or_insert_with(|| RegisteredConsoleCommands(HashMap::new()));
        registered_commands.insert(self.0, self.1);
    }
}

impl RegisterConsoleCommand {
    /// create a registration command that will register `to_register`
    pub fn new(name: Box<str>, command: ConsoleCommandRegistration) -> Self {
        Self(name, command)
    }
}
