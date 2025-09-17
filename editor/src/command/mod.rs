// Copyright (c) 2019-present Dmitry Stepanov and Fyrox Engine contributors.
//
// Permission is hereby granted, free of charge, to any person obtaining a copy
// of this software and associated documentation files (the "Software"), to deal
// in the Software without restriction, including without limitation the rights
// to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
// copies of the Software, and to permit persons to whom the Software is
// furnished to do so, subject to the following conditions:
//
// The above copyright notice and this permission notice shall be included in all
// copies or substantial portions of the Software.
//
// THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
// IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
// FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
// AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
// LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
// OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
// SOFTWARE.

use crate::fyrox::{
    core::{
        err,
        reflect::{
            is_path_to_array_element, Reflect, ResolvePath, SetFieldByPathError, SetFieldError,
        },
        some_or_return, ComponentProvider,
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
            .ok()
            .and_then(|c| c.downcast_ref())
    }

    pub fn component_mut<T>(&mut self) -> Option<&mut T>
    where
        T: 'static,
    {
        self.query_component_mut(TypeId::of::<T>())
            .ok()
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

/// An object that can be added to the editors [`CommandStack`] so the user
/// can execute it and revert it.
pub trait CommandTrait: Debug + 'static {
    /// Returns `true` if the command does significant actions, that should be saved into a file
    /// (pretty much any command that changes some scene data is significant). Otherwise, returns
    /// `false` (for example, selection change is insignificant, because this command does not
    /// really modify anything in the scene, only editor's runtime data).
    fn is_significant(&self) -> bool {
        true
    }
    /// The name that the user should see in the command stack.
    fn name(&mut self, context: &dyn CommandContext) -> String;
    /// Perform the operation that this object represents.
    /// This happens when the object is first added to the command stack,
    /// and when the object is redone after being undone.
    fn execute(&mut self, context: &mut dyn CommandContext);
    /// Undo the consequences of calling [`CommandTrait::execute`].
    fn revert(&mut self, context: &mut dyn CommandContext);
    /// This object is leaving the command stack, so it will never
    /// be executed or reverted again.
    fn finalize(&mut self, _: &mut dyn CommandContext) {}
}

/// An untyped command for the editor to execute or revert.
#[derive(Debug)]
pub struct Command(pub Box<dyn CommandTrait>);

impl Command {
    /// Create a command from the given `CommandTrait` object.
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

/// A list of commands to execute in order as a single command.
/// The commands are reverted in reverse order.
/// Use [`CommandGroup::with_custom_name`] to give the command a
/// name. Otherwise, a name is automatically constructed by listing
/// the names of the commands in the group.
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
    /// Add an object of the `CommandTriat` to the group.
    pub fn push<C: CommandTrait>(&mut self, command: C) {
        self.commands.push(Command::new(command))
    }

    /// Add a `Command` to the group.
    pub fn push_command(&mut self, command: Command) {
        self.commands.push(command)
    }

    /// Replace the automatically constructed name.
    pub fn with_custom_name<S: AsRef<str>>(mut self, name: S) -> Self {
        self.custom_name = name.as_ref().to_string();
        self
    }

    /// True if this group contains no commands.
    pub fn is_empty(&self) -> bool {
        self.commands.is_empty()
    }

    /// The number of commands in the group.
    pub fn len(&self) -> usize {
        self.commands.len()
    }
}

impl CommandTrait for CommandGroup {
    fn is_significant(&self) -> bool {
        self.commands.iter().any(|c| c.is_significant())
    }

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
                        println!("Finalizing command {dropped_command:?}");
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
            println!("Executing command {command:?}");
        }

        command.execute(context);

        self.commands.push(command);
    }

    pub fn top_command(&self) -> Option<&dyn CommandTrait> {
        self.top
            .and_then(|top| self.commands.get(top))
            .map(|v| &**v)
    }

    pub fn undo(&mut self, context: &mut dyn CommandContext) {
        if !self.commands.is_empty() {
            if let Some(top) = self.top.as_mut() {
                if let Some(command) = self.commands.get_mut(*top) {
                    if self.debug {
                        println!("Undo command {command:?}");
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
                    println!("Redo command {command:?}");
                }
                command.execute(context)
            }
        }
    }

    pub fn clear(&mut self, context: &mut dyn CommandContext) {
        for mut dropped_command in self.commands.drain(..) {
            if self.debug {
                println!("Finalizing command {dropped_command:?}");
            }
            dropped_command.finalize(context);
        }
    }
}

pub trait EntityGetter:
    FnMut(&mut dyn CommandContext) -> Option<&mut dyn Reflect> + 'static
{
}
impl<F> EntityGetter for F where
    F: 'static + FnMut(&mut dyn CommandContext) -> Option<&mut dyn Reflect>
{
}

pub fn make_command(
    property_changed: &PropertyChanged,
    entity_getter: impl EntityGetter,
) -> Option<Command> {
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
        Err(e) => {
            err!("There is no such property {path}! Reason: {e:?}")
        }
    })
}

pub struct SetPropertyCommand<F: EntityGetter> {
    value: Option<Box<dyn Reflect>>,
    path: String,
    entity_getter: F,
}

impl<F: EntityGetter> Debug for SetPropertyCommand<F> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "SetPropertyCommand")
    }
}

impl<F: EntityGetter> SetPropertyCommand<F> {
    pub fn new(path: String, value: Box<dyn Reflect>, entity_getter: F) -> Self {
        Self {
            value: Some(value),
            path,
            entity_getter,
        }
    }

    fn swap(&mut self, ctx: &mut dyn CommandContext) {
        if is_path_to_array_element(&self.path) {
            let entity = some_or_return!((self.entity_getter)(ctx));
            entity.resolve_path_mut(&self.path, &mut |result| match result {
                Err(reason) => {
                    err!(
                        "Failed to set property {}! Invalid path {:?}!",
                        self.path,
                        reason
                    );
                }
                Ok(property) => match property.set(self.value.take().unwrap()) {
                    Ok(old_value) => {
                        self.value = Some(old_value);
                    }
                    Err(current_value) => {
                        err!(
                            "Failed to set property {}! Incompatible types. \
                            Target property: {}. Value: {}!",
                            self.path,
                            property.type_name(),
                            current_value.type_name()
                        );
                        self.value = Some(current_value);
                    }
                },
            });
        } else {
            let entity = some_or_return!((self.entity_getter)(ctx));
            entity.set_field_by_path(&self.path, self.value.take().unwrap(), &mut |result| {
                match result {
                    Ok(old_value) => {
                        self.value = Some(old_value);
                    }
                    Err(result) => {
                        let value = match result {
                            SetFieldByPathError::InvalidPath { value, reason } => {
                                err!(
                                    "Failed to set property {}! Invalid path {:?}!",
                                    self.path,
                                    reason
                                );

                                value
                            }
                            SetFieldByPathError::InvalidValue {
                                field_type_name,
                                value,
                            } => {
                                err!(
                                    "Failed to set property {}! Incompatible types. \
                                    Target property: {}. Value: {}!",
                                    self.path,
                                    field_type_name,
                                    value.type_name()
                                );

                                value
                            }
                            SetFieldByPathError::SetFieldError(err) => match err {
                                SetFieldError::NoSuchField { value, .. } => {
                                    err!(
                                        "Failed to set property {}, because it does not exist!",
                                        self.path,
                                    );

                                    value
                                }
                                SetFieldError::InvalidValue {
                                    field_type_name,
                                    value,
                                } => {
                                    err!(
                                        "Failed to set property {}! Incompatible types. \
                                    Target property: {}. Value: {}!",
                                        self.path,
                                        field_type_name,
                                        value.type_name()
                                    );

                                    value
                                }
                            },
                        };
                        self.value = Some(value);
                    }
                }
            });
        }
    }
}

impl<F: EntityGetter> CommandTrait for SetPropertyCommand<F> {
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

pub struct AddCollectionItemCommand<F: EntityGetter> {
    path: String,
    item: Option<Box<dyn Reflect>>,
    entity_getter: F,
}

impl<F: EntityGetter> Debug for AddCollectionItemCommand<F> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "AddCollectionItemCommand")
    }
}

impl<F: EntityGetter> AddCollectionItemCommand<F> {
    pub fn new(path: String, item: Box<dyn Reflect>, entity_getter: F) -> Self {
        Self {
            path,
            item: Some(item),
            entity_getter,
        }
    }
}

impl<F: EntityGetter> CommandTrait for AddCollectionItemCommand<F> {
    fn name(&mut self, _: &dyn CommandContext) -> String {
        format!("Add item to {} collection", self.path)
    }

    fn execute(&mut self, ctx: &mut dyn CommandContext) {
        let entity = some_or_return!((self.entity_getter)(ctx));
        try_modify_property(entity, &self.path, |field| {
            field.as_list_mut(&mut |result| {
                if let Some(list) = result {
                    if let Err(item) = list.reflect_push(self.item.take().unwrap()) {
                        err!(
                            "Failed to push item to {} collection. Type mismatch {} and {}!",
                            self.path,
                            item.type_name(),
                            list.type_name()
                        );
                        self.item = Some(item);
                    }
                } else {
                    err!("Property {} is not a collection!", self.path)
                }
            });
        })
    }

    fn revert(&mut self, ctx: &mut dyn CommandContext) {
        let entity = some_or_return!((self.entity_getter)(ctx));
        try_modify_property(entity, &self.path, |field| {
            field.as_list_mut(&mut |result| {
                if let Some(list) = result {
                    if let Some(item) = list.reflect_pop() {
                        self.item = Some(item);
                    } else {
                        err!("Failed to pop item from {} collection!", self.path)
                    }
                } else {
                    err!("Property {} is not a collection!", self.path)
                }
            });
        })
    }
}

pub struct RemoveCollectionItemCommand<F: EntityGetter> {
    path: String,
    index: usize,
    value: Option<Box<dyn Reflect>>,
    entity_getter: F,
}

impl<F: EntityGetter> Debug for RemoveCollectionItemCommand<F> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "RemoveCollectionItemCommand")
    }
}

impl<F: EntityGetter> RemoveCollectionItemCommand<F> {
    pub fn new(path: String, index: usize, entity_getter: F) -> Self {
        Self {
            path,
            index,
            value: None,
            entity_getter,
        }
    }
}

impl<F: EntityGetter> CommandTrait for RemoveCollectionItemCommand<F> {
    fn name(&mut self, _: &dyn CommandContext) -> String {
        format!("Remove collection {} item {}", self.path, self.index)
    }

    fn execute(&mut self, ctx: &mut dyn CommandContext) {
        let entity = some_or_return!((self.entity_getter)(ctx));
        try_modify_property(entity, &self.path, |field| {
            field.as_list_mut(&mut |result| {
                if let Some(list) = result {
                    self.value = list.reflect_remove(self.index);
                } else {
                    err!("Property {} is not a collection!", self.path)
                }
            })
        })
    }

    fn revert(&mut self, ctx: &mut dyn CommandContext) {
        let entity = some_or_return!((self.entity_getter)(ctx));
        try_modify_property(entity, &self.path, |field| {
            field.as_list_mut(&mut |result| {
                if let Some(list) = result {
                    if let Err(item) = list.reflect_insert(self.index, self.value.take().unwrap()) {
                        self.value = Some(item);
                        err!(
                            "Failed to insert item to {} collection. Type mismatch!",
                            self.path
                        )
                    }
                } else {
                    err!("Property {} is not a collection!", self.path)
                }
            });
        })
    }
}
