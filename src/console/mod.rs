mod basic_commands;
mod io;
pub mod registration;

use bevy::{prelude::*, utils::HashMap};
use thiserror::Error;

pub use io::CommandOutput;
pub use io::ConsoleOpen;

pub struct Token {
    pub string: String,
}

#[derive(Error, Debug)]
pub enum ParseError {
    #[error("Unexpected character at 0:{0}; We expected you to escape either a space, backslash, or quote")]
    EscapedIncorrectCharacter(usize),
    #[error("Input ended before closing all quotes.")]
    EndQuoted(),
    #[error("Input contains a backslash at end.")]
    EndEscaped(),
}

/// do things like  escaping, split by spaces except in string
fn parse(to_parse: &str) -> Result<Vec<Token>, ParseError> {
    trace!("parsing string `{}`", to_parse);

    let mut tokens: Vec<Token> = Vec::new();
    let mut cur_string = String::new();
    let mut is_backslash_escaped = false;
    let mut is_in_quotes = false;

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
    } else {
        if cur_string != "" {
            tokens.push(Token { string: cur_string });
        }
    }

    trace!("parsed {} tokens", tokens.len());

    Ok(tokens)
}

#[reflect_trait]
pub trait ConsoleCommand: Reflect {
    /// Start the command. Must add a command to commands that updates Res<[`self::io::CommandOutput`]>
    /// or update [`self::io::ComandOutput`] in someway
    fn start_command(&self, input: Vec<Token>, commands: &mut Commands);
}

pub type ConsoleCommandRegistration = Box<dyn ConsoleCommand>;
// TODO: figure out how to reflect
#[derive(Deref, DerefMut, Resource)]
pub(self) struct RegisteredConsoleCommands(HashMap<Box<str>, ConsoleCommandRegistration>);

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
            .map(|mut r| r.0 = Some(self.0));
    }
}
