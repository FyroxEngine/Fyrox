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

use crate::fyrox::graph::BaseSceneGraph;
use crate::fyrox::{
    asset::{manager::ResourceManager, options::BaseImportOptions},
    core::{append_extension, futures::executor::block_on, log::Log, pool::Handle},
    engine::Engine,
    gui::{
        button::{ButtonBuilder, ButtonMessage},
        grid::{Column, GridBuilder, Row},
        inspector::{
            Inspector, InspectorBuilder, InspectorContext, InspectorMessage, PropertyAction,
        },
        message::{MessageDirection, UiMessage},
        scroll_viewer::ScrollViewerBuilder,
        stack_panel::StackPanelBuilder,
        widget::WidgetBuilder,
        BuildContext, HorizontalAlignment, Orientation, Thickness, UiNode, UserInterface,
    },
};
use crate::plugins::inspector::editors::make_property_editors_container;
use crate::{message::MessageSender, MSG_SYNC_FLAG};
use fyrox::gui::inspector::InspectorContextArgs;
use std::ffi::OsStr;
use std::path::{Path, PathBuf};
use std::sync::Arc;

struct Context {
    resource_path: PathBuf,
    import_options: Box<dyn BaseImportOptions>,
}

pub struct AssetInspector {
    pub container: Handle<UiNode>,
    inspector: Handle<UiNode>,
    apply: Handle<UiNode>,
    revert: Handle<UiNode>,
    context: Option<Context>,
}

impl AssetInspector {
    pub fn new(ctx: &mut BuildContext, row: usize, column: usize) -> Self {
        let inspector;
        let apply;
        let revert;
        let container = GridBuilder::new(
            WidgetBuilder::new()
                .on_row(row)
                .on_column(column)
                .with_child(
                    ScrollViewerBuilder::new(WidgetBuilder::new().on_row(0).on_column(0))
                        .with_content({
                            inspector = InspectorBuilder::new(WidgetBuilder::new()).build(ctx);
                            inspector
                        })
                        .build(ctx),
                )
                .with_child(
                    StackPanelBuilder::new(
                        WidgetBuilder::new()
                            .on_row(1)
                            .on_column(0)
                            .with_horizontal_alignment(HorizontalAlignment::Right)
                            .with_child({
                                apply = ButtonBuilder::new(
                                    WidgetBuilder::new()
                                        .with_width(100.0)
                                        .with_margin(Thickness::uniform(1.0)),
                                )
                                .with_text("Apply")
                                .build(ctx);
                                apply
                            })
                            .with_child({
                                revert = ButtonBuilder::new(
                                    WidgetBuilder::new()
                                        .with_width(100.0)
                                        .with_margin(Thickness::uniform(1.0)),
                                )
                                .with_text("Revert")
                                .build(ctx);
                                revert
                            }),
                    )
                    .with_orientation(Orientation::Horizontal)
                    .build(ctx),
                ),
        )
        .add_row(Row::stretch())
        .add_row(Row::strict(25.0))
        .add_column(Column::stretch())
        .build(ctx);

        Self {
            container,
            inspector,
            apply,
            revert,
            context: None,
        }
    }

    pub fn inspect_resource_import_options(
        &mut self,
        path: &Path,
        ui: &mut UserInterface,
        sender: MessageSender,
        resource_manager: &ResourceManager,
    ) {
        if let Some(import_options) = load_import_options_or_default(path, resource_manager) {
            import_options.as_reflect(&mut |reflect| {
                let context = InspectorContext::from_object(InspectorContextArgs {
                    object: reflect,
                    ctx: &mut ui.build_ctx(),
                    definition_container: Arc::new(make_property_editors_container(
                        sender.clone(),
                        resource_manager.clone(),
                    )),
                    environment: None,
                    sync_flag: MSG_SYNC_FLAG,
                    layer_index: 0,
                    generate_property_string_values: true,
                    filter: Default::default(),
                    name_column_width: 150.0,
                    base_path: Default::default(),
                });
                ui.send_message(InspectorMessage::context(
                    self.inspector,
                    MessageDirection::ToWidget,
                    context,
                ));
            });

            self.context = Some(Context {
                import_options,
                resource_path: path.to_owned(),
            });
        }
    }

    pub fn handle_ui_message(&mut self, message: &UiMessage, engine: &mut Engine) {
        if let Some(context) = self.context.as_mut() {
            if let Some(extension) = context.resource_path.extension() {
                let default_import_options =
                    default_import_options(extension, &engine.resource_manager);

                if let Some(ButtonMessage::Click) = message.data() {
                    if message.destination() == self.revert {
                        if let Some(default_import_options) = default_import_options {
                            context.import_options = default_import_options;

                            context.import_options.as_reflect(&mut |reflect| {
                                let inspector_context = engine
                                    .user_interfaces
                                    .first_mut()
                                    .node(self.inspector)
                                    .cast::<Inspector>()
                                    .expect("Must be inspector")
                                    .context()
                                    .clone();
                                inspector_context
                                    .sync(
                                        reflect,
                                        engine.user_interfaces.first_mut(),
                                        0,
                                        true,
                                        Default::default(),
                                        Default::default(),
                                    )
                                    .unwrap();
                            });
                        }
                    } else if message.destination() == self.apply {
                        context
                            .import_options
                            .save(&append_extension(&context.resource_path, "options"));

                        if let Ok(resource) = block_on(
                            engine
                                .resource_manager
                                .request_untyped(&context.resource_path),
                        ) {
                            engine.resource_manager.state().reload_resource(resource);
                        }
                    }
                } else if let Some(InspectorMessage::PropertyChanged(property_changed)) =
                    message.data()
                {
                    if message.destination == self.inspector {
                        context.import_options.as_reflect_mut(&mut |reflect| {
                            PropertyAction::from_field_kind(&property_changed.value).apply(
                                &property_changed.path(),
                                reflect,
                                &mut |result| {
                                    Log::verify(result);
                                },
                            );
                        });
                    }
                }
            }
        }
    }
}

fn default_import_options(
    extension: &OsStr,
    resource_manager: &ResourceManager,
) -> Option<Box<dyn BaseImportOptions>> {
    let rm_state = resource_manager.state();
    let loaders = rm_state.loaders.lock();
    for loader in loaders.iter() {
        if loader.supports_extension(&extension.to_string_lossy()) {
            return loader.default_import_options();
        }
    }
    None
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
