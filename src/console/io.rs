use crate::bevy_egui::{egui, EguiContexts};
use bevy::{prelude::*, reflect::GetTypeRegistration};

use super::parse;

/// keep track of if the console is or isn't visible
#[derive(Deref, DerefMut, Reflect, Resource)]
pub struct ConsoleOpen(pub bool);

#[derive(Deref, DerefMut, Resource, Reflect)]
pub struct CommandOutput(pub Option<String>);

const MAX_HISTORY_SIZE: usize = 500;

fn check_open_console(keys: Res<Input<KeyCode>>, mut showing_console: ResMut<ConsoleOpen>) {
    if keys.just_pressed(KeyCode::Grave) {
        trace!("showing console = true");
        *showing_console = ConsoleOpen(true);
    }
}

// should this be an exclusive system?
fn do_io(
    mut input: Local<String>,
    mut output_history: Local<String>,
    mut waiting_for_command: Local<bool>,
    mut context: EguiContexts,
    mut showing_console: ResMut<ConsoleOpen>,
    mut command_output: ResMut<CommandOutput>,
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
        output_history.push_str(&*string);
    };

    if *waiting_for_command {
        if let Some(cur_command_output) = &**command_output {
            write_output(&*cur_command_output);
            command_output.0 = None;
            *waiting_for_command = false;
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

                    // don't start another command if we're already waiting
                    if *waiting_for_command {
                        return;
                    };

                    if edited.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                        // enter was pressed: run commmand

                        match parse(&**input) {
                            Ok(tokens) => {
                                let mut tokens = tokens;
                                if tokens.len() == 0 {
                                    tokens = parse(&"echo Please enter a command").unwrap();
                                };

                                let mut tokens_iter = tokens.into_iter();

                                let command =
                                    unsafe { tokens_iter.next().unwrap_unchecked().string };

                                let command_obj = console_commands.get(&command);

                                match command_obj {
                                    Some(command_obj) => {
                                        command_obj
                                            .start_command(tokens_iter.collect(), &mut commands);
                                        *waiting_for_command = true;
                                    }
                                    None => write_output("Command not found"),
                                }
                            }
                            Err(error) => {
                                write_output(&*format!(
                                    "Error `{}` in input `{}`",
                                    error, &**input
                                ));
                            }
                        };

                        *input = "".to_string();
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
    commands.insert_resource(CommandOutput(None));
    commands.insert_resource(ConsoleOpen(false));

    let mut w = type_registry.write();
    w.add_registration(ConsoleOpen::get_type_registration());
    w.add_registration(CommandOutput::get_type_registration());
}

pub(super) struct Plugin;
impl bevy::prelude::Plugin for Plugin {
    fn build(&self, app: &mut App) {
        app.add_systems((check_open_console, do_io).chain())
            .add_startup_system(startup);
    }
}
