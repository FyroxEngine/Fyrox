use crate::fyrox::{
    core::{
        reflect::{is_path_to_array_element, Reflect, ResolvePath, SetFieldByPathError},
        ComponentProvider,
    },
    gui::inspector::{PropertyAction, PropertyChanged},
};
use std::{
    any::{type_name, TypeId},
    fmt::{Debug, Formatter},
    ops::{Deref, DerefMut, RangeBounds},
};

pub mod panel;

pub trait CommandContext: ComponentProvider {}

impl dyn CommandContext + '_ {
    pub fn component_ref<T>(&self) -> Option<&T>
    where
        T: 'static,
    {
        self.query_component_ref(TypeId::of::<T>())
            .and_then(|c| c.downcast_ref())
    }

    pub fn component_mut<T>(&mut self) -> Option<&mut T>
    where
        T: 'static,
    {
        self.query_component_mut(TypeId::of::<T>())
            .and_then(|c| c.downcast_mut())
    }

    pub fn get<T>(&self) -> &T
    where
        T: 'static,
    {
        self.component_ref().unwrap_or_else(|| {
            panic!(
                "Unable to downcast command context to {} type",
                type_name::<T>()
            )
        })
    }

    pub fn get_mut<T>(&mut self) -> &mut T
    where
        T: 'static,
    {
        self.component_mut().unwrap_or_else(|| {
            panic!(
                "Unable to downcast command context to {} type",
                type_name::<T>()
            )
        })
    }
}

pub trait CommandTrait: Debug + 'static {
    fn name(&mut self, context: &dyn CommandContext) -> String;
    fn execute(&mut self, context: &mut dyn CommandContext);
    fn revert(&mut self, context: &mut dyn CommandContext);
    fn finalize(&mut self, _: &mut dyn CommandContext) {}
}

#[derive(Debug)]
pub struct Command(pub Box<dyn CommandTrait>);

impl Command {
    pub fn new<C: CommandTrait>(cmd: C) -> Self {
        Self(Box::new(cmd))
    }
}

impl Deref for Command {
    type Target = dyn CommandTrait;

    fn deref(&self) -> &Self::Target {
        &*self.0
    }
}

impl DerefMut for Command {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut *self.0
    }
}

#[derive(Debug, Default)]
pub struct CommandGroup {
    commands: Vec<Command>,
    custom_name: String,
}

impl From<Vec<Command>> for CommandGroup {
    fn from(commands: Vec<Command>) -> Self {
        Self {
            commands,
            custom_name: Default::default(),
        }
    }
}

impl CommandGroup {
    pub fn push<C: CommandTrait>(&mut self, command: C) {
        self.commands.push(Command::new(command))
    }

    pub fn with_custom_name<S: AsRef<str>>(mut self, name: S) -> Self {
        self.custom_name = name.as_ref().to_string();
        self
    }

    pub fn is_empty(&self) -> bool {
        self.commands.is_empty()
    }

    pub fn len(&self) -> usize {
        self.commands.len()
    }
}

impl CommandTrait for CommandGroup {
    fn name(&mut self, context: &dyn CommandContext) -> String {
        if self.custom_name.is_empty() {
            let mut name = String::from("Command group: ");
            for cmd in self.commands.iter_mut() {
                name.push_str(&cmd.name(context));
                name.push_str(", ");
            }
            name
        } else {
            self.custom_name.clone()
        }
    }

    fn execute(&mut self, context: &mut dyn CommandContext) {
        for cmd in self.commands.iter_mut() {
            cmd.execute(context);
        }
    }

    fn revert(&mut self, context: &mut dyn CommandContext) {
        // revert must be done in reverse order.
        for cmd in self.commands.iter_mut().rev() {
            cmd.revert(context);
        }
    }

    fn finalize(&mut self, context: &mut dyn CommandContext) {
        for mut cmd in self.commands.drain(..) {
            cmd.finalize(context);
        }
    }
}

pub struct CommandStack {
    pub commands: Vec<Command>,
    pub top: Option<usize>,
    max_capacity: usize,
    debug: bool,
}

impl CommandStack {
    pub fn new(debug: bool, max_capacity: usize) -> Self {
        Self {
            commands: Default::default(),
            top: None,
            max_capacity,
            debug,
        }
    }

    pub fn do_command(&mut self, mut command: Command, context: &mut dyn CommandContext) {
        if self.commands.is_empty() {
            self.top = Some(0);
        } else {
            // Advance top
            match self.top.as_mut() {
                None => self.top = Some(0),
                Some(top) => *top += 1,
            }

            fn drain<R: RangeBounds<usize>>(
                commands: &mut Vec<Command>,
                range: R,
                context: &mut dyn CommandContext,
                debug: bool,
            ) {
                for mut dropped_command in commands.drain(range) {
                    if debug {
                        println!("Finalizing command {:?}", dropped_command);
                    }
                    dropped_command.finalize(context);
                }
            }

            // Drop everything after top.
            let top = self.top.unwrap_or(0);
            if top < self.commands.len() {
                drain(&mut self.commands, top.., context, self.debug);
            }

            // Drop everything after limit.
            if self.commands.len() >= self.max_capacity {
                let range = 0..(self.commands.len() - self.max_capacity);
                drain(&mut self.commands, range, context, self.debug);
                if let Some(top) = self.top.as_mut() {
                    if *top > self.commands.len() {
                        *top = self.commands.len();
                    }
                }
            }
        }

        if self.debug {
            println!("Executing command {:?}", command);
        }

        command.execute(context);

        self.commands.push(command);
    }

    pub fn undo(&mut self, context: &mut dyn CommandContext) {
        if !self.commands.is_empty() {
            if let Some(top) = self.top.as_mut() {
                if let Some(command) = self.commands.get_mut(*top) {
                    if self.debug {
                        println!("Undo command {:?}", command);
                    }
                    command.revert(context)
                }
                if *top == 0 {
                    self.top = None;
                } else {
                    *top -= 1;
                }
            }
        }
    }

    pub fn redo(&mut self, context: &mut dyn CommandContext) {
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
                command.execute(context)
            }
        }
    }

    pub fn clear(&mut self, context: &mut dyn CommandContext) {
        for mut dropped_command in self.commands.drain(..) {
            if self.debug {
                println!("Finalizing command {:?}", dropped_command);
            }
            dropped_command.finalize(context);
        }
    }
}

pub fn make_command<F>(property_changed: &PropertyChanged, entity_getter: F) -> Option<Command>
where
    F: 'static + FnMut(&mut dyn CommandContext) -> &mut dyn Reflect,
{
    match PropertyAction::from_field_kind(&property_changed.value) {
        PropertyAction::Modify { value } => Some(Command::new(SetPropertyCommand::new(
            property_changed.path(),
            value,
            entity_getter,
        ))),
        PropertyAction::AddItem { value } => Some(Command::new(AddCollectionItemCommand::new(
            property_changed.path(),
            value,
            entity_getter,
        ))),
        PropertyAction::RemoveItem { index } => Some(Command::new(
            RemoveCollectionItemCommand::new(property_changed.path(), index, entity_getter),
        )),
        // Must be handled outside, there is not enough context and it near to impossible to create universal reversion
        // for InheritableVariable<T>.
        PropertyAction::Revert => None,
    }
}

fn try_modify_property<F>(entity: &mut dyn Reflect, path: &str, func: F)
where
    F: FnOnce(&mut dyn Reflect),
{
    let mut func = Some(func);
    entity.resolve_path_mut(path, &mut |result| match result {
        Ok(field) => func.take().unwrap()(field),
        Err(e) => fyrox::core::log::Log::err(format!(
            "There is no such property {}! Reason: {:?}",
            path, e
        )),
    })
}

pub struct SetPropertyCommand<F>
where
    F: FnMut(&mut dyn CommandContext) -> &mut dyn Reflect,
{
    value: Option<Box<dyn Reflect>>,
    path: String,
    entity_getter: F,
}

impl<F> Debug for SetPropertyCommand<F>
where
    F: FnMut(&mut dyn CommandContext) -> &mut dyn Reflect,
{
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "SetPropertyCommand")
    }
}

impl<F> SetPropertyCommand<F>
where
    F: FnMut(&mut dyn CommandContext) -> &mut dyn Reflect,
{
    pub fn new(path: String, value: Box<dyn Reflect>, entity_getter: F) -> Self {
        Self {
            value: Some(value),
            path,
            entity_getter,
        }
    }

    fn swap(&mut self, ctx: &mut dyn CommandContext) {
        if is_path_to_array_element(&self.path) {
            (self.entity_getter)(ctx).resolve_path_mut(&self.path, &mut |result| match result {
                Err(reason) => {
                    fyrox::core::log::Log::err(format!(
                        "Failed to set property {}! Invalid path {:?}!",
                        self.path, reason
                    ));
                }
                Ok(property) => match property.set(self.value.take().unwrap()) {
                    Ok(old_value) => {
                        self.value = Some(old_value);
                    }
                    Err(current_value) => {
                        fyrox::core::log::Log::err(format!(
                            "Failed to set property {}! Incompatible types {}!",
                            self.path,
                            current_value.type_name()
                        ));
                        self.value = Some(current_value);
                    }
                },
            });
        } else {
            (self.entity_getter)(ctx).set_field_by_path(
                &self.path,
                self.value.take().unwrap(),
                &mut |result| match result {
                    Ok(old_value) => {
                        self.value = Some(old_value);
                    }
                    Err(result) => {
                        let value = match result {
                            SetFieldByPathError::InvalidPath { value, reason } => {
                                fyrox::core::log::Log::err(format!(
                                    "Failed to set property {}! Invalid path {:?}!",
                                    self.path, reason
                                ));

                                value
                            }
                            SetFieldByPathError::InvalidValue(value) => {
                                fyrox::core::log::Log::err(format!(
                                    "Failed to set property {}! Incompatible types {}!",
                                    self.path,
                                    value.type_name()
                                ));

                                value
                            }
                        };
                        self.value = Some(value);
                    }
                },
            );
        }
    }
}

impl<F> CommandTrait for SetPropertyCommand<F>
where
    F: 'static + FnMut(&mut dyn CommandContext) -> &mut dyn Reflect,
{
    fn name(&mut self, _: &dyn CommandContext) -> String {
        format!("Set {} property", self.path)
    }

    fn execute(&mut self, ctx: &mut dyn CommandContext) {
        self.swap(ctx);
    }

    fn revert(&mut self, ctx: &mut dyn CommandContext) {
        self.swap(ctx);
    }
}

pub struct AddCollectionItemCommand<F>
where
    F: 'static + FnMut(&mut dyn CommandContext) -> &mut dyn Reflect,
{
    path: String,
    item: Option<Box<dyn Reflect>>,
    entity_getter: F,
}

impl<F> Debug for AddCollectionItemCommand<F>
where
    F: 'static + FnMut(&mut dyn CommandContext) -> &mut dyn Reflect,
{
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "AddCollectionItemCommand")
    }
}

impl<F> AddCollectionItemCommand<F>
where
    F: 'static + FnMut(&mut dyn CommandContext) -> &mut dyn Reflect,
{
    pub fn new(path: String, item: Box<dyn Reflect>, entity_getter: F) -> Self {
        Self {
            path,
            item: Some(item),
            entity_getter,
        }
    }
}

impl<F> CommandTrait for AddCollectionItemCommand<F>
where
    F: 'static + FnMut(&mut dyn CommandContext) -> &mut dyn Reflect,
{
    fn name(&mut self, _: &dyn CommandContext) -> String {
        format!("Add item to {} collection", self.path)
    }

    fn execute(&mut self, ctx: &mut dyn CommandContext) {
        try_modify_property((self.entity_getter)(ctx), &self.path, |field| {
            field.as_list_mut(&mut |result| {
                if let Some(list) = result {
                    if let Err(item) = list.reflect_push(self.item.take().unwrap()) {
                        fyrox::core::log::Log::err(format!(
                            "Failed to push item to {} collection. Type mismatch {} and {}!",
                            self.path,
                            item.type_name(),
                            list.type_name()
                        ));
                        self.item = Some(item);
                    }
                } else {
                    fyrox::core::log::Log::err(format!(
                        "Property {} is not a collection!",
                        self.path
                    ))
                }
            });
        })
    }

    fn revert(&mut self, ctx: &mut dyn CommandContext) {
        try_modify_property((self.entity_getter)(ctx), &self.path, |field| {
            field.as_list_mut(&mut |result| {
                if let Some(list) = result {
                    if let Some(item) = list.reflect_pop() {
                        self.item = Some(item);
                    } else {
                        fyrox::core::log::Log::err(format!(
                            "Failed to pop item from {} collection!",
                            self.path
                        ))
                    }
                } else {
                    fyrox::core::log::Log::err(format!(
                        "Property {} is not a collection!",
                        self.path
                    ))
                }
            });
        })
    }
}

pub struct RemoveCollectionItemCommand<F>
where
    F: 'static + FnMut(&mut dyn CommandContext) -> &mut dyn Reflect,
{
    path: String,
    index: usize,
    value: Option<Box<dyn Reflect>>,
    entity_getter: F,
}

impl<F> Debug for RemoveCollectionItemCommand<F>
where
    F: 'static + FnMut(&mut dyn CommandContext) -> &mut dyn Reflect,
{
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "RemoveCollectionItemCommand")
    }
}

impl<F> RemoveCollectionItemCommand<F>
where
    F: 'static + FnMut(&mut dyn CommandContext) -> &mut dyn Reflect,
{
    pub fn new(path: String, index: usize, entity_getter: F) -> Self {
        Self {
            path,
            index,
            value: None,
            entity_getter,
        }
    }
}

impl<F> CommandTrait for RemoveCollectionItemCommand<F>
where
    F: 'static + FnMut(&mut dyn CommandContext) -> &mut dyn Reflect,
{
    fn name(&mut self, _: &dyn CommandContext) -> String {
        format!("Remove collection {} item {}", self.path, self.index)
    }

    fn execute(&mut self, ctx: &mut dyn CommandContext) {
        try_modify_property((self.entity_getter)(ctx), &self.path, |field| {
            field.as_list_mut(&mut |result| {
                if let Some(list) = result {
                    self.value = list.reflect_remove(self.index);
                } else {
                    fyrox::core::log::Log::err(format!(
                        "Property {} is not a collection!",
                        self.path
                    ))
                }
            })
        })
    }

    fn revert(&mut self, ctx: &mut dyn CommandContext) {
        try_modify_property((self.entity_getter)(ctx), &self.path, |field| {
            field.as_list_mut(&mut |result| {
                if let Some(list) = result {
                    if let Err(item) = list.reflect_insert(self.index, self.value.take().unwrap()) {
                        self.value = Some(item);
                        fyrox::core::log::Log::err(format!(
                            "Failed to insert item to {} collection. Type mismatch!",
                            self.path
                        ))
                    }
                } else {
                    fyrox::core::log::Log::err(format!(
                        "Property {} is not a collection!",
                        self.path
                    ))
                }
            });
        })
    }
}
