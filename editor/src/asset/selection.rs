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

use crate::command::{CommandContext, CommandTrait};
use crate::{
    command::{make_command, Command, SetPropertyCommand},
    fyrox::{
        asset::{manager::ResourceManager, options::BaseImportOptions, ResourceData},
        core::{futures::executor::block_on, reflect::Reflect, SafeLock},
        engine::Engine,
        gui::inspector::PropertyChanged,
        scene::SceneContainer,
    },
    message::MessageSender,
    scene::{controller::SceneController, SelectionContainer},
};
use fyrox::asset::untyped::UntypedResource;
use fyrox::core::log::Log;
use std::{
    cell::{RefCell, RefMut},
    path::{Path, PathBuf},
    rc::Rc,
};

#[derive(Clone, Debug)]
struct SelectedResource {
    path: PathBuf,
    import_options: Option<Rc<RefCell<Box<dyn BaseImportOptions>>>>,
}

impl PartialEq for SelectedResource {
    fn eq(&self, other: &Self) -> bool {
        self.path == other.path
    }
}

#[derive(Debug)]
struct SaveResourceCommand {
    resource: UntypedResource,
    path: PathBuf,
}

impl SaveResourceCommand {
    fn save(&self) {
        let mut guard = self.resource.lock();
        if let Some(data) = guard.state.data_mut() {
            Log::verify(data.save(&self.path));
        }
    }
}

impl CommandTrait for SaveResourceCommand {
    fn name(&mut self, _context: &dyn CommandContext) -> String {
        "Save Resource".to_string()
    }

    fn execute(&mut self, _context: &mut dyn CommandContext) {
        self.save();
    }

    fn revert(&mut self, _context: &mut dyn CommandContext) {
        self.save();
    }
}

#[derive(Clone, Debug)]
pub struct AssetSelection {
    resources: Vec<SelectedResource>,
    resource_manager: ResourceManager,
}

impl PartialEq for AssetSelection {
    fn eq(&self, other: &Self) -> bool {
        self.resources == other.resources
    }
}

fn load_import_options_or_default(
    resource_path: &Path,
    resource_manager: &ResourceManager,
) -> Option<Box<dyn BaseImportOptions>> {
    if let Some(extension) = resource_path.extension() {
        let rm_state = resource_manager.state();
        let loaders = rm_state.loaders.safe_lock();
        for loader in loaders.iter() {
            if loader.supports_extension(&extension.to_string_lossy()) {
                return if let Some(import_options) = block_on(loader.try_load_import_settings(
                    resource_path.to_owned(),
                    rm_state.resource_io.clone(),
                )) {
                    Some(import_options)
                } else {
                    loader.default_import_options()
                };
            }
        }
    }
    None
}

impl AssetSelection {
    // TODO: Add multi-selection support.
    pub fn new(path: PathBuf, resource_manager: &ResourceManager) -> Self {
        Self {
            resources: vec![SelectedResource {
                import_options: load_import_options_or_default(&path, resource_manager)
                    .map(|opt| Rc::new(RefCell::new(opt))),
                path,
            }],
            resource_manager: resource_manager.clone(),
        }
    }

    pub fn selected_path(&self) -> Option<&Path> {
        self.resources
            .first()
            .map(|r| r.path.as_path())
            .filter(|p| !p.as_os_str().is_empty())
    }

    pub fn selected_import_options(&self) -> Option<RefMut<Box<dyn BaseImportOptions>>> {
        self.resources
            .first()
            .and_then(|r| r.import_options.as_ref())
            .map(|opt| opt.borrow_mut())
    }

    pub fn contains(&self, path: &Path) -> bool {
        self.resources.iter().any(|r| r.path.as_path() == path)
    }
}

impl SelectionContainer for AssetSelection {
    fn len(&self) -> usize {
        self.resources.len()
    }

    fn first_selected_entity(
        &self,
        _controller: &dyn SceneController,
        _scenes: &SceneContainer,
        callback: &mut dyn FnMut(&dyn Reflect, bool),
    ) {
        if let Some(resource) = self.resources.first() {
            if let Some(options) = resource.import_options.as_ref() {
                let options = options.borrow();
                callback(&**options as &dyn Reflect, false)
            } else if resource.path.is_file() {
                if let Ok(resource) =
                    block_on(self.resource_manager.request_untyped(&resource.path))
                {
                    if !self.resource_manager.is_built_in_resource(&resource) {
                        let guard = resource.lock();
                        if let Some(data) = guard.state.data_ref() {
                            callback(&*data.0 as &dyn Reflect, false)
                        }
                    }
                }
            }
        }
    }

    fn on_property_changed(
        &mut self,
        _controller: &mut dyn SceneController,
        args: &PropertyChanged,
        _engine: &mut Engine,
        sender: &MessageSender,
    ) {
        let mut group = Vec::new();

        for selected_resource in self.resources.iter() {
            if let Some(import_options) = selected_resource.import_options.as_ref().cloned() {
                if let Some(command) = make_command(args, move |_| {
                    let mut options = import_options.borrow_mut();
                    let options = &mut **options as &mut dyn Reflect;
                    // SAFETY: This is safe, because the closure owns its own copy of
                    // import_options, and the entity getter is used only once per
                    // do/undo/redo calls.
                    unsafe {
                        Some(std::mem::transmute::<
                            &'_ mut dyn Reflect,
                            &'static mut dyn Reflect,
                        >(options))
                    }
                }) {
                    group.push(command);
                }
            } else if let Ok(resource) = block_on(
                self.resource_manager
                    .request_untyped(&selected_resource.path),
            ) {
                let resource2 = resource.clone();
                if !self.resource_manager.is_built_in_resource(&resource) {
                    if let Some(command) = make_command(args, move |_| {
                        let mut guard = resource.lock();
                        let data = &mut **guard.state.data_mut()?;
                        // SAFETY: This is safe, because the closure owns its own copy of
                        // resource strong ref, and the entity getter is used only once per
                        // do/undo/redo calls.
                        unsafe {
                            Some(std::mem::transmute::<
                                &'_ mut dyn ResourceData,
                                &'static mut dyn ResourceData,
                            >(data))
                        }
                    }) {
                        group.push(command);
                    }
                    group.push(Command::new(SaveResourceCommand {
                        resource: resource2,
                        path: selected_resource.path.clone(),
                    }));
                }
            }
        }

        sender.do_command_group_with_inheritance(group, args);
    }

    fn paste_property(&mut self, path: &str, value: &dyn Reflect, sender: &MessageSender) {
        let group = self
            .resources
            .iter()
            .filter_map(|selected_resource| {
                let value = value.try_clone_box()?;

                if let Some(import_options) = selected_resource.import_options.as_ref().cloned() {
                    return Some(Command::new(SetPropertyCommand::new(
                        path.to_string(),
                        value,
                        move |_| {
                            let mut options = import_options.borrow_mut();
                            let options = &mut **options as &mut dyn Reflect;
                            // SAFETY: This is safe, because the closure owns its own copy of
                            // import_options, and the entity getter is used only once per
                            // do/undo/redo calls.
                            unsafe {
                                Some(std::mem::transmute::<
                                    &'_ mut dyn Reflect,
                                    &'static mut dyn Reflect,
                                >(options))
                            }
                        },
                    )));
                } else if let Ok(resource) = block_on(
                    self.resource_manager
                        .request_untyped(&selected_resource.path),
                ) {
                    if !self.resource_manager.is_built_in_resource(&resource) {
                        return Some(Command::new(SetPropertyCommand::new(
                            path.to_string(),
                            value,
                            move |_| {
                                let mut guard = resource.lock();
                                let data = &mut **guard.state.data_mut()?;
                                // SAFETY: This is safe, because the closure owns its own copy of
                                // resource strong ref, and the entity getter is used only once per
                                // do/undo/redo calls.
                                unsafe {
                                    Some(std::mem::transmute::<
                                        &'_ mut dyn ResourceData,
                                        &'static mut dyn ResourceData,
                                    >(data))
                                }
                            },
                        )));
                    }
                }
                None
            })
            .collect::<Vec<_>>();

        sender.do_command_group(group);
    }

    fn provide_docs(&self, _controller: &dyn SceneController, _engine: &Engine) -> Option<String> {
        None
    }
}
