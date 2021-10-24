use crate::scene::commands::SceneContext;
use std::fmt::Debug;

pub mod panel;

macro_rules! define_command_stack {
    ($command_trait:ident, $command_stack:ident, $context:ty) => {
        pub trait $command_trait: Debug + Send + 'static {
            fn name(&mut self, context: &$context) -> String;
            fn execute(&mut self, context: &mut $context);
            fn revert(&mut self, context: &mut $context);
            fn finalize(&mut self, _: &mut $context) {}
        }

        pub struct $command_stack {
            commands: Vec<Box<dyn $command_trait>>,
            top: Option<usize>,
            debug: bool,
        }

        impl $command_stack {
            pub fn new(debug: bool) -> Self {
                Self {
                    commands: Default::default(),
                    top: None,
                    debug,
                }
            }

            pub fn do_command(
                &mut self,
                mut command: Box<dyn $command_trait>,
                mut context: $context,
            ) {
                if self.commands.is_empty() {
                    self.top = Some(0);
                } else {
                    // Advance top
                    match self.top.as_mut() {
                        None => self.top = Some(0),
                        Some(top) => *top += 1,
                    }
                    // Drop everything after top.
                    let top = self.top.unwrap_or(0);
                    if top < self.commands.len() {
                        for mut dropped_command in self.commands.drain(top..) {
                            if self.debug {
                                println!("Finalizing command {:?}", dropped_command);
                            }
                            dropped_command.finalize(&mut context);
                        }
                    }
                }

                if self.debug {
                    println!("Executing command {:?}", command);
                }

                command.execute(&mut context);

                self.commands.push(command);
            }

            pub fn undo(&mut self, mut context: $context) {
                if !self.commands.is_empty() {
                    if let Some(top) = self.top.as_mut() {
                        if let Some(command) = self.commands.get_mut(*top) {
                            if self.debug {
                                println!("Undo command {:?}", command);
                            }
                            command.revert(&mut context)
                        }
                        if *top == 0 {
                            self.top = None;
                        } else {
                            *top -= 1;
                        }
                    }
                }
            }

            pub fn redo(&mut self, mut context: $context) {
                if !self.commands.is_empty() {
                    let command = match self.top.as_mut() {
                        None => {
                            self.top = Some(0);
                            self.commands.first_mut()
                        }
                        Some(top) => {
                            let last = self.commands.len() - 1;
                            if *top < last {
                                *top += 1;
                                self.commands.get_mut(*top)
                            } else {
                                None
                            }
                        }
                    };

                    if let Some(command) = command {
                        if self.debug {
                            println!("Redo command {:?}", command);
                        }
                        command.execute(&mut context)
                    }
                }
            }

            pub fn clear(&mut self, mut context: $context) {
                for mut dropped_command in self.commands.drain(..) {
                    if self.debug {
                        println!("Finalizing command {:?}", dropped_command);
                    }
                    dropped_command.finalize(&mut context);
                }
            }
        }
    };
}

define_command_stack!(Command, CommandStack, SceneContext);
