//! Systems, Resources, and traits implementing a simple, extensible developer console with
//! [`crate::bevy_egui`].
//!
//! The command line starts with the [`self::ConsoleCommand`] trait, and keeps a store of
//! [`ConsoleCommand`] trait objects, which are registered through with
//! [`registration::RegisterConsoleCommand`].
//!
//! [`io`] handles command input and output during the normal game loop.

#![warn(clippy::unwrap_used)]
#![warn(clippy::perf, clippy::disallowed_types)] // performance warns
#![warn(clippy::pedantic)]
// most bevy systems violate these. Nothing I can do about it at the moment.
#![allow(
    clippy::type_complexity,
    clippy::too_many_arguments,
    clippy::needless_pass_by_value // TODO: separate out system functions from non-system 
)]
#![allow(clippy::cast_possible_truncation)]

mod io;
pub mod registration;

use std::collections::VecDeque;

use bevy_derive::{Deref, DerefMut};
use bevy_ecs::prelude::*;
use bevy_log::prelude::*;
use bevy_utils::HashMap;
use thiserror::Error;

pub use io::IsOpen;
pub use io::Output;

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
fn parse(to_parse: &str) -> Result<VecDeque<Token>, ParseError> {
    trace!("parsing string `{}`", to_parse);

    let mut tokens: VecDeque<Token> = VecDeque::new();
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
                    is_backslash_escaped = false;
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
                    tokens.push_back(Token { string: cur_string });
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
        tokens.push_back(Token { string: cur_string });
    }

    trace!("{:?}", tokens);

    tokens.shrink_to_fit(); // you shouldn't really be pushing and pulling from it at all after
                            // this
    Ok(tokens)
}

/// A console command type-object for registration
pub type CommandObject = fn(VecDeque<Token>, &mut Commands);

/// A resource to store all registered Console commands
#[derive(Deref, DerefMut, Resource)]
pub(self) struct RegisteredConsoleCommands(HashMap<Box<str>, CommandObject>);

pub struct Plugin;
impl bevy_app::Plugin for Plugin {
    fn build(&self, app: &mut bevy_app::App) {
        app.add_plugins(self::io::Plugin);
    }
}

/// A command to print self.0 to the console
#[derive(Deref, DerefMut)]
pub struct PrintStringCommand(pub String);

impl bevy_ecs::system::Command for PrintStringCommand {
    fn apply(self, world: &mut World) {
        world.send_event(Output::String(self.0));
        world.send_event(Output::End);
    }
}
