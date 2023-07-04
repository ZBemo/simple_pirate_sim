//! Systems, Resources, and traits implementing a simple, extensible developer console with
//! [`crate::bevy_egui`].
//!
//! The command line starts with the [`self::ConsoleCommand`] trait, and keeps a store of
//! [`ConsoleCommand`] trait objects, which are registered through with
//! [`registration::RegisterConsoleCommand`].
//!
//! [`io`] handles command input and output during the normal game loop.

mod basic_commands;
mod io;
pub mod registration;

use bevy::{prelude::*, utils::HashMap};
use thiserror::Error;

pub use io::CommandOutput;
pub use io::ConsoleOpen;

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct Token {
    pub string: String,
}

#[derive(Error, Debug)]
pub enum ParseError {
    #[error("Unexpected character at char {0}; We expected you to escape either a space, backslash, or quote")]
    EscapedIncorrectCharacter(usize),
    #[error("Input ended before closing all quotes.")]
    EndQuoted(),
    #[error("Input contains a backslash at end.")]
    EndEscaped(),
}

/// Parse commandline input. Currently just splits up strings with backslash and quote escaping
///
/// This needs to be moved either to io.rs or to io/parse.rs
fn parse(to_parse: &str) -> Result<Vec<Token>, ParseError> {
    trace!("parsing string `{}`", to_parse);

    let mut tokens: Vec<Token> = Vec::new();
    let mut cur_string = String::new();
    let mut is_backslash_escaped = false;
    let mut is_in_quotes = false;

    trace!("c - \"     \\");
    for (i, char) in to_parse.char_indices() {
        trace!("{} - {} {}", char, is_in_quotes, is_backslash_escaped);

        if is_backslash_escaped {
            match char {
                ' ' | '\\' | '"' => {
                    cur_string.push(char);
                    is_backslash_escaped = false
                }
                _ => return Err(ParseError::EscapedIncorrectCharacter(i)),
            }
        } else if is_in_quotes {
            match char {
                '"' => is_in_quotes = false,
                '\\' => is_backslash_escaped = true,
                c => cur_string.push(c),
            }
        } else {
            match char {
                '\\' => is_backslash_escaped = true,
                '"' => is_in_quotes = true,
                ' ' => {
                    tokens.push(Token { string: cur_string });
                    cur_string = String::new();
                }
                c => cur_string.push(c),
            }
        }
    }

    if is_backslash_escaped {
        return Err(ParseError::EndEscaped());
    } else if is_in_quotes {
        return Err(ParseError::EndQuoted());
    } else if !cur_string.is_empty() {
        tokens.push(Token { string: cur_string });
    }

    trace!("{:?}", tokens);

    tokens.shrink_to_fit(); // you shouldn't really be pushing and pulling from it at all after
                            // this
    Ok(tokens)
}

/// Objects that implement this trait & are type-object safe can be registered as a console command
///
/// You should also be able to register closures/functions with the function signature
/// (Vec<Token>,&mut Commands) -> (). Due to a blanket impl
pub trait ConsoleCommand {
    /// Start the command. Must add a command to commands that updates the [`self::CommandOutput`]
    /// resource
    fn start_command(&self, input: Vec<Token>, commands: &mut Commands);
}

impl<T: Fn(Vec<Token>, &mut Commands)> ConsoleCommand for T {
    fn start_command(&self, input: Vec<Token>, commands: &mut Commands) {
        self(input, commands)
    }
}

/// A console command type-object for registration
pub type ConsoleCommandObject = Box<dyn ConsoleCommand + Send + Sync>;

/// A resource to store all registered Console commands
#[derive(Deref, DerefMut, Resource)]
pub(self) struct RegisteredConsoleCommands(HashMap<Box<str>, ConsoleCommandObject>);

pub struct Plugin;
impl bevy::prelude::Plugin for Plugin {
    fn build(&self, app: &mut App) {
        app.add_startup_system(self::basic_commands::setup_basic_commands)
            .add_plugin(self::io::Plugin);
    }
}

/// Just print self.0 to the console
#[derive(Deref, DerefMut)]
pub struct PrintStringCommand(pub String);

impl bevy::ecs::system::Command for PrintStringCommand {
    fn write(self, world: &mut World) {
        world
            .get_resource_mut::<self::io::CommandOutput>()
            .expect("Console Command called with no Resource for output")
            .0 = Some(self.0);
    }
}
