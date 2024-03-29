use bevy_derive::{Deref, DerefMut};
use bevy_ecs::prelude::*;
use bevy_input::prelude::*;
use bevy_log::prelude::*;
use bevy_reflect::GetTypeRegistration;
use bevy_reflect::Reflect;

use bevy_inspector_egui::bevy_egui::{egui, EguiContexts};

use super::parse;

/// A resource tracking whether or not the console is currently open
#[derive(Deref, DerefMut, Reflect, Resource)]
pub struct IsOpen(pub bool);

// /// The resource for console commands to write their output to
// #[derive(Deref, DerefMut, Resource, Reflect)]
// pub struct CommandOutput(pub Sender<String>);

#[derive(Event, Clone)]
/// events for a command to output to console
pub enum Output {
    /// A string to write to the console.
    String(String),
    /// Tells the console that the current command has ended
    End,
}

const MAX_HISTORY_SIZE: usize = 500;

/// a system to open the console when backtick is pressed
fn check_open_console(keys: Res<Input<KeyCode>>, mut showing_console: ResMut<IsOpen>) {
    if keys.just_pressed(KeyCode::Grave) {
        trace!("showing console = true");
        *showing_console = IsOpen(true);
    }
}

/// behemoth system to Handle drawing the console and taking input
fn do_io(
    mut input: Local<String>,
    mut output_history: Local<String>,
    mut waiting_for_command: Local<bool>,
    mut context: EguiContexts,
    mut showing_console: ResMut<IsOpen>,
    mut command_output: EventReader<Output>,
    console_commands: Res<super::RegisteredConsoleCommands>,
    mut commands: Commands,
) {
    if !**showing_console {
        return;
    }

    let original_output_history = output_history.clone();

    let mut write_output = |string: &str| {
        if output_history.len() + string.len() > MAX_HISTORY_SIZE {
            warn!("max output size going to be exceeded; clearing buffer");
            output_history.clear();
        };

        output_history.push('\n');
        output_history.push_str(string);
    };

    if *waiting_for_command {
        for event in command_output.iter() {
            match event {
                Output::String(output) => write_output(output),
                Output::End => *waiting_for_command = false,
            }
        }
    }

    egui::Window::new("Console")
        .vscroll(true)
        .show(context.ctx_mut(), |ui| {
            ui.heading("Console");

            ui.vertical(|ui| {
                // todo: check if escape pressed. Close console if so

                ui.label(original_output_history);

                ui.horizontal(|ui| {
                    ui.label("Console:");
                    let edited = ui.text_edit_singleline(&mut *input);

                    if !*waiting_for_command
                        && edited.lost_focus()
                        && ui.input(|i| i.key_pressed(egui::Key::Enter))
                    {
                        // enter was pressed: run commmand

                        match parse(&input) {
                            Ok(tokens) => {
                                let mut tokens = tokens;

                                #[allow(clippy::unwrap_used)]
                                if tokens.is_empty() {
                                    tokens = parse("echo Please enter a command").unwrap();
                                };

                                let mut tokens_iter = tokens.into_iter();

                                // SAFETY: we just ensured tokens isn't empty so len must be >= 1
                                let command =
                                    unsafe { tokens_iter.next().unwrap_unchecked().string };

                                let command_obj = console_commands.get(&Box::from(command));

                                match command_obj {
                                    Some(command_obj) => {
                                        command_obj(tokens_iter.collect(), &mut commands);
                                        *waiting_for_command = true;
                                    }
                                    None => write_output("Command not found"),
                                }
                            }
                            Err(error) => {
                                write_output(&format!("Error `{}` in input `{}`", error, &*input));
                            }
                        };

                        *input = String::new();
                        edited.request_focus();
                    }
                    ui.input(|i| {
                        if i.key_pressed(egui::Key::Escape) {
                            showing_console.0 = false;
                        }
                    });
                })
            });
        });
}

fn startup(mut commands: Commands, type_registry: Res<AppTypeRegistry>) {
    commands.insert_resource(IsOpen(false));

    let mut w = type_registry.write();
    w.add_registration(IsOpen::get_type_registration());
}

pub(super) struct Plugin;
impl bevy_app::Plugin for Plugin {
    fn build(&self, app: &mut bevy_app::App) {
        app.add_systems(bevy_app::Update, (check_open_console, do_io).chain())
            .add_systems(bevy_app::Startup, startup)
            .add_event::<Output>();
    }
}
