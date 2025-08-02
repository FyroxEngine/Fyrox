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

use crate::{
    fyrox::{
        asset::{manager::ResourceManager, options::BaseImportOptions},
        core::{futures::executor::block_on, log::Log, reflect::Reflect},
        engine::Engine,
        gui::inspector::{PropertyAction, PropertyChanged},
        scene::SceneContainer,
    },
    message::MessageSender,
    scene::{controller::SceneController, SelectionContainer},
};
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
        let loaders = rm_state.loaders.lock();
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
        self.resources.first().map(|r| r.path.as_path())
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
        callback: &mut dyn FnMut(&dyn Reflect),
    ) {
        if let Some(resource) = self.resources.first() {
            if let Some(options) = resource.import_options.as_ref() {
                let options = options.borrow();
                callback(&**options as &dyn Reflect)
            } else if let Ok(resource) =
                block_on(self.resource_manager.request_untyped(&resource.path))
            {
                if !self.resource_manager.is_built_in_resource(&resource) {
                    let guard = resource.0.lock();
                    if let Some(data) = guard.state.data_ref() {
                        callback(&*data.0 as &dyn Reflect)
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
        _sender: &MessageSender,
    ) {
        for selected_resource in self.resources.iter() {
            if let Some(import_options) = selected_resource.import_options.as_ref() {
                let mut options = import_options.borrow_mut();
                let options = &mut **options as &mut dyn Reflect;
                options.as_reflect_mut(&mut |reflect| {
                    PropertyAction::from_field_kind(&args.value).apply(
                        &args.path(),
                        reflect,
                        &mut |result| {
                            Log::verify(result);
                        },
                    );
                });
            } else if let Ok(resource) = block_on(
                self.resource_manager
                    .request_untyped(&selected_resource.path),
            ) {
                if !self.resource_manager.is_built_in_resource(&resource) {
                    let mut guard = resource.0.lock();
                    if let Some(data) = guard.state.data_mut() {
                        data.0.as_reflect_mut(&mut |reflect| {
                            PropertyAction::from_field_kind(&args.value).apply(
                                &args.path(),
                                reflect,
                                &mut |result| {
                                    Log::verify(result);
                                },
                            );
                        });
                        Log::verify(data.save(&selected_resource.path));
                    }
                }
            }
        }
    }

    fn paste_property(&mut self, _path: &str, _value: &dyn Reflect, _sender: &MessageSender) {
        // TODO
    }

    fn provide_docs(&self, _controller: &dyn SceneController, _engine: &Engine) -> Option<String> {
        None
    }
}
