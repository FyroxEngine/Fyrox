use std::fmt::Debug;

pub mod panel;

pub trait Command<'a> {
    type Context;

    fn name(&mut self, context: &Self::Context) -> String;
    fn execute(&mut self, context: &mut Self::Context);
    fn revert(&mut self, context: &mut Self::Context);
    fn finalize(&mut self, _: &mut Self::Context) {}
}

pub struct CommandStack<C> {
    commands: Vec<C>,
    top: Option<usize>,
    debug: bool,
}

impl<C> CommandStack<C> {
    pub fn new(debug: bool) -> Self {
        Self {
            commands: Default::default(),
            top: None,
            debug,
        }
    }

    pub fn do_command<'a, Ctx>(&mut self, mut command: C, mut context: Ctx)
    where
        C: Command<'a, Context = Ctx> + Debug,
    {
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

    pub fn undo<'a, Ctx>(&mut self, mut context: Ctx)
    where
        C: Command<'a, Context = Ctx> + Debug,
    {
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

    pub fn redo<'a, Ctx>(&mut self, mut context: Ctx)
    where
        C: Command<'a, Context = Ctx> + Debug,
    {
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

    pub fn clear<'a, Ctx>(&mut self, mut context: Ctx)
    where
        C: Command<'a, Context = Ctx> + Debug,
    {
        for mut dropped_command in self.commands.drain(..) {
            if self.debug {
                println!("Finalizing command {:?}", dropped_command);
            }
            dropped_command.finalize(&mut context);
        }
    }
}
