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
        asset::{
            manager::ResourceManager,
            untyped::{ResourceKind, UntypedResource},
        },
        core::{log::Log, make_pretty_type_name, pool::Handle, SafeLock, Uuid},
        engine::Engine,
        gui::{
            button::{ButtonBuilder, ButtonMessage},
            grid::{Column, GridBuilder, Row},
            list_view::{ListViewBuilder, ListViewMessage},
            message::{MessageDirection, UiMessage},
            stack_panel::StackPanelBuilder,
            text::TextMessage,
            text_box::TextBoxBuilder,
            utils::make_dropdown_list_option,
            widget::{WidgetBuilder, WidgetMessage},
            window::{WindowBuilder, WindowMessage, WindowTitle},
            BuildContext, HorizontalAlignment, Orientation, Thickness, UiNode, UserInterface,
        },
    },
    message::MessageSender,
    Message,
};
use std::path::{Path, PathBuf};

pub struct ResourceCreator {
    pub window: Handle<UiNode>,
    resource_constructors_list: Handle<UiNode>,
    ok: Handle<UiNode>,
    cancel: Handle<UiNode>,
    name: Handle<UiNode>,
    selected: Option<usize>,
    supported_resource_data_uuids: Vec<Uuid>,
    name_str: String,
}

impl ResourceCreator {
    pub fn new(ctx: &mut BuildContext, resource_manager: &ResourceManager) -> Self {
        let rm_state = resource_manager.state();
        let mut constructors = rm_state.constructors_container.map.safe_lock();
        let mut items = Vec::new();
        let mut supported_resource_data_uuids = Vec::new();
        for (uuid, constructor) in constructors.iter_mut() {
            let instance = (constructor.callback)();
            if instance.can_be_saved() {
                supported_resource_data_uuids.push(*uuid);
                items.push(make_dropdown_list_option(
                    ctx,
                    make_pretty_type_name(&constructor.type_name),
                ))
            }
        }

        let name_str = String::from("unnamed_resource");
        let name;
        let ok;
        let cancel;
        let resource_constructors_list;
        let window = WindowBuilder::new(WidgetBuilder::new().with_width(300.0).with_height(400.0))
            .with_title(WindowTitle::text("Resource Creator"))
            .open(false)
            .with_content(
                GridBuilder::new(
                    WidgetBuilder::new()
                        .with_child({
                            name = TextBoxBuilder::new(
                                WidgetBuilder::new()
                                    .with_tab_index(Some(0))
                                    .on_row(0)
                                    .with_height(22.0)
                                    .with_margin(Thickness::uniform(1.0)),
                            )
                            .with_text(&name_str)
                            .build(ctx);
                            name
                        })
                        .with_child({
                            resource_constructors_list = ListViewBuilder::new(
                                WidgetBuilder::new().with_tab_index(Some(1)).on_row(1),
                            )
                            .with_items(items)
                            .build(ctx);
                            resource_constructors_list
                        })
                        .with_child(
                            StackPanelBuilder::new(
                                WidgetBuilder::new()
                                    .with_horizontal_alignment(HorizontalAlignment::Right)
                                    .on_row(2)
                                    .with_child({
                                        ok = ButtonBuilder::new(
                                            WidgetBuilder::new()
                                                .with_tab_index(Some(2))
                                                .with_enabled(false)
                                                .with_width(100.0)
                                                .with_height(22.0),
                                        )
                                        .with_text("OK")
                                        .build(ctx);
                                        ok
                                    })
                                    .with_child({
                                        cancel = ButtonBuilder::new(
                                            WidgetBuilder::new()
                                                .with_tab_index(Some(3))
                                                .with_width(100.0)
                                                .with_height(22.0),
                                        )
                                        .with_text("Cancel")
                                        .build(ctx);
                                        cancel
                                    }),
                            )
                            .with_orientation(Orientation::Horizontal)
                            .build(ctx),
                        ),
                )
                .add_row(Row::auto())
                .add_row(Row::stretch())
                .add_row(Row::auto())
                .add_column(Column::stretch())
                .build(ctx),
            )
            .build(ctx);

        Self {
            window,
            resource_constructors_list,
            ok,
            cancel,
            name,
            selected: None,
            name_str,
            supported_resource_data_uuids,
        }
    }

    pub fn open(&self, ui: &UserInterface) {
        ui.send(
            self.window,
            WindowMessage::OpenModal {
                center: true,
                focus_content: true,
            },
        );
    }

    #[must_use]
    pub fn handle_ui_message(
        &mut self,
        message: &UiMessage,
        engine: &mut Engine,
        sender: MessageSender,
        base_path: &Path,
    ) -> bool {
        let mut asset_added = false;

        if let Some(ListViewMessage::SelectionChanged(selection)) = message.data() {
            if message.destination() == self.resource_constructors_list
                && message.direction() == MessageDirection::FromWidget
            {
                self.selected = selection.first().cloned();
                engine
                    .user_interfaces
                    .first()
                    .send(self.ok, WidgetMessage::Enabled(true));

                // Propose extension for the resource.
                let resource_manager_state = engine.resource_manager.state();
                if let Some(data_type_uuid) = self
                    .supported_resource_data_uuids
                    .get(self.selected.unwrap_or_default())
                {
                    let loaders = resource_manager_state.loaders.safe_lock();
                    if let Some(loader) = loaders
                        .iter()
                        .find(|loader| &loader.data_type_uuid() == data_type_uuid)
                    {
                        if let Some(first) = loader.extensions().first() {
                            let mut path = PathBuf::from(&self.name_str);
                            path.set_extension(first);

                            self.name_str = path.to_string_lossy().to_string();

                            engine
                                .user_interfaces
                                .first()
                                .send(self.name, TextMessage::Text(self.name_str.clone()));
                        }
                    };
                }
            }
        } else if let Some(ButtonMessage::Click) = message.data() {
            if message.destination() == self.ok {
                let resource_manager_state = engine.resource_manager.state();
                let mut constructors = resource_manager_state
                    .constructors_container
                    .map
                    .safe_lock();

                if let Some(mut instance) = self
                    .supported_resource_data_uuids
                    .get(self.selected.unwrap_or_default())
                    .and_then(|uuid| constructors.get_mut(uuid))
                    .map(|c| c.create_instance())
                {
                    let path = base_path.join(&self.name_str);
                    match instance.save(&path) {
                        Ok(_) => {
                            let resource = UntypedResource::new_ok_untyped(
                                Uuid::new_v4(),
                                ResourceKind::External,
                                instance,
                            );

                            drop(constructors);
                            drop(resource_manager_state);

                            Log::verify(engine.resource_manager.register(resource, path));

                            sender.send(Message::ForceSync);

                            asset_added = true;
                        }
                        Err(e) => Log::err(format!("Unable to create a resource. Reason: {e:?}")),
                    }
                }
            }

            if message.destination() == self.ok || message.destination() == self.cancel {
                engine
                    .user_interfaces
                    .first()
                    .send(self.window, WindowMessage::Close);
            }
        } else if let Some(TextMessage::Text(text)) = message.data() {
            if message.destination() == self.name
                && message.direction() == MessageDirection::FromWidget
            {
                self.name_str.clone_from(text);
            }
        }

        asset_added
    }
}
